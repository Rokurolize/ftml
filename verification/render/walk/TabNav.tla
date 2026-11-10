---- MODULE TabNav ----
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS AbortOnDuplicate

Event ==
  [kind : {"vo"}, index : {0, 1}]
  \cup [kind : {"vc"}, index : {0}]
  \cup [kind : {"to"}, index : {0, 1, 2}]
  \cup [kind : {"tc"}, index : {0}]

Vo(i) == [kind |-> "vo", index |-> i]
Vc == [kind |-> "vc", index |-> 0]
To(i) == [kind |-> "to", index |-> i]
Tc == [kind |-> "tc", index |-> 0]

Known == <<Vo(0), To(0), Vo(1), To(1), Tc, Vc, Tc, To(2), Tc, Vc>>
Dup == <<Vo(0), To(0), Tc, Vc, Vo(0), To(1), Tc, Vc>>

Short ==
  UNION { [1..n -> Event] : n \in 0..3 }

RECURSIVE FindClose(_, _, _)
FindClose(events, consumed, close) ==
  IF close > Len(events) THEN 0
  ELSE IF events[close].kind = "tc" /\ ~consumed[close]
       THEN close
       ELSE FindClose(events, consumed, close + 1)

RECURSIVE ReadTabs(_, _, _)
ReadTabs(events, consumed, pos) ==
  IF pos > Len(events) \/ events[pos].kind # "to" \/ consumed[pos]
  THEN [flat |-> <<>>, closePos |-> 0]
  ELSE LET close == FindClose(events, consumed, pos + 1)
       IN IF close = 0
          THEN [flat |-> <<>>, closePos |-> 0]
          ELSE LET nxt == close + 1
                   rest ==
                     IF nxt <= Len(events) /\ events[nxt].kind = "to" /\ ~consumed[nxt]
                     THEN ReadTabs(events, consumed, nxt)
                     ELSE [flat |-> <<>>,
                           closePos |->
                             IF nxt <= Len(events) /\ events[nxt].kind = "vc" /\ ~consumed[nxt]
                             THEN nxt ELSE 0]
               IN [flat |-> <<[open |-> pos, close |-> close, tab |-> events[pos].index]>> \o rest.flat,
                   closePos |-> rest.closePos]

RECURSIVE Mark(_, _, _)
Mark(consumed, flat, i) ==
  IF i > Len(flat) THEN consumed
  ELSE Mark([consumed EXCEPT ![flat[i].open] = TRUE, ![flat[i].close] = TRUE], flat, i + 1)

RECURSIVE Pass(_, _, _, _, _)
Pass(events, consumed, assign, scan, changed) ==
  IF scan > Len(events)
  THEN [consumed |-> consumed, assign |-> assign, changed |-> changed, failed |-> FALSE]
  ELSE IF events[scan].kind # "vo" \/ consumed[scan]
       THEN Pass(events, consumed, assign, scan + 1, changed)
       ELSE LET read == ReadTabs(events, consumed, scan + 1)
                view == events[scan].index
            IN IF read.closePos = 0 \/ read.flat = <<>>
               THEN Pass(events, consumed, assign, scan + 1, changed)
               ELSE IF assign[view] # <<>> /\ AbortOnDuplicate
                    THEN [consumed |-> consumed, assign |-> assign, changed |-> TRUE, failed |-> TRUE]
                    ELSE LET marked == Mark(consumed, read.flat, 1)
                             taken == [marked EXCEPT ![scan] = TRUE, ![read.closePos] = TRUE]
                             tabs == [i \in 1..Len(read.flat) |-> read.flat[i].tab]
                             assigned == [assign EXCEPT ![view] = tabs]
                         IN Pass(events, taken, assigned, read.closePos + 1, TRUE)

RECURSIVE Fix(_, _, _, _)
Fix(events, consumed, assign, guard) ==
  IF guard = 0
  THEN [assign |-> assign, failed |-> TRUE]
  ELSE LET step == Pass(events, consumed, assign, 1, FALSE)
       IN IF step.failed \/ ~step.changed
          THEN [assign |-> step.assign, failed |-> step.failed]
          ELSE Fix(events, step.consumed, step.assign, guard - 1)

Blank(events) == [i \in 1..Len(events) |-> FALSE]
NoneAssign(n) == [v \in 0..(n - 1) |-> <<>>]
Rejected == [v \in {} |-> <<>>]

ResultViews(events, n) ==
  IF Len(events) = 0
  THEN Rejected
  ELSE LET done == Fix(events, Blank(events), NoneAssign(n), Len(events) + 1)
       IN IF done.failed \/ \E v \in 0..(n - 1) : done.assign[v] = <<>>
          THEN Rejected
          ELSE done.assign

VARIABLES events, result

Init ==
  /\ events \in Short \cup {Known, Dup}
  /\ result = IF events = Dup THEN ResultViews(events, 1) ELSE ResultViews(events, 2)

Next == UNCHANGED <<events, result>>
Spec == Init /\ [][Next]_<<events, result>>

TypeOK == events \in Seq(Event)

KnownPinned ==
  events = Known => result = [v \in {0, 1} |-> IF v = 0 THEN <<0>> ELSE <<1, 2>>]

DupRejected ==
  events = Dup => DOMAIN result = {}

CoversOrNone ==
  DOMAIN result = {}
  \/ (DOMAIN result = {0, 1}
      /\ \A v \in {0, 1} :
           \A i \in DOMAIN result[v] :
             \E j \in DOMAIN events : events[j].kind = "to" /\ events[j].index = result[v][i])

====
