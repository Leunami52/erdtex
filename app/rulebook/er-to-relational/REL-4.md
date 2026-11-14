+++
id = "REL-4"
title = "A derived attribute gets no column"
kind = "conversion"
applies = ["attribute", "relation"]
see = ["REL-1"]
+++
**Rule.** A derived attribute **becomes** no column; where its formula is given (`/Age = {…}`), it
**becomes** a view that computes it.

**Conditions.**

- The notes of the schema say that it isn't stored.
- The view is named after the relation (*PersonView*) and contains all of it and the computed attribute;
  it appears in SQL as `CREATE VIEW`.
- A derived attribute can't be a candidate key (ER-34).

**Why.** A derived value follows from others: storing it as well would let the two disagree. A view
computes it whenever it is read, such as an age from a date of birth, which changes every year.

**Examples.**

```erd converts
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Born : date, /Age = {YEAR(NOW()) - Born}}
```

```relations result
Person : {(_SSN_, Born)}
PersonView: all of Person, and Age = YEAR(NOW()) - Born
```
