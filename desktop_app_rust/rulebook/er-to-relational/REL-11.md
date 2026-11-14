+++
id = "REL-11"
title = "A relationship with key attributes always gets a relation of its own"
kind = "conversion"
applies = ["relationship type", "attribute", "relation"]
see = ["REL-8", "REL-10"]
+++
**Rule.** A relationship type with key attributes **becomes** a relation of its own, whatever its
cardinalities; its key attributes join the relation's primary key.

**Conditions.**

- The primary key is the key of the many side (or both keys for many-to-many) together with the
  relationship's key attributes; the single side's key stays a foreign key only.
- The notes say so; there is no decision.

**Why.** Key attributes say that the same pair may be related several times, told apart by the key (a
student takes an exam on several dates). A foreign key holds one value per entity, so it can't.

**Examples.**

```erd converts
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*SId}
\relationship{takes}(4,0){takes}
\attributes[anchor=north]{takes}{*Date}
\entity{exam}(8,0){Exam}
\attributes{exam}{*EId}
\link{student}{takes}{cn}
\link{takes}{exam}{1}
```

```relations result
Student : {(_SId_)}
Exam : {(_EId_)}
Student_takes_Exam : {(_/SId/_, /EId/, _Date_)}
```
