-- 利用者の仕様は生成器と別に置きます。停止位置は元ソースのreturn加算です。
namespace CeruneProof
def joinExpected (input : Nat) : Result :=
  if input < 128 then .ok (input + 3)
  else if input < 254 then .ok (input + 2)
  else .error (.integerOverflow joinFailureOrigin)

set_option maxRecDepth 32768 in
set_option maxHeartbeats 8000000 in
theorem ssa_join_expected : ∀ (x : Fin 256),
  evalSsa ssaReference ssaFuel x.val = .completed (joinExpected x.val) := by decide

theorem lowered_join_expected (x : Fin 256) :
  evalMirWithFuel loweredReference loweredFuel x.val = .completed (joinExpected x.val) :=
  (ssa_lowering_correct x).trans (ssa_join_expected x)

#print axioms ssa_join_expected
#print axioms lowered_join_expected
end CeruneProof
