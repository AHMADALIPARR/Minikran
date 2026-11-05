/-
Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
-/

import Minikran.Lease
import Mathlib.Tactic.Core

namespace Minikran

/-- Timed lease wrapping the base Lease with expiry and sequence. -/
structure TimedLease where
  base : Lease
  issuedAt : Nat
  expiresAt : Nat
  seq : Nat
  signatureOk : Bool

def isExpired (t : TimedLease) (now : Nat) : Bool :=
  now ≥ t.expiresAt

def isValid (t : TimedLease) (now : Nat) : Bool :=
  t.base.verified && t.signatureOk && !isExpired t now

/-- Gate outcome. -/
inductive GateResult where
  | COMMIT
  | SILENCE
  deriving DecidableEq, Repr

def gate (t : TimedLease) (now : Nat) (lastSeq : Nat) : GateResult :=
  if !isValid t now then .SILENCE
  else if t.seq ≤ lastSeq then .SILENCE
  else .COMMIT

/-- Expired, unverified or badly signed → SILENCE. -/
theorem invalid_to_silence (t : TimedLease) (now lastSeq : Nat)
    (h : ¬ isValid t now) :
    gate t now lastSeq = .SILENCE := by
  simp [gate, h]

/-- Replayed sequence rejected. -/
theorem replay_rejected (t : TimedLease) (now lastSeq : Nat)
    (hvalid : isValid t now) (hreplay : t.seq ≤ lastSeq) :
    gate t now lastSeq = .SILENCE := by
  simp [gate, hvalid, hreplay]

/-- Accepted sequences strictly increase. -/
theorem seq_strictly_increases (t : TimedLease) (now lastSeq : Nat)
    (h : gate t now lastSeq = .COMMIT) :
    lastSeq < t.seq := by
  simp [gate] at h
  by_cases hv : isValid t now
  · by_cases hs : t.seq ≤ lastSeq
    · simp [hv, hs] at h
    · exact Nat.lt_of_not_le hs
  · simp [hv] at h

/-- COMMIT means every check passed. -/
theorem commit_implies_all_checks (t : TimedLease) (now lastSeq : Nat)
    (h : gate t now lastSeq = .COMMIT) :
    isValid t now ∧ lastSeq < t.seq := by
  simp [gate] at h
  by_cases hv : isValid t now
  · by_cases hs : t.seq ≤ lastSeq
    · simp [hv, hs] at h
    · exact ⟨hv, Nat.lt_of_not_le hs⟩
  · simp [hv] at h

end Minikran
