+++
id = "REL-13"
title = "A relationship of more than two entity types (n-notation)"
kind = "conversion"
applies = ["relationship type", "relation"]
see = ["REL-14", "NOT-1"]
+++
**Rule.** A relationship type of three or more entity types, in n-notation, **becomes** a relation of
its own with all their keys; its primary key depends on the sides marked single (`1` or `c`).

**Conditions.**

- No single side: the primary key is all the keys.
- One single side: the primary key is the keys of the other (complex) sides; the single side's key is a
  foreign key only.
- Several single sides: a choice.
  - The default: one candidate key per single side, the keys of all the other sides. The first is the
    primary key, the others `UNIQUE`. This is always correct.
  - The keys of the complex sides alone as primary key, or, with one complex side only, the relationship
    folded into that side's relation as foreign keys. These are valid only if each entity of the complex
    sides takes part at most once, which n-notation can't say; the option's note says so.
- In n-notation, a label at a side counts that side's entities for each combination of the others, so a
  `1` says that combination determines it.

**Choices.** *Foreign keys in ‹entity type›*, *Own relation, key: …* (one option per possible key), in
the Relational Schema window.

**Why.** The primary key must tell the related combinations apart. A side marked `1` is determined by
the others, so its key needn't, and shouldn't, be part of it. With several such sides, each gives a
candidate key; the keys of the complex sides alone would be a key only if the diagram said more than
n-notation can.

**Examples.**

```erd converts
\entity{lecturer}(0,0){Lecturer}
\attributes[anchor=west]{lecturer}{*LId}
\relationship{teaches}(4,0){teaches}
\entity{course}(8,0){Course}
\attributes{course}{*CId}
\entity{room}(4,-3.5){Room}
\attributes{room}{*RId}
\link{lecturer}{teaches}{cn}
\link{teaches}{course}{cn}
\link{teaches}{room}{1}
```

```relations result
teaches : {(_/LId/_, _/CId/_, /RId/)}
```

```erd converts
\entity{lecturer}(0,0){Lecturer}
\attributes[anchor=west]{lecturer}{*LId}
\relationship{teaches}(4,0){teaches}
\entity{course}(8,0){Course}
\attributes{course}{*CId}
\entity{room}(4,-3.5){Room}
\attributes{room}{*RId}
\link{lecturer}{teaches}{cn}
\link{teaches}{course}{1}
\link{teaches}{room}{1}
```

```relations result
teaches : {(_/LId/_, /CId/, _/RId/_)}
teaches.(LId, CId)
```

Folded into the only complex side, if chosen (`rm=fold`), when each lecturer teaches at most once:

```erd converts
\entity{lecturer}(0,0){Lecturer}
\attributes[anchor=west]{lecturer}{*LId}
\relationship[rm=fold]{teaches}(4,0){teaches}
\entity{course}(8,0){Course}
\attributes{course}{*CId}
\entity{room}(4,-3.5){Room}
\attributes{room}{*RId}
\link{lecturer}{teaches}{cn}
\link{teaches}{course}{1}
\link{teaches}{room}{1}
```

```relations result
Lecturer : {(_LId_, /CId/, /RId/)}
```
