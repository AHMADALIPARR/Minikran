/-
Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
-/

import Minikran.Lease

/-!
# Timed leases, expiry, signature validity and replay protection

Extends `Minikran.Lease` (unchanged) with a TTL, a sequence number and a
signature-validity flag, and defines the COMMIT/SILENCE gate.
-/

namespace Minikran

/-- A lease wrapped with issue time, time-to-live, sequence number and signature validity. -/
structure TimedLease where
  lease : Lease
  issued : Nat
  ttl : Nat
  seq : Nat
  sigOk : Bool

/-- Gate decision. -/
inductive Decision where
  | COMMIT
  | SILENCE
  deriving DecidableEq, Repr

/-- Structural ceilings, the same bounds as `production_lease_within_ceiling`. -/
def withinCeiling (l : Lease) : Bool :=
  decide (l.cpu ≤ 16) && decide (l.mem ≤ 32) && decide (l.store ≤ 100) && decide (l.power ≤ 50)

/-- A timed lease is expired at `now` iff `now > issued + ttl`. -/
def expired (t : TimedLease) (now : Nat) : Bool :=
  decide (t.issued + t.ttl < now)

/-- The gate: COMMIT only if within ceiling, verified, signed, unexpired and `seq > last`. -/
def gate (t : TimedLease) (now last : Nat) : Decision :=
  if withinCeiling t.lease && t.lease.verified && t.sigOk && !(expired t now)
      && decide (last < t.seq) then
    Decision.COMMIT
  else
    Decision.SILENCE

/-- (a) An expired lease is silenced. -/
theorem gate_expired_silence (t : TimedLease) (now last : Nat)
    (h : now > t.issued + t.ttl) : gate t now last = Decision.SILENCE := by
  have : expired t now = true := by simp [expired]; omega
  simp [gate, this]

/-- (b1) An unverified lease is silenced. -/
theorem gate_unverified_silence (t : TimedLease) (now last : Nat)
    (h : t.lease.verified = false) : gate t now last = Decision.SILENCE := by
  simp [gate, h]

/-- (b2) A lease with an invalid signature is silenced. -/
theorem gate_unsigned_silence (t : TimedLease) (now last : Nat)
    (h : t.sigOk = false) : gate t now last = Decision.SILENCE := by
  simp [gate, h]

/-- (c) COMMIT implies every precondition. -/
theorem gate_commit_sound (t : TimedLease) (now last : Nat)
    (h : gate t now last = Decision.COMMIT) :
    withinCeiling t.lease = true ∧ t.lease.verified = true ∧ t.sigOk = true ∧
      now ≤ t.issued + t.ttl ∧ last < t.seq := by
  unfold gate at h
  split at h
  · rename_i hc
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hc
    obtain ⟨⟨⟨⟨h1, h2⟩, h3⟩, h4⟩, h5⟩ := hc
    simp [expired] at h4
    exact ⟨h1, h2, h3, h4, h5⟩
  · cases h

/-- COMMIT implies ceiling bounds on every resource. -/
theorem gate_commit_ceiling (t : TimedLease) (now last : Nat)
    (h : gate t now last = Decision.COMMIT) :
    t.lease.cpu ≤ 16 ∧ t.lease.mem ≤ 32 ∧ t.lease.store ≤ 100 ∧ t.lease.power ≤ 50 := by
  have := (gate_commit_sound t now last h).1
  simp [withinCeiling] at this
  omega

/-- (d1) A replayed lease (`seq ≤ last`) is rejected. -/
theorem gate_replay_silence (t : TimedLease) (now last : Nat)
    (h : t.seq ≤ last) : gate t now last = Decision.SILENCE := by
  have : ¬ last < t.seq := by omega
  simp [gate, this]

/-- Run a list of `(lease, now)` requests; returns the accepted sequence numbers. -/
def run : Nat → List (TimedLease × Nat) → List Nat
  | _, [] => []
  | last, r :: rs =>
    if gate r.1 r.2 last = Decision.COMMIT then r.1.seq :: run r.1.seq rs else run last rs

/-- Every accepted seq is strictly greater than the starting `last`. -/
theorem run_gt (rs : List (TimedLease × Nat)) :
    ∀ (last x : Nat), x ∈ run last rs → last < x := by
  induction rs with
  | nil => intro last x hx; simp [run] at hx
  | cons r rs ih =>
    intro last x hx
    by_cases hg : gate r.1 r.2 last = Decision.COMMIT
    · have hs := (gate_commit_sound _ _ _ hg).2.2.2.2
      simp [run, hg] at hx
      rcases hx with rfl | hx
      · exact hs
      · exact Nat.lt_trans hs (ih _ _ hx)
    · simp [run, hg] at hx
      exact ih _ _ hx

/-- (d2) The accepted seq sequence is strictly increasing over any run. -/
theorem run_strictly_monotone (rs : List (TimedLease × Nat)) :
    ∀ last : Nat, (run last rs).Pairwise (· < ·) := by
  induction rs with
  | nil => intro last; simp [run]
  | cons r rs ih =>
    intro last
    by_cases hg : gate r.1 r.2 last = Decision.COMMIT
    · simp only [run, hg, ↓reduceIte]
      exact List.Pairwise.cons (fun x hx => run_gt rs _ x hx) (ih _)
    · simp only [run, hg, ↓reduceIte]
      exact ih _

/-- (e) `productionLease`, signed, with any TTL window and a fresh seq, commits inside the window. -/
theorem production_timed_commits (issued ttl seq last now : Nat)
    (hseq : last < seq) (hnow : now ≤ issued + ttl) :
    gate ⟨productionLease, issued, ttl, seq, true⟩ now last = Decision.COMMIT := by
  have hne : ¬ issued + ttl < now := by omega
  simp [gate, withinCeiling, expired, productionLease, hseq, hne]

end Minikran
