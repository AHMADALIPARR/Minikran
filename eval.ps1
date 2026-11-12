# Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
# Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
# See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
#
# PowerShell equivalent of `make -f eval.mk eval`.
# From the repository root:  pwsh -File eval.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $Root

Write-Host "=== lean ==="
lake -f "$Root/lakefile-eval.toml" build Minikran Minikran.Timed MinikranProofs

Write-Host "=== lean axioms ==="
New-Item -ItemType Directory -Force -Path "$Root/docs/measurements" | Out-Null
lake -f "$Root/lakefile-eval.toml" env lean MinikranProofs/AxiomCheck.lean | Tee-Object -FilePath "$Root/docs/measurements/lean-axioms.txt"

Write-Host "=== datalog (eval fixtures) ==="
bash "$Root/datalog/tests/run.sh"
bash "$Root/datalog/admission_tests/run.sh"

Write-Host "=== datalog (programs already on main) ==="
foreach ($f in @("datalog/minikran_gate.dl","datalog/minikran_admission.dl","datalog/minikran_admission_checked.dl")) {
  Write-Host "==== $f ===="
  & souffle --parse-errors --show=type-analysis "$Root/$f"
  if ($LASTEXITCODE -ne 0) { Write-Host "TYPECHECK_FAIL $f (recorded, not patched)" }
}

Write-Host "=== rust ==="
Push-Location "$Root/node"
cargo test -- --test-threads=1
Pop-Location

Write-Host "=== Minikran full eval complete ==="
