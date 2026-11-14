+++
id = "REL-21"
title = "What the schema can't enforce"
kind = "conversion"
applies = ["relation", "relationship type"]
see = ["REL-7", "REL-8", "REL-9", "NOT-1"]
+++
**Rule.** Cardinalities a relational schema can't express **become** a list *Not enforced by the
schema*, in the text form and as comments in SQL.

**Conditions.**

- A minimum of 1 on a side that doesn't hold the foreign key: *"each Student has, in attends, Course: at
  least 1"* (a foreign key can say "exactly one" and "at most one", but not "at least one" from the
  other side).
- Bounds other than 0, 1 and many (`(2,5)`): listed with their numbers.
- A minimum that isn't known (`·`) counts as 0, with a note.
- Such conditions need triggers or checks in the application; the schema is correct without them, but
  allows more than the diagram does.

**Why.** The diagram says more than relations, keys and foreign keys can: listing it keeps that
knowledge visible instead of losing it silently.

**Examples.**

```erd converts
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*SId}
\relationship{attends}(4,0){attends}
\entity{course}(8,0){Course}
\attributes{course}{*CId}
\link{student}{attends}{cn}
\link{attends}{course}{n}
```

```relations result
Student_attends_Course : {(_/SId/_, _/CId/_)}
each Student has, in attends, Course: at least 1
```
