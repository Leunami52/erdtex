+++
id = "ER-1"
title = "An entity type should have a key"
kind = "hint"
applies = ["entity type", "weak entity type"]
+++
**Rule.** Every entity type **should** have a key: one or more attributes whose values tell its
entities apart.

**What the program does.** A hint, *"this entity has no key attribute"*, shown when *Show hints* is
on. No hint is given

- for a subtype of an IS_A or CAN_BE: it inherits the key of its supertype;
- for a supertype above a *complete* IS_A whose subtypes all have a key: every one of its entities
  belongs to one of the subtypes, and their keys tell them apart.

A key attribute is marked in the attribute list (the *Key* tick) or, for a Chen attribute, by its
*Key* option; it is drawn underlined.

**Why.** Without a key, two entities with the same values can't be told apart, and the entity type's
relation in the relational schema gets no primary key.

**Examples.**

```erd violates ER-1
\entity{room}(0,0){Room}
\attributes{room}{Size, Floor}
```

```erd follows
\entity{room}(0,0){Room}
\attributes{room}{*Number, Size, Floor}
```

```erd follows
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name}
\isa{isa}(0,-2){person}{co}
\entity{employee}(0,-4){Employee}
\attributes[anchor=west]{employee}{Salary}
\link{employee}{isa}
```
