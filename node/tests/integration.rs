// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

use minikran_eval::{admission, bidding::Bid, dispatch, gate::{Decision, TimedLease}, ledger::Ledger, mkr, sig, telemetry};
use serde_json::json;
use std::path::{Path, PathBuf};

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf() }
fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("minikran-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap(); d
}

#[test]
fn parses_example_mkr() {
    let f = mkr::parse(&std::fs::read_to_string(root().join("example.mkr")).unwrap()).unwrap();
    assert_eq!(f.leases.len(), 1);
    let (n, l) = &f.leases[0];
    assert_eq!(n, "production");
    assert_eq!((l.cpu, l.mem, l.store, l.power, l.verified), (16, 32, 100, 50, true));
    assert_eq!(f.asserts, vec!["production.within_ceiling", "production.verified"]);
    mkr::check_asserts(&f).unwrap();
}

#[test]
fn rejects_bad_mkr() {
    assert!(mkr::parse("lease x {\n cpu: abc\n}").is_err());
    assert!(mkr::parse("lease x {\n cpu: 1\n").is_err());
    assert!(mkr::parse("bogus").is_err());
}

fn ledger3(name: &str) -> (PathBuf, Ledger) {
    let p = tmp(name).join("ledger.jsonl");
    let l = Ledger::open(&p).unwrap();
    for i in 0..3 { l.append(json!({"kind": "activation", "i": i})).unwrap(); }
    (p, l)
}

#[test]
fn ledger_verifies() { let (_, l) = ledger3("ok"); assert_eq!(l.verify().unwrap().0, 4); }

#[test]
fn ledger_detects_byte_flip() {
    let (p, l) = ledger3("flip");
    let mut b = std::fs::read(&p).unwrap();
    let i = b.iter().rposition(|&c| c == b'2').unwrap(); b[i] = b'7';
    std::fs::write(&p, b).unwrap();
    assert!(l.verify().is_err());
}

#[test]
fn ledger_detects_deletion() {
    let (p, l) = ledger3("del");
    let s = std::fs::read_to_string(&p).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    std::fs::write(&p, format!("{}\n{}\n{}\n", lines[0], lines[1], lines[3])).unwrap();
    assert!(l.verify().is_err());
}

#[test]
fn ledger_detects_reorder() {
    let (p, l) = ledger3("reorder");
    let s = std::fs::read_to_string(&p).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    std::fs::write(&p, format!("{}\n{}\n{}\n{}\n", lines[0], lines[2], lines[1], lines[3])).unwrap();
    assert!(l.verify().is_err());
}

#[test]
fn ledger_detects_tail_prune() {
    let (p, l) = ledger3("prune");
    let s = std::fs::read_to_string(&p).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    std::fs::write(&p, format!("{}\n{}\n{}\n", lines[0], lines[1], lines[2])).unwrap();
    let e = l.verify().unwrap_err();
    assert!(e.contains("head mismatch"), "{e}");
}

#[test]
fn signature_roundtrip_and_tamper() {
    let mut b = Bid { timed: TimedLease { lease: minikran_eval::gate::Lease { cpu: 16, mem: 32, store: 100, power: 50, verified: true }, issued: 1, ttl: 2, seq: 3, sig_ok: false }, bid_id: 4, node_id: 5, priority: 6 };
    let s = sig::sign(b"k", &b);
    assert!(sig::verify(b"k", &b, &s));
    assert!(!sig::verify(b"other", &b, &s));
    b.node_id = 6;
    assert!(!sig::verify(b"k", &b, &s));
}

/// Rust checked admission must equal expected_checked (= souffle = Lean) on every fixture.
#[test]
fn admission_matches_fixtures() {
    let dir = root().join("datalog/admission_tests");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.path().is_dir()) {
        let (c, l, b) = admission::load_dir(&e.path());
        let mut got: Vec<String> = admission::admitted(&c, &l, &b).into_iter().map(|(t, n)| format!("{t}\t{n}")).collect();
        let mut exp: Vec<String> = std::fs::read_to_string(e.path().join("expected_checked/admitted_dispatch.csv")).unwrap().lines().map(str::to_string).collect();
        got.sort(); exp.sort();
        assert_eq!(got, exp, "fixture {:?}", e.file_name());
        n += 1;
    }
    assert!(n >= 7);
}

#[test]
fn workload_verification() {
    assert!(dispatch::verify_workload(10, "sum=55\n"));
    assert!(!dispatch::verify_workload(10, "sum=56\n"));
}

#[test]
fn end_to_end_dispatch() {
    let d = tmp("e2e");
    let led = Ledger::open(&d.join("l.jsonl")).unwrap();
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_minikran-eval"));
    let f = mkr::parse(&std::fs::read_to_string(root().join("example.mkr")).unwrap()).unwrap();
    let log = dispatch::scenario(b"key", f.leases[0].1, &led, &exe, 1000).unwrap();
    let commits = log.iter().filter(|l| l.contains("=> COMMIT")).count();
    let silences = log.iter().filter(|l| l.contains("=> SILENCE")).count();
    assert_eq!((commits, silences), (2, 4), "{log:#?}");
    assert_eq!(log.iter().filter(|l| l.contains("verify compute-sum ok")).count(), 2);
    let (n, _) = led.verify().unwrap();
    assert_eq!(n, 1 + 6 + 2); // genesis + 6 activations + 2 accounting
    let _ = Decision::COMMIT;
}

#[test]
fn telemetry_endpoint_serves_json() {
    let d = tmp("tel");
    let lp = d.join("l.jsonl");
    Ledger::open(&lp).unwrap();
    let st = telemetry::State { ledger: lp, lean_check: d.join("missing.txt") };
    let lis = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = lis.local_addr().unwrap().to_string(); drop(lis);
    let a2 = addr.clone();
    let h = std::thread::spawn(move || telemetry::serve(&st, &a2, Some(2)).unwrap());
    std::thread::sleep(std::time::Duration::from_millis(200));
    use std::io::{Read, Write};
    for path in ["/telemetry", "/ledger/verify"] {
        let mut s = std::net::TcpStream::connect(&addr).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        let mut r = String::new(); s.read_to_string(&mut r).unwrap();
        let body = r.split("\r\n\r\n").nth(1).unwrap();
        let v: serde_json::Value = serde_json::from_str(body).unwrap();
        if path == "/telemetry" { assert!(v["cpu"]["user"].is_u64()); assert!(v["lean_checked_state"].is_null()); }
        else { assert_eq!(v["ok"], true); }
    }
    h.join().unwrap();
}
