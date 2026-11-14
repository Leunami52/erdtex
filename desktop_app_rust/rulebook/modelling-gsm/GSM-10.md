+++
id = "GSM-10"
title = "A subtype should have a supertype"
kind = "warning"
applies = ["subtype"]
see = ["GSM-4", "GSM-9"]
+++
**Rule.** Every subtype **should** have at least one IS-A edge (generalization or specialization) to a
supertype.

**What the program does.** A warning, *"this subtype has no IS-A edge to a supertype"*.

**Why.** A subtype is a specialisation of another type; without its IS-A edge it specialises nothing. If
it stands for objects in their own right, it is an abstract type.

**Examples.**

```gsmd violates GSM-10
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\subtype{car}(0,-3){Car}
```

```gsmd follows
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
\subtype{car}(0,-3){Car}
\generalization{car}{vehicle}
```
