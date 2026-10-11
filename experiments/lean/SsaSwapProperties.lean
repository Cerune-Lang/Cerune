-- 入替え3回の後、右側には入力値が戻ります。
namespace CeruneProof
set_option maxRecDepth 32768 in
set_option maxHeartbeats 8000000 in
theorem ssa_swap_expected : ∀ (x : Fin 256),
  evalSsa ssaReference ssaFuel x.val = .completed (.ok x.val) := by decide

theorem lowered_swap_expected (x : Fin 256) :
  evalMirWithFuel loweredReference loweredFuel x.val = .completed (.ok x.val) :=
  (ssa_lowering_correct x).trans (ssa_swap_expected x)

#print axioms ssa_swap_expected
#print axioms lowered_swap_expected
end CeruneProof
