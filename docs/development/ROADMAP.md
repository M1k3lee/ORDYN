# ORDYN Master Roadmap

**Baseline:** v0.1.0-alpha  
**Engineering Methodology:** Incremental, Test-Driven, Security-First  

---

## Milestone Schedule

| Milestone | Title | Scope & Deliverables | Status |
|---|---|---|---|
| **M1** | **Language Foundations** | Formal syntax, EBNF grammar, lexer, AST, parser, basic types, type checker, tests. | **Active / In Progress** |
| **M2** | **Core Semantics** | Six constructs (`entity`, `capability`, `contract`, `flow`, `boundary`, `interface`), effect analysis, TIR. | Planned |
| **M3** | **Security Foundations** | Security Kernel prototype, capability enforcement, sandbox boundaries, audit logging. | Planned |
| **M4** | **Compiler & Execution** | WASM/WASI code generation, Wasmtime runtime integration, structured concurrency. | Planned |
| **M5** | **Application Development** | Standard library, storage/network interfaces, transactional workflows, sample app. | Planned |
| **M6** | **Developer Tooling** | `ordyn` CLI (`new`, `build`, `test`, `verify`, `run`), package format, Forge initial design. | Planned |
| **M7** | **ORDYN Studio** | Desktop environment, visual architecture editor, language server (LSP). | Planned |
| **M8** | **Security Validation** | Red Lab adversarial test suite, compiler/runtime fuzzing, verification proofs. | Planned |
| **M9** | **Public Release** | Public release, comprehensive documentation, reproducible toolchain, community RFC. | Planned |
