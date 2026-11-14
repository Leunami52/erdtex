+++
id = "ER-2"
title = "Every id must be unique"
kind = "error"
applies = ["entity type", "relationship type", "Chen attribute", "IS_A", "CAN_BE", "PART_OF", "junction point"]
see = ["ER-3"]
+++
**Rule.** Every element that has an id **must** have one that no other element of the diagram has.

**What the program does.** An error, *"the id 'x' is used n times"*, shown once for each id that occurs
more than once, at the first element that uses it.

- The id is the element's name in the file (`\entity{room}…`: the id is `room`), not its label. Two
  elements **may** have the same label, such as two relationship types both called *has*.
- Attribute lists, lines, notes and subdomains have no ids and are never counted.
- Ids are compared exactly: `Room` and `room` are different ids.
- The program gives every element it creates an id of its own, so this happens only in a file written or
  changed by hand, or pasted together from several files.

**Why.** Lines, attribute lists and generalisations refer to elements by their ids. With two elements
of one id, it is undecidable which of them a line belongs to.

**Examples.**

```erd violates ER-2
\entity{room}(0,0){Room}
\attributes{room}{*Number}
\entity{room}(5,0){Hall}
\attributes[anchor=west]{room}{*Name}
```

```erd follows
\entity{room}(0,0){Room}
\attributes{room}{*Number}
\entity{hall}(5,0){Room}
\attributes{hall}{*Name}
```
