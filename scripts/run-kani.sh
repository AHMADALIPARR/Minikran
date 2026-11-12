#!/usr/bin/env bash
# Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
# Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
# See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
# Run each Kani harness individually from node/. Print the exact VERIFICATION line.
set -u
cd "$(dirname "$0")/../node"
HARNESSES=(
  gate_never_commits_expired
  gate_never_commits_unsigned_or_unverified
  gate_never_commits_replay
  gate_commit_implies_all_conditions
  expiry_no_overflow_matches_nat
  seq_strictly_monotone_bounded
  admission_no_commit_on_overcommit
  capacity_accounting_no_underflow_bounded
  scheduler_deterministic_canonical
  ledger_chain_linkage_bounded
)
fail=0
echo "kani $(kani --version 2>/dev/null | tr '\n' ' ')"
echo "date: $(date -Iseconds)"
for h in "${HARNESSES[@]}"; do
  echo
  echo "========== harness $h =========="
  # 10 minute cap per harness; default unwind is set on the harness attributes.
  if timeout 600 cargo kani --harness "$h" --output-format terse 2>&1; then
    :
  else
    ec=$?
    echo "COMMAND_EXIT=$ec"
    fail=1
  fi
done
echo
if [ $fail -eq 0 ]; then echo "ALL_HARNESSES_OK"; else echo "SOME_HARNESSES_FAILED"; exit 1; fi
