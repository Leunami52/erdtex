+++
id = "GSM-5"
title = "An IS-A edge ends at a subtype or an abstract type"
kind = "error"
applies = ["generalization", "specialization", "subtype", "abstract type"]
see = ["DRAW-8", "GSM-4"]
+++
**Rule.** A generalization or specialization edge (IS-A) **must** end at a subtype or an abstract type:
the supertype.

**What the program does.** An error, *"an IS-A edge must end at a subtype or an abstract type"*, when it
ends at a representable type, an aggregation or a grouping.

- A junction point may be its end.
- While drawing, such an edge is refused (DRAW-8); the error finds it in a file written by hand.

**Why.** A supertype is a type of objects that the subtype's objects belong to as well. Representable
types are values, and aggregations and groupings are built from other types: neither has subtypes.

**Examples.**

```gsmd violates GSM-5
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\subtype{car}(0,-3){Car}
\generalization{car}{plate}
```

```gsmd follows
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\subtype{car}(0,-3){Car}
\generalization{car}{vehicle}
```
