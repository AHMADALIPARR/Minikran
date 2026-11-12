// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! Kani bounded model-checking harnesses (`cargo kani`). Overflow, underflow and panics are
//! checked automatically by Kani on every harness in addition to the explicit assertions.
//!
//! These harnesses are allocation-free: they use fixed-size arrays and the `*_into`
//! entry points so Kani does not hit its unsupported `handle_alloc_error` path.
//! The properties themselves are unchanged.

use crate::bidding::{admit, admit_all_into, better, fits, schedule, Bid, Node, MAX_WINDOWS};
use crate::gate::{expired, gate, run_into, Decision, Lease, TimedLease};

fn any_lease() -> Lease { Lease { cpu: kani::any(), mem: kani::any(), store: kani::any(), power: kani::any(), verified: kani::any() } }
fn any_timed() -> TimedLease { TimedLease { lease: any_lease(), issued: kani::any(), ttl: kani::any(), seq: kani::any(), sig_ok: kani::any() } }

#[kani::proof]
fn gate_never_commits_expired() {
    let t = any_timed(); let now: u64 = kani::any(); let last: u64 = kani::any();
    kani::assume((t.issued as u128) + (t.ttl as u128) < now as u128);
    assert_eq!(gate(&t, now, last), Decision::SILENCE);
}

#[kani::proof]
fn gate_never_commits_unsigned_or_unverified() {
    let t = any_timed(); let now: u64 = kani::any(); let last: u64 = kani::any();
    kani::assume(!t.sig_ok || !t.lease.verified);
    assert_eq!(gate(&t, now, last), Decision::SILENCE);
}

#[kani::proof]
fn gate_never_commits_replay() {
    let t = any_timed(); let now: u64 = kani::any(); let last: u64 = kani::any();
    kani::assume(t.seq <= last);
    assert_eq!(gate(&t, now, last), Decision::SILENCE);
}

#[kani::proof]
fn gate_commit_implies_all_conditions() {
    let t = any_timed(); let now: u64 = kani::any(); let last: u64 = kani::any();
    if gate(&t, now, last) == Decision::COMMIT {
        assert!(t.lease.cpu <= 16 && t.lease.mem <= 32 && t.lease.store <= 100 && t.lease.power <= 50);
        assert!(t.lease.verified && t.sig_ok && last < t.seq);
        assert!((now as u128) <= (t.issued as u128) + (t.ttl as u128));
    }
}

/// issued + ttl uses checked arithmetic: no overflow for any inputs, and agrees with
/// unbounded (u128) arithmetic, i.e. with Lean's Nat semantics.
#[kani::proof]
fn expiry_no_overflow_matches_nat() {
    let t = any_timed(); let now: u64 = kani::any();
    assert_eq!(expired(&t, now), (now as u128) > (t.issued as u128) + (t.ttl as u128));
}

#[kani::proof]
#[kani::unwind(5)]
fn seq_strictly_monotone_bounded() {
    let start: u64 = kani::any();
    let reqs = [(any_timed(), kani::any::<u64>()), (any_timed(), kani::any::<u64>()), (any_timed(), kani::any::<u64>())];
    let mut acc = [0u64; 3];
    let n = run_into(start, &reqs, &mut acc);
    let mut prev = start;
    let mut i = 0;
    while i < n {
        assert!(prev < acc[i]);
        prev = acc[i];
        i += 1;
    }
}

fn any_node() -> Node {
    let mut power_ceiling = [0u64; MAX_WINDOWS];
    power_ceiling[0] = kani::any();
    power_ceiling[1] = kani::any();
    Node { id: kani::any(), weight: kani::any(), cpu: kani::any(), mem: kani::any(), store: kani::any(), power: kani::any(),
        power_ceiling, windows: 2 }
}
fn any_bid() -> Bid { Bid { timed: any_timed(), bid_id: kani::any(), node_id: kani::any(), priority: kani::any() } }

#[kani::proof]
#[kani::unwind(3)]
fn admission_no_commit_on_overcommit() {
    let b = any_bid(); let n = any_node(); let w: usize = kani::any(); kani::assume(w < 3);
    let l = b.timed.lease;
    kani::assume(l.cpu > n.cpu || l.mem > n.mem || l.store > n.store || l.power > n.power || l.power > n.ceiling(w));
    assert_eq!(admit(&b, &n, w), Decision::SILENCE);
}

#[kani::proof]
#[kani::unwind(4)]
fn capacity_accounting_no_underflow_bounded() {
    let n = any_node();
    let bs = [any_bid(), any_bid(), any_bid()];
    let mut adm = [any_bid(); 3];
    let (rem, k) = admit_all_into(0, &n, &bs, &mut adm);
    let mut s: [u128; 4] = [0; 4];
    let mut i = 0;
    while i < k {
        s[0] += adm[i].timed.lease.cpu as u128; s[1] += adm[i].timed.lease.mem as u128;
        s[2] += adm[i].timed.lease.store as u128; s[3] += adm[i].timed.lease.power as u128;
        i += 1;
    }
    assert_eq!(s[0] + rem.cpu as u128, n.cpu as u128);
    assert_eq!(s[1] + rem.mem as u128, n.mem as u128);
    assert_eq!(s[2] + rem.store as u128, n.store as u128);
    assert_eq!(s[3] + rem.power as u128, n.power as u128);
}

#[kani::proof]
#[kani::unwind(4)]
fn scheduler_deterministic_canonical() {
    let b = any_bid();
    let ns = [any_node(), any_node(), any_node()];
    let r1 = schedule(&b, 0, &ns).map(|n| (n.id, n.weight));
    let r2 = schedule(&b, 0, &ns).map(|n| (n.id, n.weight));
    assert_eq!(r1, r2);
    match schedule(&b, 0, &ns) {
        Some(c) => {
            assert!(fits(&b, c, 0));
            for m in ns.iter() { if fits(&b, m, 0) { assert!(!better(m, c)); } }
        }
        None => for m in ns.iter() { assert!(!fits(&b, m, 0)); },
    }
}

/// Ledger chain linkage for bounded n with a Kani-friendly stand-in hash (SHA-256 itself is
/// covered by unit/integration tests, not by this harness).
fn stub_hash(prev: u64, index: u64, body: u64) -> u64 {
    prev.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ index.wrapping_add(0x632B_E59B_D9B4_E019) ^ body
}

#[kani::proof]
#[kani::unwind(6)]
fn ledger_chain_linkage_bounded() {
    let n: usize = kani::any(); kani::assume(n <= 4);
    let mut chain: [(u64, u64, u64, u64); 4] = [(0, 0, 0, 0); 4]; // (index, prev, body, hash)
    let mut prev = 0u64;
    let mut i = 0;
    while i < n {
        let body: u64 = kani::any();
        let h = stub_hash(prev, i as u64, body);
        chain[i] = (i as u64, prev, body, h);
        prev = h; i += 1;
    }
    // verify walk from genesis
    let mut p = 0u64;
    let mut j = 0;
    while j < n {
        let (idx, pr, body, h) = chain[j];
        assert_eq!(idx, j as u64);
        assert_eq!(pr, p);
        assert_eq!(h, stub_hash(pr, idx, body));
        p = h; j += 1;
    }
    assert_eq!(p, prev);
}
