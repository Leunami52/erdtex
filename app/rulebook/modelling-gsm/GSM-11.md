+++
id = "GSM-11"
title = "A component line starts at an aggregation or grouping"
kind = "error"
applies = ["component line", "aggregation", "grouping"]
see = ["DRAW-10", "GSM-1", "GSM-12"]
+++
**Rule.** A component line (a dashed arrow) **must** start at an aggregation or a grouping, and lead to
one of the types it is made of.

**What the program does.** An error, *"a component edge must start at an aggregation or grouping"*.

- A junction point may be its start.
- While drawing, such a line is refused (DRAW-10); the error finds it in a file written by hand.

**Why.** Components are what an aggregation or grouping is built from. Other types aren't built from
anything: they are related by attribute lines instead.

**Examples.**

```gsmd violates GSM-11
\abstract{car}(0,0){Car}
\representable{day}(3,0){Day}
\component{car}{day}
```

```gsmd follows
\aggregation{date}(0,0){Date}
\representable{day}(3,1){Day}
\representable{month}(3,-1){Month}
\component{date}{day}
\component{date}{month}
```
