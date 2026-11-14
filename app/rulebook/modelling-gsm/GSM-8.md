+++
id = "GSM-8"
title = "All IS-A paths from a subtype end at the same root"
kind = "error"
applies = ["subtype", "generalization", "specialization"]
see = ["GSM-7", "GSM-9"]
+++
**Rule.** Every path of IS-A edges from a subtype upwards **must** end at the same type, the root of its
hierarchy.

**What the program does.** An error, *"IS-A paths from this subtype end at different roots: x, y"*, naming
the roots by their ids.

- A subtype **may** have several supertypes, as long as they all lead up to the same root (a *taxi* that
  is both a *car* and a *service vehicle*, both of them vehicles).
- A root is a type with no IS-A edge of its own: normally an abstract type.

**Why.** A subtype's objects belong to one kind of thing, the root's. Two roots would make each object
two different things at once, of two unrelated types: an amphibious vehicle can be modelled under a
common root *Vehicle*, with *Car* and *Boat* as its subtypes.

**Examples.**

```gsmd violates GSM-8
\abstract{car}(0,0){Car}
\abstract{boat}(6,0){Boat}
\subtype{amphibian}(3,-3){Amphibious vehicle}
\specialization{amphibian}{car}
\specialization{amphibian}{boat}
```

```gsmd follows
\abstract{vehicle}(0,0){Vehicle}
\subtype{car}(-2,-3){Car}
\subtype{service}(2,-3){Service vehicle}
\subtype{taxi}(0,-6){Taxi}
\specialization{car}{vehicle}
\specialization{service}{vehicle}
\specialization{taxi}{car}
\specialization{taxi}{service}
```
