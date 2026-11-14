+++
id = "ER-4"
title = "An attribute must belong to an existing element"
kind = "error"
applies = ["attribute list", "Chen attribute"]
see = ["ER-3", "ER-9"]
+++
**Rule.** The element an attribute list belongs to, and the element a Chen attribute names with `of=`,
**must** exist in the diagram.

**What the program does.** An error, *"belongs to the unknown element 'x'"*, at the attribute list
(`\attributes{x}{…}`) or the Chen attribute (`\attribute[of=x]…`).

- As with lines, an anchor (`x.east`) counts as its element `x`.
- The program moves and deletes attribute lists together with their element, so this happens only in a
  file written or changed by hand.
- A Chen attribute without `of=` belongs to whatever its lines connect it to; that is ER-9.

**Why.** An attribute describes the entities or relationships of one element; without that element it
describes nothing, and neither the drawing nor the relational schema can place it.

**Examples.**

```erd violates ER-4
\entity{room}(0,0){Room}
\attributes{room}{*Number}
\attributes{hall}{Size}
```

```erd violates ER-4
\entity{room}(0,0){Room}
\attributes{room}{*Number}
\attribute[of=hall]{size}(3,2){Size}
```

```erd follows
\entity{room}(0,0){Room}
\attributes{room}{*Number}
\attribute[of=room]{size}(3,2){Size}
```
