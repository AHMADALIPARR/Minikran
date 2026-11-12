// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! COMMIT/SILENCE gate. Mirrors `MinikranProofs/LeaseTTL.lean` exactly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease { pub cpu: u64, pub mem: u64, pub store: u64, pub power: u64, pub verified: bool }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedLease { pub lease: Lease, pub issued: u64, pub ttl: u64, pub seq: u64, pub sig_ok: bool }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision { COMMIT, SILENCE }

/// Same ceilings as `production_lease_within_ceiling`.
pub fn within_ceiling(l: &Lease) -> bool { l.cpu <= 16 && l.mem <= 32 && l.store <= 100 && l.power <= 50 }

/// `now > issued + ttl` with unbounded (Nat) semantics: if the sum overflows u64 it exceeds
/// every u64 `now`, so the lease is not expired.
pub fn expired(t: &TimedLease, now: u64) -> bool {
    match t.issued.checked_add(t.ttl) { Some(end) => now > end, None => false }
}

pub fn gate(t: &TimedLease, now: u64, last: u64) -> Decision {
    if within_ceiling(&t.lease) && t.lease.verified && t.sig_ok && !expired(t, now) && last < t.seq {
        Decision::COMMIT
    } else { Decision::SILENCE }
}

/// Allocation-free `run` used by Kani harnesses. Writes accepted seq numbers into `out`.
/// Returns the number of accepted seqs (at most `out.len()` and `reqs.len()`).
pub fn run_into(mut last: u64, reqs: &[(TimedLease, u64)], out: &mut [u64]) -> usize {
    let mut n = 0;
    for (t, now) in reqs {
        if gate(t, *now, last) == Decision::COMMIT {
            if n < out.len() { out[n] = t.seq; n += 1; }
            last = t.seq;
        }
    }
    n
}

/// Mirrors Lean `run`: accepted seq numbers over a request list.
pub fn run(last: u64, reqs: &[(TimedLease, u64)]) -> Vec<u64> {
    let mut out = vec![0; reqs.len()];
    let n = run_into(last, reqs, &mut out);
    out.truncate(n);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    pub const PROD: Lease = Lease { cpu: 16, mem: 32, store: 100, power: 50, verified: true };
    fn tl(seq: u64, sig: bool) -> TimedLease { TimedLease { lease: PROD, issued: 1000, ttl: 3600, seq, sig_ok: sig } }
    #[test] fn commits_in_window() { assert_eq!(gate(&tl(1, true), 4600, 0), Decision::COMMIT); }
    #[test] fn expired_silence() { assert_eq!(gate(&tl(1, true), 4601, 0), Decision::SILENCE); }
    #[test] fn unsigned_silence() { assert_eq!(gate(&tl(1, false), 2000, 0), Decision::SILENCE); }
    #[test] fn unverified_silence() { let mut t = tl(1, true); t.lease.verified = false; assert_eq!(gate(&t, 2000, 0), Decision::SILENCE); }
    #[test] fn replay_silence() { assert_eq!(gate(&tl(5, true), 2000, 5), Decision::SILENCE); }
    #[test] fn over_ceiling_silence() { let mut t = tl(1, true); t.lease.cpu = 17; assert_eq!(gate(&t, 2000, 0), Decision::SILENCE); }
    #[test] fn overflow_ttl_not_expired() { let mut t = tl(1, true); t.issued = u64::MAX; t.ttl = 1; assert!(!expired(&t, u64::MAX)); }
    #[test] fn run_monotone() {
        let r = run(0, &[(tl(3, true), 2000), (tl(2, true), 2000), (tl(3, true), 2000), (tl(9, true), 2000)]);
        assert_eq!(r, vec![3, 9]);
    }
}
