// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! WORM hash-chain ledger (append-only JSON lines file, SHA-256 linked from a genesis entry).
//!
//! entry.hash = SHA-256(prev_hash || "\n" || index || "\n" || canonical JSON of body)
//! A sidecar `<ledger>.head` records the entry count and head hash after each append so
//! truncation of the tail (pruning) is detected too. A real deployment would anchor the
//! head externally (see docs/handoff-boundary.md); that is not implemented here.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const GENESIS_PREV: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry { pub index: u64, pub prev_hash: String, pub body: Value, pub hash: String }

pub fn entry_hash(prev: &str, index: u64, body: &Value) -> String {
    let mut h = Sha256::new();
    h.update(prev.as_bytes()); h.update(b"\n"); h.update(index.to_string().as_bytes()); h.update(b"\n");
    h.update(serde_json::to_string(body).expect("json").as_bytes());
    hex::encode(h.finalize())
}

pub fn sha256_hex(b: &[u8]) -> String { hex::encode(Sha256::digest(b)) }

pub struct Ledger { pub path: PathBuf }

fn head_path(p: &Path) -> PathBuf { let mut s = p.as_os_str().to_owned(); s.push(".head"); PathBuf::from(s) }

impl Ledger {
    /// Open, creating the genesis entry if the file does not exist.
    pub fn open(path: &Path) -> std::io::Result<Ledger> {
        let l = Ledger { path: path.to_path_buf() };
        if !path.exists() {
            l.append(serde_json::json!({"kind": "genesis", "ledger": "minikran-node"}))?;
        }
        Ok(l)
    }
    pub fn entries(&self) -> Result<Vec<Entry>, String> {
        let s = std::fs::read_to_string(&self.path).map_err(|e| e.to_string())?;
        s.lines().enumerate().map(|(i, l)| serde_json::from_str(l).map_err(|e| format!("line {}: {e}", i + 1))).collect()
    }
    pub fn append(&self, body: Value) -> std::io::Result<Entry> {
        let (index, prev) = match self.entries() {
            Ok(es) if !es.is_empty() => { let last = es.last().unwrap(); (last.index + 1, last.hash.clone()) }
            _ => (0, GENESIS_PREV.to_string()),
        };
        let hash = entry_hash(&prev, index, &body);
        let e = Entry { index, prev_hash: prev, body, hash };
        let mut f = OpenOptions::new().create(true).append(true).open(&self.path)?;
        writeln!(f, "{}", serde_json::to_string(&e).unwrap())?;
        f.sync_all()?;
        std::fs::write(head_path(&self.path), format!("{}\t{}\n", e.index + 1, e.hash))?;
        Ok(e)
    }
    /// Walk from genesis; detects byte flips, deletions, reordering and tail pruning.
    pub fn verify(&self) -> Result<(u64, String), String> {
        let es = self.entries()?;
        let (n, head) = verify_chain(&es)?;
        let hp = head_path(&self.path);
        let h = std::fs::read_to_string(&hp).map_err(|e| format!("head file: {e}"))?;
        let mut it = h.trim().split('\t');
        let cnt: u64 = it.next().and_then(|x| x.parse().ok()).ok_or("bad head file")?;
        let hh = it.next().ok_or("bad head file")?;
        if cnt != n || hh != head { return Err(format!("head mismatch: head file says {cnt} entries/{hh}, chain has {n}/{head} (pruned or rewritten)")); }
        Ok((n, head))
    }
}

pub fn verify_chain(es: &[Entry]) -> Result<(u64, String), String> {
    if es.is_empty() { return Err("empty ledger (no genesis)".into()); }
    if es[0].body.get("kind").and_then(|k| k.as_str()) != Some("genesis") { return Err("entry 0 is not genesis".into()); }
    let mut prev = GENESIS_PREV.to_string();
    for (i, e) in es.iter().enumerate() {
        if e.index != i as u64 { return Err(format!("entry {i}: index {} out of order", e.index)); }
        if e.prev_hash != prev { return Err(format!("entry {i}: prev_hash does not link")); }
        if entry_hash(&e.prev_hash, e.index, &e.body) != e.hash { return Err(format!("entry {i}: hash mismatch (tampered)")); }
        prev = e.hash.clone();
    }
    Ok((es.len() as u64, prev))
}
