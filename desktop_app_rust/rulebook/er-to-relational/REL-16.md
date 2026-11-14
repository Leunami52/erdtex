+++
id = "REL-16"
title = "A weak entity type with an existence dependency"
kind = "conversion"
applies = ["weak entity type", "weak relationship type", "relation"]
see = ["REL-15", "REL-12"]
+++
**Rule.** A weak entity type that depends on its owner by existence only (**E**) **becomes** a relation
whose primary key is its own key; its owner's key is a mandatory foreign key, not part of the key.

**Conditions.**

- Its key is drawn with a solid underline: it is unique on its own.
- An entity type depending on itself by existence is REL-12.

**Why.** An existence dependency only says that the dependent can't exist without its owner: the
mandatory foreign key says exactly that, while the dependent's own key already identifies it.

**Examples.**

```erd converts
\entity{building}(0,0){Building}
\attributes[anchor=west]{building}{*Name}
\relationship[dependency=E]{contains}(4,0){contains}
\entity[weak]{room}(8,0){Room}
\attributes{room}{*Number}
\link{building}{contains}{1}
\link[double]{contains}{room}{n}
```

```relations result
Building : {(_Name_)}
Room : {(_Number_, /Name/)}
Room.Name -> Building.Name
```
