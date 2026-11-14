+++
id = "DRAW-5"
title = "A Chen attribute belongs to an entity type, a relationship type or an attribute"
kind = "refused"
applies = ["Chen attribute", "line"]
see = ["ER-9", "ER-10"]
+++
**Rule.** A line from a Chen attribute (an oval) **must** lead to an entity type, a relationship type or
another Chen attribute; to a PART_OF or a generalisation it **must not**.

**What the program does.** Refused: *"attributes belong to entities or relationships"*.

- A line between two Chen attributes is allowed while drawing; the attribute it leads to then has to be
  composite (ER-10).
- Weak entity types and weak relationship types count as entity types and relationship types.

**Why.** Attributes describe entities or relationships. PART_OF and generalisations aren't sets of
things that could have values: their attributes belong to the whole, the parts, the supertype or a
subtype.

**Examples.** In each, the last line is the one being drawn.

```erd refused DRAW-5
\entity{person}(0,0){Person}
\attributes{person}{*SSN}
\isa{g}(0,-2){person}{cd}
\attribute{since}(3,-2){Since}
\link{since}{g}
```

```erd allowed
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\attribute{name}(3,2){Name}
\link{student}{name}
```
