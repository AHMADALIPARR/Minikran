# Full evaluation harness

The repository `Makefile` and `docs/eval.md` cover the original skeleton
(`lake build Minikran.Timed`, `cargo check` in `rust/`). This document
covers the fuller stack, which lives in new paths only:

| Piece | Path |
| --- | --- |
| Lean timed-lease, bidding, admission proofs (21 theorems) | `MinikranProofs/` |
| Lean admission evaluator (fixture cross-check) | `lean-eval/AdmissionEval.lean` |
| Full Rust node (parse, gate, admission, ledger, telemetry, Kani) | `node/` |
| Soufflé gate (TTL / signature / replay) | `datalog/gate.dl` |
| Soufflé admission (original + checked, fixture schema) | `datalog/eval/` |
| Admission fixtures | `datalog/admission_tests/` |
| Gate fixtures | `datalog/tests/{basic,replay}/` |
| Make / PowerShell / CI | `eval.mk`, `eval.ps1`, `.github/workflows/eval.yml` |
| Recorded checker output | `docs/measurements/` |

Run everything except Kani:

```bash
make -f eval.mk eval
```

On PowerShell:

```powershell
pwsh -File eval.ps1
```

Kani (each harness individually):

```bash
make -f eval.mk kani
```

The programs already on main (`datalog/minikran_*.dl`, `rust/`, `Makefile`)
are left untouched. `make -f eval.mk datalog-his` only typechecks those
Soufflé programs.

A `[[lean_lib]] name = "MinikranProofs"` line is deliberately not added to
`lakefile.toml`. See `docs/lakefile-MinikranProofs.md`.
