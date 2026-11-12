// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! minikran-eval: full local Minikran node (lives in `node/` so it does not replace `rust/`).

pub mod admission;
pub mod bidding;
pub mod dispatch;
pub mod gate;
pub mod ledger;
pub mod mkr;
pub mod sig;
pub mod telemetry;

#[cfg(kani)]
mod kani_proofs;

pub fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
