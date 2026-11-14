+++
id = "NOT-6"
title = "The symbols of GSM diagrams"
kind = "meaning"
applies = ["abstract type", "representable type", "subtype", "aggregation", "grouping", "attribute line", "multi-valued attribute line", "component line", "generalization", "specialization"]
see = ["NOT-2", "NOT-4", "NOT-5"]
+++
**Rule.** Each element of a GSM diagram has its own symbol; the kind of line between two types says how
they are related.

| Symbol | Element | Means |
|---|---|---|
| triangle | abstract type | objects in their own right |
| grey box | representable type | values |
| circle | subtype | objects of another type, with more to say about them |
| circle with a cross, its name beside it | aggregation | a type made of several others |
| circle with a cross and a bar | grouping | sets of objects of one type |
| arrow | attribute line | each object has one value of the type it points to |
| arrow with two heads | multi-valued attribute line | each object has a set of such values |
| dashed arrow | component line | from an aggregation or grouping to what it is made of |
| thick arrow | generalization | from a subtype to its supertype; the subtypes are disjoint |
| double arrow | specialization | from a subtype to its supertype; the subtypes may overlap |

**Conditions.**

- An attribute line's label names the attribute; a part in square brackets marks it as derived
  (*height [measured]*).
- A label at a subtype's IS-A edge in square brackets says how its objects are chosen (*[derived]*).

**Why.** GSM tells objects from values by their shape, and says by the line alone whether a value is
single, a set, a part or a supertype; converting it to ER reads exactly these (G2E-1).

**Examples.**

```gsmd shows
\abstract{person}(0,0){Person}
\representable{name}(4,1.5){Name}
\attribute{person}{name}{has name}
\grouping{langs}(4,-1.5){Languages}
\representable{lang}(8,-1.5){Language}
\component{langs}{lang}
\attribute{person}{langs}{speaks}
```

```gsmd shows
\abstract{vehicle}(0,0){Vehicle}
\subtype{car}(-2.5,-3){Car}
\subtype{bike}(2.5,-3){Bike}
\generalization{car}{vehicle}
\generalization{bike}{vehicle}
\aggregation{date}(8,0){Date}
\representable{day}(6.5,-3){Day}
\representable{month}(9.5,-3){Month}
\component{date}{day}
\component{date}{month}
```
