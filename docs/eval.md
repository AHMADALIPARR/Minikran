# Minikran Evaluation

Run the full pipeline:

```bash
make eval
```

Or PowerShell:

```powershell
lake build
# souffle datalog/minikran_gate.dl ...
cd rust; cargo check
```

Hardware root of trust is an integration point (not yet built). The gate and ledger provide the software handoff boundary.
