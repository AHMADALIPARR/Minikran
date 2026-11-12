#!/usr/bin/env bash
# Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
# Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
# See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
# Runs datalog/eval/minikran_admission.dl (author's original policy, verbatim) and
# datalog/eval/minikran_admission_checked.dl
# on every fixture and diffs admitted_dispatch.csv against expected_his/ and expected_checked/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"; fail=0
for t in "$here"/*/; do
  name="$(basename "$t")"
  for v in his checked; do
    prog="$here/../eval/minikran_admission.dl"; [ $v = checked ] && prog="$here/../eval/minikran_admission_checked.dl"
    out="$(mktemp -d)"
    souffle -F "$t" -D "$out" "$prog" || { echo "FAIL $name [$v] souffle error"; fail=1; continue; }
    got="$(sort "$out/admitted_dispatch.csv" | tr '\t\n' ' ;')"
    if diff <(sort "$t/expected_$v/admitted_dispatch.csv") <(sort "$out/admitted_dispatch.csv") >/dev/null; then
      echo "PASS $name [$v] admitted: ${got:-<none>}"
    else echo "FAIL $name [$v] got: ${got:-<none>}"; fail=1; fi
    rm -rf "$out"
  done
done
exit $fail
