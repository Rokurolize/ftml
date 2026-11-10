---- MODULE TabLift ----
EXTENDS Naturals, FiniteSets

CONSTANTS FlushOuter, RestoreDepth

VARIABLES schedule, which, entry, depth, pending, armed, emitted, pc

Schedules == {"single", "pair", "content"}

Init ==
  /\ schedule \in Schedules
  /\ which = 1
  /\ entry = 0
  /\ depth = 0
  /\ pending = 0
  /\ armed = FALSE
  /\ emitted = <<0, 0>>
  /\ pc = "enter"

Enter ==
  /\ pc = "enter"
  /\ entry' = depth
  /\ depth' = depth + 1
  /\ pc' = IF schedule = "content" /\ which = 1 THEN "content" ELSE "arm"
  /\ UNCHANGED <<schedule, which, pending, armed, emitted>>

Content ==
  /\ pc = "content"
  /\ pc' = "exit"
  /\ UNCHANGED <<schedule, which, entry, depth, pending, armed, emitted>>

Arm ==
  /\ pc = "arm"
  /\ armed' = (entry = 0)
  /\ pc' = "queue"
  /\ UNCHANGED <<schedule, which, entry, depth, pending, emitted>>

Queue ==
  /\ pc = "queue"
  /\ IF armed
     THEN /\ pending' = pending + 1
          /\ armed' = FALSE
     ELSE UNCHANGED <<pending, armed>>
  /\ pc' = IF entry = 0 THEN "flush" ELSE "exit"
  /\ UNCHANGED <<schedule, which, entry, depth, emitted>>

Flush ==
  /\ pc = "flush"
  /\ IF FlushOuter
     THEN /\ emitted' = [emitted EXCEPT ![which] = pending]
          /\ pending' = 0
     ELSE UNCHANGED <<emitted, pending>>
  /\ pc' = "exit"
  /\ UNCHANGED <<schedule, which, entry, depth, armed>>

Exit ==
  /\ pc = "exit"
  /\ depth' = IF RestoreDepth THEN entry ELSE depth
  /\ IF schedule = "pair" /\ which = 1
     THEN /\ which' = 2
          /\ pc' = "enter"
     ELSE /\ pc' = "done"
          /\ UNCHANGED which
  /\ UNCHANGED <<schedule, entry, pending, armed, emitted>>

Next == Enter \/ Content \/ Arm \/ Queue \/ Flush \/ Exit
Vars == <<schedule, which, entry, depth, pending, armed, emitted, pc>>
Spec == Init /\ [][Next]_Vars

TypeOK ==
  /\ pc \in {"enter", "content", "arm", "queue", "flush", "exit", "done"}
  /\ depth \in 0..3
  /\ pending \in 0..2
  /\ which \in 1..2

LiftsConsumed ==
  pc = "done" => pending = 0 /\ (schedule = "content" => emitted[1] = 0)

DepthRestored ==
  pc = "done" =>
    /\ depth = 0
    /\ schedule = "pair" => emitted[1] = 1 /\ emitted[2] = 1

NeverEmitted == emitted[1] = 0 /\ emitted[2] = 0

====
