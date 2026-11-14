+++
id = "DRAW-11"
title = "A grouping has only one component"
kind = "refused"
applies = ["grouping", "component line"]
see = ["GSM-12"]
+++
**Rule.** In a GSM diagram, a second component line **must not** be drawn from a grouping.

**What the program does.** Refused: *"a grouping has exactly one component"*.

**Why.** A grouping stands for sets of objects of one type (GSM-12); a second component would make it an
aggregation.

**Examples.** In each, the last line is the one being drawn.

```gsmd refused DRAW-11
\grouping{team}(0,0){Team}
\abstract{player}(4,1){Player}
\abstract{coach}(4,-1){Coach}
\component{team}{player}
\component{team}{coach}
```

```gsmd allowed
\grouping{team}(0,0){Team}
\abstract{player}(4,0){Player}
\component{team}{player}
```
