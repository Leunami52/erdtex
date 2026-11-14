+++
id = "REL-14"
title = "A relationship of more than two entity types (min-max)"
kind = "conversion"
applies = ["relationship type", "relation"]
see = ["REL-13", "NOT-1"]
+++
**Rule.** A relationship type of three or more entity types, in min-max notation, **becomes** a relation
of its own; a side whose entities take part at most once (`(0,1)`, `(1,1)`) **may** instead hold the
relationship as foreign keys to the others.

**Conditions.**

- In min-max, a label counts how often each entity of its side takes part at all. A maximum of 1 at a side
  makes that side's key identify the relationship: it becomes the primary key of the own relation.
- With such a side, the choice is between the own relation (the default) and foreign keys in that side's
  relation.
- A minimum of 1 can't be enforced: it is listed (REL-21).

**Choices.** *Own relation, key: ‹entity type›* or *Foreign keys in ‹entity type›*, in the Relational
Schema window.

**Why.** Min-max and n-notation say different things about relationships of more than two (NOT-1):
here every label is about its own side alone, so a side taking part at most once is what decides.

**Examples.**

```erd converts
\setup{card notation=minmax}
\entity{lecturer}(0,0){Lecturer}
\attributes[anchor=west]{lecturer}{*LId}
\relationship{teaches}(4,0){teaches}
\entity{course}(8,0){Course}
\attributes{course}{*CId}
\entity{room}(4,-3.5){Room}
\attributes{room}{*RId}
\link{lecturer}{teaches}{(0,*)}
\link{teaches}{course}{(0,*)}
\link{teaches}{room}{(1,1)}
```

```relations result
teaches : {(/LId/, /CId/, _/RId/_)}
```
