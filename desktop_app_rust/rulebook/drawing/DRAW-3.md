+++
id = "DRAW-3"
title = "An element isn't connected to itself"
kind = "refused"
applies = ["line"]
see = ["ER-5", "GSM-6", "ER-17"]
+++
**Rule.** A line **must not** start and end at the same element, in ER and in GSM diagrams.

**What the program does.** Refused: *"an element cannot be connected to itself"*.

- A recursive relationship (*Employee supervises Employee*) is drawn with two lines from the entity type
  to the relationship type, which is allowed; the two lines then need roles (ER-17).

**Why.** A line relates two elements; from an element to itself it relates nothing, in either kind of
diagram: an entity type linked to itself would bypass a relationship (ER-5), an IS-A edge to itself would
make a type its own supertype (GSM-6).

**Examples.** In each, the last line is the one being drawn.

```erd refused DRAW-3
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{tutors}(4,0){tutors}
\link{tutors}{tutors}
```

```erd allowed
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{tutors}(4,0){tutors}
\link[role=tutor, bend left=30]{student}{tutors}{cn}
\link[role=pupil, bend right=30]{student}{tutors}{cn}
```

```gsmd refused DRAW-3
\subtype{car}(0,0){Car}
\specialization{car}{car}
```
