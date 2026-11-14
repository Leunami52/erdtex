+++
id = "GSM-2"
title = "Every id must be unique"
kind = "error"
applies = ["abstract type", "representable type", "subtype", "aggregation", "grouping", "junction point"]
see = ["GSM-3"]
+++
**Rule.** Every element that has an id **must** have one that no other element of the diagram has.

**What the program does.** An error, *"the id 'x' is used n times"*, shown once for each id that occurs
more than once, at the first element that uses it.

- The id is the element's name in the file (`\abstract{car}…`: the id is `car`), not its label. Two
  elements **may** have the same label.
- Lines, notes and subdomains have no ids and are never counted.
- Ids are compared exactly: `Car` and `car` are different ids.
- The program gives every element it creates an id of its own, so this happens only in a file written or
  changed by hand, or pasted together from several files.

**Why.** Lines refer to elements by their ids. With two elements of one id, it is undecidable which of
them a line belongs to.

**Examples.**

```gsmd violates GSM-2
\abstract{car}(0,0){Car}
\abstract{car}(5,0){Boat}
```

```gsmd follows
\abstract{car}(0,0){Car}
\abstract{boat}(5,0){Car}
```
