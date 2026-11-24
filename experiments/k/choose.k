module CHOOSE-SYNTAX
  imports INT
  imports BOOL
  syntax Site ::= site(Int, Int, Int, Int) [symbol(site)]
  syntax Val ::= num(Int) | truth(Bool)
  syntax Exp ::= Val | "argument"
               | add(Site, Exp, Exp) [seqstrict(2,3)]
               | lt(Exp, Exp) [seqstrict]
               | andThen(Exp, Exp) [strict(1)]
  syntax Stmt ::= when(Site, Exp, Stmt, Stmt) [strict(2)]
                | ret(Site, Exp) [strict(2)]
endmodule

module CHOOSE
  imports CHOOSE-SYNTAX
  imports LIST
  syntax KResult ::= Val
  syntax KItem ::= "again" | "advance"
  syntax Observation ::= decided(Int, Bool, Site) [symbol(decided)]
                       | returned(Int, Int, Site) [symbol(returned)]
                       | integerOverflow(Int, Site) [symbol(integerOverflow)]

  configuration <state>
                  <k> again </k> <body> $PGM:Stmt </body>
                  <input> 0 </input> <events> .List </events>
                </state>

  rule <k> again => B </k> <body> B:Stmt </body> <input> I </input> requires I <Int 256
  rule <k> again => .K </k> <input> 256 </input>
  rule <k> advance => again </k> <input> I => I +Int 1 </input>

  rule <k> argument => num(I) ... </k> <input> I </input>
  rule add(_, num(A), num(B)) => num(A +Int B) requires A +Int B <=Int 255
  rule <k> (add(S, num(A), num(B)) ~> _:K) => advance </k>
       <input> I </input> <events> L => L ListItem(integerOverflow(I, S)) </events>
       requires A +Int B >Int 255
  rule lt(num(A), num(B)) => truth(A <Int B)
  rule andThen(truth(false), _) => truth(false)
  rule andThen(truth(true), R) => R

  rule <k> when(S, truth(true), T, _) => T ... </k> <input> I </input>
       <events> L => L ListItem(decided(I, true, S)) </events>
  rule <k> when(S, truth(false), _, F) => F ... </k> <input> I </input>
       <events> L => L ListItem(decided(I, false, S)) </events>
  rule <k> ret(S, num(V)) => advance </k> <input> I </input>
       <events> L => L ListItem(returned(I, V, S)) </events>
endmodule
