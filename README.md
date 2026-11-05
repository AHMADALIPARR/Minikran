<p align="center">
  <img src="docs/assets/minikran-hero.svg" alt="Minikran — verified execution leases" width="100%"/>
</p>

# Minikran

**Verified execution leases, admission control, and hash-chained receipts.**

Minikran is a formal-verification stack for resource-bounded workload admission: Lean 4 proofs of lease validity and gate behavior, Soufflé Datalog admission policy, a Rust node that parses `.mkr` specs and appends WORM ledger receipts, and Kani checks that the gate never commits expired, unsigned, or replayed leases.

Authored by **Ahmad Ali Parr** and Team · [SnapKitty Research Lab](https://github.com/AHMADALIPARR)

[![CI](https://github.com/AHMADALIPARR/Minikran/actions/workflows/ci.yml/badge.svg)](https://github.com/AHMADALIPARR/Minikran/actions/workflows/ci.yml)
[![License: FSL-1.1-ALv2](https://img.shields.io/badge/license-FSL--1.1--ALv2-blue)](LICENSE)

---

## What it does

| Layer | Role |
|-------|------|
| **Lean 4** | `Lease` and timed leases under the `Minikran` namespace. Theorems: invalid/expired/unsigned → `SILENCE`; replay rejected; accepted sequence numbers strictly increase; `COMMIT` implies every check passed. No `sorry`, no custom axioms. |
| **Datalog** | Soufflé admission rules. `minikran_admission.dl` is the naive policy; `minikran_admission_checked.dl` is the real gate (priority order, remaining capacity, one node per task, no overflow). |
| **Rust** | Parses `.mkr`, runs the signed-lease gate, writes hash-chained WORM receipts, exposes telemetry (CPU / memory / power when available). |
| **Kani** | Symbolic proofs that the gate never commits an expired, unsigned, or replayed lease; sequence monotonicity and ledger linkage. |
| **`.mkr`** | Custom lease syntax, indexed as Minikran language via Linguist + TextMate grammar. |

Hardware root of trust is documented as a future integration point at the cloud-to-node handoff boundary.

---

## Quick start

```bash
lake update
lake exe cache get   # Mathlib cache
lake build
```

Toolchain is pinned to Lean / Mathlib `v4.36.0-rc2`.

End-to-end harness:

```bash
make eval
```

---

## Layout

```
Minikran/
  Lease.lean              # base Lease + production ceilings
  Timed/Lease.lean        # timed lease, gate, SILENCE / COMMIT theorems
datalog/
  minikran_gate.dl
  minikran_admission.dl
  minikran_admission_checked.dl
  tests/
rust/                     # node + Kani proofs
editors/vscode/           # .mkr syntax
grammars/
example.mkr
docs/
```

---

## Authors

**Ahmad Ali Parr** and Team  
SnapKitty Research Lab  
Research owner: Bel esprit D'accord Trust

---

## License

Functional Source License, Version 1.1, ALv2 Future License (**FSL-1.1-ALv2**).  
Converts to Apache-2.0 two years after each release. See [LICENSE](LICENSE).
