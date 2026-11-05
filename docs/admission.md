# Admission Policy Notes

## Original (`minikran_admission.dl`)
As written, vulnerable to:
1. One-at-a-time checks (two bids that each fit can both be admitted)
2. Multi-node admission of the same task
3. 32-bit overflow on `Req + Load`

## Checked (`minikran_admission_checked.dl`)
- Processes bids per node ordered by priority
- Subtracts from remaining capacity after each admission
- Each task admitted to at most one node
- Uses `req <= remaining` (no addition, no overflow)

The checked version is the real gate. Lean and Rust should agree with it on the test fixtures in `datalog/tests/`.
