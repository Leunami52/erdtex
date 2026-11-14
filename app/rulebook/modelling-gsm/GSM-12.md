+++
id = "GSM-12"
title = "A grouping has exactly one component"
kind = "warning"
applies = ["grouping", "component line"]
see = ["DRAW-11", "GSM-1"]
+++
**Rule.** A grouping **should** have exactly one component: the type whose sets it groups.

**What the program does.** A warning, *"a grouping needs exactly one component (dashed arrow), it has
n"*, for none as well as for two or more.

- While drawing, a second component line from a grouping is refused (DRAW-11).

**Why.** A grouping stands for sets of objects of one type, such as a *team* as a set of *players*. With
no component it groups nothing; with two it would be an aggregation.

**Examples.**

```gsmd violates GSM-12
\grouping{team}(0,0){Team}
```

```gsmd follows
\grouping{team}(0,0){Team}
\abstract{player}(4,0){Player}
\component{team}{player}
```
