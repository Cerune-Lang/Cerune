-- 演算はMIRと共有し、値・辺・block引数の評価は独立して定義します。
namespace CeruneProof

structure SsaEdge where
  target : Nat
  arguments : List Nat

inductive SsaTerminator where
  | ret : Nat → SsaTerminator
  | jump : SsaEdge → SsaTerminator
  | branch : Nat → SsaEdge → SsaEdge → SsaTerminator

structure SsaBlock where
  arguments : List Nat
  instructions : List MirInstruction
  terminator : SsaTerminator

structure SsaFunction where
  parameter : Nat
  entry : Nat
  blocks : List SsaBlock

-- 読取りをすべて終えてから束縛します。循環する受渡しでも上書き前の値を使います。
def ssaReadArguments (arguments : List Nat) (values : MirLocals) : Option (List MirValue) :=
  arguments.mapM values

def ssaBindArguments (names : List Nat) (incoming : List MirValue)
    (values : MirLocals) : Option MirLocals :=
  match names, incoming with
  | [], [] => some values
  | name :: names, value :: incoming =>
    ssaBindArguments names incoming
      (fun index => if index = name then some value else values index)
  | _, _ => none

def ssaTransfer (f : SsaFunction) (edge : SsaEdge) (values : MirLocals) : Option MirLocals := do
  let target ← f.blocks[edge.target]?
  let incoming ← ssaReadArguments edge.arguments values
  ssaBindArguments target.arguments incoming values

def ssaRunBlocks (f : SsaFunction) (fuel blockId : Nat) (values : MirLocals) : ExecutionOutcome :=
  match fuel with
  | 0 => .exhausted
  | remaining + 1 =>
    match f.blocks[blockId]? with
    | none => .invalid
    | some block =>
      match mirExecute block.instructions values with
      | none => .invalid
      | some (.error failure) => .completed (.error failure)
      | some (.ok values) =>
        match block.terminator with
        | .ret index =>
          match values index with
          | some (.byte value) => .completed (.ok value)
          | _ => .invalid
        | .jump edge =>
          match ssaTransfer f edge values with
          | none => .invalid
          | some next => ssaRunBlocks f remaining edge.target next
        | .branch condition yes no =>
          match values condition with
          | some (.boolean b) =>
            let edge := if b then yes else no
            match ssaTransfer f edge values with
            | none => .invalid
            | some next => ssaRunBlocks f remaining edge.target next
          | _ => .invalid

def evalSsa (f : SsaFunction) (fuel input : Nat) : ExecutionOutcome :=
  ssaRunBlocks f fuel f.entry
    (fun index => if index = f.parameter then some (.byte input) else none)

end CeruneProof
