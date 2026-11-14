+++
id = "REL-2"
title = "A composite attribute becomes its parts"
kind = "conversion"
applies = ["attribute", "relation"]
see = ["REL-1"]
+++
**Rule.** A composite attribute **becomes** one column for each of its parts; the composite attribute
itself gets no column.

**Conditions.**

- Parts of parts become columns too.
- A column is named after its part; where two parts would get the same name (*Street* of *Home* and of
  *Work*), the later one is prefixed with its attribute's name (*workStreet*).

**Why.** A relation's columns hold single values. A composite attribute is a group of values, so its
values are stored one by one; the grouping is lost, which the prefix makes up for where needed.

**Examples.**

```erd converts
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name{First, Last}}
```

```relations result
Person : {(_SSN_, First, Last)}
```

```erd converts
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Home{Street, Town}, Work{Street, Town}}
```

```relations result
Person : {(_SSN_, Street, Town, workStreet, workTown)}
```
