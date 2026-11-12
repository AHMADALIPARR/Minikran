/-
Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
-/

import MinikranProofs.Admission
open Minikran.Admission

/-- Read a tab-separated facts file into rows of fields. -/
def readRows (path : System.FilePath) : IO (List (List String)) := do
  if ← path.pathExists then
    let s ← IO.FS.readFile path
    return (s.splitOn "\n").filter (· ≠ "") |>.map (·.splitOn "\t")
  else return []

def int! (s : String) : Int := s.toInt?.getD (-1)

def nodeRow : List String → Option NodeRow
  | [i, a, b, c, d] => some ⟨i, int! a, int! b, int! c, int! d⟩
  | _ => none

def bidRow : List String → Option BidRow
  | [t, n, a, b, c, d, p] => some ⟨int! t, n, int! a, int! b, int! c, int! d, int! p⟩
  | _ => none

def main (args : List String) : IO Unit := do
  let dir : System.FilePath := args.headD "."
  let caps := (← readRows (dir / "node_capacity.facts")).filterMap nodeRow
  let loads := (← readRows (dir / "current_load.facts")).filterMap nodeRow
  let bids := (← readRows (dir / "task_bid.facts")).filterMap bidRow
  for (t, n) in admitted caps loads bids do
    IO.println s!"{t}\t{n}"
