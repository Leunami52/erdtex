+++
id = "REL-9"
title = "A one-to-one relationship becomes a unique foreign key"
kind = "conversion"
applies = ["relationship type", "relation"]
see = ["REL-8"]
+++
**Rule.** A relationship type between two entity types where both sides are single **becomes** a unique
foreign key in one of the two relations; which one, or an own relation instead, is a choice.

**Conditions.**

- `1`:`c` (one side optional): by default the foreign key goes into the relation of the side whose
  entities must take part, where it is never NULL; choices: that foreign key, or an own relation.
- `1`:`1` or `c`:`c`: the foreign key may go into either relation; by default into the first one drawn.
  With `1`:`1` it is mandatory; with `c`:`c` it may be NULL. Choices: either relation, or an own relation
  (both keys, the first as primary key, the other unique).
- The foreign key is `UNIQUE`: each entity of the other side is referred to at most once.
- What the foreign key can't say (the other side's minimum of 1) is listed as not enforced (REL-21).

**Choices.** In the Relational Schema window: *Foreign key in ‹entity type›* (for either side where both
are possible) or *Own relation* (`rm=fk`, `rm=table`, or `rm=‹entity id›` for the side).

**Why.** Either relation can hold the other's key; the one whose entities always take part gives no
NULL values. Where neither does, an own relation avoids NULLs altogether, at the price of a join.

**Examples.**

```erd converts
\entity{person}(0,0){Person}
\attributes[anchor=west]{person}{*SSN}
\relationship{has}(4,0){has}
\entity{passport}(8,0){Passport}
\attributes{passport}{*No}
\link{person}{has}{c}
\link{has}{passport}{1}
```

```relations result
Person : {(_SSN_, /No/)}
Passport : {(_No_)}
Person.No -> Passport.No
```

```erd converts
\entity{country}(0,0){Country}
\attributes[anchor=west]{country}{*Code}
\relationship[rm=capital]{has}(4,0){has}
\entity{capital}(8,0){Capital}
\attributes{capital}{*Name}
\link{country}{has}{1}
\link{has}{capital}{1}
```

```relations result
Country : {(_Code_)}
Capital : {(_Name_, /Code/)}
```
