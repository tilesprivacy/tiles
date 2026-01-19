"""Tests for VenvStackExecutor sandbox functionality.

Verifies:
- Isolated environment creation
- Network access denial
- File system restriction
- State persistence across executions
- VenvStacks integration
"""

import os
import sys
import tempfile
import pytest
from unittest import mock

# Add project root to path
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))

from mem_agent.engine import (
    VenvStackExecutor,
    execute_sandboxed_venvstack,
    cleanup_executor,
    _sanitize_session_id,
    _validate_sandbox_path,
    _validate_path_containment,
)


class TestVenvStackExecutor:
    """Test suite for VenvStackExecutor."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.session_id = "test_session_123"
        self.executor = VenvStackExecutor(
            session_id=self.session_id,
            workspace_path=self.temp_dir,
        )

    def teardown_method(self):
        """Clean up after tests."""
        self.executor.cleanup()
        import shutil

        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)

    def test_basic_execution(self):
        """Test basic code execution works."""
        code = "x = 1 + 1"
        result, error = self.executor.execute(code, use_sandbox=False)

        assert error == ""
        assert result is not None
        assert result.get("x") == 2

    def test_math_sqrt_execution(self):
        """Test math.sqrt execution - the most common code interpreter use case."""
        code = """import math
result = math.sqrt(123456789)
"""
        result, error = self.executor.execute(code, use_sandbox=False)

        assert error == "", f"Unexpected error: {error}"
        assert result is not None, "Result should not be None"
        assert "result" in result, f"'result' not in result dict: {result.keys()}"
        assert abs(result.get("result") - 11111.1110606) < 0.001, (
            f"Incorrect sqrt result: {result.get('result')}"
        )

    def test_import_and_compute(self):
        """Test that imports work correctly in the sandbox."""
        code = """import json
data = json.dumps({"key": "value"})
parsed = json.loads(data)
"""
        result, error = self.executor.execute(code, use_sandbox=False)

        assert error == "", f"Unexpected error: {error}"
        assert result is not None
        assert result.get("parsed") == {"key": "value"}

    def test_state_persistence(self):
        """Test that state persists across executions."""
        # First execution: set a variable
        code1 = "counter = 42"
        result1, error1 = self.executor.execute(code1, use_sandbox=False)

        assert error1 == ""
        assert result1.get("counter") == 42

        # Second execution: use the variable
        code2 = "result = counter * 2"
        result2, error2 = self.executor.execute(code2, use_sandbox=False)

        assert error2 == ""
        assert result2.get("result") == 84

    def test_file_creation_in_workspace(self):
        """Test that files can be created in the workspace."""
        code = """
with open("test_file.txt", "w") as f:
    f.write("Hello, sandbox!")
file_created = True
"""
        result, error = self.executor.execute(code, use_sandbox=False)

        assert error == ""
        assert result.get("file_created") is True
        assert os.path.exists(os.path.join(self.temp_dir, "test_file.txt"))

    def test_file_read_in_workspace(self):
        """Test that files can be read from the workspace."""
        # Create a file first
        test_file = os.path.join(self.temp_dir, "readable.txt")
        with open(test_file, "w") as f:
            f.write("test content")

        code = """
with open("readable.txt", "r") as f:
    content = f.read()
"""
        result, error = self.executor.execute(code, use_sandbox=False)

        assert error == ""
        assert result.get("content") == "test content"

    def test_timeout_handling(self):
        """Test that execution timeout is enforced."""
        code = """
import time
time.sleep(30)  # Should timeout
"""
        result, error = self.executor.execute(code, timeout=2, use_sandbox=False)

        assert result is None
        assert "timeout" in error.lower()

    @pytest.mark.skipif(sys.platform != "darwin", reason="sandbox-exec is macOS only")
    def test_sandbox_network_denied(self):
        """Test that network access is denied in sandbox mode."""
        code = """
import urllib.request
try:
    urllib.request.urlopen("https://google.com", timeout=5)
    network_allowed = True
except Exception:
    network_allowed = False
"""
        result, error = self.executor.execute(code, use_sandbox=True)

        # Either the sandbox blocks it or we get an error
        if result is not None:
            assert result.get("network_allowed") is False
        else:
            # Sandbox may have blocked the entire execution
            pass

    @pytest.mark.skipif(sys.platform != "darwin", reason="sandbox-exec is macOS only")
    def test_sandbox_file_access_restricted(self):
        """Test that file access outside workspace is denied."""
        code = """
try:
    with open("/etc/passwd", "r") as f:
        etc_passwd = f.read()
    file_access_denied = False
except Exception:
    file_access_denied = True
"""
        result, error = self.executor.execute(code, use_sandbox=True)

        if result is not None:
            assert result.get("file_access_denied") is True


class TestExecuteSandboxedVenvstack:
    """Test the convenience function."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()

    def teardown_method(self):
        """Clean up after tests."""
        import shutil

        # Clean up executor from cache
        cleanup_executor("convenience_test")
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)

    def test_convenience_function(self):
        """Test execute_sandboxed_venvstack works."""
        result, error = execute_sandboxed_venvstack(
            code="y = 5 * 5",
            session_id="convenience_test",
            workspace_path=self.temp_dir,
            use_sandbox=False,
        )

        assert error == ""
        assert result is not None
        assert result.get("y") == 25


if __name__ == "__main__":
    pytest.main([__file__, "-v"])


class TestSecurityHelpers:
    """Test security helper functions."""

    def test_sanitize_session_id_valid(self):
        """Test that valid session IDs pass through unchanged."""
        assert _sanitize_session_id("abc123") == "abc123"
        assert _sanitize_session_id("test_session") == "test_session"
        assert _sanitize_session_id("session-123") == "session-123"

    def test_sanitize_session_id_invalid_chars(self):
        """Test that invalid characters are replaced."""
        assert _sanitize_session_id("abc/123") == "abc_123"
        assert _sanitize_session_id("session..id") == "session__id"
        assert _sanitize_session_id("test@user") == "test_user"

    def test_sanitize_session_id_empty(self):
        """Test that empty session IDs raise ValueError."""
        with pytest.raises(ValueError):
            _sanitize_session_id("")

    def test_validate_sandbox_path_valid(self):
        """Test valid paths pass validation."""
        assert _validate_sandbox_path("/tmp/test") is True
        assert _validate_sandbox_path("/Users/test/workspace") is True
        assert _validate_sandbox_path("/var/folders/abc-123") is True

    def test_validate_sandbox_path_invalid(self):
        """Test paths with dangerous characters fail validation."""
        assert _validate_sandbox_path("/tmp/test; rm -rf /") is False
        assert _validate_sandbox_path("/tmp/test\ninjection") is False
        assert _validate_sandbox_path("/tmp/test'quote") is False
        assert _validate_sandbox_path("") is False

    def test_validate_path_containment_valid(self):
        """Test valid contained paths."""
        assert _validate_path_containment("/tmp/parent/child", "/tmp/parent") is True
        assert _validate_path_containment("/tmp/parent", "/tmp/parent") is True

    def test_validate_path_containment_invalid(self):
        """Test path traversal is blocked."""
        assert _validate_path_containment("/tmp/other", "/tmp/parent") is False
        assert (
            _validate_path_containment("/tmp/parent/../other", "/tmp/parent") is False
        )


class TestVenvStacksIntegration:
    """Test VenvStacks integration."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()

    def teardown_method(self):
        """Clean up after tests."""
        import shutil

        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)

    def test_executor_falls_back_when_no_venvstacks(self):
        """Test that executor falls back to per-session venv when venvstacks not available."""
        executor = VenvStackExecutor(
            session_id="test_fallback",
            workspace_path=self.temp_dir,
        )

        # Execute to trigger initialization
        result, error = executor.execute("x = 1", use_sandbox=False)

        # Should work (using fallback venv)
        assert error == ""
        assert result is not None
        assert result.get("x") == 1

        # Check that we're NOT using venvstacks (since they're not built)
        assert executor.using_venvstacks is False

        executor.cleanup()

    def test_executor_framework_parameter(self):
        """Test that framework parameter is stored correctly."""
        executor = VenvStackExecutor(
            session_id="test_framework",
            workspace_path=self.temp_dir,
            framework="minimal",
        )

        assert executor.framework == "minimal"
        executor.cleanup()

    def test_install_package_denied_with_venvstacks(self):
        """Test that package installation is denied when using venvstacks."""
        executor = VenvStackExecutor(
            session_id="test_install_denied",
            workspace_path=self.temp_dir,
        )

        # Mock venvstacks being available
        executor._using_venvstacks = True
        executor._initialized = True

        success, message = executor.install_package("some-package")

        assert success is False
        assert "Cannot install" in message
        assert "shared venvstacks" in message

        executor.cleanup()


class TestVenvStacksManager:
    """Test VenvStacksManager class."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()

    def teardown_method(self):
        """Clean up after tests."""
        import shutil

        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)

    def test_manager_creation(self):
        """Test VenvStacksManager can be created."""
        try:
            from mem_agent.venvstacks_manager import (
                VenvStacksManager,
                VENVSTACKS_CONFIG,
            )

            # Only test if config exists
            if VENVSTACKS_CONFIG.exists():
                manager = VenvStacksManager()
                assert manager.config_path == VENVSTACKS_CONFIG
        except ImportError:
            pytest.skip("venvstacks_manager not available")

    def test_manager_not_built_initially(self):
        """Test that is_built returns False when nothing is built."""
        try:
            from mem_agent.venvstacks_manager import (
                VenvStacksManager,
                VENVSTACKS_CONFIG,
            )
            from pathlib import Path

            if not VENVSTACKS_CONFIG.exists():
                pytest.skip("venvstacks.toml not found")

            manager = VenvStacksManager(
                build_dir=Path(self.temp_dir) / "build",
                export_dir=Path(self.temp_dir) / "export",
            )

            assert manager.is_built() is False
            assert manager.is_exported() is False
        except ImportError:
            pytest.skip("venvstacks_manager not available")
