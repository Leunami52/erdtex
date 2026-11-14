+++
id = "REL-1"
title = "An entity type becomes a relation"
kind = "conversion"
applies = ["entity type", "attribute", "relation"]
+++
**Rule.** Each entity type **becomes** a relation of the same name, with a column for each of its
attributes; its key attributes **become** the relation's primary key.

**Conditions.** Weak entity types, subtypes, composite, multi-valued and
derived attributes have rules of their own.

**Why.** An entity type is a set of entities described by the same attributes; a relation is a set of
rows with the same columns. The key tells the entities apart, as the primary key tells the rows apart.

**Examples.**

```erd converts
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name, Birthday}
```

```relations result
Person : {(_SSN_, Name, Birthday)}
```
