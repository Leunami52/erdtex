+++
id = "NOT-2"
title = "Terms used in the rule books"
kind = "meaning"
applies = ["entity type", "abstract type"]
see = ["NOT-1"]
+++
**Rule.** The rule books use these terms in these meanings.

**The rule books**

| Term | Meaning |
|---|---|
| **must** | breaking the rule is an **error** |
| **should** | breaking it is a **warning**, or, where the rule says so, a **hint** (a suggestion, shown when *Show hints* is on) |
| **may** | allowed |
| **must not** (drawing) | the program doesn't draw the line |
| **becomes** | what an element turns into in a conversion |
| **id** | an element's name in the file (`\entity{room}…`: `room`), unlike its **label**, the text shown |

**ER diagrams**

| Term | Meaning |
|---|---|
| **entity type** | a set of things of one kind (*Student*); each one is an **entity** |
| **relationship type** | a set of connections between entities (*attends*); each one is a **relationship**. Drawn as a diamond, linked to the entity types it connects |
| **attribute** | a value that describes an entity or relationship (*Name*). **Simple**, or **composite** (made of parts: *Address* of *Street*, *Town*), **multi-valued** (several values: `{Email}`), **derived** (computed from others: `/Age`) |
| **attribute list** | an element's attributes drawn as a list beside it; a **Chen attribute** is one drawn as an oval |
| **key** | attributes whose values tell the entities of an entity type apart; drawn underlined. A **candidate key** is a further set of attributes that could serve as key (*Unique*) |
| **cardinality** | how often an entity takes part in a relationship (NOT-1) |
| **role** | the part an entity type plays in a relationship it takes part in more than once (*boss*, *staff*) |
| **weak entity type** | an entity type whose entities depend on an **owner** entity: drawn with a double border, linked by a double line to a **weak relationship type** (double diamond) |
| **existence dependency (E)** | the dependent can't exist without its owner; its key is unique on its own |
| **identification dependency (Id)** | includes existence dependency; the dependent's key is unique only together with its owner's key |
| **generalisation** | IS_A or CAN_BE: the entities of each **subtype** are entities of the **supertype** too, and inherit its attributes and key. The program treats both alike, except that only a *complete* IS_A lets a supertype be identified by its subtypes' keys (ER-1) |
| **complete / incomplete** | every entity of the supertype is in a subtype / some are in none |
| **disjoint / overlapping** | an entity is in at most one subtype / may be in several |
| **PART_OF** | a whole made of parts, drawn as a diamond with a fixed label |
| **junction point** | a point where lines meet or bend; it connects nothing by itself |
| **subdomain** | a box grouping part of a diagram; it doesn't change what the diagram means |

**GSM diagrams**

| Term | Meaning |
|---|---|
| **abstract type** | a type of objects in their own right (*Person*), drawn as a triangle |
| **representable type** | a type of values (*Name*), drawn as a rounded box |
| **subtype** | a type whose objects are objects of its supertype too (a circle) |
| **IS-A edge** | from a subtype to its supertype: a **generalization** (the subtypes are disjoint) or a **specialization** (they may overlap); the type at the top of a hierarchy is its **root** |
| **attribute line** | from a type to the type of an attribute's values: **single-valued** or **multi-valued** |
| **aggregation** | a type made of several others (*Date* of *Day*, *Month*, *Year*) |
| **grouping** | a type of sets of objects of one type (*Team* of *Players*) |
| **component line** | a dashed arrow from an aggregation or grouping to what it is made of |

**Relational schemas**

| Term | Meaning |
|---|---|
| **relation** | a table: a name and its columns |
| **primary key** | the columns that tell the rows apart (`_name_` in the text form) |
| **foreign key** | columns that refer to the primary key of a relation (`/name/`) |
