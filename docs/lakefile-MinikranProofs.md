# Proposed lakefile.toml append

The proofs under `MinikranProofs/` are built with the sidecar config
`lakefile-eval.toml` (a copy of `lakefile.toml` plus a `MinikranProofs`
library and the `AdmissionEval` executable). The committed `lakefile.toml`
is not modified.

```
lake -f lakefile-eval.toml build MinikranProofs
lake -f lakefile-eval.toml env lean MinikranProofs/AxiomCheck.lean
```

(`make -f eval.mk lean`).

If you want `lake build MinikranProofs` to work against the committed
lakefile, append these two lines to `lakefile.toml`:

```toml
[[lean_lib]]
name = "MinikranProofs"
```

The existing `[[lean_lib]]` entries (`Minikran` and `Minikran.Timed`) should be
left as they are. This file is a proposal only; `lakefile.toml` was not changed.

`Minikran/Timed.lean` is a new root module so `lake build Minikran.Timed`
(the target already used in the committed Makefile) has a file to load.
It only imports `Minikran.Timed.Lease`.
