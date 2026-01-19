"""VenvStacks Manager for portable Python environment stacks.

This module provides integration with venvstacks for creating portable,
layered Python environments for code execution.

Key concepts:
- Runtime: Base Python installation
- Framework: Shared packages (numpy, pandas, etc.) reused across sessions
- Application: Per-session lightweight environments inheriting from frameworks

Usage:
    manager = VenvStacksManager()

    # One-time setup: build the stacks
    manager.build_stacks()

    # Export to a deployment location
    manager.export_stacks("/path/to/deployment")

    # Get the Python executable for a session
    python_path = manager.get_interpreter_python("datascience")
"""

import json
import logging
import os
import subprocess
import sys
from pathlib import Path
from typing import Optional, Dict, Any, Tuple, List

logger = logging.getLogger(__name__)

# Path to the venvstacks.toml configuration
VENVSTACKS_CONFIG = Path(__file__).parent / "venvstacks.toml"

# Default locations for built and exported stacks
DEFAULT_BUILD_DIR = Path(__file__).parent.parent / ".venvstacks_build"
DEFAULT_EXPORT_DIR = Path(__file__).parent.parent / ".venvstacks_export"


class VenvStacksError(Exception):
    """Base exception for venvstacks-related errors."""

    pass


class VenvStacksManager:
    """Manages venvstacks-based portable Python environments.

    This class provides a high-level interface for:
    - Building venvstacks from the configuration
    - Exporting stacks for deployment
    - Getting Python paths for code execution
    - Managing stack lifecycle
    """

    def __init__(
        self,
        config_path: Optional[Path] = None,
        build_dir: Optional[Path] = None,
        export_dir: Optional[Path] = None,
    ):
        """Initialize the VenvStacks manager.

        Args:
            config_path: Path to venvstacks.toml (defaults to bundled config)
            build_dir: Directory for built environments
            export_dir: Directory for exported/deployed environments
        """
        self.config_path = Path(config_path) if config_path else VENVSTACKS_CONFIG
        self.build_dir = Path(build_dir) if build_dir else DEFAULT_BUILD_DIR
        self.export_dir = Path(export_dir) if export_dir else DEFAULT_EXPORT_DIR

        if not self.config_path.exists():
            raise VenvStacksError(f"Config not found: {self.config_path}")

    def _run_venvstacks(
        self,
        command: List[str],
        check: bool = True,
    ) -> subprocess.CompletedProcess:
        """Run a venvstacks CLI command.

        Args:
            command: Command and arguments to pass to venvstacks
            check: Whether to raise on non-zero exit

        Returns:
            CompletedProcess result
        """
        full_cmd = [sys.executable, "-m", "venvstacks"] + command
        logger.info(f"Running: {' '.join(full_cmd)}")

        result = subprocess.run(
            full_cmd,
            capture_output=True,
            text=True,
        )

        if result.returncode != 0:
            logger.error(f"venvstacks failed: {result.stderr}")
            if check:
                raise VenvStacksError(f"venvstacks command failed: {result.stderr}")

        return result

    def lock_stacks(self) -> bool:
        """Lock the stack dependencies.

        This resolves all package versions and creates lock files.
        Should be run once when dependencies change.

        Returns:
            True if successful
        """
        logger.info("Locking venvstacks dependencies...")
        result = self._run_venvstacks(
            [
                "lock",
                str(self.config_path),
            ]
        )
        return result.returncode == 0

    def build_stacks(self, lock_first: bool = True) -> bool:
        """Build the environment stacks.

        This creates all runtime, framework, and application environments.

        Args:
            lock_first: Whether to lock dependencies before building

        Returns:
            True if successful
        """
        logger.info(f"Building venvstacks to {self.build_dir}...")

        # Ensure build directory exists
        self.build_dir.mkdir(parents=True, exist_ok=True)

        cmd = ["build", str(self.config_path)]
        if lock_first:
            cmd.append("--lock")

        result = self._run_venvstacks(cmd)
        return result.returncode == 0

    def export_stacks(
        self,
        output_dir: Optional[Path] = None,
        include: Optional[List[str]] = None,
    ) -> bool:
        """Export stacks for local deployment.

        This copies built environments to a deployment location and runs
        post-installation to configure them.

        Args:
            output_dir: Destination directory (defaults to export_dir)
            include: List of layer patterns to include (e.g., ["app-interpreter"])

        Returns:
            True if successful
        """
        dest = Path(output_dir) if output_dir else self.export_dir
        logger.info(f"Exporting venvstacks to {dest}...")

        dest.mkdir(parents=True, exist_ok=True)

        cmd = [
            "local-export",
            "--output-dir",
            str(dest),
            str(self.config_path),
        ]

        if include:
            for pattern in include:
                cmd.extend(["--include", pattern])

        result = self._run_venvstacks(cmd)
        return result.returncode == 0

    def get_stack_status(self) -> Dict[str, Any]:
        """Get the current status of all stacks.

        Returns:
            Dictionary with stack status information
        """
        result = self._run_venvstacks(
            [
                "show",
                "--json",
                str(self.config_path),
            ],
            check=False,
        )

        if result.returncode == 0 and result.stdout:
            try:
                return json.loads(result.stdout)
            except json.JSONDecodeError:
                pass

        return {"error": "Failed to get stack status"}

    def is_built(self) -> bool:
        """Check if stacks have been built.

        The venvstacks CLI writes output into a "__venvstacks__" subfolder,
        so we check for that marker directory.

        Returns:
            True if build directory exists with environments
        """
        if not self.build_dir.exists():
            return False

        # The venvstacks CLI creates a __venvstacks__ subfolder with metadata
        venvstacks_marker = self.build_dir / "__venvstacks__"
        return venvstacks_marker.exists()

    def is_exported(self) -> bool:
        """Check if stacks have been exported.

        Returns:
            True if export directory exists with environments
        """
        if not self.export_dir.exists():
            return False

        # Check for metadata directory
        metadata_dir = self.export_dir / "__venvstacks__"
        return metadata_dir.exists()

    def get_layer_python(
        self,
        layer_name: str,
        layer_type: str = "applications",
    ) -> Optional[Path]:
        """Get the Python executable path for a layer.

        Args:
            layer_name: Name of the layer (e.g., "interpreter")
            layer_type: Type of layer ("runtimes", "frameworks", "applications")

        Returns:
            Path to Python executable, or None if not found
        """
        # Check exported environments first
        if self.is_exported():
            base_dir = self.export_dir
        elif self.is_built():
            base_dir = self.build_dir
        else:
            return None

        # Construct environment path based on layer type
        if layer_type == "runtimes":
            env_name = layer_name
        elif layer_type == "frameworks":
            env_name = f"framework-{layer_name}"
        else:  # applications
            env_name = f"app-{layer_name}"

        env_path = base_dir / env_name

        if not env_path.exists():
            logger.warning(f"Layer environment not found: {env_path}")
            return None

        # Read layer config to get Python path
        layer_config = (
            env_path / "share" / "venv" / "metadata" / "venvstacks_layer.json"
        )
        if layer_config.exists():
            try:
                config = json.loads(layer_config.read_text())
                python_rel = config.get("python")
                if python_rel:
                    python_path = env_path / python_rel
                    if python_path.exists():
                        return python_path
            except (json.JSONDecodeError, KeyError) as e:
                logger.warning(f"Failed to read layer config: {e}")

        # Fallback: try standard locations
        if sys.platform == "win32":
            candidates = [
                env_path / "Scripts" / "python.exe",
                env_path / "python.exe",
            ]
        else:
            candidates = [
                env_path / "bin" / "python3",
                env_path / "bin" / "python",
            ]

        for candidate in candidates:
            if candidate.exists():
                return candidate

        return None

    def get_interpreter_python(
        self,
        framework: str = "datascience",
    ) -> Optional[Path]:
        """Get the Python path for the interpreter application.

        This is the main entry point for code execution.

        Args:
            framework: Which framework to use ("datascience" or "minimal")

        Returns:
            Path to Python executable for code interpreter
        """
        if framework == "minimal":
            return self.get_layer_python("interpreter-minimal", "applications")
        else:
            return self.get_layer_python("interpreter", "applications")

    def cleanup(self) -> None:
        """Clean up built and exported environments."""
        import shutil

        if self.build_dir.exists():
            try:
                shutil.rmtree(self.build_dir)
                logger.info(f"Cleaned up build directory: {self.build_dir}")
            except Exception as e:
                logger.warning(f"Failed to cleanup build: {e}")

        if self.export_dir.exists():
            try:
                shutil.rmtree(self.export_dir)
                logger.info(f"Cleaned up export directory: {self.export_dir}")
            except Exception as e:
                logger.warning(f"Failed to cleanup export: {e}")


# Global manager instance (lazy initialization)
_global_manager: Optional[VenvStacksManager] = None


def get_venvstacks_manager() -> VenvStacksManager:
    """Get or create the global VenvStacksManager instance."""
    global _global_manager
    if _global_manager is None:
        _global_manager = VenvStacksManager()
    return _global_manager


def ensure_stacks_ready(
    framework: str = "datascience",
    force_rebuild: bool = False,
) -> Tuple[bool, str]:
    """Ensure venvstacks are built and ready for use.

    This is the main initialization function. Call this before
    using venvstacks-based execution.

    Args:
        framework: Which framework stack to ensure ("datascience" or "minimal")
        force_rebuild: Force rebuild even if stacks exist

    Returns:
        Tuple of (success, message)
    """
    manager = get_venvstacks_manager()

    def _validate_interpreter() -> Tuple[bool, str]:
        """Validate that the interpreter exists and is usable."""
        python = manager.get_interpreter_python(framework)
        if python and python.exists():
            return True, f"Stacks ready at {manager.export_dir}"
        return False, (
            f"Interpreter not found for framework '{framework}' at {manager.export_dir}. "
            f"Expected Python at: {python}"
        )

    # Check if already exported and valid
    if manager.is_exported() and not force_rebuild:
        valid, msg = _validate_interpreter()
        if valid:
            return True, msg
        # Interpreter missing despite being exported - attempt rebuild
        logger.warning(f"Exported stacks invalid: {msg}. Attempting rebuild...")
        force_rebuild = True

    try:
        # Build stacks
        if not manager.is_built() or force_rebuild:
            logger.info("Building venvstacks (this may take a while)...")
            if not manager.build_stacks():
                return False, "Failed to build venvstacks"

        # Export for deployment
        if not manager.is_exported() or force_rebuild:
            logger.info("Exporting venvstacks for deployment...")
            if not manager.export_stacks():
                return False, "Failed to export venvstacks"

        # Validate interpreter after build/export
        valid, msg = _validate_interpreter()
        if not valid:
            return False, msg

        return True, f"Stacks ready at {manager.export_dir}"

    except VenvStacksError as e:
        return False, str(e)
    except Exception as e:
        return False, f"Unexpected error: {e}"
