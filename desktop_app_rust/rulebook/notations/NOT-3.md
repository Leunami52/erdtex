+++
id = "NOT-3"
title = "Crow's foot symbols"
kind = "meaning"
applies = ["line", "relationship type"]
see = ["NOT-1", "ER-14"]
+++
**Rule.** A diagram **may** draw its n-notation cardinalities as crow's foot symbols instead of labels
(*Crow's foot symbols* in the diagram's *Element* pane, or *Cardinalities drawn as* for new diagrams in
Preferences; `\setup{card symbols=crowsfoot}`). The labels stay in the file; only the drawing changes.

| Label | Symbol at the end of the line, outwards from the entity type | Means |
|---|---|---|
| `1` | two bars | exactly one |
| `c` | a bar, then a circle | at most one |
| `n` | a crow's foot, then a bar | one or more |
| `cn` | a crow's foot, then a circle | any number, also none |

**Conditions.**

- The symbol stands where the label would: at the entity type it counts, as n-notation does (NOT-1). The
  part next to the entity type says how many at most (a bar: one; a crow's foot: many), the part further
  out how few at least (a bar: one; a circle: none).
- Only the n-notation letters have symbols, older letter forms too (`m`, `nc`, …, ER-14). Other labels,
  such as `*`, `1..*` or `[2,5]`, are drawn as labels as before.
- It is for n-notation diagrams only: switching a diagram to min-max turns it off.
- Diamonds and attributes stay, so relationships with attributes and of more than two entity types are
  drawn as usual.

**Why.** Crow's foot symbols are a common way of drawing cardinalities, and some readers know them
better than letters. Drawing them from the same labels keeps one diagram readable in both ways.

**Examples.**

```erd shows
\setup{card symbols=crowsfoot}
\entity{owner}(0,0){Owner}
\attributes[anchor=west]{owner}{*Owner Id}
\relationship{owns}(4,0){owns}
\entity{dog}(8,0){Dog}
\attributes{dog}{*Dog Id}
\link{owner}{owns}{1}
\link{owns}{dog}{n}
```

```erd shows
\setup{card symbols=crowsfoot}
\entity{person}(0,0){Person}
\attributes[anchor=west]{person}{*SSN}
\relationship{has}(4,0){has}
\entity{passport}(8,0){Passport}
\attributes{passport}{*No}
\link{person}{has}{c}
\link{has}{passport}{cn}
```
