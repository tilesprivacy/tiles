import builtins
import importlib
import io
import logging
import os
import re
import sys
import traceback
import pickle
import subprocess
import base64
import tempfile
import shutil
import threading
from pathlib import Path
from typing import Optional, Dict, Any, Tuple, Set, TYPE_CHECKING

if TYPE_CHECKING:
    from .venvstacks_manager import VenvStacksManager

SANDBOX_TIMEOUT = 10
SANDBOX_PROFILE_PATH = Path(__file__).parent / "sandbox.sb"

# Pattern for validating safe paths (alphanumerics, hyphen, underscore, dot, slash)
# Rejects sandbox-significant characters: quotes, parentheses, semicolons, newlines, backslashes
_SAFE_PATH_PATTERN = re.compile(r"^[A-Za-z0-9._/\-]+$")

# Pattern for validating session IDs (alphanumerics, hyphen, underscore only)
_SAFE_SESSION_ID_PATTERN = re.compile(r"^[A-Za-z0-9_\-]+$")

# Configure a logger for the sandbox (in real use, configure handlers/level as needed)
logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)  # or DEBUG for more verbosity


def _run_user_code(
    code: str,
    allow_installs: bool,
    allowed_path: str,
    blacklist: list,
    available_functions: dict,
    log: bool = False,
) -> tuple[dict, str]:
    """
    Execute code under sandboxed conditions (limited file access, optional installs,
    and blacklisting) and return the resulting locals and an error message.
    """
    try:
        # Optional: apply working directory and file access restriction
        if allowed_path:
            allowed = os.path.abspath(allowed_path)
            try:
                os.chdir(allowed)  # Change working dir to the allowed_path
            except Exception as e:
                # If we cannot chdir, log but continue (the open wrapper will still enforce path)
                logger.warning(
                    "Could not change working directory to %s: %s", allowed, e
                )
            # Wrap builtins.open to restrict file access
            orig_open = builtins.open

            def secure_open(file, *args, **kwargs):
                """Open that restricts file access to allowed_path."""
                # If file is a file object or path-like, get its string path
                path = (
                    file if isinstance(file, str) else getattr(file, "name", str(file))
                )
                full_path = os.path.abspath(path if path is not None else "")
                if not full_path.startswith(allowed):
                    raise PermissionError(
                        f"Access to '{full_path}' is denied by sandbox."
                    )
                return orig_open(file, *args, **kwargs)

            builtins.open = secure_open

            # Optionally, restrict other file-related functions (remove, rename, etc.) similarly
            # We'll patch a couple of common ones as an example:
            orig_remove = os.remove

            def secure_remove(path, *args, **kwargs):
                full_path = os.path.abspath(path)
                if not full_path.startswith(allowed):
                    raise PermissionError(
                        f"Removal of '{full_path}' is denied by sandbox."
                    )
                return orig_remove(path, *args, **kwargs)

            os.remove = secure_remove

            orig_rename = os.rename

            def secure_rename(src, dst, *args, **kwargs):
                full_src = os.path.abspath(src)
                full_dst = os.path.abspath(dst)
                if not full_src.startswith(allowed) or not full_dst.startswith(allowed):
                    raise PermissionError(
                        "Rename operation outside allowed path is denied by sandbox."
                    )
                return orig_rename(src, dst, *args, **kwargs)

            os.rename = secure_rename

        # Apply blacklist restrictions by removing or disabling blacklisted builtins or attributes
        if blacklist:
            for name in blacklist:
                # If the name has a dot, like "os.system", handle module attributes
                if "." in name:
                    mod_name, attr_name = name.split(".", 1)
                    try:
                        mod_obj = importlib.import_module(mod_name)
                    except ImportError:
                        mod_obj = None
                    # If module is imported in sandbox, remove the attribute
                    if mod_obj and hasattr(mod_obj, attr_name):
                        try:
                            setattr(
                                mod_obj, attr_name, None
                            )  # simple way: nullify the attribute
                        except Exception:
                            pass  # if we cannot set it, ignore (might be read-only)
                else:
                    # It's a built-in or global name; remove from builtins if present
                    if name in builtins.__dict__:
                        builtins.__dict__[name] = (
                            None  # or we could del, but setting None prevents use
                        )
            # Additionally, we can ensure __builtins__ in the exec env doesn't contain them (handled below in exec)

        # If allowed, handle package installations inside sandbox (in case code itself triggers ImportError)
        if allow_installs:
            # We will install missing imports on the fly during execution if an ImportError occurs.
            # One approach: wrap __import__ to catch failed imports and pip install.
            orig_import = builtins.__import__

            def custom_import(name, globals=None, locals=None, fromlist=(), level=0):
                try:
                    return orig_import(name, globals, locals, fromlist, level)
                except ImportError as e:
                    pkg = name.split(".")[0]
                    logger.info(
                        "Sandbox: attempting to install missing package '%s'", pkg
                    )
                    try:
                        subprocess.run(
                            [sys.executable, "-m", "pip", "install", pkg],
                            check=True,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL,
                        )
                    except Exception as inst_err:
                        # If installation fails, re-raise the original ImportError
                        logger.error(
                            "Sandbox: failed to install package %s: %s", pkg, inst_err
                        )
                        raise e
                    # Retry the import after installation
                    return orig_import(name, globals, locals, fromlist, level)

            builtins.__import__ = custom_import

        # Prepare an isolated execution namespace. We use an empty globals dict with a fresh builtins.
        exec_globals = {"__builtins__": builtins.__dict__}

        # Add any provided functions to the execution environment
        if available_functions:
            exec_globals.update(available_functions)

        exec_locals = {}  # local variables will be collected here

        error_msg = None
        try:
            exec(code, exec_globals, exec_locals)  # Execute the user's code
        except Exception as e:
            # Catch any exception and format it
            tb = traceback.format_exc()
            error_msg = f"Exception in sandboxed code:\n{tb}"
            if log:
                logger.error("Sandbox: code raised an exception: %s", e)
        except SystemExit as e:
            # Handle sys.exit calls (which raise SystemExit)
            code_val = e.code if isinstance(e.code, int) or e.code else 0
            if code_val != 0:
                error_msg = f"Sandboxed code called sys.exit({code_val})"
                if log:
                    logger.warning(
                        "Sandbox: code exited with non-zero status %s", code_val
                    )
            # For sys.exit(0), we treat it as normal termination (no error)

        # Clean up any blacklisted or internal entries in locals
        exec_locals.pop("__builtins__", None)

        # Collect only picklable locals for returning
        safe_locals = {}
        for var, val in exec_locals.items():
            try:
                pickle.dumps(val)  # test picklability
                safe_locals[var] = val
            except Exception:
                safe_locals[var] = repr(val)  # fallback: use string representation

        if log:
            logger.info("Sandbox execution finished")

        return safe_locals, error_msg

    except Exception as e:
        # Catch any unhandled exceptions in the worker process
        if log:
            logger.error(
                "Unhandled exception in sandbox worker: %s", traceback.format_exc()
            )
        return None, f"Sandbox worker error: {str(e)}"


def execute_sandboxed_code(
    code: str,
    timeout: int = SANDBOX_TIMEOUT,
    allow_installs: bool = False,
    requirements_path: str = None,
    allowed_path: str = None,
    blacklist: list = None,
    available_functions: dict = None,
    import_module: str = None,
    log: bool = False,
) -> tuple[dict, str]:
    """
    Execute the given Python code string in a sandboxed subprocess with specified restrictions.

    Parameters:
        code (str): The Python code to execute.
        timeout (int): Maximum execution time in seconds for the sandboxed code (default 10 seconds).
        allow_installs (bool): If True, allow installing missing packages via pip (default False).
        requirements_path (str): Path to a requirements.txt file to install before execution.
        allowed_path (str): Directory path that the code is allowed to access for file I/O.
                            File operations outside this path will be blocked. If None, no extra file restrictions are applied.
        blacklist (list): List of names (builtins or module attributes) that are disallowed in the code.
                          If the code uses any of these, it will be prevented or result in an error.
        available_functions (dict): Dictionary of functions to make available in the sandboxed environment.
                                   The keys are the function names, and the values are the function objects.
        import_module (str): Name of a Python module to import and make all its functions available in the sandbox.

    Returns:
        (dict, str): A tuple containing the dictionary of local variables from the executed code (or None on failure),
                     and an error message (str) if an error/exception occurred, or None if execution was successful.
    """
    # Step 1: If package installs are allowed, handle requirements and prepare environment
    if requirements_path:
        if os.path.isfile(requirements_path):
            logger.info(
                "Installing packages from requirements file: %s", requirements_path
            )
            try:
                subprocess.run(
                    [sys.executable, "-m", "pip", "install", "-r", requirements_path],
                    check=True,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
            except Exception as e:
                logger.error(
                    "Failed to install requirements from %s: %s", requirements_path, e
                )
                # If requirements fail to install, we can choose to abort or continue. Here, abort execution.
                return None, f"Failed to install requirements: {e}"
        else:
            logger.error("Requirements file %s not found.", requirements_path)
            return None, f"Requirements file not found: {requirements_path}"

    # If a module name is provided, import it and add its functions to available_functions
    if isinstance(available_functions, str) and not import_module:
        import_module = available_functions
        available_functions = None

    if import_module:
        try:
            module = importlib.import_module(import_module)
            if available_functions is None:
                available_functions = {}
            for name in dir(module):
                if not name.startswith("_"):
                    attr = getattr(module, name)
                    if callable(attr):
                        available_functions[name] = attr
        except ImportError as e:
            logger.error(f"Failed to import module {import_module}: {e}")
            return None, f"Failed to import module {import_module}: {e}"

    # Step 2: Execute the code in a separate Python subprocess
    params = {
        "code": code,
        "allow_installs": allow_installs,
        "allowed_path": allowed_path,
        "blacklist": blacklist or [],
        "available_functions": available_functions or {},
        "log": log,
    }

    env = os.environ.copy()
    env["SANDBOX_PARAMS"] = base64.b64encode(pickle.dumps(params)).decode()

    try:
        result = subprocess.run(
            [sys.executable, "-m", "mem_agent.engine"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            env=env,
        )
    except subprocess.TimeoutExpired:
        logger.error(
            "Sandboxed code exceeded time limit of %d seconds; terminating.", timeout
        )
        return None, f"TimeoutError: Code execution exceeded {timeout} seconds."

    if result.returncode != 0:
        return None, result.stderr.decode().strip()

    # print("stderr:", result.stderr.decode())
    # print("stdout:", result.stdout[:200])

    try:
        local_vars, error_msg = pickle.loads(result.stdout)
    except Exception as e:
        return None, f"Failed to decode sandbox output: {e}"

    if error_msg is None:
        error_msg = ""

    return local_vars, error_msg


def _subprocess_entry() -> None:
    """Entry point for sandbox subprocess."""
    params_b64 = os.environ.get("SANDBOX_PARAMS")
    if not params_b64:
        sys.exit(1)
    params = pickle.loads(base64.b64decode(params_b64))
    locals_dict, error = _run_user_code(
        params["code"],
        params.get("allow_installs", False),
        params.get("allowed_path"),
        params.get("blacklist", []),
        params.get("available_functions", {}),
        params.get("log", False),
    )
    sys.stdout.buffer.write(pickle.dumps((locals_dict, error)))


if __name__ == "__main__":
    _subprocess_entry()


# =============================================================================
# VenvStackExecutor: Secure, stateful code execution with isolated environments
# =============================================================================


def _get_venv_bin_dir() -> str:
    """Get the correct bin directory name for the current platform."""
    return "Scripts" if sys.platform == "win32" else "bin"


def _validate_sandbox_path(path: str) -> bool:
    """Validate a path is safe for sandbox profile substitution.

    Rejects paths containing sandbox-significant characters that could
    enable sandbox rule injection: quotes, parentheses, semicolons,
    newlines, backslashes, and other metacharacters.

    Args:
        path: The path to validate

    Returns:
        True if the path is safe, False otherwise
    """
    if not path:
        return False
    return bool(_SAFE_PATH_PATTERN.match(path))


def _sanitize_session_id(session_id: str) -> str:
    """Sanitize a session ID to prevent path traversal attacks.

    Only allows alphanumerics, hyphens, and underscores.
    Invalid characters are replaced with underscores.

    Args:
        session_id: The raw session ID

    Returns:
        Sanitized session ID safe for use in paths

    Raises:
        ValueError: If session_id is empty or None
    """
    if not session_id:
        raise ValueError("session_id cannot be empty")

    # If already safe, return as-is
    if _SAFE_SESSION_ID_PATTERN.match(session_id):
        return session_id

    # Replace unsafe characters with underscores
    sanitized = re.sub(r"[^A-Za-z0-9_\-]", "_", session_id)

    # Ensure we don't have path traversal attempts
    sanitized = sanitized.replace("..", "__")

    return sanitized


def _validate_path_containment(child_path: str, parent_path: str) -> bool:
    """Verify that child_path is contained within parent_path.

    Uses secure path normalization to prevent path traversal attacks.

    Args:
        child_path: The path that should be inside parent_path
        parent_path: The containing directory

    Returns:
        True if child_path is inside parent_path, False otherwise
    """
    try:
        child_normalized = os.path.normpath(os.path.abspath(child_path))
        parent_normalized = os.path.normpath(os.path.abspath(parent_path))
        common = os.path.commonpath([child_normalized, parent_normalized])
        return common == parent_normalized
    except (ValueError, TypeError):
        return False


# Allowed types for RestrictedUnpickler - only safe, basic types
_PICKLE_SAFE_TYPES: Set[Tuple[str, str]] = {
    ("builtins", "dict"),
    ("builtins", "list"),
    ("builtins", "tuple"),
    ("builtins", "set"),
    ("builtins", "frozenset"),
    ("builtins", "str"),
    ("builtins", "bytes"),
    ("builtins", "bytearray"),
    ("builtins", "int"),
    ("builtins", "float"),
    ("builtins", "complex"),
    ("builtins", "bool"),
    ("builtins", "type"),
    ("builtins", "NoneType"),
    # Allow common safe stdlib types
    ("datetime", "datetime"),
    ("datetime", "date"),
    ("datetime", "time"),
    ("datetime", "timedelta"),
    ("decimal", "Decimal"),
    ("fractions", "Fraction"),
    ("collections", "OrderedDict"),
    ("collections", "defaultdict"),
    ("collections", "Counter"),
}


class RestrictedUnpickler(pickle.Unpickler):
    """Restricted unpickler that only allows safe types.

    This prevents arbitrary code execution via pickle deserialization
    by only allowing a whitelist of safe, basic types.

    Security Note:
        The subprocess runs user code, so if the sandbox is bypassed,
        malicious pickle data could be returned. This unpickler limits
        the damage by preventing deserialization of dangerous types
        like code objects, functions, or classes.
    """

    def find_class(self, module: str, name: str) -> type:
        """Only allow safe types to be unpickled."""
        if (module, name) in _PICKLE_SAFE_TYPES:
            return getattr(__import__(module, fromlist=[name]), name)

        # Special case for NoneType which isn't directly importable
        if module == "builtins" and name == "NoneType":
            return type(None)

        raise pickle.UnpicklingError(f"Forbidden type in pickle data: {module}.{name}")


def _safe_pickle_loads(data: bytes) -> Any:
    """Safely deserialize pickle data using RestrictedUnpickler.

    Args:
        data: Pickle-serialized bytes

    Returns:
        The deserialized object

    Raises:
        pickle.UnpicklingError: If data contains forbidden types
    """
    return RestrictedUnpickler(io.BytesIO(data)).load()


class VenvStackExecutor:
    """Secure code executor with isolated virtual environments and Apple sandbox-exec.

    Provides:
    - Shared venvstacks-based Python environments (when available)
    - Fallback to per-session virtual environments
    - Kernel-level sandboxing via Apple's sandbox-exec (macOS)
    - Persistent state across tool calls within a session
    - Automatic package installation

    Architecture:
    - When venvstacks are built/exported, uses shared Python from the
      application layer (datascience or minimal framework)
    - When venvstacks are not available, falls back to creating a
      per-session virtual environment with python -m venv
    """

    def __init__(
        self,
        session_id: str,
        workspace_path: str,
        base_venv_path: Optional[str] = None,
        framework: str = "datascience",
    ):
        """Initialize the executor.

        Args:
            session_id: Unique identifier for this execution session
            workspace_path: Directory the code is allowed to access
            base_venv_path: Optional base path for fallback virtual environments
            framework: Which venvstacks framework to use ("datascience" or "minimal")

        Raises:
            ValueError: If session_id is invalid or paths fail validation
        """
        # Sanitize session_id to prevent path traversal
        self.session_id = _sanitize_session_id(session_id)
        self.workspace_path = os.path.abspath(workspace_path)
        self.base_venv_path = base_venv_path or os.path.join(
            tempfile.gettempdir(), "tiles_venvstacks"
        )
        self.framework = framework

        # Compute venv_path with sanitized session_id (for fallback mode)
        self.venv_path = os.path.join(self.base_venv_path, self.session_id)

        # Verify venv_path is inside base_venv_path (prevent path traversal)
        if not _validate_path_containment(self.venv_path, self.base_venv_path):
            raise ValueError(
                f"Invalid session_id: resulting venv_path escapes base directory"
            )

        # Session-specific temp directory for sandbox isolation
        # This prevents cross-session data exposure via temp files
        self.session_temp_path = os.path.join(
            tempfile.gettempdir(), "tiles_sessions", self.session_id
        )

        self._session_state: Dict[str, Any] = {}
        self._initialized = False
        self._using_venvstacks = False
        self._venvstacks_python: Optional[Path] = None
        self._venvstacks_manager: Optional["VenvStacksManager"] = None

    def _get_python_path(self) -> str:
        """Get the path to the Python executable.

        Returns venvstacks Python if available, otherwise the fallback venv Python.
        """
        # If using venvstacks, return the shared Python path
        if self._using_venvstacks and self._venvstacks_python:
            return str(self._venvstacks_python)

        # Fallback: return per-session venv Python
        bin_dir = _get_venv_bin_dir()
        python_name = "python.exe" if sys.platform == "win32" else "python3"
        return os.path.join(self.venv_path, bin_dir, python_name)

    def _get_pip_path(self) -> str:
        """Get the path to pip in the venv."""
        bin_dir = _get_venv_bin_dir()
        pip_name = "pip.exe" if sys.platform == "win32" else "pip"
        return os.path.join(self.venv_path, bin_dir, pip_name)

    def _ensure_venv(self) -> None:
        """Ensure a Python environment is available.

        Tries venvstacks first (shared, pre-built environments with data science
        packages), then falls back to creating a per-session venv if venvstacks
        are not available.
        """
        if self._initialized:
            return

        # Create session-specific temp directory
        os.makedirs(self.session_temp_path, exist_ok=True)

        # Try venvstacks first
        if self._try_init_venvstacks():
            self._initialized = True
            return

        # Fallback: create per-session venv
        logger.info(
            f"Venvstacks not available, creating per-session venv for {self.session_id}"
        )
        os.makedirs(self.venv_path, exist_ok=True)

        # Check if venv already exists
        python_path = self._get_python_path()
        if not os.path.exists(python_path):
            # Create venv using system Python
            subprocess.run(
                [sys.executable, "-m", "venv", self.venv_path],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            logger.info(f"Created fallback venv at {self.venv_path}")

        self._initialized = True

    def _try_init_venvstacks(self) -> bool:
        """Try to initialize using venvstacks.

        Returns:
            True if venvstacks are available and initialized, False otherwise
        """
        try:
            from .venvstacks_manager import VenvStacksManager, VenvStacksError
        except ImportError:
            logger.debug("venvstacks_manager not available")
            return False

        try:
            self._venvstacks_manager = VenvStacksManager()

            # Check if stacks are exported or built
            if not (
                self._venvstacks_manager.is_exported()
                or self._venvstacks_manager.is_built()
            ):
                logger.debug("Venvstacks not built or exported yet")
                return False

            # Get Python path from the appropriate application layer
            python_path = self._venvstacks_manager.get_interpreter_python(
                self.framework
            )

            if python_path and python_path.exists():
                self._venvstacks_python = python_path
                self._using_venvstacks = True
                logger.info(f"Using venvstacks Python: {python_path}")
                return True
            else:
                logger.debug(
                    f"Venvstacks Python not found for framework: {self.framework}"
                )
                return False

        except VenvStacksError as e:
            logger.warning(f"Failed to initialize venvstacks: {e}")
            return False
        except Exception as e:
            logger.debug(f"Unexpected error initializing venvstacks: {e}")
            return False

    def _get_sandbox_profile(self) -> str:
        """Generate sandbox profile with paths substituted.

        Validates paths before substitution to prevent sandbox rule injection.

        Returns:
            The sandbox profile with paths substituted, or empty string if
            the profile is not found or paths fail validation.
        """
        if not SANDBOX_PROFILE_PATH.exists():
            logger.warning("Sandbox profile not found, running without sandbox")
            return ""

        # Determine the Python environment path for sandbox rules
        # When using venvstacks, use the export directory
        # When using fallback, use the per-session venv path
        if self._using_venvstacks and self._venvstacks_manager:
            python_env_path = str(self._venvstacks_manager.export_dir)
        else:
            python_env_path = self.venv_path

        # Validate all paths before substitution to prevent sandbox bypass
        paths_to_validate = [
            ("venv_path", python_env_path),
            ("workspace_path", self.workspace_path),
            ("session_temp_path", self.session_temp_path),
        ]

        for name, path in paths_to_validate:
            if not _validate_sandbox_path(path):
                logger.warning(
                    f"{name} contains unsafe characters, running without sandbox: "
                    f"{path!r}"
                )
                return ""

        profile = SANDBOX_PROFILE_PATH.read_text()
        profile = profile.replace("${VENV_PATH}", python_env_path)
        profile = profile.replace("${WORKSPACE_PATH}", self.workspace_path)
        profile = profile.replace("${SESSION_TEMP_PATH}", self.session_temp_path)
        return profile

    def install_package(self, package: str) -> Tuple[bool, str]:
        """Install a package in the session's environment.

        Note: When using venvstacks, package installation is not supported
        because the shared environments should not be modified. Use the
        fallback venv mode for custom package installation.

        Args:
            package: Package name to install

        Returns:
            Tuple of (success, message)
        """
        self._ensure_venv()

        # Venvstacks environments are shared and should not be modified
        if self._using_venvstacks:
            return (
                False,
                f"Cannot install '{package}': using shared venvstacks environment. "
                f"Pre-installed packages: numpy, pandas, matplotlib, scikit-learn, "
                f"scipy, seaborn, requests, pillow, sympy (datascience framework) "
                f"or numpy, requests (minimal framework). "
                f"To install custom packages, rebuild without venvstacks.",
            )

        pip_path = self._get_pip_path()

        try:
            result = subprocess.run(
                [pip_path, "install", package],
                capture_output=True,
                text=True,
                timeout=120,
            )
            if result.returncode == 0:
                return True, f"Successfully installed {package}"
            else:
                return False, f"Failed to install {package}: {result.stderr}"
        except subprocess.TimeoutExpired:
            return False, f"Timeout installing {package}"
        except Exception as e:
            return False, f"Error installing {package}: {e}"

    def execute(
        self,
        code: str,
        timeout: int = SANDBOX_TIMEOUT,
        use_sandbox: bool = False,
    ) -> Tuple[Optional[Dict[str, Any]], str]:
        """Execute code in the sandboxed environment.

        Args:
            code: Python code to execute
            timeout: Maximum execution time in seconds
            use_sandbox: Whether to use Apple sandbox-exec (macOS only).
                        Defaults to False as sandbox profile may need configuration.

        Returns:
            Tuple of (locals_dict, error_message)
        """
        logger.info(f"[SANDBOX] execute() called with code:\n{code[:500]}...")

        self._ensure_venv()

        python_path = self._get_python_path()
        logger.debug(f"[SANDBOX] Using python at: {python_path}")

        # Serialize state using base64+pickle for safe transfer (avoids repr issues)
        state_b64 = base64.b64encode(pickle.dumps(self._session_state)).decode()

        # Prepare code with state restoration/saving
        wrapped_code = f'''
import pickle
import base64
import sys
import os

# Restore session state safely via pickle
_state_b64 = "{state_b64}"
_session_state = pickle.loads(base64.b64decode(_state_b64))

# Make session state available as globals
globals().update(_session_state)

# Change to workspace
os.chdir({repr(self.workspace_path)})

# Execute user code
{code}

# Capture updated state (only picklable values)
_new_state = {{}}
for k, v in dict(locals()).items():
    if not k.startswith("_"):
        try:
            pickle.dumps(v)
            _new_state[k] = v
        except Exception:
            pass

# Output state
sys.stdout.buffer.write(pickle.dumps((_new_state, None)))
'''

        # Write code to temp file
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".py", delete=False, dir=self.workspace_path
        ) as f:
            f.write(wrapped_code)
            temp_script = f.name

        logger.debug(f"[SANDBOX] Wrote wrapped code to: {temp_script}")

        profile_path: Optional[str] = None
        try:
            # Build command
            cmd = [python_path, temp_script]

            # Use sandbox-exec on macOS if available and enabled
            if use_sandbox and sys.platform == "darwin":
                profile = self._get_sandbox_profile()
                if profile:
                    # Write profile to temp file
                    with tempfile.NamedTemporaryFile(
                        mode="w", suffix=".sb", delete=False
                    ) as pf:
                        pf.write(profile)
                        profile_path = pf.name
                    cmd = ["sandbox-exec", "-f", profile_path] + cmd
                    logger.debug(f"[SANDBOX] Using sandbox profile: {profile_path}")

            logger.info(f"[SANDBOX] Executing command: {' '.join(cmd)}")

            # Execute
            result = subprocess.run(
                cmd,
                capture_output=True,
                timeout=timeout,
                cwd=self.workspace_path,
            )

            logger.debug(f"[SANDBOX] Return code: {result.returncode}")
            logger.debug(f"[SANDBOX] Stderr: {result.stderr.decode()[:500]}")

            if result.returncode != 0:
                error_msg = result.stderr.decode().strip()
                logger.error(f"[SANDBOX] Execution failed: {error_msg}")
                return None, error_msg

            # Parse output using RestrictedUnpickler for safety
            # Security Note: The subprocess runs user code. If the sandbox is
            # bypassed, malicious pickle data could be returned. Using
            # RestrictedUnpickler limits damage by only allowing safe types.
            try:
                new_state, error = _safe_pickle_loads(result.stdout)
                if new_state:
                    self._session_state.update(new_state)
                logger.info(
                    f"[SANDBOX] Execution success: vars={list(new_state.keys())}, error={error}"
                )
                return new_state, error or ""
            except pickle.UnpicklingError as e:
                error_msg = f"Unsafe pickle data rejected: {e}"
                logger.error(f"[SANDBOX] Security: {error_msg}")
                return None, error_msg
            except Exception as e:
                error_msg = (
                    f"Failed to parse output: {e}\nStderr: {result.stderr.decode()}"
                )
                logger.error(f"[SANDBOX] Parse error: {error_msg}")
                return None, error_msg

        except subprocess.TimeoutExpired:
            logger.error(f"[SANDBOX] Timeout after {timeout}s")
            return None, f"Execution timeout ({timeout}s)"
        except Exception as e:
            logger.exception(f"[SANDBOX] Exception during execution: {e}")
            return None, f"Execution error: {e}"
        finally:
            # Cleanup temp files
            try:
                os.unlink(temp_script)
            except OSError:
                pass
            if profile_path:
                try:
                    os.unlink(profile_path)
                except OSError:
                    pass

    def cleanup(self) -> None:
        """Clean up the session's resources.

        When using venvstacks: Only cleans up the session temp directory
        (the venvstacks Python environment is shared and should not be deleted).

        When using fallback venv: Cleans up both the per-session venv and
        the session temp directory.
        """
        # Only clean up per-session venv if we're NOT using venvstacks
        # (venvstacks environments are shared across sessions)
        if not self._using_venvstacks and os.path.exists(self.venv_path):
            try:
                shutil.rmtree(self.venv_path)
                logger.info(f"Cleaned up fallback venv at {self.venv_path}")
            except Exception as e:
                logger.warning(f"Failed to cleanup venv: {e}")

        # Always clean up session temp directory
        if os.path.exists(self.session_temp_path):
            try:
                shutil.rmtree(self.session_temp_path)
                logger.info(f"Cleaned up session temp at {self.session_temp_path}")
            except Exception as e:
                logger.warning(f"Failed to cleanup session temp: {e}")

        # Clear session state
        self._session_state.clear()

    @property
    def using_venvstacks(self) -> bool:
        """Check if this executor is using venvstacks (vs fallback venv)."""
        return self._using_venvstacks


# Session executor cache with thread-safe access
_executor_cache: Dict[str, VenvStackExecutor] = {}
_executor_cache_lock = threading.Lock()


def get_or_create_executor(session_id: str, workspace_path: str) -> VenvStackExecutor:
    """Get or create an executor for a session (thread-safe).

    Args:
        session_id: Session identifier (will be sanitized)
        workspace_path: Directory the code is allowed to access

    Returns:
        VenvStackExecutor instance for the session
    """
    # Sanitize session_id to get the key that VenvStackExecutor will use
    sanitized_id = _sanitize_session_id(session_id)

    with _executor_cache_lock:
        if sanitized_id not in _executor_cache:
            _executor_cache[sanitized_id] = VenvStackExecutor(
                session_id, workspace_path
            )
        return _executor_cache[sanitized_id]


def cleanup_executor(session_id: str) -> None:
    """Clean up and remove an executor from the cache (thread-safe).

    Args:
        session_id: Session identifier (will be sanitized)
    """
    # Sanitize session_id to match the key in cache
    sanitized_id = _sanitize_session_id(session_id)

    with _executor_cache_lock:
        if sanitized_id in _executor_cache:
            _executor_cache[sanitized_id].cleanup()
            del _executor_cache[sanitized_id]


def execute_sandboxed_venvstack(
    code: str,
    session_id: str,
    workspace_path: str,
    timeout: int = SANDBOX_TIMEOUT,
    use_sandbox: bool = False,
) -> Tuple[Optional[Dict[str, Any]], str]:
    """Execute code using VenvStackExecutor.

    This is the recommended entry point for sandboxed code execution with
    state persistence across calls.

    Args:
        code: Python code to execute
        session_id: Session identifier for state persistence
        workspace_path: Directory the code is allowed to access
        timeout: Maximum execution time
        use_sandbox: Whether to use Apple sandbox-exec (macOS only).
                    Defaults to False as sandbox profile may need configuration.

    Returns:
        Tuple of (locals_dict, error_message)
    """
    executor = get_or_create_executor(session_id, workspace_path)
    return executor.execute(code, timeout=timeout, use_sandbox=use_sandbox)
