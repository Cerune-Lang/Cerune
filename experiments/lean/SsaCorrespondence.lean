namespace CeruneProof
-- 各経路の訪問block数の上限です。値の比較とは別に完了を要求します。
def baselineFuel : Nat := 64
def ssaFuel : Nat := 64
def loweredFuel : Nat := 128

set_option maxRecDepth 32768 in
set_option maxHeartbeats 8000000 in
theorem ssa_translation_correct : ∀ (x : Fin 256),
  evalSsa ssaReference ssaFuel x.val =
    evalMirWithFuel mirReference baselineFuel x.val := by decide

set_option maxRecDepth 32768 in
set_option maxHeartbeats 8000000 in
theorem ssa_lowering_correct : ∀ (x : Fin 256),
  evalMirWithFuel loweredReference loweredFuel x.val =
    evalSsa ssaReference ssaFuel x.val := by decide

def ssaCompleted (outcome : ExecutionOutcome) : Bool :=
  match outcome with
  | .completed _ => true
  | _ => false

set_option maxRecDepth 32768 in
set_option maxHeartbeats 8000000 in
theorem baseline_completed : ∀ (x : Fin 256),
  ssaCompleted (evalMirWithFuel mirReference baselineFuel x.val) = true := by decide

-- 完了を3回計算せず、検査済みの対応からSSAと変換後MIRへ移送します。
theorem ssa_completed : ∀ (x : Fin 256),
  ssaCompleted (evalMirWithFuel mirReference baselineFuel x.val) = true ∧
  ssaCompleted (evalSsa ssaReference ssaFuel x.val) = true ∧
  ssaCompleted (evalMirWithFuel loweredReference loweredFuel x.val) = true := by
  intro x
  rw [ssa_lowering_correct x, ssa_translation_correct x]
  exact ⟨baseline_completed x, baseline_completed x, baseline_completed x⟩

#print axioms ssa_translation_correct
#print axioms ssa_lowering_correct
#print axioms baseline_completed
#print axioms ssa_completed
end CeruneProof
