// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! Bids, nodes, admission and deterministic scheduler. Mirrors `MinikranProofs/Bidding.lean`.

use crate::gate::{gate, Decision, TimedLease};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bid { pub timed: TimedLease, pub bid_id: u64, pub node_id: u64, pub priority: u64 }

/// Maximum number of sliding-window power-ceiling entries stored on a node.
pub const MAX_WINDOWS: usize = 8;

/// Node capacity. The sliding power ceiling is given per time window index
/// (`power_ceiling[w]` for `w < windows`; windows past the end have ceiling 0).
/// Fixed-size storage so Kani harnesses can exercise admission without heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub id: u64,
    pub weight: u64,
    pub cpu: u64,
    pub mem: u64,
    pub store: u64,
    pub power: u64,
    pub power_ceiling: [u64; MAX_WINDOWS],
    pub windows: usize,
}

impl Node {
    pub fn from_ceilings(id: u64, weight: u64, cpu: u64, mem: u64, store: u64, power: u64, ceilings: &[u64]) -> Node {
        let mut power_ceiling = [0u64; MAX_WINDOWS];
        let windows = ceilings.len().min(MAX_WINDOWS);
        power_ceiling[..windows].copy_from_slice(&ceilings[..windows]);
        Node { id, weight, cpu, mem, store, power, power_ceiling, windows }
    }
    pub fn ceiling(&self, w: usize) -> u64 {
        if w < self.windows && w < MAX_WINDOWS { self.power_ceiling[w] } else { 0 }
    }
}

pub fn fits(b: &Bid, n: &Node, w: usize) -> bool {
    let l = &b.timed.lease;
    l.cpu <= n.cpu && l.mem <= n.mem && l.store <= n.store && l.power <= n.power && l.power <= n.ceiling(w)
}

pub fn admit(b: &Bid, n: &Node, w: usize) -> Decision { if fits(b, n, w) { Decision::COMMIT } else { Decision::SILENCE } }

/// Lean `better`: higher weight, then lower id.
pub fn better(a: &Node, c: &Node) -> bool { c.weight < a.weight || (a.weight == c.weight && a.id < c.id) }

/// Lean `schedule` (right fold).
pub fn schedule<'a>(b: &Bid, w: usize, ns: &'a [Node]) -> Option<&'a Node> {
    match ns.split_first() {
        None => None,
        Some((n, rest)) => match schedule(b, w, rest) {
            None => if fits(b, n, w) { Some(n) } else { None },
            Some(m) => if fits(b, n, w) && better(n, m) { Some(n) } else { Some(m) },
        },
    }
}

/// Lean `consume`, with checked subtraction (only called when `fits`, so never fails).
pub fn consume(n: &Node, b: &Bid) -> Option<Node> {
    let l = &b.timed.lease;
    Some(Node {
        id: n.id, weight: n.weight,
        cpu: n.cpu.checked_sub(l.cpu)?, mem: n.mem.checked_sub(l.mem)?,
        store: n.store.checked_sub(l.store)?, power: n.power.checked_sub(l.power)?,
        power_ceiling: n.power_ceiling, windows: n.windows,
    })
}

/// Allocation-free `admitAll` used by Kani harnesses. Writes admitted bids into `out`.
/// Returns remaining node capacity and the number of admitted bids.
pub fn admit_all_into(w: usize, n: &Node, bs: &[Bid], out: &mut [Bid]) -> (Node, usize) {
    let mut cur = *n;
    let mut k = 0;
    for b in bs {
        if fits(b, &cur, w) {
            cur = match consume(&cur, b) {
                Some(next) => next,
                None => return (cur, k), // unreachable when fits; keep going without panic
            };
            if k < out.len() { out[k] = *b; k += 1; }
        }
    }
    (cur, k)
}

/// Lean `admitAll`: remaining node and admitted bids.
pub fn admit_all(w: usize, n: &Node, bs: &[Bid]) -> (Node, Vec<Bid>) {
    let mut buf = vec![*bs.first().unwrap_or(&Bid {
        timed: TimedLease { lease: crate::gate::Lease { cpu: 0, mem: 0, store: 0, power: 0, verified: false }, issued: 0, ttl: 0, seq: 0, sig_ok: false },
        bid_id: 0, node_id: 0, priority: 0,
    }); bs.len()];
    let (cur, k) = admit_all_into(w, n, bs, &mut buf);
    buf.truncate(k);
    (cur, buf)
}

/// Lean `fullGate`.
pub fn full_gate(b: &Bid, n: &Node, w: usize, now: u64, last: u64) -> Decision {
    if gate(&b.timed, now, last) == Decision::COMMIT && b.node_id == n.id && fits(b, n, w) { Decision::COMMIT } else { Decision::SILENCE }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::Lease;
    fn bid(cpu: u64, power: u64) -> Bid {
        Bid { timed: TimedLease { lease: Lease { cpu, mem: 1, store: 1, power, verified: true }, issued: 0, ttl: 100, seq: 1, sig_ok: true }, bid_id: 1, node_id: 2, priority: 0 }
    }
    fn node(id: u64, weight: u64, cpu: u64) -> Node { Node::from_ceilings(id, weight, cpu, 10, 10, 50, &[50, 10]) }
    #[test] fn overcommit_silence() { assert_eq!(admit(&bid(11, 1), &node(1, 0, 10), 0), Decision::SILENCE); }
    #[test] fn sliding_ceiling() { assert_eq!(admit(&bid(1, 20), &node(1, 0, 10), 0), Decision::COMMIT); assert_eq!(admit(&bid(1, 20), &node(1, 0, 10), 1), Decision::SILENCE); }
    #[test] fn schedule_tiebreak() {
        let ns = [node(5, 1, 10), node(3, 1, 10), node(9, 0, 10), node(1, 2, 0)];
        assert_eq!(schedule(&bid(4, 1), 0, &ns).map(|n| n.id), Some(3));
    }
    #[test] fn accounting_conserves() {
        let n = node(1, 0, 10);
        let (rem, adm) = admit_all(0, &n, &[bid(4, 1), bid(4, 1), bid(4, 1)]);
        assert_eq!(adm.len(), 2);
        assert_eq!(rem.cpu + adm.iter().map(|b| b.timed.lease.cpu).sum::<u64>(), n.cpu);
    }
    #[test] fn full_gate_node_binding() { let n = node(2, 0, 16); assert_eq!(full_gate(&bid(1, 1), &n, 0, 5, 0), Decision::COMMIT); assert_eq!(full_gate(&bid(1, 1), &node(3, 0, 16), 0, 5, 0), Decision::SILENCE); }
}
