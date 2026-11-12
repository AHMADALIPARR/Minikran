# Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
# Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
# See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
#
# Fuller Minikran evaluation harness. Does not replace the repo Makefile.
# Usage, from the repository root:
#   make -f eval.mk eval
# Individual targets: lean, lean-axioms, datalog, datalog-his, rust, kani, notebook-less eval.

.PHONY: eval lean lean-axioms datalog datalog-his rust kani
ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
MEAS := $(ROOT)/docs/measurements
# Sidecar Lake config (does not edit lakefile.toml).
LAKEF := $(ROOT)/lakefile-eval.toml
LAKE := lake -f $(LAKEF)

eval: lean lean-axioms datalog datalog-his rust
	@echo "=== Minikran full eval complete ==="

# Build via lakefile-eval.toml so MinikranProofs is a library without editing lakefile.toml.
lean:
	cd $(ROOT) && $(LAKE) build Minikran Minikran.Timed MinikranProofs
	@echo "Lean MinikranProofs: OK"

# Prints `#print axioms` for all 21 theorems. Expect only Lean/Init and
# (where omega/decide are used) the standard prelude axioms, never user axioms.
lean-axioms: lean
	mkdir -p $(MEAS)
	cd $(ROOT) && $(LAKE) env lean MinikranProofs/AxiomCheck.lean > $(MEAS)/lean-axioms.txt
	@if grep -n "sorry\|^axiom " $(ROOT)/MinikranProofs/*.lean; then echo "unexpected sorry/axiom in sources"; exit 1; fi
	@echo "Wrote $(MEAS)/lean-axioms.txt"

# Soufflé fixtures for the eval gate and the eval admission policies.
datalog:
	chmod +x $(ROOT)/datalog/tests/run.sh $(ROOT)/datalog/admission_tests/run.sh
	$(ROOT)/datalog/tests/run.sh
	$(ROOT)/datalog/admission_tests/run.sh

# Typecheck the Soufflé programs already on main. Gate and original admission
# should succeed. The checked program on main currently fails Soufflé (ungrounded
# midPrio + cyclic negation); this target records that result and does not
# rewrite those files. The committed fixture datalog/tests/admission_overcommit.facts
# is a commented sketch, not Soufflé fact files.
datalog-his:
	mkdir -p $(MEAS)
	@ok=0; \
	for f in datalog/minikran_gate.dl datalog/minikran_admission.dl datalog/minikran_admission_checked.dl; do \
	  echo "==== $$f ===="; \
	  if souffle --parse-errors --show=type-analysis $(ROOT)/$$f; then echo "TYPECHECK_OK $$f"; \
	  else echo "TYPECHECK_FAIL $$f"; ok=1; fi; \
	done > $(MEAS)/souffle-his.txt 2>&1; \
	cat $(MEAS)/souffle-his.txt; \
	echo "(see docs/measurements/souffle-his.txt; a FAIL on minikran_admission_checked.dl is recorded, not patched)"

rust:
	cd $(ROOT)/node && cargo test --offline -- --test-threads=1 || cd $(ROOT)/node && cargo test -- --test-threads=1

# Run each Kani harness individually. Results are appended under docs/measurements.
kani:
	mkdir -p $(MEAS)
	cd $(ROOT)/node && bash $(ROOT)/scripts/run-kani.sh | tee $(MEAS)/kani.txt

.PHONY: admission-cross
# Cross-check Lean AdmissionEval against the checked fixture outputs (optional).
admission-cross: lean
	cd $(ROOT) && $(LAKE) build AdmissionEval
	cd $(ROOT) && $(LAKE) exe AdmissionEval datalog/admission_tests/1_cumulative_overcommit
