+++
id = "REL-15"
title = "A weak entity type with an identification dependency"
kind = "conversion"
applies = ["weak entity type", "weak relationship type", "relation"]
see = ["REL-16", "ER-19", "ER-23"]
+++
**Rule.** A weak entity type that depends on its owner by identification (**Id**, or *weak* without
saying) **becomes** a relation whose primary key is its owner's key together with its own key; the
owner's key is also a foreign key.

**Conditions.**

- The weak relationship type itself gets no relation and no foreign key of its own.
- A chain of weak entity types (a seat in a room in a building) takes all owners' keys, from the top.
- Between two weak entity types, the double line says which is the owner (ER-23); without it the program
  decides by the cardinalities and says so.

**Why.** The dependent's own key (the room number) is unique only within its owner (the building): only
together they tell the rooms apart, so both form the key.

**Examples.**

```erd converts
\entity{building}(0,0){Building}
\attributes[anchor=west]{building}{*Name}
\relationship[dependency=Id]{contains}(4,0){contains}
\entity[weak]{room}(8,0){Room}
\attributes{room}{*Number}
\link{building}{contains}{1}
\link[double]{contains}{room}{n}
```

```relations result
Building : {(_Name_)}
Room : {(_/Name/_, _Number_)}
Room.Name -> Building.Name
```

```erd converts
\entity{building}(0,0){Building}
\attributes[anchor=west]{building}{*Name}
\relationship[dependency=Id]{contains}(4,0){contains}
\entity[weak]{room}(8,0){Room}
\attributes{room}{*Number}
\link{building}{contains}{1}
\link[double]{contains}{room}{n}
\relationship[dependency=Id]{has}(8,-3){has}
\entity[weak]{seat}(8,-6){Seat}
\attributes{seat}{*SeatNo}
\link{room}{has}{1}
\link[double]{has}{seat}{n}
```

```relations result
Seat : {(_/Name/_, _/Number/_, _SeatNo_)}
```
