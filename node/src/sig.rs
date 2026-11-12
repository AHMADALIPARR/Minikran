// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! HMAC-SHA256 lease signatures (orchestrator ↔ node shared key).

use crate::bidding::Bid;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type H = Hmac<Sha256>;

/// Canonical bytes covered by the signature: every lease field, ttl, seq, bid id and node id.
pub fn canonical(b: &Bid) -> String {
    let t = &b.timed; let l = &t.lease;
    format!("minikran-lease-v1|cpu={}|mem={}|store={}|power={}|verified={}|issued={}|ttl={}|seq={}|bid={}|node={}|prio={}",
        l.cpu, l.mem, l.store, l.power, l.verified, t.issued, t.ttl, t.seq, b.bid_id, b.node_id, b.priority)
}

pub fn sign(key: &[u8], b: &Bid) -> String {
    let mut m = H::new_from_slice(key).expect("hmac key");
    m.update(canonical(b).as_bytes());
    hex::encode(m.finalize().into_bytes())
}

/// Constant-time verification.
pub fn verify(key: &[u8], b: &Bid, sig_hex: &str) -> bool {
    let Ok(sig) = hex::decode(sig_hex) else { return false };
    let mut m = H::new_from_slice(key).expect("hmac key");
    m.update(canonical(b).as_bytes());
    m.verify_slice(&sig).is_ok()
}
