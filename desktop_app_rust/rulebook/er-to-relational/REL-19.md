+++
id = "REL-19"
title = "PART_OF is converted as a relationship"
kind = "conversion"
applies = ["PART_OF", "relation"]
see = ["REL-7", "REL-8", "REL-9"]
+++
**Rule.** A PART_OF **becomes** what a relationship type with the same cardinalities becomes (REL-7 to
REL-9), named *part of*.

**Conditions.** The same decisions are offered as for a relationship type.

**Why.** A whole and its parts are related as two entity types are: the cardinalities say how many parts
a whole has and whether a part belongs to one whole.

**Examples.**

```erd converts
\entity{car}(0,0){Car}
\attributes[anchor=west]{car}{*VIN}
\partof{p}(4,0)
\entity{engine}(8,0){Engine}
\attributes{engine}{*No}
\link{car}{p}{1}
\link{p}{engine}{n}
```

```relations result
Car : {(_VIN_)}
Engine : {(_No_, /VIN/)}
```
