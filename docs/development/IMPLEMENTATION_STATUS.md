# ORDYN Implementation Status Matrix

**Last Updated:** Phase 1 Initialization (Milestone 1 Foundations)  
**Reference Specification:** ORDYN Language Specification v0.1  

---

## 1. Feature & Component Matrix

| Subsystem | Feature | Status | Automated Test Coverage | Verification Status | Notes |
|---|---|---|---|---|---|
| **Language Specs** | EBNF Formal Grammar | Implemented | Verified via parser | Documented | `docs/language/GRAMMAR.md` |
| **Language Specs** | Type System Spec | Implemented | Specification complete | Documented | `docs/language/TYPE_SYSTEM.md` |
| **Language Specs** | Effect System Spec | Implemented | Specification complete | Documented | `docs/language/EFFECTS.md` |
| **Language Specs** | Capability Model | Implemented | Specification complete | Documented | `docs/language/CAPABILITIES.md` |
| **Language Specs** | Contract Model | Implemented | Specification complete | Documented | `docs/language/CONTRACTS.md` |
| **Compiler (Core)** | Diagnostics (`ordyn-diagnostics`) | Implemented | Integration verified | Tested | Source spans, miette diagnostics |
| **Compiler (Core)** | AST (`ordyn-ast`) | Implemented | Integration verified | Tested | All 6 constructs represented |
| **Compiler (Core)** | Lexer (`ordyn-lexer`) | Implemented | 3 unit tests passing | Tested | Zero-copy tokens, spans, keywords |
| **Compiler (Core)** | Parser (`ordyn-parser`)| Implemented | 3 unit/integration tests | Tested | Recursive descent, Pratt expressions |
| **CLI Tooling** | `ordyn` CLI (`ordyn-cli`)| Implemented | Verified on canonical example | Tested | `check`, `verify`, `new` commands |
| **Compiler (Core)** | Type Checker | Planned (M1 Phase 2)| Planned | Planned | Static type inference |
| **Compiler (Core)** | Effect Analyzer | Planned (M2) | Planned | Planned | Static effect lattice |
| **Compiler (Core)** | Information-Flow | Planned (M2) | Planned | Planned | Label lattice checking |
| **Compiler (Core)** | Contract Verifier | Planned (M2) | Planned | Planned | Invariant & pre/post checks |
| **Compiler (Core)** | TIR & Optimizer | Planned (M2) | Planned | Planned | SSA-form IR |
| **Compiler (Core)** | WASM Backend | Planned (M4) | Planned | Planned | Deterministic WASM output |
| **Security (Guard)**| Security Kernel | Planned (M3) | Planned | Planned | TCB capability management |
| **Security (Guard)**| Capability Table | Planned (M3) | Planned | Planned | Unforgeable handle directory |
| **Security (Guard)**| Audit Logger | Planned (M3) | Planned | Planned | Structured binary audit events |
| **Runtime** | Wasmtime Host | Planned (M4) | Planned | Planned | WASM sandbox isolation |
| **Runtime** | Fuel / Memory Limits | Planned (M4) | Planned | Planned | DoS prevention |

---

## 2. Test Evidence Log

- `ordyn_lexer::tests::test_lex_basic_tokens`: PASSED
- `ordyn_lexer::tests::test_lex_keywords_and_symbols`: PASSED
- `ordyn_lexer::tests::test_unterminated_string_error`: PASSED
- `ordyn_parser::tests::test_parse_entity_and_contracts`: PASSED
- `ordyn_parser::tests::test_parse_all_six_constructs`: PASSED
- `ordyn_parser::tests::language_tests::test_canonical_example_parses_completely`: PASSED
- CLI verification: `ordyn check examples/hello_boundary.ordyn` -> `Parsing succeeded: discovered 5 top-level declarations.`
