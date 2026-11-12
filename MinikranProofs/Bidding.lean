/-
Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
-/

import MinikranProofs.LeaseTTL

/-!
# Multi-node bidding, admission and deterministic scheduling

A `Bid` wraps a `TimedLease` (and therefore the unchanged `Lease`). Nodes declare
available capacity and a sliding power ceiling as a function of the time window.
-/

namespace Minikran

/-- A bid for compute: a timed lease plus the node it is bound to (covered by the
signature, see `TimedLease.sigOk`), a bid id and a priority weight. -/
structure Bid where
  timed : TimedLease
  bidId : Nat
  nodeId : Nat
  priority : Nat

/-- A node's available capacity, its scheduler weight and its sliding power ceiling. -/
structure Node where
  id : Nat
  weight : Nat
  cpu : Nat
  mem : Nat
  store : Nat
  power : Nat
  powerCeiling : Nat → Nat

/-- The bid fits on the node in time window `w`. -/
def fits (b : Bid) (n : Node) (w : Nat) : Prop :=
  b.timed.lease.cpu ≤ n.cpu ∧ b.timed.lease.mem ≤ n.mem ∧ b.timed.lease.store ≤ n.store ∧
    b.timed.lease.power ≤ n.power ∧ b.timed.lease.power ≤ n.powerCeiling w

instance (b : Bid) (n : Node) (w : Nat) : Decidable (fits b n w) := by
  unfold fits; infer_instance

/-- Admission of a bid on one node. -/
def admit (b : Bid) (n : Node) (w : Nat) : Decision :=
  if fits b n w then Decision.COMMIT else Decision.SILENCE

/-- (a) COMMIT implies every requested resource fits and power is under the sliding ceiling. -/
theorem admit_commit_fits (b : Bid) (n : Node) (w : Nat)
    (h : admit b n w = Decision.COMMIT) :
    b.timed.lease.cpu ≤ n.cpu ∧ b.timed.lease.mem ≤ n.mem ∧ b.timed.lease.store ≤ n.store ∧
      b.timed.lease.power ≤ n.power ∧ b.timed.lease.power ≤ n.powerCeiling w := by
  unfold admit at h
  split at h
  · assumption
  · cases h

/-- (b) Any overcommit yields SILENCE. -/
theorem admit_overcommit_silence (b : Bid) (n : Node) (w : Nat)
    (h : n.cpu < b.timed.lease.cpu ∨ n.mem < b.timed.lease.mem ∨ n.store < b.timed.lease.store ∨
      n.power < b.timed.lease.power ∨ n.powerCeiling w < b.timed.lease.power) :
    admit b n w = Decision.SILENCE := by
  have : ¬ fits b n w := by unfold fits; omega
  simp [admit, this]

/-- `better a c`: a is preferred over c (higher weight, then lower id). -/
def better (a c : Node) : Prop :=
  c.weight < a.weight ∨ (a.weight = c.weight ∧ a.id < c.id)

instance (a c : Node) : Decidable (better a c) := by
  unfold better; infer_instance

theorem better_irrefl (a : Node) : ¬ better a a := by unfold better; omega

theorem better_trans {a c d : Node} (h1 : better a c) (h2 : better c d) : better a d := by
  unfold better at *; omega

theorem better_total {a c : Node} (h1 : ¬ better a c) (h2 : ¬ better c a) :
    a.weight = c.weight ∧ a.id = c.id := by
  unfold better at *; omega

/-- Deterministic scheduler: the best fitting node, or none. -/
def schedule (b : Bid) (w : Nat) : List Node → Option Node
  | [] => none
  | n :: ns =>
    match schedule b w ns with
    | none => if fits b n w then some n else none
    | some m => if fits b n w then (if better n m then some n else some m) else some m

theorem schedule_none (b : Bid) (w : Nat) (ns : List Node) (h : schedule b w ns = none) :
    ∀ m ∈ ns, ¬ fits b m w := by
  induction ns with
  | nil => simp
  | cons n ns ih =>
    intro m hm
    unfold schedule at h
    cases hs : schedule b w ns with
    | none =>
      rw [hs] at h
      simp only at h
      split at h
      · cases h
      · rename_i hf
        rcases List.mem_cons.mp hm with rfl | hm
        · exact hf
        · exact ih hs m hm
    | some m' =>
      rw [hs] at h
      simp only at h
      split at h <;> (try split at h) <;> cases h

/-- (c) Scheduler soundness and canonicity: the chosen node is a member, fits, and
no fitting node is strictly better. -/
theorem schedule_sound (b : Bid) (w : Nat) (ns : List Node) (n : Node)
    (h : schedule b w ns = some n) :
    n ∈ ns ∧ fits b n w ∧ ∀ m ∈ ns, fits b m w → ¬ better m n := by
  induction ns generalizing n with
  | nil => simp [schedule] at h
  | cons x ns ih =>
    unfold schedule at h
    cases hs : schedule b w ns with
    | none =>
      rw [hs] at h
      simp only at h
      split at h
      · rename_i hf
        cases h
        refine ⟨List.mem_cons_self .., hf, ?_⟩
        intro m hm hfm
        rcases List.mem_cons.mp hm with rfl | hm
        · exact better_irrefl _
        · exact absurd hfm (schedule_none b w ns hs m hm)
      · cases h
    | some m0 =>
      rw [hs] at h
      simp only at h
      obtain ⟨hm0, hf0, hb0⟩ := ih m0 hs
      split at h
      · rename_i hfx
        split at h
        · rename_i hbx
          cases h
          refine ⟨List.mem_cons_self .., hfx, ?_⟩
          intro m hm hfm
          rcases List.mem_cons.mp hm with rfl | hm
          · exact better_irrefl _
          · intro hmn; exact hb0 m hm hfm (better_trans hmn hbx)
        · rename_i hbx
          cases h
          refine ⟨List.mem_cons_of_mem _ hm0, hf0, ?_⟩
          intro m hm hfm
          rcases List.mem_cons.mp hm with rfl | hm
          · exact hbx
          · exact hb0 m hm hfm
      · rename_i hfx
        cases h
        refine ⟨List.mem_cons_of_mem _ hm0, hf0, ?_⟩
        intro m hm hfm
        rcases List.mem_cons.mp hm with rfl | hm
        · exact absurd hfm hfx
        · exact hb0 m hm hfm

/-- (c') Uniqueness: any other canonical fitting node has the same weight and id. -/
theorem schedule_unique (b : Bid) (w : Nat) (ns : List Node) (n n' : Node)
    (h : schedule b w ns = some n) (hn' : n' ∈ ns) (hf' : fits b n' w)
    (hmax : ∀ m ∈ ns, fits b m w → ¬ better m n') :
    n'.weight = n.weight ∧ n'.id = n.id := by
  obtain ⟨hn, hf, hb⟩ := schedule_sound b w ns n h
  exact better_total (hb n' hn' hf') (hmax n hn hf)

/-- Remaining capacity after admitting a bid. -/
def consume (n : Node) (b : Bid) : Node :=
  { n with cpu := n.cpu - b.timed.lease.cpu, mem := n.mem - b.timed.lease.mem,
           store := n.store - b.timed.lease.store, power := n.power - b.timed.lease.power }

/-- Admit a sequence of bids on one node; returns remaining node and admitted bids. -/
def admitAll (w : Nat) : Node → List Bid → Node × List Bid
  | n, [] => (n, [])
  | n, b :: bs =>
    if fits b n w then
      let r := admitAll w (consume n b) bs
      (r.1, b :: r.2)
    else admitAll w n bs

def sumCpu (bs : List Bid) : Nat := (bs.map (·.timed.lease.cpu)).sum
def sumMem (bs : List Bid) : Nat := (bs.map (·.timed.lease.mem)).sum
def sumStore (bs : List Bid) : Nat := (bs.map (·.timed.lease.store)).sum
def sumPower (bs : List Bid) : Nat := (bs.map (·.timed.lease.power)).sum

/-- (d) Exact capacity conservation: admitted + remaining = initial, for every resource,
with no truncated subtraction ever taking effect. Hence admitted ≤ initial. -/
theorem admitAll_conserves (w : Nat) (bs : List Bid) :
    ∀ n : Node,
      sumCpu (admitAll w n bs).2 + (admitAll w n bs).1.cpu = n.cpu ∧
      sumMem (admitAll w n bs).2 + (admitAll w n bs).1.mem = n.mem ∧
      sumStore (admitAll w n bs).2 + (admitAll w n bs).1.store = n.store ∧
      sumPower (admitAll w n bs).2 + (admitAll w n bs).1.power = n.power := by
  induction bs with
  | nil => intro n; simp [admitAll, sumCpu, sumMem, sumStore, sumPower]
  | cons b bs ih =>
    intro n
    by_cases hf : fits b n w
    · have := ih (consume n b)
      simp only [admitAll, hf, ↓reduceIte, sumCpu, sumMem, sumStore, sumPower,
        List.map_cons, List.sum_cons] at this ⊢
      generalize admitAll w (consume n b) bs = r at this ⊢
      simp only [consume] at this
      unfold fits at hf
      omega
    · simp only [admitAll, hf, ↓reduceIte]
      exact ih n

theorem admitAll_le_initial (w : Nat) (bs : List Bid) (n : Node) :
    sumCpu (admitAll w n bs).2 ≤ n.cpu ∧ sumMem (admitAll w n bs).2 ≤ n.mem ∧
      sumStore (admitAll w n bs).2 ≤ n.store ∧ sumPower (admitAll w n bs).2 ≤ n.power := by
  have := admitAll_conserves w bs n
  omega

/-- Full decision: TTL/signature/seq gate, node binding, and admission. -/
def fullGate (b : Bid) (n : Node) (w now last : Nat) : Decision :=
  if gate b.timed now last = Decision.COMMIT ∧ b.nodeId = n.id ∧ fits b n w then
    Decision.COMMIT
  else Decision.SILENCE

/-- (e) Full COMMIT requires every condition at once. -/
theorem fullGate_commit_sound (b : Bid) (n : Node) (w now last : Nat)
    (h : fullGate b n w now last = Decision.COMMIT) :
    withinCeiling b.timed.lease = true ∧ b.timed.lease.verified = true ∧
      b.timed.sigOk = true ∧ now ≤ b.timed.issued + b.timed.ttl ∧ last < b.timed.seq ∧
      b.nodeId = n.id ∧ fits b n w := by
  unfold fullGate at h
  split at h
  · rename_i hc
    obtain ⟨hg, hid, hf⟩ := hc
    obtain ⟨h1, h2, h3, h4, h5⟩ := gate_commit_sound _ _ _ hg
    exact ⟨h1, h2, h3, h4, h5, hid, hf⟩
  · cases h

end Minikran
