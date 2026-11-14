+++
id = "DRAW-9"
title = "IS-A edges don't form a cycle"
kind = "refused"
applies = ["generalization", "specialization", "subtype"]
see = ["GSM-7"]
+++
**Rule.** In a GSM diagram, an IS-A edge **must not** be drawn if its supertype already leads up, through
IS-A edges, to the subtype it starts at.

**What the program does.** Refused: *"this IS-A edge would create a cycle"*.

**Why.** In a cycle every type would be its own supertype (GSM-7).

**Examples.** In each, the last line is the one being drawn.

```gsmd refused DRAW-9
\subtype{car}(0,0){Car}
\subtype{taxi}(0,-3){Taxi}
\specialization{taxi}{car}
\specialization[bend left=40]{car}{taxi}
```

```gsmd allowed
\abstract{vehicle}(0,0){Vehicle}
\subtype{car}(0,-3){Car}
\subtype{taxi}(0,-6){Taxi}
\generalization{car}{vehicle}
\specialization{taxi}{car}
```
