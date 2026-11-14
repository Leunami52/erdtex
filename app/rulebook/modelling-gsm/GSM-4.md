+++
id = "GSM-4"
title = "An IS-A edge starts at a subtype"
kind = "error"
applies = ["generalization", "specialization", "subtype"]
see = ["DRAW-7", "GSM-5", "GSM-10"]
+++
**Rule.** A generalization or specialization edge (IS-A) **must** start at a subtype (a circle) and lead
to its supertype.

**What the program does.** An error, *"an IS-A edge must start at a subtype (circle)"*, when it starts at
an abstract type, a representable type, an aggregation or a grouping.

- A junction point may be its start; the edge from the point is judged where it ends.
- While drawing, such an edge is refused (DRAW-7); the error finds it in a file written by hand.
- An abstract type is the top of an IS-A hierarchy: it is never a subtype itself. A type that specialises
  another is drawn as a subtype.

**Why.** In the model, the objects of a subtype are objects of its supertype too; only subtypes are
defined that way. An abstract type stands for objects in their own right, and a representable type for
values, which belong to no hierarchy.

**Examples.**

```gsmd violates GSM-4
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\abstract{car}(0,-3){Car}
\generalization{car}{vehicle}
```

```gsmd follows
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\subtype{car}(0,-3){Car}
\generalization{car}{vehicle}
```
