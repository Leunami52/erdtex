+++
id = "NOT-1"
title = "Cardinality notations"
kind = "meaning"
applies = ["relationship type", "line"]
see = ["NOT-3", "ER-14", "ER-16"]
+++
**Rule.** A cardinality says how often an entity takes part in a relationship. Two notations are
understood; a label's form says which one it is in, so a diagram **may** use either, and the program
converts between them (*Element* pane of the diagram or of a relationship type).

| n-notation | written at | min-max | written at | means |
|---|---|---|---|---|
| `1` | the **other** end | `(1,1)` | the entity type **counted** | exactly one |
| `c` | the other end | `(0,1)` | the entity type counted | at most one |
| `n` (also `m`, `k`, `p`, `q`) | the other end | `(1,*)` | the entity type counted | one or more |
| `cn` (also `mc`, …) | the other end | `(0,*)` | the entity type counted | any number, also none |
| | | `(2,5)`, `(3,*)`, … | the entity type counted | between, or at least, those numbers |

Also understood in n-notation, as older forms that a hint offers to rewrite (ER-14): `m`, `k`, `p`, `q`
and `+`, `1..*` for `n`; `mc`, `nc`, `cm` and `*`, `0..*` for `cn`; `?` and `0..1` for `c`. Ranges in square brackets (`[2,5]`, `[1,*]`) are read
as n-notation is, from the other end; they have no n-notation letter (ER-14). In min-max, `·` stands for
a minimum that isn't known.

**The same relationship in both notations.** Each dog has exactly one owner; an owner has one dog or
more. In n-notation the `1` stands at the owner, counting the owners of each dog; in min-max the
`(1,1)` stands at the dog, counting how often each dog takes part. (The second diagram says
`\setup{card notation=minmax}`; otherwise the program would hint that it uses another notation than
the diagram, ER-16.)

```erd follows
\entity{owner}(0,0){Owner}
\attributes[anchor=west]{owner}{*Owner Id}
\relationship{owns}(4,0){owns}
\entity{dog}(8,0){Dog}
\attributes{dog}{*Dog Id}
\link{owner}{owns}{1}
\link{owns}{dog}{n}
```

```erd follows
\setup{card notation=minmax}
\entity{owner}(0,0){Owner}
\attributes[anchor=west]{owner}{*Owner Id}
\relationship{owns}(4,0){owns}
\entity{dog}(8,0){Dog}
\attributes{dog}{*Dog Id}
\link{owner}{owns}{(1,*)}
\link{owns}{dog}{(1,1)}
```

**Relationships of more than two entity types.** There the two notations say different things: in
min-max, how often each entity takes part at all; in n-notation, how many entities of one type there
are for each *combination* of the others. They can't be converted into one another.

**Why.** Both notations are in common use. Reading a label by its form keeps diagrams in either one
valid without any setting; the diagram's notation setting only decides what the editor offers.
