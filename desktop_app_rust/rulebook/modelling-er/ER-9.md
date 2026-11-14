+++
id = "ER-9"
title = "A Chen attribute should belong to something"
kind = "warning"
applies = ["Chen attribute"]
see = ["ER-4", "ER-10"]
+++
**Rule.** A Chen attribute (an oval) **should** belong to an element: named with `of=`, or connected by
a line.

**What the program does.** A warning, *"this attribute is not connected to anything"*, for a Chen
attribute with neither `of=` nor any line.

- `of=` naming an element that doesn't exist is an error instead (ER-4).
- The program places a new Chen attribute by clicking its element, so this happens when that element is
  deleted while the attribute is kept, or in a file written by hand.

**Why.** An attribute describes the entities or relationships of one element; on its own it describes
nothing.

**Examples.**

```erd violates ER-9
\entity{room}(0,0){Room}
\attributes{room}{*Number}
\attribute{size}(4,2){Size}
```

```erd follows
\entity{room}(0,0){Room}
\attribute[of=room, key]{number}(-3,2){Number}
\attribute[of=room]{size}(3,2){Size}
```
