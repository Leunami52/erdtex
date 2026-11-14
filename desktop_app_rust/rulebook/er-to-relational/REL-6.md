+++
id = "REL-6"
title = "The data types of the columns"
kind = "conversion"
applies = ["attribute", "relation"]
see = ["REL-1"]
+++
**Rule.** Each column's data type **becomes** the one written with its attribute (`Born : date`), or, if
none is written, one guessed from its name.

**Conditions.**

- The guess goes by the name, the first rule that applies (letter case doesn't matter, except where
  capitals are shown):

  | The name | Type |
  |---|---|
  | starts with *is*, *has* or *can* and a capital (*isActive*) | boolean |
  | ends in *date*, *birth* or *day*, starts with *date*, or contains *birthday* | date |
  | is *from*, *to*, *since*, *until*, *start*, *end*, *begin*, *valid from*, *valid to*, or contains *timestamp* | datetime |
  | ends in *time* | time |
  | ends in *price*, *salary*, *cost*, *amount*, *total*, *balance*, *fee*, *wage*, *budget*, *bonus*, *payment* | decimal |
  | ends in *length*, *speed*, *weight*, *height*, *width*, *diameter*, *range*, *size*, *capacity*, *rate*, *distance*, *area*, *volume*, *duration*, *hours* | float |
  | ends in *no*, *nr*, *num*, *number*, *id*, *count*, *qty*, *quantity*, *seats*, *year*, *age*, *level*, *rank*, *semester*, *floor*, *room*, *code* (or *ID*, *No*, *Nr*) | integer |
  | anything else | string |

  The notes say how many types were guessed.
- The Relational Schema window lists every attribute's type, and changes it in the diagram (`name :
  type`).
- A foreign key column takes the type of the key it refers to.
- The text form shows types only when asked to; SQL always has them, in the chosen dialect.

**Why.** SQL needs a type for every column. A written type is the modeller's decision; a guess keeps the
schema complete and is marked as a guess, to be checked.

**Examples.**

```erd converts
\entity{person}(0,0){Person}
\attributes{person}{*SSN : integer, Born : date}
```

```relations result
Person : {(_SSN_, Born)}
```
