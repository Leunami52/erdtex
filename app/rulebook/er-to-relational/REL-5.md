+++
id = "REL-5"
title = "A further candidate key becomes UNIQUE"
kind = "conversion"
applies = ["attribute", "relation"]
see = ["ER-34", "REL-1"]
+++
**Rule.** An attribute marked as a further candidate key (*Unique*, `+Name`) **becomes** a column with
`UNIQUE`.

**Conditions.**

- It stays nullable, unless something else makes it mandatory: `UNIQUE` allows several NULL values.
- The text form lists it under *Unique (alternate keys)*.

**Why.** Only one candidate key can be the primary key; the others still identify each entity, and
`UNIQUE` keeps the database from storing two rows with the same value.

**Examples.**

```erd converts
\entity{person}(0,0){Person}
\attributes{person}{*SSN, +Passport}
```

```relations result
Person : {(_SSN_, Passport)}
Person.Passport
```
