// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! Checked multi-node admission. Mirrors `datalog/eval/minikran_admission_checked.dl` and
//! `MinikranProofs/Admission.lean` (same schema as `datalog/eval/minikran_admission.dl`).

use std::path::Path;

pub const MAXV: i64 = 2147483647;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRow { pub id: String, pub cpu: i64, pub mem: i64, pub store: i64, pub power: i64 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidRow { pub task: i64, pub node: String, pub cpu: i64, pub mem: i64, pub store: i64, pub power: i64, pub prio: i64 }

fn in_range(x: i64) -> bool { (0..=MAXV).contains(&x) }

pub fn bad_input(caps: &[NodeRow], loads: &[NodeRow], bids: &[BidRow]) -> bool {
    let node_ok = |r: &NodeRow| in_range(r.cpu) && in_range(r.mem) && in_range(r.store) && in_range(r.power);
    let bid_ok = |b: &BidRow| [b.task, b.cpu, b.mem, b.store, b.power, b.prio].iter().all(|&x| in_range(x));
    caps.iter().any(|r| !node_ok(r)) || loads.iter().any(|r| !node_ok(r)) || bids.iter().any(|b| !bid_ok(b))
        || bids.iter().any(|b| bids.iter().filter(|c| c.task == b.task && c.node == b.node).count() > 1)
        || caps.iter().any(|r| caps.iter().filter(|c| c.id == r.id).count() > 1)
        || loads.iter().any(|r| loads.iter().filter(|c| c.id == r.id).count() > 1)
}

/// Cap - Load, only if Load <= Cap on every resource (never computes Req + Load).
pub fn headroom(caps: &[NodeRow], loads: &[NodeRow], n: &str) -> Option<[i64; 4]> {
    let c = caps.iter().find(|c| c.id == n)?;
    let l = loads.iter().find(|l| l.id == n)?;
    if l.cpu <= c.cpu && l.mem <= c.mem && l.store <= c.store && l.power <= c.power {
        Some([c.cpu - l.cpu, c.mem - l.mem, c.store - l.store, c.power - l.power])
    } else { None }
}

fn fits_row(caps: &[NodeRow], loads: &[NodeRow], b: &BidRow) -> bool {
    match headroom(caps, loads, &b.node) {
        Some(h) => b.cpu <= h[0] && b.mem <= h[1] && b.store <= h[2] && b.power <= h[3],
        None => false,
    }
}

fn prio(bids: &[BidRow], t: i64) -> i64 { bids.iter().filter(|b| b.task == t).fold(0, |m, b| m.max(b.prio)) }

/// Returns admitted (task_id, node) pairs, in input order.
pub fn admitted(caps: &[NodeRow], loads: &[NodeRow], bids: &[BidRow]) -> Vec<(i64, String)> {
    if bad_input(caps, loads, bids) { return vec![]; }
    let cs: Vec<&BidRow> = bids.iter().filter(|b| fits_row(caps, loads, b)
        && !bids.iter().any(|c| c.task == b.task && fits_row(caps, loads, c) && c.node < b.node)).collect();
    let mut out = Vec::new();
    for b in &cs {
        let before = |c: &BidRow| c.task == b.task || prio(bids, b.task) < prio(bids, c.task)
            || (prio(bids, c.task) == prio(bids, b.task) && c.task < b.task);
        let mut s = [0i128; 4];
        for c in cs.iter().filter(|c| c.node == b.node && before(c)) {
            s[0] += c.cpu as i128; s[1] += c.mem as i128; s[2] += c.store as i128; s[3] += c.power as i128;
        }
        if let Some(h) = headroom(caps, loads, &b.node) {
            if (0..4).all(|i| s[i] <= h[i] as i128) { out.push((b.task, b.node.clone())); }
        }
    }
    out
}

fn rows(p: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(p).unwrap_or_default().lines().filter(|l| !l.is_empty())
        .map(|l| l.split('\t').map(str::to_string).collect()).collect()
}
fn int(s: &str) -> i64 { s.parse().unwrap_or(-1) }

/// Load the souffle fact files (`node_capacity.facts`, `current_load.facts`, `task_bid.facts`).
pub fn load_dir(dir: &Path) -> (Vec<NodeRow>, Vec<NodeRow>, Vec<BidRow>) {
    let node = |r: Vec<String>| if r.len() == 5 { Some(NodeRow { id: r[0].clone(), cpu: int(&r[1]), mem: int(&r[2]), store: int(&r[3]), power: int(&r[4]) }) } else { None };
    let caps = rows(&dir.join("node_capacity.facts")).into_iter().filter_map(node).collect();
    let loads = rows(&dir.join("current_load.facts")).into_iter().filter_map(node).collect();
    let bids = rows(&dir.join("task_bid.facts")).into_iter().filter_map(|r| if r.len() == 7 {
        Some(BidRow { task: int(&r[0]), node: r[1].clone(), cpu: int(&r[2]), mem: int(&r[3]), store: int(&r[4]), power: int(&r[5]), prio: int(&r[6]) }) } else { None }).collect();
    (caps, loads, bids)
}
