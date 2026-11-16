# ORDYN Sovereign Security Challenge Demo: Hack the Sovereign Vault

An interactive live demonstration and exploit playground built to test and prove the mathematical inviolability of the **ORDYN Sovereign Security Architecture**.

---

## 🎯 The Challenge: 1,000,000 SOV Simulated Bounty (CTF Arena)

The target is the `SovereignVault` entity defined in `examples/challenge_vault.ordyn`, initialized with **1,000,000 SOV tokens** and isolated behind a strict `DefensePerimeterBoundary`.

**Winning Condition:**
Break into the vault by executing any unearned withdrawal, forging an administrative capability, bypassing formal contract invariants, or exfiltrating the private bounty flag without a sovereign grant.

---

## 🛡️ Six Interactive Exploit Vectors

Visitors can test and launch real attacks directly against the running ORDYN Security Kernel, Compiler, and Runtime:

1. **Capability Token Forgery (`ordyn-guard`):**  
   Attempts to fabricate arbitrary integer handles (e.g. `CapHandle(0xDEADBEEF)`). Intercepted by kernel table lookup; trapped with irreversible binary audit record.

2. **Monotonic Attenuation Escape (`Security Kernel`):**  
   Attempts to widen read-only permissions into `Write` and `DrainVault`. Rejected by lattice monotonicity proofs.

3. **Contract Invariant Violation (`ordyn-runtime`):**  
   Attempts negative withdrawals (`withdraw(-500000)`) or balance overflows. Intercepted by runtime formal verification contract traps with automatic transactional rollback.

4. **Ambient Network Exfiltration / SSRF (`ordyn-effects`):**  
   Attempts unauthorized TCP/HTTP network egress. Detected and blocked at compile time by algebraic effect analysis.

5. **Computational Starvation / ReDoS (`Fuel Governor`):**  
   Attempts infinite loops inside the boundary. Deterministically terminated when fuel quota is exhausted, leaving host threads unaffected.

6. **Linear Memory Sandbox Escape (`WASM Core`):**  
   Attempts out-of-bounds memory dereferencing beyond the 32MB boundary limit. Hardware-assisted fault traps execution.

---

## 💻 Live Custom Payload Compiler

The website features an embedded code playground connected to the live `ordyn-cli` binary (`ordyn check`). Users can write custom ORDYN code, definitions, or exploit payloads and receive real-time compiler diagnostics, TIR lowering output, and effect verification logs.

---

## 🚀 Running the Demo Website

Start the local server:
```bash
python demo/server.py
```

Then open your browser to:
[http://localhost:8088](http://localhost:8088)
