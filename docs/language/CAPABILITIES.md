# ORDYN Capability-Based Security

**Specification:** Capability Declarations, Granularity & Lifecycle  
**Baseline:** v0.1.0 (Post-Hardening Standard)  

---

## 1. Capability Paradigm

In traditional systems, permissions are identity-based (ACLs) or ambient (e.g. any process running as a given user can read any file that user owns).

ORDYN employs **Object Capabilities (OCaps)** enforced by the Security Kernel (`ordyn-guard`):
1. **Unforgeable Opaque Handles:** Inside user code, a capability is an unforgeable 64-bit handle indexed in kernel memory. Synthesizing integer handles triggers an uncatchable runtime trap (`KernelError::InvalidHandle`).
2. **Owner-Bound Authority:** Every capability handle is bound to its originating architectural boundary (`owner_boundary`). If an attacker in Component B presents Component A's capability handle, the kernel rejects the invocation with `KernelError::UnauthorizedCaller`.
3. **Monotonic Attenuation:** Authority can only decrease during attenuation; attempts to expand operations or broaden resource scopes trigger immediate compile-time or runtime rejection (`IllegalAttenuation`).
4. **Time-Limited Capabilities (TTL):** High-privilege operational capabilities can be issued with explicit expiration timestamps. Expired handles cannot be exercised (`CapabilityExpired`).
5. **Delegation Controls:** Capabilities can be marked non-delegable or bounded by a maximum delegation depth, preventing unauthorized multi-hop delegation.
6. **Cascading Revocation:** Revoking a parent capability walks the delegation DAG and atomically invalidates all descendent capabilities.

---

## 2. Declaration & Scoping

Capabilities are declared with strongly typed parameter constraints:

```ordyn
capability FileAccess {
    root_directory: String;
    read_only: Bool;
}
```

### Attenuation (Narrowing)
An entity holding a capability can attenuate (restrict) it before delegating to a child or helper:

```ordyn
let restricted_cap = cap.attenuate(read_only: true, root_directory: "/var/log/app");
```

Authority can only decrease during attenuation:
$$\text{Ops}(C_{\text{child}}) \subseteq \text{Ops}(C_{\text{parent}}) \quad \land \quad \text{Prefix}(C_{\text{child}}) \sqsubseteq \text{Prefix}(C_{\text{parent}}) \quad \land \quad \text{TTL}(C_{\text{child}}) \le \text{TTL}(C_{\text{parent}})$$

Attempts to expand scope or extend TTL beyond the parent capability are strictly rejected.

---

## 3. Linear (Single-Use) Capabilities

Linear capabilities represent single-use, consumable tokens of authority. Once authorized by the Security Kernel, the capability transitions permanently to `is_consumed = true`. Any subsequent attempt to present or reuse the handle triggers an uncatchable `CapabilityConsumed` trap and logs an adversarial security event in the cryptographic audit trail:

```ordyn
linear capability OneTimePayoutToken {
    recipient: String;
    amount: Int;
}
```

### Invariant:
$$\forall c \in \text{LinearCaps}, \quad \text{Executions}(c) \le 1$$

Any replay attack ($t_2 > t_1$) results in:
$$\text{KernelAuthorize}(c) \longrightarrow \text{Trap}(\text{CapabilityConsumed})$$

---

## 4. Contextual Boundary Auto-Injection

To maintain ergonomic, zero-boilerplate code without sacrificing security boundaries, ORDYN supports **Contextual Capability Injection**. When code inside an entity calls an operation requiring a capability granted to its enclosing `boundary`, the runtime kernel automatically resolves and binds the granted capability handle:

```ordyn
boundary StorageBoundary {
    grant StorageWrite { namespace: "prod/v1" };
    bind UserRegistry;
}

entity UserRegistry {
    // Zero-boilerplate: StorageWrite is automatically injected from StorageBoundary
    pub fn save(id: String) -> Result<Unit, String> with [StorageWrite] {
        return Ok(());
    }
}
```
