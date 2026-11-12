#!/usr/bin/env bash
# Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
# Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
# See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
# Runs souffle on every fixture under datalog/tests and diffs against expected/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"; fail=0
for t in "$here"/*/; do
  name="$(basename "$t")"; out="$(mktemp -d)"
  souffle -F "$t" -D "$out" "$here/../gate.dl" || { echo "FAIL $name (souffle error)"; fail=1; continue; }
  ok=1
  for rel in commit silence; do
    if ! diff <(sort "$t/expected/$rel.csv") <(sort "$out/$rel.csv"); then ok=0; fi
  done
  if [ $ok = 1 ]; then echo "PASS $name"; else echo "FAIL $name"; fail=1; fi
  rm -rf "$out"
done
exit $fail
