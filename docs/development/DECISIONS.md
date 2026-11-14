# ORDYN Architecture Decision Records (ADRs)

---

## ADR-0001: Compiler and Runtime Implementation Language

- **Status:** Accepted
- **Date:** 2026-11-14
- **Context:**
  ORDYN requires a host language for its compiler (`ORDYN Core`), security kernel (`ORDYN Guard`), and runtime execution host (`ORDYN Runtime`). The language must provide strong memory safety guarantees, zero-cost abstractions, deterministic performance without a tracing garbage collector, first-class WebAssembly/WASI tooling, and high-performance compilation libraries.
- **Decision:**
  Use **Rust** as the primary implementation language for the ORDYN compiler, security kernel, and runtime host.
- **Consequences:**
  - *Positive:* Native memory safety, mature WASM ecosystem (`wasmtime`, `walrus`, `cranelift`), robust diagnostic libraries (`miette`, `thiserror`), pattern matching, and type safety across all compiler stages.
  - *Risk:* Steeper initial development learning curve and compile times; mitigated by structuring the project into fine-grained Cargo workspace crates.

---

## ADR-0002: Target Execution Architecture & Isolation Backend

- **Status:** Accepted
- **Date:** 2026-11-14
- **Context:**
  ORDYN requires an execution backend that supports deterministic compilation, zero-ambient authority, memory safety bounds enforcement, and sandboxing.
- **Decision:**
  Target **WebAssembly (WASM Core) / WASI Preview 2** hosted within Bytecode Alliance **Wasmtime**.
- **Consequences:**
  - *Positive:* Hardware-enforced bounds checking, shared-nothing process isolation, instruction fuel metering, platform portability across Windows, Linux, and macOS.
  - *Risk:* JIT warmup latency for short-lived micro-tasks; mitigated by WASM AOT (Ahead-of-Time) module pre-compilation.

---

## ADR-0003: Parser Architecture

- **Status:** Accepted
- **Date:** 2026-11-14
- **Context:**
  The compiler requires high diagnostic fidelity, helpful error messages with source spans, and deterministic parsing of expressions and declarations.
- **Decision:**
  Implement a custom hand-written recursive descent parser with Pratt / precedence climbing for expressions, backed by a fast zero-copy lexer.
- **Consequences:**
  - *Positive:* Complete control over syntax error recovery, span propagation, actionable compiler error hints, and zero opaque parser-generator dependencies.
  - *Negative:* Slightly higher initial code volume compared to parser generator DSLs.
