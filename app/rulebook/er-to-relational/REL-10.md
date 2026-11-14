+++
id = "REL-10"
title = "A relationship's attributes go where the relationship goes"
kind = "conversion"
applies = ["relationship type", "attribute", "relation"]
see = ["REL-7", "REL-8", "REL-11"]
+++
**Rule.** The attributes of a relationship type **become** columns of the relation that holds the
relationship: its own relation, or the relation that got its foreign key.

**Conditions.**

- Composite, multi-valued and derived attributes follow REL-2 to REL-4.
- Key attributes of a relationship are REL-11.

**Why.** An attribute of a relationship describes one related pair (the grade of one student in one
course); it belongs where that pair is stored.

**Examples.**

```erd converts
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*SId}
\relationship{attends}(4,0){attends}
\attributes[anchor=north]{attends}{Grade}
\entity{course}(8,0){Course}
\attributes{course}{*CId}
\link{student}{attends}{cn}
\link{attends}{course}{cn}
```

```relations result
Student_attends_Course : {(_/SId/_, _/CId/_, Grade)}
```

```erd converts
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*SId}
\relationship{advises}(4,0){advised by}
\attributes[anchor=north]{advises}{Since}
\entity{professor}(8,0){Professor}
\attributes{professor}{*PId}
\link{student}{advises}{cn}
\link{advises}{professor}{1}
```

```relations result
Student : {(_SId_, /PId/, Since)}
```
