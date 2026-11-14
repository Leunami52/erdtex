+++
id = "DRAW-10"
title = "A component line starts at an aggregation or grouping"
kind = "refused"
applies = ["component line", "aggregation", "grouping"]
see = ["GSM-11"]
+++
**Rule.** In a GSM diagram, a component line **must not** start anywhere but at an aggregation or a
grouping.

**What the program does.** Refused: *"component edges start at an aggregation or grouping"*. A component
line is drawn from the aggregation or grouping to the type it is made of.

**Why.** Only aggregations and groupings are built from components (GSM-11).

**Examples.** In each, the last line is the one being drawn.

```gsmd refused DRAW-10
\abstract{car}(0,0){Car}
\representable{day}(3,0){Day}
\component{car}{day}
```

```gsmd allowed
\aggregation{date}(0,0){Date}
\representable{day}(3,0){Day}
\component{date}{day}
```
