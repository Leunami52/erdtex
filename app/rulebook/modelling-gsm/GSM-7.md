+++
id = "GSM-7"
title = "IS-A edges must not form a cycle"
kind = "error"
applies = ["generalization", "specialization", "subtype"]
see = ["DRAW-9", "GSM-6", "GSM-8"]
+++
**Rule.** Following IS-A edges (generalizations and specializations) from a subtype to its supertypes
**must** never lead back to where it started.

**What the program does.** An error, *"the IS-A edges form a cycle"*, reported once per diagram, at the
first subtype found on a cycle.

- While drawing, an edge that would close a cycle is refused (DRAW-9); the error finds it in a file
  written by hand.
- An edge from an element to itself is the shortest cycle (GSM-6).

**Why.** In a cycle every type would be its own supertype: car is a vehicle is a car. Such a hierarchy has
no top, so no type in it is defined.

**Examples.**

```gsmd violates GSM-7
\subtype{car}(0,0){Car}
\subtype{auto}(0,-3){Automobile}
\specialization{auto}{car}
\specialization{car}{auto}
```

```gsmd follows
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\subtype{car}(0,-3){Car}
\subtype{taxi}(0,-6){Taxi}
\generalization{car}{vehicle}
\specialization{taxi}{car}
```
