+++
id = "REL-8"
title = "A one-to-many relationship becomes a foreign key"
kind = "conversion"
applies = ["relationship type", "PART_OF", "relation"]
see = ["REL-7", "REL-9", "REL-10"]
+++
**Rule.** A relationship type between two entity types where exactly one side is single (*each student
has one advisor*) **becomes**, by default, a foreign key in the relation of the other side, referring to
the single side's key; or, if chosen, a relation of its own.

**Conditions.**

- In n-notation the single side is the one labelled `1` or `c` (the label stands at the side counted
  from the other); the foreign key goes into the relation at the *other* end of that label's line.
- `c` at the single side makes the foreign key nullable; `1` makes it mandatory.
- As its own relation: named after both entity types and the relationship, with the many side's key as
  primary key and the single side's key as foreign key.
- PART_OF is converted the same way.
- A relationship with key attributes always gets its own relation (REL-11).

**Choices.** *Foreign key in ‹entity type›* (the default) or *Own relation*, in the
Relational Schema window (`rm=fk` or `rm=table` on the relationship type).

**Why.** Each student has one advisor: storing the advisor's key with the student says it in one place,
without a relation of its own. An own relation avoids empty foreign keys where few students have an
advisor, at the price of a join.

**Examples.**

```erd converts
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*SId}
\relationship{advises}(4,0){advised by}
\entity{professor}(8,0){Professor}
\attributes{professor}{*PId}
\link{student}{advises}{cn}
\link{advises}{professor}{1}
```

```relations result
Student : {(_SId_, /PId/)}
Professor : {(_PId_)}
Student.PId -> Professor.PId
```

```erd converts
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*SId}
\relationship[rm=table]{advises}(4,0){advised by}
\entity{professor}(8,0){Professor}
\attributes{professor}{*PId}
\link{student}{advises}{cn}
\link{advises}{professor}{1}
```

```relations result
Student : {(_SId_)}
Student_advisedBy_Professor : {(_/SId/_, /PId/)}
```
