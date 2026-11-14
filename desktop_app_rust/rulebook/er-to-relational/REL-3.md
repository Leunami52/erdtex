+++
id = "REL-3"
title = "A multi-valued attribute becomes a relation of its own"
kind = "conversion"
applies = ["attribute", "relation"]
see = ["REL-1", "REL-20"]
+++
**Rule.** A multi-valued attribute **becomes** a relation of its own, named after its owner and the
attribute, with the owner's key and a column for the value; both together are its primary key, and the
owner's key is a foreign key.

**Conditions.**

- A multi-valued attribute of a relationship type belongs to the relation that holds the relationship.

**Why.** A relation's column holds one value per row; several values per entity need several rows, one
for each value, each saying whose value it is.

**Examples.**

```erd converts
\entity{person}(0,0){Person}
\attributes{person}{*SSN, \{Email\}}
```

```relations result
Person : {(_SSN_)}
PersonEmail : {(_/SSN/_, _Email_)}
PersonEmail.SSN -> Person.SSN
```
