# ORDYN Known Limitations & Technical Constraints

**Current Baseline:** v0.1.0-alpha  

---

## 1. Specification Baseline
- The current grammar and specification represent the foundational baseline (Milestone 1 & 2 design).
- Generics are currently specified for nominal types and functions; higher-kinded types and associated type families are deferred to subsequent RFCs.

## 2. Compilation Targets
- The initial target is 32-bit WebAssembly (`wasm32-wasip2`). 64-bit WASM (`wasm64`) and direct native LLVM/Cranelift machine code emission are planned for later optimization phases.

## 3. Formal Verification Scope
- Milestone 1 through 3 verifies contracts and invariants using static abstract interpretation and defensive runtime assertion envelopes. Full automated SMT-based theorem proving (e.g. Z3 / CVC5 integration) will be investigated in Milestone 8 (Security Validation).
