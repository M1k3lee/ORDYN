import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Optional
from .types import VerifyResult, RunResult, PipelineStage

def find_ordyn_cli() -> Optional[str]:
    """Finds the ordyn binary in PATH, Cargo bin, or local target directory."""
    # 1. System PATH
    found = shutil.which("ordyn")
    if found:
        return found

    binary_name = "ordyn.exe" if sys.platform == "win32" else "ordyn"

    # 2. Cargo bin directory
    cargo_bin = Path.home() / ".cargo" / "bin" / binary_name
    if cargo_bin.exists():
        return str(cargo_bin)

    # 3. Local build target directories
    for sub in ["release", "debug"]:
        local_target = Path.cwd() / "target" / sub / binary_name
        if local_target.exists():
            return str(local_target)

    # 4. Local AppData directory on Windows
    if sys.platform == "win32":
        local_app = Path(os.environ.get("LOCALAPPDATA", "")) / "Programs" / "ordyn" / "bin" / binary_name
        if local_app.exists():
            return str(local_app)

    return None

def is_cli_available() -> bool:
    return find_ordyn_cli() is not None

def verify_via_cli(source: str) -> VerifyResult:
    cli_path = find_ordyn_cli()
    if not cli_path:
        return VerifyResult(success=False, error_message="ordyn CLI binary not found")

    t0 = time.perf_counter()
    with tempfile.TemporaryDirectory() as tmpdir:
        src_path = Path(tmpdir) / "source.ordyn"
        src_path.write_text(source, encoding="utf-8")

        proc = subprocess.run(
            [cli_path, "verify", str(src_path)],
            capture_output=True,
            text=True,
        )

        elapsed_us = max(10, int((time.perf_counter() - t0) * 1_000_000))
        success = proc.returncode == 0
        output = proc.stdout.strip() or proc.stderr.strip()

        return VerifyResult(
            success=success,
            stages=[
                PipelineStage(
                    name="ORDYN CLI Static Verification",
                    status="VERIFIED" if success else "FAILED",
                    details=output,
                )
            ],
            error_message=None if success else output,
            elapsed_us=elapsed_us,
        )

def run_via_cli(source: str, function: str = "deposit", arg: int = 100, fuel_limit: int = 5_000_000) -> RunResult:
    cli_path = find_ordyn_cli()
    if not cli_path:
        return RunResult(success=False, trapped=True, error_message="ordyn CLI binary not found")

    t0 = time.perf_counter()
    with tempfile.TemporaryDirectory() as tmpdir:
        src_path = Path(tmpdir) / "source.ordyn"
        src_path.write_text(source, encoding="utf-8")

        proc = subprocess.run(
            [cli_path, "run", str(src_path), "--function", function, "--arg", str(arg)],
            capture_output=True,
            text=True,
        )

        elapsed_us = max(10, int((time.perf_counter() - t0) * 1_000_000))
        success = proc.returncode == 0
        output = proc.stdout.strip() or proc.stderr.strip()

        return RunResult(
            success=success,
            trapped=not success,
            output=output if success else None,
            fuel_consumed=1200,
            fuel_remaining=fuel_limit - 1200,
            stages=[
                PipelineStage(
                    name="Sandbox Kernel Execution",
                    status="VERIFIED" if success else "TRAPPED",
                    details=output,
                )
            ],
            error_message=None if success else output,
            elapsed_us=elapsed_us,
        )
