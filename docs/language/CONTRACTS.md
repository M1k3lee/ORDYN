# ORDYN Contract Verification

**Specification:** Design-by-Contract & Invariant Guarantees  
**Baseline:** v0.1.0  

---

## 1. Principles of Contract Verification

Contracts express formal invariants, preconditions, and postconditions over state and operations. They bridge static type checking and dynamic assertions.

---

## 2. Clauses

### 2.1 Preconditions (`requires`)
A precondition specifies the constraints that the caller must guarantee prior to invoking the function:
```ordyn
fn withdraw(amount: Int) -> Result<Int, Error>
    requires amount > 0 && amount <= self.balance
```

### 2.2 Postconditions (`ensures`)
A postcondition specifies the guarantees that the function provides upon successful execution. Postconditions may reference the prior value using `old(...)`:
```ordyn
fn withdraw(amount: Int) -> Result<Int, Error>
    ensures self.balance == old(self.balance) - amount
```

### 2.3 State Invariants (`invariant`)
An invariant specifies conditions that must hold true before and after every public operation on an entity:
```ordyn
contract PositiveReserves {
    invariant total_reserves >= 0;
}
```

---

## 3. Enforcement Strategy

1. **Static Analysis:** Where expressions involve constant bounds or monotonic integer ranges, the compiler's abstract interpreter attempts static proof.
2. **Defensive Runtime Wrappers:** When static proof is incomplete, non-strippable runtime assertion checks are generated at the boundary interface. Violations raise an uncatchable security trap and emit an audit event.
