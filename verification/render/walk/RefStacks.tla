---- MODULE RefStacks ----
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS FnContains, FnExitOnlyElements, BibLegacyReturns

VARIABLES fnStack, bibStack, frames, starts, reentered, underflow

Ids == 1..2
Labels == {"a", "b"}
Empty == <<>>

Range(s) == {s[i] : i \in DOMAIN s}
NoDup(s) == Cardinality(Range(s)) = Len(s)
Top == frames[Len(frames)]
Drop == SubSeq(frames, 1, Len(frames) - 1)
Replace(f) == Drop \o <<f>>

Vars == <<fnStack, bibStack, frames, starts, reentered, underflow>>

Init ==
  /\ fnStack = Empty
  /\ bibStack = Empty
  /\ frames = Empty
  /\ starts = 0
  /\ reentered = FALSE
  /\ underflow = FALSE

Pop(s) ==
  IF Len(s) = 0
  THEN [stack |-> s, under |-> TRUE]
  ELSE [stack |-> SubSeq(s, 1, Len(s) - 1), under |-> FALSE]

Start ==
  /\ frames = Empty
  /\ starts < 2
  /\ \E kind \in {"fn", "bib"},
       id \in Ids,
       label \in Labels,
       tooltip \in BOOLEAN,
       legacy \in BOOLEAN,
       child \in {"none", "self", "other"} :
       /\ kind = "bib" => tooltip = FALSE
       /\ frames' = <<[
            kind |-> kind,
            id |-> id,
            label |-> label,
            tooltip |-> tooltip,
            legacy |-> legacy,
            phase |-> "enter",
            pushed |-> FALSE,
            child |-> child
          ]>>
       /\ starts' = starts + 1
       /\ UNCHANGED <<fnStack, bibStack, reentered, underflow>>

Enter ==
  /\ frames # Empty
  /\ Top.phase = "enter"
  /\ LET f == Top
     IN IF f.kind = "fn"
        THEN IF f.tooltip
             THEN /\ frames' = Replace([f EXCEPT !.phase = "body"])
                  /\ UNCHANGED <<fnStack, bibStack, reentered, underflow>>
             ELSE IF FnContains /\ f.id \in Range(fnStack)
                  THEN /\ reentered' = TRUE
                       /\ frames' = Drop
                       /\ UNCHANGED <<fnStack, bibStack, underflow>>
                  ELSE /\ fnStack' = Append(fnStack, f.id)
                       /\ frames' = Replace([f EXCEPT !.phase = "body", !.pushed = TRUE])
                       /\ UNCHANGED <<bibStack, reentered, underflow>>
        ELSE IF f.label \in Range(bibStack)
             THEN /\ reentered' = TRUE
                  /\ frames' = Drop
                  /\ UNCHANGED <<fnStack, bibStack, underflow>>
             ELSE /\ bibStack' = Append(bibStack, f.label)
                  /\ frames' = Replace([f EXCEPT !.phase = "body", !.pushed = TRUE])
                  /\ UNCHANGED <<fnStack, reentered, underflow>>
  /\ UNCHANGED starts

Body ==
  /\ frames # Empty
  /\ Top.phase = "body"
  /\ LET f == Top
     IN IF f.child = "none" \/ f.tooltip \/ (f.kind = "bib" /\ f.legacy) \/ Len(frames) >= 3
        THEN /\ frames' = Replace([f EXCEPT !.phase = "exit", !.child = "none"])
             /\ UNCHANGED <<fnStack, bibStack, reentered, underflow>>
        ELSE /\ frames' = Replace([f EXCEPT !.phase = "wait", !.child = "none"]) \o
                  <<[
                       kind |-> f.kind,
                       id |-> IF f.child = "self" THEN f.id ELSE 3 - f.id,
                       label |-> IF f.child = "self" THEN f.label ELSE IF f.label = "a" THEN "b" ELSE "a",
                       tooltip |-> FALSE,
                       legacy |-> FALSE,
                       phase |-> "enter",
                       pushed |-> FALSE,
                       child |-> "none"
                     ]>>
             /\ UNCHANGED <<fnStack, bibStack, reentered, underflow>>
  /\ UNCHANGED starts

Resume ==
  /\ frames # Empty
  /\ Top.phase = "wait"
  /\ frames' = Replace([Top EXCEPT !.phase = "exit"])
  /\ UNCHANGED <<fnStack, bibStack, starts, reentered, underflow>>

Exit ==
  /\ frames # Empty
  /\ Top.phase = "exit"
  /\ LET f == Top
         popFn == f.kind = "fn" /\ (~f.tooltip \/ ~FnExitOnlyElements)
         fnDone == IF popFn THEN Pop(fnStack) ELSE [stack |-> fnStack, under |-> FALSE]
         extra == f.kind = "bib" /\ f.legacy /\ ~BibLegacyReturns
         first == IF f.kind = "bib" THEN Pop(bibStack) ELSE [stack |-> bibStack, under |-> FALSE]
         second == IF extra THEN Pop(first.stack) ELSE [stack |-> first.stack, under |-> FALSE]
     IN /\ fnStack' = fnDone.stack
        /\ bibStack' = second.stack
        /\ frames' = Drop
        /\ starts' = starts
        /\ reentered' = reentered
        /\ underflow' = (underflow \/ fnDone.under \/ first.under \/ second.under)

Next == Start \/ Enter \/ Body \/ Resume \/ Exit

Spec == Init /\ [][Next]_Vars

TypeOK ==
  /\ fnStack \in Seq(Ids)
  /\ bibStack \in Seq(Labels)
  /\ starts \in 0..2
  /\ reentered \in BOOLEAN
  /\ underflow \in BOOLEAN

Balanced ==
  frames = Empty => fnStack = Empty /\ bibStack = Empty

NoDupFn == NoDup(fnStack)
NoDupBib == NoDup(bibStack)
NoUnder == ~underflow
NeverReentered == ~reentered

====
