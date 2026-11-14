+++
id = "GSM-9"
title = "An IS-A hierarchy should end at an abstract type"
kind = "warning"
applies = ["subtype", "abstract type"]
see = ["GSM-8", "GSM-10"]
+++
**Rule.** The root of an IS-A hierarchy **should** be an abstract type.

**What the program does.** A warning, *"the IS-A hierarchy ends at the subtype 'x' instead of an abstract
type"*, at every subtype whose IS-A edges lead up to a subtype without IS-A edges of its own. That subtype
itself gets GSM-10.

**Why.** A subtype is defined by its supertype. A hierarchy whose top is a subtype has nothing defining
its top: the top should be an abstract type, or the top subtype should get its IS-A edge.

**Examples.**

```gsmd violates GSM-9
\subtype{car}(0,0){Car}
\subtype{taxi}(0,-3){Taxi}
\specialization{taxi}{car}
```

```gsmd follows
\abstract{car}(0,0){Car}
\subtype{taxi}(0,-3){Taxi}
\specialization{taxi}{car}
```
