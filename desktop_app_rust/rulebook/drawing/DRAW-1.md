+++
id = "DRAW-1"
title = "Two entity types need a relationship type in between"
kind = "refused"
applies = ["entity type", "line"]
see = ["ER-5", "DRAW-3"]
+++
**Rule.** A line **must not** join two entity types directly: entity types are connected through a
relationship type.

**What the program does.** Refused while drawing: the entity type you are pointing at turns red, and
the status bar says *"Not allowed: two entities need a relationship in between (DRAW-1)"*.

- Weak entity types count as entity types; a line from an entity type to itself is refused by DRAW-3.
- A line to a junction point is allowed (see the book's introduction); a file written by hand may join
  two entity types directly anyway, which ER-5 reports.

**Why.** Entity types are related to one another only through relationship types, which carry what
the connection means: its name, the cardinalities and the roles.

**Examples.** In each, the last line is the one being drawn.

```erd refused DRAW-1
\entity{student}(0,0){Student}
\entity{course}(5,0){Course}
\link{student}{course}
```

```erd allowed
\entity{student}(0,0){Student}
\relationship{attends}(4,0){attends}
\entity{course}(8,0){Course}
\link{student}{attends}{cn}
\link{attends}{course}{cn}
```
