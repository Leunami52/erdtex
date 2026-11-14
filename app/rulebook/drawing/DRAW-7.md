+++
id = "DRAW-7"
title = "An IS-A edge starts at a subtype"
kind = "refused"
applies = ["generalization", "specialization", "subtype"]
see = ["GSM-4"]
+++
**Rule.** In a GSM diagram, a generalization or specialization edge **must not** start anywhere but at a
subtype (a circle).

**What the program does.** Refused: *"IS-A edges start at a subtype (circle)"*. An IS-A edge is drawn
from the subtype to its supertype.

**Why.** Only subtypes are defined by a supertype (GSM-4).

**Examples.** In each, the last line is the one being drawn.

```gsmd refused DRAW-7
\abstract{vehicle}(0,0){Vehicle}
\abstract{car}(0,-3){Car}
\generalization{car}{vehicle}
```

```gsmd allowed
\abstract{vehicle}(0,0){Vehicle}
\subtype{car}(0,-3){Car}
\generalization{car}{vehicle}
```
