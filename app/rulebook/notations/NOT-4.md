+++
id = "NOT-4"
title = "Generalisation letters"
kind = "meaning"
applies = ["IS_A", "CAN_BE", "generalization", "specialization"]
see = ["ER-33", "REL-17", "NOT-2"]
+++
**Rule.** The letters at an IS_A or CAN_BE say whether the generalisation is complete and whether it is
disjoint; a diagram **may** show them as letters or as words.

| Letters | Words | Every entity of the supertype is in a subtype | An entity is in at most one subtype |
|---|---|---|---|
| `cd` | complete, disjoint | yes | yes |
| `co` | complete, overlapping | yes | no |
| `id` | incomplete, disjoint | no | yes |
| `io` | incomplete, overlapping | no | no |

**Conditions.**

- One letter alone says one of the two and leaves the other open (`c`, `i`, `d`, `o`); nothing at all
  leaves both open (ER-33).
- Words are understood too, in any order and with any punctuation: *complete*, *total*, *incomplete*,
  *partial*, *disjoint*, *overlapping*, and their German forms (*vollständig*, *partiell*, *disjunkt*,
  *überlappend*). They are kept as written.
- *IS_A type shown as* in the diagram's *Element* pane (also in Preferences, for new diagrams;
  `\setup{constraint style=words}`) draws the letters as words, beside the symbol, since words don't fit
  inside it.
- IS_A and CAN_BE mean the same to the checks and conversions, except that only a complete IS_A lets a
  supertype be identified by its subtypes' keys (ER-1).
- In GSM diagrams the kind of IS-A edge says it instead: a **generalization** is disjoint, a
  **specialization** may overlap; GSM says nothing about completeness.
- The letters decide which relational representations are possible (REL-17).

**Why.** Completeness and disjointness decide what a generalisation means and how it can be stored. Two
letters say it compactly, the words say it plainly; either reads the same way.

**Examples.**

```erd shows
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name}
\isa{g}(0,-2){person}{cd}
\entity{employee}(-2.5,-4){Employee}
\attributes[anchor=west]{employee}{Salary}
\entity{student}(2.5,-4){Student}
\attributes{student}{Field}
\link{employee}{g}
\link{student}{g}
```

```erd shows
\setup{constraint style=words}
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name}
\isa{g}(0,-2){person}{io}
\entity{employee}(-2.5,-4){Employee}
\attributes[anchor=west]{employee}{Salary}
\entity{student}(2.5,-4){Student}
\attributes{student}{Field}
\link{employee}{g}
\link{student}{g}
```
