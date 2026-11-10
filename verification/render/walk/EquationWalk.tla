---- MODULE EquationWalk ----
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS WalkFootnoteBodies

VARIABLES shape, cursor, index, walked, numbers, pc, fresh, rendered, numbered

Shapes == {
  [id |-> "ref-first", page |-> <<"eref", "fn", "mathP">>, needs |-> TRUE],
  [id |-> "body-then-fn", page |-> <<"mathP", "fn">>, needs |-> FALSE],
  [id |-> "fn-only", page |-> <<"fn">>, needs |-> FALSE],
  [id |-> "block-only", page |-> <<>>, needs |-> TRUE]
}

Has(name) ==
  \/ name = "a"
  \/ name = "p" /\ \E i \in DOMAIN shape.page : shape.page[i] = "mathP"

Assign(name) ==
  /\ numbers' = [numbers EXCEPT ![name] = index]
  /\ index' = index + 1
  /\ numbered' = TRUE

Init ==
  /\ shape \in Shapes
  /\ cursor = 1
  /\ index = 1
  /\ walked = FALSE
  /\ numbers = [m \in {"a", "p"} |-> 0]
  /\ pc = "pre"
  /\ fresh = 1
  /\ rendered = [m \in {"a", "p"} |-> 0]
  /\ numbered = FALSE

PreMath ==
  /\ pc = "pre"
  /\ cursor <= Len(shape.page)
  /\ shape.page[cursor] = "mathP"
  /\ cursor' = cursor + 1
  /\ Assign("p")
  /\ UNCHANGED <<shape, walked, pc, fresh, rendered>>

PreFn ==
  /\ pc = "pre"
  /\ cursor <= Len(shape.page)
  /\ shape.page[cursor] = "fn"
  /\ cursor' = cursor + 1
  /\ IF WalkFootnoteBodies /\ ~walked
     THEN /\ Assign("a")
          /\ walked' = TRUE
     ELSE UNCHANGED <<numbers, index, walked, numbered>>
  /\ UNCHANGED <<shape, pc, fresh, rendered>>

PreEref ==
  /\ pc = "pre"
  /\ cursor <= Len(shape.page)
  /\ shape.page[cursor] = "eref"
  /\ cursor' = cursor + 1
  /\ UNCHANGED <<shape, index, walked, numbers, pc, fresh, rendered, numbered>>

PreBlock ==
  /\ pc = "pre"
  /\ cursor > Len(shape.page)
  /\ IF shape.needs /\ ~walked
     THEN /\ Assign("a")
          /\ walked' = TRUE
          /\ pc' = "render"
          /\ fresh' = index + 1
     ELSE /\ pc' = "render"
          /\ fresh' = index
          /\ UNCHANGED <<numbers, index, walked, numbered>>
  /\ UNCHANGED <<shape, cursor, rendered>>

Render(name) ==
  IF numbers[name] = 0
  THEN /\ rendered' = [rendered EXCEPT ![name] = fresh]
       /\ fresh' = fresh + 1
  ELSE /\ rendered' = [rendered EXCEPT ![name] = numbers[name]]
       /\ UNCHANGED fresh

RenderA ==
  /\ pc = "render"
  /\ rendered["a"] = 0
  /\ Render("a")
  /\ UNCHANGED <<shape, cursor, index, walked, numbers, pc, numbered>>

RenderP ==
  /\ pc = "render"
  /\ Has("p")
  /\ rendered["a"] # 0
  /\ rendered["p"] = 0
  /\ Render("p")
  /\ UNCHANGED <<shape, cursor, index, walked, numbers, pc, numbered>>

Finish ==
  /\ pc = "render"
  /\ rendered["a"] # 0
  /\ rendered["p"] # 0 \/ ~Has("p")
  /\ pc' = "done"
  /\ UNCHANGED <<shape, cursor, index, walked, numbers, fresh, rendered, numbered>>

Next == PreMath \/ PreFn \/ PreEref \/ PreBlock \/ RenderA \/ RenderP \/ Finish
Vars == <<shape, cursor, index, walked, numbers, pc, fresh, rendered, numbered>>
Spec == Init /\ [][Next]_Vars

TypeOK ==
  /\ pc \in {"pre", "render", "done"}
  /\ index \in 1..4
  /\ fresh \in 1..5

NumbersAgree ==
  pc = "done" =>
    /\ numbers["a"] # 0
    /\ rendered["a"] = numbers["a"]
    /\ shape.id = "ref-first" => numbers["a"] = 1 /\ numbers["p"] = 2 /\ rendered["p"] = 2
    /\ shape.id = "body-then-fn" => numbers["p"] = 1 /\ numbers["a"] = 2 /\ rendered["p"] = 1
    /\ shape.id = "fn-only" => numbers["a"] = 1 /\ numbers["p"] = 0
    /\ shape.id = "block-only" => numbers["a"] = 1 /\ numbers["p"] = 0

NeverNumbered == ~numbered

====
