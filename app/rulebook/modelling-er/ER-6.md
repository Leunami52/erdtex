+++
id = "ER-6"
title = "Relationship types are not linked to one another"
kind = "error"
applies = ["relationship type", "PART_OF", "line"]
see = ["DRAW-4", "ER-5", "ER-7"]
+++
**Rule.** A line **must not** join two relationship types: a relationship type connects entity types,
not other relationships.

**What the program does.** An error, *"two relationships are linked directly"*, at the line.

- PART_OF counts as a relationship type here, and weak relationship types (with E or Id) as well: a
  line between any two of them is reported.
- While drawing, such a line is refused (DRAW-4); the error finds it in a file written by hand.
- A line between a relationship type and a generalisation (IS_A, CAN_BE) is a different error: ER-7.

**Why.** A relationship relates entities. To relate a relationship to something else, the relationship
first has to become an entity type of its own (a relationship type turned into an entity type with two
relationships of its own), which then may be connected.

**Examples.**

```erd violates ER-6
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{attends}(4,0){attends}
\relationship{grades}(8,0){grades}
\link{student}{attends}{cn}
\link{attends}{grades}
```

```erd follows
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{attends}(4,0){attends}
\entity{course}(8,0){Course}
\attributes{course}{*Course Id}
\link{student}{attends}{cn}
\link{attends}{course}{cn}
```
