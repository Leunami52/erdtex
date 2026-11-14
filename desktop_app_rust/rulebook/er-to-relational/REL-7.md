+++
id = "REL-7"
title = "A many-to-many relationship becomes a relation of its own"
kind = "conversion"
applies = ["relationship type", "relation"]
see = ["REL-8", "REL-10"]
+++
**Rule.** A relationship type between two entity types where neither side is single (`cn`:`cn`,
`n`:`n`, `cn`:`n` …) **becomes** a relation named after the two entity types and the relationship,
with both entity types' keys as its primary key, each a foreign key.

**Conditions.**

- A side is *single* when its cardinality allows at most one (`1`, `c`; `(0,1)`, `(1,1)`), as seen from
  the other side.
- A minimum of 1 (`n`) can't be enforced by the schema: it is listed under *Not enforced by the schema*
  (REL-21).
- The relationship's attributes become columns of this relation (REL-10).
- There is no decision.

**Why.** Each pair of related entities is one row: neither entity's relation could hold the pair, since
each entity may be related to many of the other kind.

**Examples.**

```erd converts
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*SId}
\relationship{attends}(4,0){attends}
\entity{course}(8,0){Course}
\attributes{course}{*CId}
\link{student}{attends}{cn}
\link{attends}{course}{cn}
```

```relations result
Student : {(_SId_)}
Course : {(_CId_)}
Student_attends_Course : {(_/SId/_, _/CId/_)}
```
