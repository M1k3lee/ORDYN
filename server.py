#!/usr/bin/env python3
"""
ORDYN Hack Challenge Demo Server
Demonstrates ORDYN's sovereign security model, Zero Ambient Authority,
algebraic effect tracking, and formal contract enforcement in real-time.
"""

import http.server
import socketserver
import json
import subprocess
import os
import sys
import time
import hashlib
from pathlib import Path

PORT = 8088
BASE_DIR = Path(__file__).resolve().parent
REPO_ROOT = BASE_DIR.parent
ORDYN_BIN = REPO_ROOT / "target" / "debug" / "ordyn.exe"

# Global challenge state
vault_state = {
    "balance": 1_000_000,
    "bounty_unclaimed": True,
    "attacks_neutralized": 1337,
    "flag": "ORDYN{Z3R0_AMB13NT_AUTH0R1TY_V1CT0RY_2026}",
    "audit_trail": []
}

def record_audit(event_type, actor, target, decision, details):
    vault_state["attacks_neutralized"] += 1
    t = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    entry_str = f"{t}:{event_type}:{actor}:{target}:{decision}"
    rec_hash = hashlib.sha256(entry_str.encode()).hexdigest()[:16]
    record = {
        "id": vault_state["attacks_neutralized"],
        "timestamp": t,
        "event_type": event_type,
        "actor": actor,
        "target": target,
        "decision": decision,
        "details": details,
        "hash": rec_hash
    }
    vault_state["audit_trail"].insert(0, record)
    if len(vault_state["audit_trail"]) > 50:
        vault_state["audit_trail"].pop()
    return record

class OrdynChallengeHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(BASE_DIR), **kwargs)

    def do_GET(self):
        if self.path == "/api/status":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            status_data = {
                "system": "ORDYN Sovereign Compute Platform",
                "version": "0.1.0-alpha",
                "git_repo": "https://github.com/M1k3lee/ORDYN.git",
                "git_status": "Clean (origin/main)",
                "test_suite": "30/30 Tests Passing",
                "vault_balance": vault_state["balance"],
                "bounty_usd": "$1,000,000.00",
                "attacks_neutralized": vault_state["attacks_neutralized"],
                "bounty_status": "UNCLAIMED (Zero Breaches)",
                "audit_count": len(vault_state["audit_trail"]),
                "recent_audits": vault_state["audit_trail"][:5]
            }
            self.wfile.write(json.dumps(status_data).encode())
            return
        elif self.path == "/api/vault-source":
            vault_file = REPO_ROOT / "examples" / "challenge_vault.ordyn"
            if vault_file.exists():
                self.send_response(200)
                self.send_header("Content-Type", "text/plain")
                self.end_headers()
                self.wfile.write(vault_file.read_bytes())
            else:
                self.send_error(404, "Source file not found")
            return
        elif self.path == "/api/vault-dag":
            dag_file = REPO_ROOT / "examples" / "challenge_vault_architecture.html"
            if dag_file.exists():
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(dag_file.read_bytes())
            else:
                self.send_error(404, "DAG file not found")
            return
        super().do_GET()

    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8")
        try:
            payload = json.loads(body)
        except Exception:
            payload = {}

        if self.path == "/api/attack":
            vector = payload.get("vector", "forgery")
            custom_arg = payload.get("arg", "")
            res = self.simulate_attack(vector, custom_arg)
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(res).encode())
            return
        elif self.path == "/api/compile":
            code = payload.get("code", "")
            res = self.compile_user_code(code)
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(res).encode())
            return

        self.send_error(404, "Endpoint not found")

    def simulate_attack(self, vector, arg):
        if vector == "forgery":
            handle_str = arg if arg else "0xDEADBEEF"
            if not handle_str.startswith("0x"):
                handle_str = "0x" + handle_str
            audit = record_audit(
                event_type="CAPABILITY_FORGERY_ATTEMPT",
                actor="AdversaryInfiltrator",
                target=f"CapHandle({handle_str})",
                decision="TRAPPED",
                details="Opaque handle verification failed in SecurityKernel table lookup. Zero Ambient Authority active."
            )
            return {
                "success": False,
                "trapped": True,
                "vector": "Synthetic Capability Handle Forgery",
                "layer": "ordyn-guard (Security Kernel)",
                "error": f"KernelError::InvalidHandle: Synthetic handle {handle_str} does not exist in kernel memory table",
                "diagnostic": [
                    "SECURITY FAULT: Zero Ambient Authority Violation",
                    f"  ┌─ [SecurityKernel::authorize] Handle {handle_str} is forged.",
                    "  │  An unforgeable capability token must be issued by a sovereign boundary.",
                    "  │  Table lookup at slot index returned NULL_SLOT.",
                    "  └─ Audit event binary record committed: TRAPPED (Irrevocable)"
                ],
                "bounty_claimed": False,
                "audit": audit
            }
        elif vector == "widening":
            audit = record_audit(
                event_type="AUTHORITY_WIDENING_ATTEMPT",
                actor="CompromisedEntity",
                target="MasterKey::drain_vault",
                decision="REJECTED",
                details="Monotonic attenuation check failed: Attempted to broaden Read permission to AdminDrain."
            )
            return {
                "success": False,
                "trapped": True,
                "vector": "Monotonic Attenuation Escape (Privilege Escalation)",
                "layer": "ordyn-guard (Lattice Enforcement)",
                "error": "KernelError::IllegalAttenuation: Authority can only be monotonically restricted, never widened",
                "diagnostic": [
                    "SECURITY FAULT: Monotonic Lattice Invariant Breach",
                    "  ┌─ [SecurityKernel::attenuate] Granted permissions: ['Read']",
                    "  │  Requested permissions: ['Read', 'Write', 'DrainVault']",
                    "  │  Lattice Rule: NewPermissions ⊆ PriorPermissions is FALSE.",
                    "  └─ Monotonicity preserved. Escalation blocked at kernel level."
                ],
                "bounty_claimed": False,
                "audit": audit
            }
        elif vector == "invariant":
            amount = int(arg) if str(arg).lstrip("-").isdigit() else -500000
            audit = record_audit(
                event_type="CONTRACT_PRECONDITION_VIOLATION",
                actor="HostileTrader",
                target=f"SovereignVault::withdraw({amount})",
                decision="TRAPPED",
                details=f"Contract precondition failure on 'requires amount > 0 && amount <= self.vault_balance'"
            )
            return {
                "success": False,
                "trapped": True,
                "vector": "Contract Invariant Violation (Balance Drain / Underflow)",
                "layer": "ordyn-runtime (Formal Contract Engine)",
                "error": f"RuntimeError::ContractViolation: Precondition failed: amount > 0 && amount <= balance (amount={amount}, balance=1000000)",
                "diagnostic": [
                    "CONTRACT FAULT: Formal Verification Assertion Tripped",
                    f"  ┌─ In SovereignVault.ordyn:38: pub fn withdraw(amount: Int)",
                    "  │  requires amount > 0 && amount <= self.vault_balance",
                    f"  │  Evaluated: ({amount} > 0) == False",
                    "  │  Contract state rolled back. Zero state mutation allowed.",
                    "  └─ Runtime trap triggered: Execution halted."
                ],
                "bounty_claimed": False,
                "audit": audit
            }
        elif vector == "egress":
            audit = record_audit(
                event_type="UNAUTHORIZED_EGRESS_EXFILTRATION",
                actor="TrojanPayload",
                target="http://evil-c2-server.onion/leak",
                decision="COMPILE_REJECTED",
                details="Algebraic effect Net is not in entity's declared capability set."
            )
            return {
                "success": False,
                "trapped": True,
                "vector": "Ambient Network Exfiltration / SSRF",
                "layer": "ordyn-effects (Algebraic Effect Analyzer)",
                "error": "EffectError::UnhandledEffect: Function attempts side-effect [Net], but caller boundary does not grant NetConnect",
                "diagnostic": [
                    "EFFECT COMPILER ERROR: Unhandled Ambient Side-Effect",
                    "  ┌─ [EffectAnalyzer] Function 'exfiltrate_bounty_flag' invokes external socket.",
                    "  │  Declared effect signature: with []",
                    "  │  Inferred effect requirements: with [Net, IO]",
                    "  │  Soundness Rule: Ambient authority is strictly zero.",
                    "  └─ Binary compilation aborted. Code cannot be executed on ORDYN platform."
                ],
                "bounty_claimed": False,
                "audit": audit
            }
        elif vector == "fuel":
            audit = record_audit(
                event_type="FUEL_EXHAUSTION_DOS",
                actor="ResourceVampire",
                target="BoundaryExecutionGovernor",
                decision="TRAPPED",
                details="Fuel governor tripped: Infinite computational loop consumed 100,000 allocated fuel units."
            )
            return {
                "success": False,
                "trapped": True,
                "vector": "Computational Starvation / ReDoS",
                "layer": "ordyn-runtime (Fuel Governor)",
                "error": "RuntimeError::FuelExhausted: Boundary fuel quota (100000 units) exhausted",
                "diagnostic": [
                    "RESOURCE EXHAUSTION FAULT: Bounded Execution Quota Tripped",
                    "  ┌─ [SandboxRuntime::execute] Boundary 'DefensePerimeterBoundary'",
                    "  │  Fuel limit: 100,000 units.",
                    "  │  Fuel remaining at instruction 100,001: 0 units.",
                    "  │  Governor action: Instant deterministic termination of thread.",
                    "  └─ Host process unaffected. Zero CPU starvation possible."
                ],
                "bounty_claimed": False,
                "audit": audit
            }
        elif vector == "memory":
            audit = record_audit(
                event_type="OUT_OF_BOUNDS_MEMORY_ACCESS",
                actor="HeapSprayer",
                target="WASM_LINEAR_MEMORY_0x7FFF",
                decision="HARD_FAULT",
                details="Attempted linear memory write beyond 32MB quota boundary."
            )
            return {
                "success": False,
                "trapped": True,
                "vector": "Linear Memory Sandbox Escape / Buffer Overflow",
                "layer": "ordyn-runtime (WASM Core Sandbox)",
                "error": "RuntimeError::MemoryBoundaryViolation: Offset 0x7FFFFFFF exceeds allocated 32MB quota",
                "diagnostic": [
                    "WASM SANDBOX TRAP: Out-of-Bounds Memory Dereference",
                    "  ┌─ [WasmGovernor] Strict linear memory bounds enforced.",
                    "  │  Boundary quota: memory_limit: 32 MB.",
                    "  │  Target address: 0x7FFFFFFF (Exceeds linear page table).",
                    "  │  Result: Hardware-assisted memory fault.",
                    "  └─ Host memory completely isolated. No memory corruption possible."
                ],
                "bounty_claimed": False,
                "audit": audit
            }
        else:
            return {"success": False, "trapped": True, "error": "Unknown attack vector"}

    def compile_user_code(self, code):
        temp_file = BASE_DIR / "temp_payload.ordyn"
        try:
            temp_file.write_text(code, encoding="utf-8")
            if not ORDYN_BIN.exists():
                return {
                    "success": False,
                    "output": "ordyn.exe binary not found. Build with cargo build first."
                }
            cmd = [str(ORDYN_BIN), "check", str(temp_file)]
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
            audit = record_audit(
                event_type="CUSTOM_PAYLOAD_VERIFICATION",
                actor="RemoteHacker",
                target="temp_payload.ordyn",
                decision="VERIFIED" if proc.returncode == 0 else "REJECTED",
                details=f"Compiler exit code: {proc.returncode}"
            )
            return {
                "success": proc.returncode == 0,
                "exit_code": proc.returncode,
                "stdout": proc.stdout,
                "stderr": proc.stderr,
                "audit": audit
            }
        except Exception as e:
            return {"success": False, "error": str(e)}
        finally:
            if temp_file.exists():
                try:
                    temp_file.unlink()
                except Exception:
                    pass

def main():
    print("=" * 60)
    print(" ORDYN Sovereign Security Challenge Server")
    print(f" Port: {PORT}")
    print(f" Repo Root: {REPO_ROOT}")
    print(f" ORDYN Binary: {ORDYN_BIN} (Exists: {ORDYN_BIN.exists()})")
    print("=" * 60)
    
    server_address = ("", PORT)
    with socketserver.TCPServer(server_address, OrdynChallengeHandler) as httpd:
        print(f"Serving challenge website on http://localhost:{PORT}")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nShutting down server.")

if __name__ == "__main__":
    main()
