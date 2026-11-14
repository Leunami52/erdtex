+++
id = "GSM-1"
title = "An aggregation needs at least two components"
kind = "warning"
applies = ["aggregation", "component line"]
+++
**Rule.** An aggregation **should** have at least two components: component lines (dashed arrows) from
the aggregation to the types it is made of.

**What the program does.** A warning, *"an aggregation needs at least two components (dashed
arrows)"*.

**Why.** An aggregation is a type made of several others, as a date is made of a day, a month and a
year. With a single component it would just be that component again.

**Examples.**

```gsmd violates GSM-1
\aggregation{date}(0,0){Date}
\representable{day}(3,0){Day}
\component{date}{day}
```

```gsmd follows
\aggregation{date}(0,0){Date}
\representable{day}(3,1){Day}
\representable{month}(3,-1){Month}
\component{date}{day}
\component{date}{month}
```
