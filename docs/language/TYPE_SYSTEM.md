# ORDYN Type System

**Specification:** Formal Typing Rules & Invariants  
**Baseline:** v0.1.0  

---

## 1. Type Judgments & Inference

The ORDYN type system utilizes bidirectional type checking with Hindley-Milner style type inference, augmented by nominal subtyping for security labels and linear / affine consumption rules for capabilities.

### Typing Context $\Gamma$
The typing context $\Gamma$ tracks:
- Variable and value bindings: $x : \tau$
- Security classifications: $x : \tau @ \ell$ where $\ell \in \{\text{Public}, \text{Internal}, \text{Confidential}, \text{Restricted}\}$
- Capability permissions: $c : \text{Cap}(K)$
- Effect environment: $\mathcal{E} \subseteq \{\text{IO}, \text{Net}, \text{Storage}, \text{State}, \text{Audit}\}$

---

## 2. Security Label Lattice

Security classifications form a bounded join-semilattice:

$$\mathcal{L} = \langle \{\text{Public}, \text{Internal}, \text{Confidential}, \text{Restricted}\}, \sqsubseteq, \sqcup, \sqcap, \bot, \top \rangle$$

Where:
- $\bot = \text{Public}$
- $\top = \text{Restricted}$
- $\text{Public} \sqsubseteq \text{Internal} \sqsubseteq \text{Confidential} \sqsubseteq \text{Restricted}$

### Information Flow Subtyping Rule
$$\frac{\tau_1 = \tau_2 \quad \ell_1 \sqsubseteq \ell_2}{\tau_1 @ \ell_1 \le \tau_2 @ \ell_2}$$

Values of lower sensitivity can be safely promoted to higher sensitivity without restriction. The converse requires an explicit, capability-authorized declassification boundary.

---

## 3. Affine Capability Types

Capabilities are modeled as **affine types** (used at most once in linear transformations, or bound non-forgeably to entity lifecycles).

### Affine Consumption Rule
$$\frac{\Gamma, c : \text{AffineCap}(K) \vdash e_1 : \tau_1 \dashv \Gamma' \quad c \notin \Gamma'}{\Gamma, c : \text{AffineCap}(K) \vdash \text{consume}(c); e_2 : \tau_2}$$

Attempting to reuse an affine capability handle after consumption triggers a static compile-time error: `Use of consumed capability handle`.

---

## 4. Algebraic Data Types & Pattern Matching

ORDYN natively supports sum types (`enum`) and product types (`struct` / tuples):

```ordyn
enum PaymentStatus {
    Pending,
    Authorized(String),
    Failed(ErrorCode),
}
```

Exhaustiveness checking is strictly enforced: every match expression must handle all variants or provide an explicit default wildcard `_`.
