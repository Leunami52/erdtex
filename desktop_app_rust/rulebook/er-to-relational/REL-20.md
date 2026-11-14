+++
id = "REL-20"
title = "The names of relations and columns"
kind = "conversion"
applies = ["relation", "entity type", "relationship type", "attribute"]
see = ["REL-1", "REL-7", "REL-8"]
+++
**Rule.** Relations and columns are named after the diagram's labels: an entity type's relation
**becomes** its label in one word (*Airplane type* → *AirplaneType*), a relationship's relation the two
entity types and the relationship (*Student_attends_Course*), a column its attribute (*date of birth* →
*dateOfBirth*).

**Conditions.**

- *Relation name* in an element's *Element* pane (`rm name=…`) names its relation instead.
- In the Relational Schema window: *Relationship relations* named *Entity_relationship_Entity* (the
  default) or by the relationship alone; *Foreign keys* named by the key (qualified only on clashes, the
  default) or by relationship and key (*employsDeptNo*).
- Names are made SQL-safe: spaces removed, accents replaced (*Café* → *Cafe*), a leading digit
  prefixed. Two relations that would get the same name are numbered (*Course*, *Course2*); a relation
  and a view never share a name. *Names as drawn* keeps the labels' own spelling.
- A relation without a primary key is reported in the window.

**Why.** The schema should read like the diagram it comes from; where labels can't be names as they
are, the rules above make them so the same way every time.

**Examples.**

```erd converts
\entity{type}(0,0){Airplane type}
\attributes{type}{*type no, date of first flight}
```

```relations result
AirplaneType : {(_typeNo_, dateOfFirstFlight)}
```

```erd converts
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*SId}
\relationship[rm name=Enrolment]{attends}(4,0){attends}
\entity{course}(8,0){Course}
\attributes{course}{*CId}
\link{student}{attends}{cn}
\link{attends}{course}{cn}
```

```relations result
Enrolment : {(_/SId/_, _/CId/_)}
```
