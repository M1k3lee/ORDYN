# Contributing to ORDYN

Thank you for contributing to the ORDYN computing ecosystem.

ORDYN is an open-source, security-first computing infrastructure project. We hold contributions to high standards of rigor, test coverage, and security design.

---

## 1. Core Operating Principles

1. **Security First:** Every pull request and proposal must analyze its security and threat model implications.
2. **Test-Driven:** No code is merged without automated unit and integration tests.
3. **Reproducibility:** Builds must be deterministic across supported architectures.
4. **Documentation:** Documentation updates are required alongside any functional changes.

---

## 2. Development Workflow

1. Discuss major language or architectural changes via an **RFC** (see `RFC_PROCESS.md`).
2. Run test suites locally before submitting PRs:
   ```bash
   cargo test --workspace
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```
3. Never bypass or suppress compiler security lints.
