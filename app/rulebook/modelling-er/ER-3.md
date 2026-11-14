+++
id = "ER-3"
title = "A line must connect existing elements"
kind = "error"
applies = ["line"]
see = ["ER-2", "ER-4"]
+++
**Rule.** Both ends of a line **must** be elements of the diagram.

**What the program does.** An error, *"connects to an unknown element: x"*, naming the ids that don't
exist (one or both).

- An end may name an anchor of an element, such as `room.north`; only the id before the dot counts.
- Deleting an element in the program deletes its lines with it, so this happens only in a file written or
  changed by hand, for instance when an element was renamed or removed there and its lines weren't.
- Such a line isn't counted anywhere else: a relationship type with one good line and one to an unknown
  element connects fewer than two elements (ER-8).

**Why.** A line stands for a connection between two elements; with an end missing it connects nothing,
and the diagram can't be drawn as meant.

**Examples.**

```erd violates ER-3
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{attends}(4,0){attends}
\link{student}{attends}{cn}
\link{attends}{course}{cn}
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
