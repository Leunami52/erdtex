+++
id = "GSM-3"
title = "A line must connect existing elements"
kind = "error"
applies = ["attribute line", "multi-valued attribute line", "component line", "generalization", "specialization", "line"]
see = ["GSM-2"]
+++
**Rule.** Both ends of a line **must** be elements of the diagram.

**What the program does.** An error, *"connects to an unknown element: x"*, naming the ids that don't
exist (one or both).

- An end may name an anchor of an element, such as `car.north`; only the id before the dot counts.
- Deleting an element in the program deletes its lines with it, so this happens only in a file written or
  changed by hand.

**Why.** A line stands for an attribute, a component or an IS-A relationship between two elements; with
an end missing it relates nothing.

**Examples.**

```gsmd violates GSM-3
\abstract{vehicle}(0,0){Vehicle}
\attribute{vehicle}{plate}{has plate}
```

```gsmd follows
\abstract{vehicle}(0,0){Vehicle}
\representable{plate}(4,0){Plate}
\attribute{vehicle}{plate}{has plate}
```
