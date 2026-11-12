// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! Real machine counters and a read-only localhost HTTP telemetry endpoint.
//! Nothing here is synthesized: unavailable sources are reported as `null`.

use crate::ledger::{sha256_hex, Ledger};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

/// Aggregate CPU jiffies from the first line of /proc/stat.
pub fn cpu_counters() -> Value {
    let Ok(s) = std::fs::read_to_string("/proc/stat") else { return Value::Null };
    let Some(line) = s.lines().next() else { return Value::Null };
    let names = ["user", "nice", "system", "idle", "iowait", "irq", "softirq", "steal"];
    let v: Vec<u64> = line.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
    let mut m = serde_json::Map::new();
    for (n, x) in names.iter().zip(v.iter()) { m.insert(n.to_string(), json!(x)); }
    m.insert("source".into(), json!("/proc/stat"));
    Value::Object(m)
}

pub fn meminfo() -> Value {
    let Ok(s) = std::fs::read_to_string("/proc/meminfo") else { return Value::Null };
    let get = |k: &str| s.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|x| x.parse::<u64>().ok());
    json!({"mem_total_kb": get("MemTotal:"), "mem_available_kb": get("MemAvailable:"), "source": "/proc/meminfo"})
}

/// Sum of RAPL energy counters (µJ) under /sys/class/powercap, if readable.
pub fn rapl_energy_uj() -> Option<u64> {
    let rd = std::fs::read_dir("/sys/class/powercap").ok()?;
    let mut total = None;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        // top-level packages only (intel-rapl:N), not subzones, to avoid double counting
        if name.starts_with("intel-rapl:") && name.matches(':').count() == 1 {
            if let Some(v) = std::fs::read_to_string(e.path().join("energy_uj")).ok().and_then(|s| s.trim().parse::<u64>().ok()) {
                total = Some(total.unwrap_or(0) + v);
            }
        }
    }
    total
}

/// Jetson / INA3221 style hwmon power inputs (µW), if present.
pub fn ina_power_uw() -> Option<u64> {
    let rd = std::fs::read_dir("/sys/class/hwmon").ok()?;
    let mut total = None;
    for e in rd.flatten() {
        let n = std::fs::read_to_string(e.path().join("name")).unwrap_or_default();
        if !n.to_lowercase().contains("ina") { continue; }
        for f in std::fs::read_dir(e.path()).ok()?.flatten() {
            let fname = f.file_name().to_string_lossy().to_string();
            if fname.starts_with("power") && fname.ends_with("_input") {
                if let Some(v) = std::fs::read_to_string(f.path()).ok().and_then(|s| s.trim().parse::<u64>().ok()) { total = Some(total.unwrap_or(0) + v); }
            }
        }
    }
    total
}

/// Power in watts: INA reading, else RAPL delta over `sample_ms`, else null.
pub fn power() -> Value {
    if let Some(uw) = ina_power_uw() { return json!({"watts": uw as f64 / 1e6, "source": "hwmon ina"}); }
    if let Some(a) = rapl_energy_uj() {
        let t = std::time::Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(200));
        if let Some(b) = rapl_energy_uj() {
            let dt = t.elapsed().as_secs_f64();
            return json!({"watts": (b.wrapping_sub(a)) as f64 / 1e6 / dt, "source": "powercap rapl"});
        }
    }
    Value::Null
}

pub fn lean_state_hash(path: &Path) -> Value {
    match std::fs::read(path) { Ok(b) => json!({"sha256": sha256_hex(&b), "file": path.display().to_string()}), Err(_) => Value::Null }
}

pub struct State { pub ledger: PathBuf, pub lean_check: PathBuf }

pub fn snapshot(st: &State) -> Value {
    let l = Ledger { path: st.ledger.clone() };
    let es = l.entries().unwrap_or_default();
    let last_act = es.iter().rev().find(|e| e.body.get("kind").and_then(|k| k.as_str()) == Some("activation"));
    json!({
        "lean_checked_state": lean_state_hash(&st.lean_check),
        "lease": last_act.and_then(|e| e.body.get("lease").cloned()),
        "decision": last_act.and_then(|e| e.body.get("decision").cloned()),
        "cpu": cpu_counters(),
        "mem": meminfo(),
        "power": power(),
        "ledger": {"entries": es.len(), "head": es.last().map(|e| e.hash.clone())},
        "unix_time": crate::now_unix(),
    })
}

pub fn route(st: &State, path: &str) -> (u16, Value) {
    let l = Ledger { path: st.ledger.clone() };
    match path {
        "/telemetry" => (200, snapshot(st)),
        "/ledger" => match l.entries() { Ok(es) => (200, json!(es)), Err(e) => (500, json!({"error": e})) },
        "/ledger/verify" => match l.verify() {
            Ok((n, h)) => (200, json!({"ok": true, "entries": n, "head": h})),
            Err(e) => (200, json!({"ok": false, "error": e})),
        },
        _ => (404, json!({"error": "not found", "routes": ["/telemetry", "/ledger", "/ledger/verify"]})),
    }
}

/// Serve read-only GET requests on `addr` (localhost). `max` limits the number of requests.
pub fn serve(st: &State, addr: &str, max: Option<usize>) -> std::io::Result<()> {
    let lis = TcpListener::bind(addr)?;
    eprintln!("minikran-node telemetry listening on http://{}", lis.local_addr()?);
    let mut served = 0usize;
    for s in lis.incoming() {
        let mut s = s?;
        let mut line = String::new();
        BufReader::new(&s).read_line(&mut line)?;
        let mut it = line.split_whitespace();
        let (method, path) = (it.next().unwrap_or(""), it.next().unwrap_or("/"));
        let (code, body) = if method == "GET" { route(st, path) } else { (405, json!({"error": "read-only"})) };
        let b = serde_json::to_string_pretty(&body).unwrap();
        write!(s, "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}", b.len())?;
        served += 1;
        if max.is_some_and(|m| served >= m) { break; }
    }
    Ok(())
}
