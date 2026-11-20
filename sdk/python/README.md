# ordyn-sandbox (Python SDK)

Zero-ambient-authority execution and formal SMT verification sandbox for **AI agents, LLM tool execution, and untrusted code**.

Powered by the **ORDYN Sovereign Language Engine**.

---

## ⚡ Quickstart in 10 Lines

```python
from ordyn import run_ordyn

code = """
entity Vault {
    mut balance: Int;
    pub fn withdraw(amount: Int) -> Result<Int, String>
        requires amount > 0 && amount <= self.balance
        ensures  self.balance == old(self.balance) - amount
    {
        self.balance = self.balance - amount;
        return Ok(self.balance);
    }
}
"""

result = run_ordyn(code, fuel_limit=50_000)

if result.is_valid:
    print(f"[PASS in {result.elapsed_us}µs] Contracts verified sound, zero ambient leaks!")
else:
    print(f"[TRAPPED] {result.error_message}")
```

---

## 🤖 Why AI Agents Need ORDYN

When building autonomous agents with **LangChain, CrewAI, AutoGen, or OpenAI Assistants**, running LLM-generated Python or shell code carries critical risks:
- The model can hallucinate runaway loops (`while True:`), exhausting worker CPU.
- Prompt injections can trick the LLM into accessing `os.environ` or reading AWS credentials.
- Arithmetic bugs and missing edge checks cause silent financial underflows.

### How ORDYN Protects Agent Workflows:
1. **Zero Ambient Authority**: Third-party packages and untrusted code cannot open sockets or touch disk without explicit, unforgeable capability tokens.
2. **Presburger SMT Prover**: If the LLM generates buggy contract math, the Presburger solver rejects it at compile time and provides the exact counterexample.
3. **Deterministic Fuel Governors**: Execution halts safely when instruction fuel runs out.

---

## 🛠️ LangChain Tool Integration Example

```python
from langchain.tools import tool
from ordyn import OrdynSandbox

sandbox = OrdynSandbox(fuel_limit=100_000)

@tool
def execute_ordyn_code(source_code: str) -> str:
    """Execute untrusted code in a mathematically proven, zero-ambient-authority sandbox."""
    res = sandbox.execute(source_code)
    if not res.is_valid:
        # Returns the exact SMT counterexample to the LLM so it self-corrects:
        return f"TRAPPED: {res.error_message}"
    return f"VERIFIED SOUND: {res.output} (Fuel consumed: {res.fuel_consumed})"
```

---

## 📦 Installation

```bash
pip install ordyn-sandbox
```

---

## 📄 License

Apache-2.0 OR MIT
