+++
id = "ER-5"
title = "Entity types are linked only through a relationship type"
kind = "error"
applies = ["entity type", "line"]
see = ["DRAW-1", "ER-6"]
+++
**Rule.** A line **must not** join two entity types directly: entity types are connected through a
relationship type (or a PART_OF, or a generalisation).

**What the program does.** An error, *"two entities are linked directly; a relationship is needed in
between"*, at the line; or, where the line runs through junction points, *"two entities are linked
through a junction point; a relationship is needed in between"*, at its first piece, once for each pair
of entity types.

- While drawing, such a line is refused in the first place (DRAW-1); the error finds it in a file written
  by hand, or one made by an older program.
- Weak entity types count as entity types.
- A line from an entity type to itself is such a line too.
- A junction point only shapes lines: a line from an entity type through one or more junction points to
  another entity type (or back to itself) is a direct line too.

**Why.** Entity types are related only through relationship types, which carry what the connection
means: its name, the cardinalities and the roles. A bare line between two entity types says none of it.

**Examples.**

```erd violates ER-5
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\entity{course}(5,0){Course}
\attributes{course}{*Course Id}
\link{student}{course}
```

```erd violates ER-5
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\point{p}(3,0)
\entity{course}(6,0){Course}
\attributes{course}{*Course Id}
\link{student}{p}
\link{p}{course}
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
