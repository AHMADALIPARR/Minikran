// Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
// Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
// See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.

//! Parser for the `.mkr` resource spec syntax (see `example.mkr`).

use crate::gate::Lease;

#[derive(Debug, Clone, PartialEq)]
pub struct MkrFile {
    pub leases: Vec<(String, Lease)>,
    pub asserts: Vec<String>,
}

fn strip_comment(l: &str) -> &str {
    match l.find('#') { Some(i) => &l[..i], None => l }
}

pub fn parse(src: &str) -> Result<MkrFile, String> {
    let mut leases = Vec::new();
    let mut asserts = Vec::new();
    let mut cur: Option<(String, [Option<u64>; 4], Option<bool>)> = None;
    for (no, raw) in src.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() { continue; }
        let ln = no + 1;
        if let Some((name, nums, ver)) = cur.as_mut() {
            if line == "}" {
                let get = |i: usize, k: &str| nums[i].ok_or(format!("lease {name}: missing {k}"));
                let lease = Lease {
                    cpu: get(0, "cpu")?, mem: get(1, "mem")?, store: get(2, "store")?, power: get(3, "power")?,
                    verified: ver.ok_or(format!("lease {name}: missing verified"))?,
                };
                leases.push((name.clone(), lease));
                cur = None;
                continue;
            }
            let (k, v) = line.split_once(':').ok_or(format!("line {ln}: expected key: value"))?;
            let (k, v) = (k.trim(), v.trim());
            let idx = match k { "cpu" => Some(0), "mem" => Some(1), "store" => Some(2), "power" => Some(3), _ => None };
            match (idx, k) {
                (Some(i), _) => nums[i] = Some(v.parse().map_err(|_| format!("line {ln}: bad number {v}"))?),
                (None, "verified") => *ver = Some(match v { "true" => true, "false" => false, _ => return Err(format!("line {ln}: bad bool {v}")) }),
                _ => return Err(format!("line {ln}: unknown field {k}")),
            }
        } else if let Some(rest) = line.strip_prefix("lease ") {
            let name = rest.trim().strip_suffix('{').ok_or(format!("line {ln}: expected '{{'"))?.trim();
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(format!("line {ln}: bad lease name"));
            }
            cur = Some((name.to_string(), [None; 4], None));
        } else if let Some(rest) = line.strip_prefix("assert ") {
            asserts.push(rest.trim().to_string());
        } else {
            return Err(format!("line {ln}: unexpected `{line}`"));
        }
    }
    if cur.is_some() { return Err("unterminated lease block".into()); }
    Ok(MkrFile { leases, asserts })
}

/// Evaluate `assert name.within_ceiling` / `assert name.verified`.
pub fn check_asserts(f: &MkrFile) -> Result<(), String> {
    for a in &f.asserts {
        let (n, p) = a.split_once('.').ok_or(format!("bad assert {a}"))?;
        let l = f.leases.iter().find(|(x, _)| x == n).map(|(_, l)| l).ok_or(format!("unknown lease {n}"))?;
        let ok = match p { "within_ceiling" => crate::gate::within_ceiling(l), "verified" => l.verified, _ => return Err(format!("unknown property {p}")) };
        if !ok { return Err(format!("assert failed: {a}")); }
    }
    Ok(())
}
