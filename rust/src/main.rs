//! Minikran node: parse .mkr, check signed lease, gate, WORM ledger, telemetry.
//! Kani-checked for never-commit expired/unsigned/replayed.

use sha2::{Sha256, Digest};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Lease {
    pub cpu: u32,
    pub mem: u32,
    pub store: u32,
    pub power: u32,
    pub verified: bool,
}

#[derive(Debug, Clone)]
pub struct TimedLease {
    pub base: Lease,
    pub issued_at: u64,
    pub expires_at: u64,
    pub seq: u64,
    pub signature_ok: bool,
}

#[derive(Debug, PartialEq)]
pub enum GateResult { COMMIT, SILENCE }

pub fn is_valid(t: &TimedLease, now: u64) -> bool {
    t.base.verified && t.signature_ok && now < t.expires_at
}

pub fn gate(t: &TimedLease, now: u64, last_seq: u64) -> GateResult {
    if !is_valid(t, now) || t.seq <= last_seq {
        GateResult::SILENCE
    } else {
        GateResult::COMMIT
    }
}

/// Hash-chained WORM ledger entry.
#[derive(Debug)]
pub struct Receipt {
    pub prev_hash: [u8; 32],
    pub seq: u64,
    pub result: String,
    pub hash: [u8; 32],
}

pub fn append_receipt(prev: &[u8; 32], seq: u64, result: &str) -> Receipt {
    let mut hasher = Sha256::new();
    hasher.update(prev);
    hasher.update(seq.to_le_bytes());
    hasher.update(result.as_bytes());
    let hash: [u8; 32] = hasher.finalize().into();
    Receipt { prev_hash: *prev, seq, result: result.to_string(), hash }
}

fn main() {
    println!("Minikran node skeleton ready. Parse .mkr and run gate.");
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn never_commit_invalid() {
        let t = TimedLease {
            base: Lease { cpu: 16, mem: 32, store: 100, power: 50, verified: kani::any() },
            issued_at: 0,
            expires_at: kani::any(),
            seq: kani::any(),
            signature_ok: kani::any(),
        };
        let now: u64 = kani::any();
        let last: u64 = kani::any();
        if !is_valid(&t, now) || t.seq <= last {
            assert!(gate(&t, now, last) == GateResult::SILENCE);
        }
    }

    #[kani::proof]
    fn commit_implies_monotonic() {
        let t = TimedLease {
            base: Lease { cpu: 16, mem: 32, store: 100, power: 50, verified: true },
            issued_at: 0,
            expires_at: 1000,
            seq: kani::any(),
            signature_ok: true,
        };
        let now: u64 = 500;
        let last: u64 = kani::any();
        if gate(&t, now, last) == GateResult::COMMIT {
            assert!(last < t.seq);
        }
    }
}
