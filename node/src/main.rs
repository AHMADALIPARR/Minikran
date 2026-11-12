// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

use minikran_eval::{admission, dispatch, ledger::Ledger, mkr, telemetry};
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!("usage: minikran-eval <parse FILE.mkr | admission FACTS_DIR | dispatch FILE.mkr LEDGER [KEY] | \
ledger verify LEDGER | telemetry LEDGER LEAN_CHECK | serve LEDGER LEAN_CHECK ADDR [MAX_REQUESTS] | workload N>");
    std::process::exit(2)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |i: usize| a.get(i).cloned().unwrap_or_else(|| usage());
    match a.get(1).map(String::as_str) {
        Some("parse") => {
            let f = mkr::parse(&std::fs::read_to_string(arg(2)).expect("read")).unwrap_or_else(|e| { eprintln!("parse error: {e}"); std::process::exit(1) });
            println!("{f:?}");
            match mkr::check_asserts(&f) { Ok(()) => println!("asserts: all {} ok", f.asserts.len()), Err(e) => { println!("{e}"); std::process::exit(1) } }
        }
        Some("admission") => {
            let (c, l, b) = admission::load_dir(&PathBuf::from(arg(2)));
            for (t, n) in admission::admitted(&c, &l, &b) { println!("{t}\t{n}"); }
        }
        Some("dispatch") => {
            let f = mkr::parse(&std::fs::read_to_string(arg(2)).expect("read")).expect("parse");
            let led = Ledger::open(&PathBuf::from(arg(3))).expect("ledger");
            let key = a.get(4).cloned().unwrap_or_else(|| "minikran-demo-shared-key".into());
            let exe = std::env::current_exe().expect("exe");
            for l in dispatch::scenario(key.as_bytes(), f.leases[0].1, &led, &exe, 5_000_000).expect("dispatch") { println!("{l}"); }
            match led.verify() { Ok((n, h)) => println!("ledger verify: OK {n} entries, head {h}"), Err(e) => { println!("ledger verify: FAILED {e}"); std::process::exit(1) } }
        }
        Some("ledger") if a.get(2).map(String::as_str) == Some("verify") => {
            match (Ledger { path: PathBuf::from(arg(3)) }).verify() {
                Ok((n, h)) => println!("OK: {n} entries from genesis, head {h}"),
                Err(e) => { println!("TAMPER DETECTED: {e}"); std::process::exit(1) }
            }
        }
        Some("telemetry") => {
            let st = telemetry::State { ledger: PathBuf::from(arg(2)), lean_check: PathBuf::from(arg(3)) };
            println!("{}", serde_json::to_string_pretty(&telemetry::snapshot(&st)).unwrap());
        }
        Some("serve") => {
            let st = telemetry::State { ledger: PathBuf::from(arg(2)), lean_check: PathBuf::from(arg(3)) };
            let addr = arg(4);
            if !addr.starts_with("127.0.0.1:") && !addr.starts_with("localhost:") { eprintln!("refusing non-localhost bind"); std::process::exit(2) }
            telemetry::serve(&st, &addr, a.get(5).and_then(|x| x.parse().ok())).expect("serve");
        }
        Some("workload") => { let n: u64 = arg(2).parse().expect("n"); println!("sum={}", dispatch::workload(n)); }
        _ => usage(),
    }
}
