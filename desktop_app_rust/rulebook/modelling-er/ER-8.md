+++
id = "ER-8"
title = "A relationship type connects at least two elements"
kind = "warning"
applies = ["relationship type", "weak relationship type", "PART_OF"]
see = ["ER-17"]
+++
**Rule.** A relationship type, and a PART_OF, **should** have at least two lines.

**What the program does.** A warning, *"this relationship connects fewer than two elements"*.

- What is counted is lines, not different elements: a recursive relationship type with two lines to
  the same entity type (*supervises*, from *Employee* to *Employee*) has two and is fine. Its two lines
  then need roles (ER-17).
- Every line at the relationship counts, also one to a Chen attribute.
- A line whose other end doesn't exist is not counted (ER-3).

**Why.** A relationship relates entities: with a single line it relates an entity type to nothing, and
with none it is left over from editing.

**Examples.**

```erd violates ER-8
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{attends}(4,0){attends}
\link{student}{attends}{cn}
```

```erd follows
\entity{employee}(0,0){Employee}
\attributes[anchor=west]{employee}{*SSN}
\relationship{supervises}(5,0){supervises}
\link[role=boss, bend left=30]{employee}{supervises}{cn}
\link[role=staff, bend right=30]{employee}{supervises}{c}
```
