/-
Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
-/

/-!
# Checked multi-node admission (executable model)

Executable mirror of `datalog/eval/minikran_admission_checked.dl` and `node/src/admission.rs`,
using the same input schema as `datalog/eval/minikran_admission.dl`. Used for the
Datalog / Rust / Lean decision cross-check (`lean-eval/AdmissionEval.lean`).
-/

namespace Minikran.Admission

structure NodeRow where
  id : String
  cpu : Int
  mem : Int
  store : Int
  power : Int

structure BidRow where
  task : Int
  node : String
  cpu : Int
  mem : Int
  store : Int
  power : Int
  prio : Int

def MAXV : Int := 2147483647

def inRange (x : Int) : Bool := decide (0 ≤ x) && decide (x ≤ MAXV)

def nodeOk (r : NodeRow) : Bool :=
  inRange r.cpu && inRange r.mem && inRange r.store && inRange r.power

def bidOk (b : BidRow) : Bool :=
  inRange b.task && inRange b.cpu && inRange b.mem && inRange b.store && inRange b.power &&
    inRange b.prio

def badInput (caps loads : List NodeRow) (bids : List BidRow) : Bool :=
  caps.any (fun r => !nodeOk r) || loads.any (fun r => !nodeOk r) ||
    bids.any (fun b => !bidOk b) ||
    bids.any (fun b => decide (1 < (bids.filter (fun c => c.task == b.task && c.node == b.node)).length)) ||
    caps.any (fun r => decide (1 < (caps.filter (fun c => c.id == r.id)).length)) ||
    loads.any (fun r => decide (1 < (loads.filter (fun c => c.id == r.id)).length))

/-- Headroom = Cap - Load, only if Load ≤ Cap on every resource. -/
def headroom (caps loads : List NodeRow) (n : String) : Option (Int × Int × Int × Int) :=
  match caps.find? (fun c => c.id == n), loads.find? (fun l => l.id == n) with
  | some c, some l =>
    if decide (l.cpu ≤ c.cpu) && decide (l.mem ≤ c.mem) && decide (l.store ≤ c.store) &&
        decide (l.power ≤ c.power) then
      some (c.cpu - l.cpu, c.mem - l.mem, c.store - l.store, c.power - l.power)
    else none
  | _, _ => none

def fitsRow (caps loads : List NodeRow) (b : BidRow) : Bool :=
  match headroom caps loads b.node with
  | some (h1, h2, h3, h4) =>
    decide (b.cpu ≤ h1) && decide (b.mem ≤ h2) && decide (b.store ≤ h3) && decide (b.power ≤ h4)
  | none => false

/-- One candidate per task: lowest node id among nodes it fits on. -/
def candidates (caps loads : List NodeRow) (bids : List BidRow) : List BidRow :=
  bids.filter fun b => fitsRow caps loads b &&
    !(bids.any fun c => c.task == b.task && fitsRow caps loads c && decide (c.node < b.node))

def prio (bids : List BidRow) (t : Int) : Int :=
  (bids.filter (fun b => b.task == t)).foldl (fun m b => max m b.prio) 0

/-- `c` is ranked at or before `b`: priority desc, then task id asc. -/
def atOrBefore (bids : List BidRow) (c b : BidRow) : Bool :=
  c.task == b.task ||
    decide (prio bids b.task < prio bids c.task) ||
    (prio bids c.task == prio bids b.task && decide (c.task < b.task))

def admitted (caps loads : List NodeRow) (bids : List BidRow) : List (Int × String) :=
  if badInput caps loads bids then [] else
  let cs := candidates caps loads bids
  cs.filterMap fun b =>
    let pre := cs.filter fun c => c.node == b.node && atOrBefore bids c b
    let s1 := (pre.map (·.cpu)).sum
    let s2 := (pre.map (·.mem)).sum
    let s3 := (pre.map (·.store)).sum
    let s4 := (pre.map (·.power)).sum
    match headroom caps loads b.node with
    | some (h1, h2, h3, h4) =>
      if decide (s1 ≤ h1) && decide (s2 ≤ h2) && decide (s3 ≤ h3) && decide (s4 ≤ h4) then
        some (b.task, b.node)
      else none
    | none => none

/-- Bad input (out of range or duplicate rows) admits nothing. -/
theorem admitted_bad_input_empty (caps loads : List NodeRow) (bids : List BidRow)
    (h : badInput caps loads bids = true) : admitted caps loads bids = [] := by
  simp [admitted, h]

end Minikran.Admission
