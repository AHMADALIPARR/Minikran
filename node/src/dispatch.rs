// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! Mock cloud orchestrator → node dispatch. Orchestrator signs a bid; the node verifies the
//! signature, runs the gate and checked admission, writes an activation receipt, and on
//! COMMIT spawns a mock workload, verifies its output and writes an accounting receipt.

use crate::admission::{self, BidRow, NodeRow};
use crate::bidding::Bid;
use crate::gate::{gate, Decision, Lease, TimedLease};
use crate::ledger::Ledger;
use crate::{sig, telemetry};
use serde_json::{json, Value};
use std::io::Read;
use std::process::{Command, Stdio};

/// Node-side configuration: declared (not measured) capacity and current load.
pub struct NodeConfig { pub name: String, pub id: u64, pub capacity: [i64; 4], pub load: [i64; 4] }

pub struct NodeState { pub last_seq: u64 }

/// Signed message from the orchestrator.
pub struct SignedBid { pub bid: Bid, pub sig: String }

pub fn orchestrator_sign(key: &[u8], bid: Bid) -> SignedBid { SignedBid { sig: sig::sign(key, &bid), bid } }

pub struct Usage { pub wall_ms: f64, pub user_ms: f64, pub sys_ms: f64, pub maxrss_kb: i64, pub energy_j: Option<f64>, pub stdout: String }

/// Spawn `exe workload <n>` and measure it with wait4 (real rusage of the child).
pub fn run_workload(exe: &std::path::Path, n: u64) -> std::io::Result<Usage> {
    let e0 = telemetry::rapl_energy_uj();
    let t0 = std::time::Instant::now();
    let mut child = Command::new(exe).arg("workload").arg(n.to_string()).stdout(Stdio::piped()).spawn()?;
    let mut out = String::new();
    child.stdout.take().unwrap().read_to_string(&mut out)?;
    let pid = child.id() as libc::pid_t;
    let mut status = 0;
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::wait4(pid, &mut status, 0, &mut ru) };
    if r < 0 { return Err(std::io::Error::last_os_error()); }
    let wall = t0.elapsed().as_secs_f64() * 1e3;
    let e1 = telemetry::rapl_energy_uj();
    let tv = |t: libc::timeval| t.tv_sec as f64 * 1e3 + t.tv_usec as f64 / 1e3;
    Ok(Usage { wall_ms: wall, user_ms: tv(ru.ru_utime), sys_ms: tv(ru.ru_stime), maxrss_kb: ru.ru_maxrss as i64,
        energy_j: match (e0, e1) { (Some(a), Some(b)) => Some(b.wrapping_sub(a) as f64 / 1e6), _ => None }, stdout: out })
}

/// The mock workload: sum 1..=n.
pub fn workload(n: u64) -> u128 { (1..=n as u128).sum() }

/// Independent check: closed form n(n+1)/2.
pub fn verify_workload(n: u64, out: &str) -> bool {
    let expect = (n as u128) * (n as u128 + 1) / 2;
    out.trim().strip_prefix("sum=").and_then(|s| s.parse::<u128>().ok()) == Some(expect)
}

pub fn lease_json(b: &Bid) -> Value {
    let l = b.timed.lease;
    json!({"cpu": l.cpu, "mem": l.mem, "store": l.store, "power": l.power, "verified": l.verified,
        "issued": b.timed.issued, "ttl": b.timed.ttl, "seq": b.timed.seq, "bid_id": b.bid_id, "node_id": b.node_id, "priority": b.priority})
}

/// Node handling of one signed bid. Returns the decision.
pub fn handle(key: &[u8], node: &NodeConfig, st: &mut NodeState, ledger: &Ledger, sb: &SignedBid, now: u64,
              exe: &std::path::Path, work_n: u64, log: &mut Vec<String>) -> std::io::Result<Decision> {
    // 1. signature verified before anything else
    let sig_ok = sig::verify(key, &sb.bid, &sb.sig);
    let timed = TimedLease { sig_ok, ..sb.bid.timed };
    // 2. TTL / verified / signature / seq gate (Lean `gate`)
    let g = gate(&timed, now, st.last_seq);
    // 3. node binding + checked admission (same rules as minikran_admission_checked.dl)
    let l = timed.lease;
    let caps = [NodeRow { id: node.name.clone(), cpu: node.capacity[0], mem: node.capacity[1], store: node.capacity[2], power: node.capacity[3] }];
    let loads = [NodeRow { id: node.name.clone(), cpu: node.load[0], mem: node.load[1], store: node.load[2], power: node.load[3] }];
    let bids = [BidRow { task: sb.bid.bid_id as i64, node: node.name.clone(), cpu: l.cpu as i64, mem: l.mem as i64, store: l.store as i64, power: l.power as i64, prio: sb.bid.priority as i64 }];
    let adm = sb.bid.node_id == node.id && !admission::admitted(&caps, &loads, &bids).is_empty();
    let d = if g == Decision::COMMIT && adm { Decision::COMMIT } else { Decision::SILENCE };
    let reason = if !sig_ok { "signature invalid" } else if crate::gate::expired(&timed, now) { "expired" }
        else if timed.seq <= st.last_seq { "replay (seq <= last accepted)" } else if g != Decision::COMMIT { "gate" }
        else if !adm { "admission" } else { "ok" };
    let act = ledger.append(json!({"kind": "activation", "timestamp": now, "lease": lease_json(&sb.bid),
        "lease_hash": crate::ledger::sha256_hex(sig::canonical(&sb.bid).as_bytes()), "lease_signature": sb.sig,
        "seq": timed.seq, "decision": format!("{d:?}"), "reason": reason, "node": node.name}))?;
    log.push(format!("bid {} seq {}: sig_ok={} gate={:?} admission={} => {:?} ({}) receipt #{} {}", sb.bid.bid_id, timed.seq, sig_ok, g, adm, d, reason, act.index, &act.hash[..16]));
    if d == Decision::SILENCE { return Ok(d); }
    st.last_seq = timed.seq;
    let u = run_workload(exe, work_n)?;
    let ok = verify_workload(work_n, &u.stdout);
    let v = if ok { "verify compute-sum ok" } else { "verify compute-sum FAILED" };
    let acc = ledger.append(json!({"kind": "accounting", "timestamp": crate::now_unix(), "activation_hash": act.hash,
        "bid_id": sb.bid.bid_id, "node_id": node.id, "lease_signature": sb.sig, "verify": v, "workload": format!("compute-sum n={work_n}"),
        "usage": {"wall_ms": u.wall_ms, "user_cpu_ms": u.user_ms, "sys_cpu_ms": u.sys_ms, "max_rss_kb": u.maxrss_kb,
                  "energy_j": u.energy_j, "source": "wait4(2) rusage of child; energy from powercap RAPL if present"}}))?;
    log.push(format!("  workload: {} | {} | wall {:.3} ms, user {:.3} ms, sys {:.3} ms, maxrss {} KB, energy_j {:?} | receipt #{} {}",
        u.stdout.trim(), v, u.wall_ms, u.user_ms, u.sys_ms, u.maxrss_kb, u.energy_j, acc.index, &acc.hash[..16]));
    Ok(d)
}

/// End-to-end scenario using the first lease in an .mkr file.
pub fn scenario(key: &[u8], lease: Lease, ledger: &Ledger, exe: &std::path::Path, work_n: u64) -> std::io::Result<Vec<String>> {
    let node = NodeConfig { name: "node-a".into(), id: 1, capacity: [16, 32, 100, 50], load: [0, 0, 0, 0] };
    let mut st = NodeState { last_seq: 0 };
    let now = crate::now_unix();
    let mk = |seq, issued, bid_id| Bid { timed: TimedLease { lease, issued, ttl: 3600, seq, sig_ok: false }, bid_id, node_id: 1, priority: 1 };
    let mut log = vec![format!("node {} capacity(declared) {:?} load {:?}, now={now}", node.name, node.capacity, node.load)];
    let valid = orchestrator_sign(key, mk(1, now, 100));
    let mut tampered = orchestrator_sign(key, mk(2, now, 101));
    tampered.bid.timed.lease.cpu = 8; // modified after signing
    let expired = orchestrator_sign(key, mk(3, now.saturating_sub(7200), 102));
    let wrong_key = orchestrator_sign(b"not-the-shared-key", mk(4, now, 103));
    let valid2 = orchestrator_sign(key, mk(5, now, 104));
    let cases = [("valid", &valid), ("replay of valid", &valid), ("tampered after signing", &tampered),
        ("expired", &expired), ("signed with wrong key", &wrong_key), ("valid, next seq", &valid2)];
    for (name, sb) in cases {
        log.push(format!("[{name}]"));
        handle(key, &node, &mut st, ledger, sb, now, exe, work_n, &mut log)?;
    }
    Ok(log)
}
