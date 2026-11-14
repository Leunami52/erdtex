+++
id = "DRAW-8"
title = "An IS-A edge ends at a subtype or an abstract type"
kind = "refused"
applies = ["generalization", "specialization", "subtype", "abstract type"]
see = ["GSM-5"]
+++
**Rule.** In a GSM diagram, a generalization or specialization edge **must not** end anywhere but at a
subtype or an abstract type.

**What the program does.** Refused: *"IS-A edges end at a subtype or an abstract type"*.

**Why.** Representable types, aggregations and groupings have no subtypes (GSM-5).

**Examples.** In each, the last line is the one being drawn.

```gsmd refused DRAW-8
\representable{plate}(0,0){Plate}
\subtype{car}(0,-3){Car}
\generalization{car}{plate}
```

```gsmd allowed
\abstract{vehicle}(0,0){Vehicle}
\subtype{car}(0,-3){Car}
\generalization{car}{vehicle}
```
