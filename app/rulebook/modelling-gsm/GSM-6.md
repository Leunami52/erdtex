+++
id = "GSM-6"
title = "An IS-A edge doesn't lead from an element to itself"
kind = "error"
applies = ["generalization", "specialization"]
see = ["DRAW-3", "GSM-7"]
+++
**Rule.** A generalization or specialization edge **must not** start and end at the same element.

**What the program does.** An error, *"an IS-A edge from an element to itself"*. Such an edge is also a
cycle, so GSM-7 is reported for it too.

- While drawing, a line from an element to itself is refused (DRAW-3).

**Why.** "Every car is a car" says nothing; as an IS-A edge it would make the type its own supertype.

**Examples.**

```gsmd violates GSM-6
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\subtype{car}(0,-3){Car}
\generalization{car}{vehicle}
\specialization{car}{car}
```

```gsmd follows
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\subtype{car}(0,-3){Car}
\generalization{car}{vehicle}
```
