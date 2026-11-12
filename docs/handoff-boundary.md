# Cloud-to-node handoff boundary

This document describes the software boundary at which a cloud orchestrator
hands a workload to a local Minikran node. It is implemented by the crate in
`node/`. A hardware root of trust or secure enclave that verifies the lease
signature before execution is an integration point; it is not built here.

## Sequence

1. The orchestrator constructs a `Bid` wrapping a timed lease (resource
   ceilings, TTL, sequence number, node id, priority) and signs the canonical
   encoding with an HMAC-SHA256 shared key (`node/src/sig.rs`).
2. The node verifies the signature first. A bad signature forces
   `sig_ok = false` and the gate returns SILENCE.
3. The node runs the COMMIT/SILENCE gate (`node/src/gate.rs`, matching
   `MinikranProofs/LeaseTTL.lean`): the lease must be within ceiling, marked
   verified, signed, unexpired, and carry a sequence number strictly greater
   than the last accepted one. Replay, expiry, or a missing signature is
   SILENCE.
4. The node then runs checked admission (`node/src/admission.rs`, matching
   `datalog/eval/minikran_admission_checked.dl` and
   `MinikranProofs/Admission.lean`) against its declared capacity and current
   load. Overcommit is SILENCE.
5. Every decision, COMMIT or SILENCE, is appended to the hash-chained WORM
   ledger as an `activation` receipt. On COMMIT the node spawns the mock
   workload, independently checks the result (`verify compute-sum ok`), and
   appends an `accounting` receipt with the child's real `wait4(2)` rusage
   (wall, user, sys, max RSS) and, when the platform exposes it, energy.
6. Anyone holding the ledger file can walk the chain from the genesis entry
   and detect a byte flip, a deleted or reordered entry, or a pruned tail
   (the sidecar `.head` file records the last index and hash).

## What is not built

- A hardware root of trust, TPM, or secure-enclave quote that binds the
  signature key or the ledger head to a specific machine.
- A production cloud orchestrator. `node/src/dispatch.rs` is a mock of the
  handshake used by the evaluation harness.
- Live power telemetry on machines that do not expose RAPL or an INA-style
  hwmon. The telemetry endpoint reports `power: null` in that case rather
  than inventing a number.
