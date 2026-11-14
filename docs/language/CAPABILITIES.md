# ORDYN Capability-Based Security

**Specification:** Capability Declarations, Granularity & Lifecycle  
**Baseline:** v0.1.0  

---

## 1. Capability Paradigm

In traditional systems, permissions are identity-based (ACLs) or ambient (e.g. any process running as a given user can read any file that user owns).

ORDYN employs **Object Capabilities (OCaps)**:
1. Authority is bundled with the capability reference itself.
2. If an entity does not hold the capability handle, it cannot perform the operation—even if it knows the target resource's name or URI.
3. Capabilities cannot be forged; they can only be granted by an authorized authority (the Security Kernel or a parent boundary) or derived through attenuation.

---

## 2. Declaration & Attenuation

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

Authority can only decrease during attenuation; attempts to expand scope trigger compiler or runtime rejection.
