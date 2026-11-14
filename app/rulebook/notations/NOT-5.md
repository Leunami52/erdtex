+++
id = "NOT-5"
title = "The symbols of ER diagrams"
kind = "meaning"
applies = ["entity type", "weak entity type", "relationship type", "weak relationship type", "attribute list", "Chen attribute", "IS_A", "CAN_BE", "PART_OF", "line", "junction point", "note", "subdomain"]
see = ["NOT-1", "NOT-2", "NOT-3", "NOT-4"]
+++
**Rule.** Each element of an ER diagram has its own symbol; the marks on attributes say what kind of
attribute each is.

| Symbol | Element | Means |
|---|---|---|
| rectangle | entity type | a set of things of one kind |
| double rectangle | weak entity type | its entities depend on an owner (ER-19) |
| diamond | relationship type | connections between entities |
| double diamond, labelled **E** or **Id** | weak relationship type | the dependency of a weak entity type on its owner: by existence or by identification |
| hexagon (or diamond) labelled IS_A or CAN_BE, with letters | generalisation | the subtypes below it are kinds of the supertype above (NOT-4) |
| diamond labelled PART_OF | PART_OF | a whole made of parts |
| a list beside an element | attribute list | its attributes, marked as below |
| oval | Chen attribute | one attribute, linked to its element |
| line | link | connects a relationship type to an entity type, or a generalisation to its types |
| double line | link of a weak entity type | to its weak relationship type |
| label at a line | cardinality, role | how often (NOT-1, NOT-3); the part an entity type plays |
| dot | junction point | where lines meet or bend |
| text | note | a comment; it means nothing to the checks |
| coloured outline with a title | subdomain | a part of the diagram; it means nothing to the checks |

**Marks on attributes.**

| In a list | As an oval | In the file | Means |
|---|---|---|---|
| underlined | label underlined | `*SSN` | key |
| dashed underline | label with dashed underline | `*Number` of a weak entity type | partial key: unique only with the owner's key (identification dependency) |
| `{Email}` | double oval | `\{Email\}` | multi-valued |
| `/Age` | dashed oval | `/Age` | derived; `/Age = {…}` gives its formula |
| parts indented below it | rounded rectangle, with its parts linked | `Name{First, Last}` | composite |
| `Born : date` | the same | `Born : date` | its data type, shown when *Show data types* is on |
| not drawn | not drawn | `+Passport` | a further candidate key (*Unique*) |

**Why.** One symbol per kind of element, and one mark per kind of attribute, make a diagram readable
at a glance and say exactly what the checks and conversions read from it.

**Examples.**

```erd shows
\entity{building}(0,0){Building}
\attributes[anchor=north]{building}{*Name, Address{Street, Town}, \{Phone\}, /Rooms}
\relationship[dependency=Id]{contains}(5,0){contains}
\entity[weak]{room}(10,0){Room}
\attributes[anchor=north]{room}{*Number, Seats}
\link{building}{contains}{1}
\link[double]{contains}{room}{n}
```

```erd shows
\setup{attribute style=chen}
\entity{person}(0,0){Person}
\attribute[of=person, key]{ssn}(-3,1.5){SSN}
\attribute[of=person, multivalued]{email}(-3,-1.5){Email}
\attribute[of=person, derived]{age}(3,-1.5){Age}
\attribute[of=person, composite]{name}(3,1.5){Name}
\attribute[of=name]{first}(1.5,3.2){First}
\attribute[of=name]{last}(4.5,3.2){Last}
```

```erd shows
\entity{car}(0,0){Car}
\attributes[anchor=west]{car}{*VIN}
\partof{p}(4,0)
\entity{engine}(8,0){Engine}
\attributes{engine}{*No}
\link{car}{p}{1}
\link{p}{engine}{n}
```
