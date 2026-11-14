# ORDYN Effect System

**Specification:** Algebraic Effect Tracking & Analysis  
**Baseline:** v0.1.0  

---

## 1. Principles of Static Effect Tracking

In ORDYN, pure computation is the default. Any interaction with the execution environment, persistent storage, network sockets, random number generators, or mutable state across boundaries is an **Effect**.

Functions declare their effects explicitly:

```ordyn
fn read_secure_config(key: String, cap: StorageRead) -> Result<String, Error>
    with [Storage, Audit]
{
    // ...
}
```

---

## 2. Standard Effect Hierarchy

| Effect | Description | Authorized Capability Required |
|---|---|---|
| `Pure` | Zero side effects; mathematical function. | None |
| `State` | Local mutable state within current entity. | Entity ownership |
| `Storage` | Persistent disk read or write operations. | `StorageRead`, `StorageWrite` |
| `Net` | Network transmission or socket connection. | `NetConnect`, `NetListen` |
| `Audit` | Security audit log emission. | `AuditEmit` |
| `Time` | Access to wall-clock time (potential side-channel). | `ClockRead` |
| `Random` | Cryptographic random number generation. | `EntropySource` |

---

## 3. Effect Propagation & Polymorphism

If function $A$ calls function $B$, the effect set of $A$ must be a superset of the effect set of $B$:

$$\mathcal{E}(B) \subseteq \mathcal{E}(A)$$

Failure to declare an invoked effect results in a compiler error:
```text
error[E0412]: unhandled effect `Net` in pure function `calculate_hash`
  --> src/service.ordyn:14:5
   |
14 |     send_beacon(url);
   |     ^^^^^^^^^^^^^^^^ requires effect `Net`, but enclosing function declares `with []`
```
