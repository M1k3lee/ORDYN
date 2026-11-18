<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/ordyn-banner-dark.png">
    <img src="assets/ordyn-banner-light.png" alt="ORDYN - A New Foundation for Software" width="600"/>
  </picture>
</p>

<p align="center">
  <strong>A New Foundation for Software.</strong><br/>
  <em>Security first. Engineered beyond code.</em>
</p>

<p align="center">
  <a href="https://github.com/M1k3lee/ORDYN/actions"><img src="https://github.com/M1k3lee/ORDYN/workflows/CI/badge.svg" alt="CI Status" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg" alt="License" /></a>
  <a href="docs/development/IMPLEMENTATION_STATUS.md"><img src="https://img.shields.io/badge/status-active%20development-emerald.svg" alt="Status" /></a>
  <a href="https://m1k3lee.github.io/ORDYN/"><img src="https://img.shields.io/badge/Home%20Portal-Sovereign%20Computing-gold.svg" alt="Official Portal" /></a>
  <a href="https://m1k3lee.github.io/ORDYN/studio/"><img src="https://img.shields.io/badge/Live%20Studio-Web%20IDE%20%26%20Prover-purple.svg" alt="Live Studio" /></a>
  <a href="https://m1k3lee.github.io/ORDYN/arena/"><img src="https://img.shields.io/badge/CTF%20Arena-Hack%20The%20Vault-crimson.svg" alt="CTF Arena" /></a>
</p>

---

## 🏛️ Interactive Live Online Tools & Ecosystem

Experience the ORDYN ecosystem directly in your browser with zero installation:

- 🏛️ **[Official Sovereign Home & Vibe-Coding Portal](https://m1k3lee.github.io/ORDYN/)**: The flagship architectural portal featuring live WebAssembly verification, interactive tutorials for AI vibe coders and systems engineers, and the complete 6 foundational constructs breakdown.
- 🛡️ **[An Honest Threat Model: Enterprise Breach Analysis](https://m1k3lee.github.io/ORDYN/security/)**: A rigorous architectural whitepaper detailing how ORDYN defends against real-world enterprise breaches (Log4j, Equifax, supply-chain poisoning, ledger underflows), blast-radius containment, and transparent boundaries.
- 📖 **[Developer Experience Guide: Building Webapps](https://m1k3lee.github.io/ORDYN/dx/)**: An honest teardown on building webapps in ORDYN—why 60 seconds of upfront boundary declaration enables fearless refactoring and 10x more reliable AI vibe-coding.
- 💻 **[ORDYN Sovereign Studio](https://m1k3lee.github.io/ORDYN/studio/)**: An interactive in-browser IDE with live architectural DAG synthesis, SMT-LIB2 contract prover inspector, and Red-Lab exploit arena.
- ⚡ **[Hack the Sovereign Vault CTF Challenge](https://m1k3lee.github.io/ORDYN/arena/)**: An interactive zero-day security challenge where you launch real exploit payloads (Capability Forgery, Lattice Widening, Contract Underflows, SSRF Egress) directly against the WebAssembly-compiled Security Kernel.

---

## ⚡ Quickstart in 60 Seconds

### 1. Install the ORDYN Toolchain
```bash
# Clone the repository
git clone https://github.com/M1k3lee/ORDYN.git
cd ORDYN

# Install the unified ordyn CLI binary
cargo install --path crates/ordyn-cli
```

### 2. Scaffold a New Project
Choose from pre-configured production templates:
```bash
# Options: --template web | cli | ledger | minimal
ordyn new my_cloud_service --template web
cd my_cloud_service
```

### 3. Verify Security Invariants & Formal Contracts
Run static semantic checks, algebraic effect verification, and SMT contract proving:
```bash
ordyn verify main.ordyn
```

### 4. Run Inside the Sandboxed Runtime
Execute functions with strict instruction fuel budgets and non-strippable contract traps:
```bash
ordyn run main.ordyn --function get_status
```

### 5. Generate Visual Architecture Diagrams
Export an interactive Mermaid DAG showing capability delegations, boundaries, and quotas:
```bash
ordyn visual main.ordyn
# Opens an interactive HTML explorer: main.html
```

---

## 🛠️ Building Any Type of Application in ORDYN

ORDYN is engineered as a **general-purpose, sovereign computing platform**. The language provides native constructs that replace complex middleware, access controls, and firewall configurations:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        WHAT CAN YOU BUILD IN ORDYN?                    │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
    ┌───────────────────┬───────────┴───────────┬───────────────────┐
    ▼                   ▼                       ▼                   ▼
Cloud Microservices  Financial Systems      AI Agent Sandboxes  WASM Edge Services
(ORDYN Orbit)       (SMT Verified)          (Zero Ambience)     (Component Model)
```

### 1. Zero-Trust Cloud Microservices (ORDYN Orbit)
Build high-throughput web APIs and microservices where handlers are isolated inside cryptographic `boundary` sandboxes. A vulnerability in an image parser or logging library cannot leak environment variables, read unauthorized database tables, or open outbound sockets.
- **Reference:** [`examples/orbit_payment_service.ordyn`](examples/orbit_payment_service.ordyn)

### 2. High-Assurance Financial & Ledger Engines
Write financial code with mathematical certainty. Invariants (`balance >= 0`), preconditions (`amount > 0`), and postconditions (`self.balance == old(self.balance) + amount`) are either proven at compile-time by the SMT prover or enforced via non-strippable hardware/software traps.
- **Reference:** [`examples/secure_banking_service.ordyn`](examples/secure_banking_service.ordyn)

### 3. Autonomous AI Agent Execution Environments
The safest runtime for running AI-generated code or untrusted autonomous agents. Entities run with strict fuel limits (preventing infinite loops), memory ceilings, and zero ambient file or network access without explicit unforgeable capability handles.

### 4. WebAssembly Edge Components (WASI 0.2 / Component Model)
Compile ORDYN components to standard WebAssembly Component Model `.wasm` and `.wit` interfaces for deployment on Cloudflare Workers, Fastly Compute, or Wasmtime:
```bash
ordyn wit main.ordyn --output main.wit
ordyn build main.ordyn --output main.wasm
```

### 5. Secure CLI Tools & System Daemons
Build command-line utilities and system daemons with capability-gated access to environment variables (`EnvRead`), monotonic clock timers (`Clock`), and sandboxed filesystems (`StorageRead`, `StorageWrite`).

---

## 🧩 The Six Foundational Constructs

ORDYN eliminates accidental complexity by replacing fragile middleware with 6 first-class language constructs:

```ordyn
// 1. CAPABILITY: Unforgeable, typed token of authority
capability StorageWrite {
    namespace: String;
}

// 2. INTERFACE: Typed cross-boundary contract
interface VaultProtocol {
    fn deposit(amount: Int) -> Result<Int, String> with [StorageWrite];
}

// 3. ENTITY: Autonomous, stateful domain component
entity BankVault {
    let vault_id: String;
    mut balance: Int;

    // 4. CONTRACT: Mathematical pre/postconditions
    pub fn deposit(amount: Int) -> Result<Int, String>
        requires amount > 0
        ensures self.balance == old(self.balance) + amount
    {
        self.balance = self.balance + amount;
        return Ok(self.balance);
    }
}

// 5. BOUNDARY: Hardware/Software isolated execution sandbox
boundary FinancialCoreBoundary {
    grant StorageWrite { namespace: "prod/vault/v1" };

    quota {
        memory_limit: 128; // Megabytes
        fuel_limit: 1000000; // Instructions
    }

    bind BankVault;
}

// 6. FLOW: Declarative, verifiable data pipeline
flow AuditPipeline(entry: String) -> Unit with [StorageWrite] {
    // Declarative verifiable stream processing
}
```

---

## ⚖️ Why ORDYN? (Platform Comparison)

| Feature | ORDYN | Rust | Go | TypeScript / Node |
|---|:---:|:---:|:---:|:---:|
| **Zero Ambient Authority** | **Native (Kernel)** | ❌ (Ambient OS) | ❌ (Ambient OS) | ❌ (Ambient OS) |
| **Formal SMT Contract Prover** | **Built-in (Z3/LIA)** | ❌ (External Tools) | ❌ (None) | ❌ (None) |
| **Non-Strippable Contract Traps** | **Intrinsic** | Optional (`debug_assert`) | Manual | Manual |
| **Algebraic Effect System** | **Pure by default** | ❌ (No Effects) | ❌ (No Effects) | ❌ (No Effects) |
| **Architectural Boundaries** | **First-Class (`boundary`)** | ❌ (Processes/VMs) | ❌ (Processes/VMs) | ❌ (Containers) |
| **Supply-Chain Capability Diffs** | **Built-in (`ordyn add`)** | ❌ (Cargo Audits) | ❌ (None) | ❌ (NPM Audits) |
| **Instruction Fuel Governor** | **Built-in** | ❌ (None) | ❌ (None) | ❌ (None) |
| **WASM Component Model (`.wit`)**| **Native Generator** | Via External Crates | Partial | ❌ (None) |

---

## 💻 Complete CLI Reference

| Command | Usage | Description |
|---|---|---|
| `ordyn new` | `ordyn new <name> [--template <type>]` | Scaffolds a new project (`web`, `cli`, `ledger`, `minimal`) |
| `ordyn check` | `ordyn check <file.ordyn>` | Static type checking and algebraic effect analysis |
| `ordyn verify` | `ordyn verify <file.ordyn>` | Formal contract proving via SMT-LIB2 / Presburger LIA |
| `ordyn run` | `ordyn run <file.ordyn> [--function <fn>]` | Executes functions inside sandboxed fuel-governed runtime |
| `ordyn build` | `ordyn build <file.ordyn>` | Compiles Typed SSA IR to WebAssembly Core bytecode |
| `ordyn visual` | `ordyn visual <file.ordyn>` | Generates Mermaid security DAG and interactive HTML explorer |
| `ordyn wit` | `ordyn wit <file.ordyn>` | Emits standard WebAssembly Component Model `.wit` interfaces |
| `ordyn add` | `ordyn add <pkg> [--grant-cap <cap>]` | Adds dependency with zero-ambient capability constraints |
| `ordyn lock` | `ordyn lock` | Generates immutable SHA-256 cryptographic lockfile (`ordyn.lock`) |
| `ordyn publish`| `ordyn publish <file.ordyn>` | Digitally signs and packages project into `.ordyn-pkg` archive |
| `ordyn inspect`| `ordyn inspect <pkg.ordyn-pkg>` | Statically inspects package capabilities, quotas, and hashes |
| `ordyn studio` | `ordyn studio [--port 3000]` | Launches the interactive Sovereign Studio in your browser |
| `ordyn repl` | `ordyn repl` | Starts an interactive evaluation and security check session |

---

## 🏛️ Ecosystem Architecture (17 Workspace Crates)

```text
/ORDYN
├── assets/                          # Official brand identity & dark/light banners
├── demo/                            # Interactive CTF Challenge & Sovereign Studio Web App
├── studio/                          # Standalone Sovereign Studio sources
├── editors/code/                    # Official VS Code Language Extension
├── tree-sitter-ordyn/               # Official Tree-sitter Grammar (Neovim, Zed, Helix)
├── examples/                        # Canonical sample applications (.ordyn)
└── crates/
    ├── ordyn-diagnostics/           # Source byte spans & miette rich diagnostics
    ├── ordyn-ast/                   # Typed AST for all 6 foundational constructs
    ├── ordyn-lexer/                 # Zero-copy tokenizer with exact span tracking
    ├── ordyn-parser/                # Recursive descent & Pratt precedence parser
    ├── ordyn-types/                 # Bidirectional type checker & null-safety
    ├── ordyn-effects/               # Algebraic effect checker (Pure by default)
    ├── ordyn-ir/                    # Typed SSA Intermediate Representation
    ├── ordyn-guard/                 # Security Kernel & Capability Table
    ├── ordyn-prover/                # SMT-LIB2 / Presburger LIA formal contract prover
    ├── ordyn-ffi/                   # C-ABI foreign function bridge & capability gate
    ├── ordyn-orbit/                 # Zero-trust cloud microservices framework
    ├── ordyn-wit/                   # WASM Component Model WIT interface emitter
    ├── ordyn-runtime/               # Sandboxed execution engine & fuel governor
    ├── ordyn-stdlib/                # Transactional storage, clocks, and environment
    ├── ordyn-forge/                 # Package manager, capability differ, digital signatures
    ├── ordyn-lsp/                   # Language Server Protocol engine & daemon
    ├── ordyn-visual/                # Visual DAG compiler & interactive HTML explorer
    ├── ordyn-red-lab/               # Adversarial attack harness & CVE proofs (Log4j, Heartbleed)
    ├── ordyn-wasm/                  # In-browser WebAssembly compiler exports
    └── ordyn-cli/                   # Unified developer CLI binary (`ordyn`)
```

---

## 📜 License

Dual-licensed under either:
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE) or `http://www.apache.org/licenses/LICENSE-2.0`)
- **MIT license** ([LICENSE-MIT](LICENSE) or `http://opensource.org/licenses/MIT`)

at your option.

---

<sub>Initiated by Michael Wady (aka M1k3) and maintained by the ORDYN project contributors.</sub>
