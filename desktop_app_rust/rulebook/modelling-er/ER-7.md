+++
id = "ER-7"
title = "Only entity types are linked to a generalisation"
kind = "error"
applies = ["IS_A", "CAN_BE", "line"]
see = ["DRAW-4", "DRAW-5", "ER-30"]
+++
**Rule.** The lines of a generalisation (IS_A or CAN_BE) **must** lead to entity types: its subtypes.

**What the program does.** An error, *"only entities can be linked to a generalisation"*, at a line
between an IS_A or CAN_BE and anything that isn't an entity type: a relationship type, a PART_OF, a
Chen attribute or another generalisation.

- A junction point may be in between; the line to the point is not reported.
- Weak entity types count as entity types.
- The supertype is not linked by a line of its own: it is named in the generalisation itself
  (`\isa{g}(x,y){supertype}{cd}`), and the arrow to it is drawn by the program. A supertype that isn't
  an entity type is ER-30.
- While drawing, such a line is refused (DRAW-4 for a relationship type, a PART_OF or another
  generalisation, DRAW-5 for a Chen attribute); the error finds it in a file written by hand.

**Why.** A generalisation says that the entities of its subtypes are entities of the supertype too.
That only makes sense between sets of entities.

**Examples.**

```erd violates ER-7
\entity{person}(0,0){Person}
\attributes{person}{*SSN}
\isa{g}(0,-2){person}{cd}
\relationship{works}(0,-4){works}
\link{works}{g}
```

```erd follows
\entity{person}(0,0){Person}
\attributes{person}{*SSN}
\isa{g}(0,-2){person}{cd}
\entity{employee}(-2,-4){Employee}
\entity{student}(2,-4){Student}
\link{employee}{g}
\link{student}{g}
```
