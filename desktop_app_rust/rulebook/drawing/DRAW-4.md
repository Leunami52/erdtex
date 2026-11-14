+++
id = "DRAW-4"
title = "Relationship types and generalisations aren't linked to one another"
kind = "refused"
applies = ["relationship type", "PART_OF", "IS_A", "CAN_BE", "line"]
see = ["ER-6", "ER-7"]
+++
**Rule.** In an ER diagram, a line **must not** join two of these: relationship types (weak or not),
PART_OF, IS_A and CAN_BE.

**What the program does.** Refused: *"relationships cannot be linked to each other"*.

- This includes a line from a generalisation to a relationship type or to another generalisation: the
  lines of a generalisation lead to entity types only.

**Why.** All four relate entity types: a relationship type its participants, PART_OF a whole and its
parts, a generalisation a supertype and its subtypes. None of them relates other relationships (ER-6,
ER-7).

**Examples.** In each, the last line is the one being drawn.

```erd refused DRAW-4
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{attends}(4,0){attends}
\relationship{grades}(8,0){grades}
\link{student}{attends}{cn}
\link{attends}{grades}
```

```erd refused DRAW-4
\entity{person}(0,0){Person}
\attributes{person}{*SSN}
\isa{g}(0,-2){person}{cd}
\relationship{works}(4,-2){works}
\link{works}{g}
```

```erd allowed
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{attends}(4,0){attends}
\entity{course}(8,0){Course}
\link{student}{attends}{cn}
\link{attends}{course}{cn}
```
