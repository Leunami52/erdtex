+++
id = "REL-18"
title = "A subtype with several supertypes"
kind = "conversion"
applies = ["IS_A", "CAN_BE", "relation"]
see = ["REL-17"]
+++
**Rule.** A subtype of several supertypes **becomes** one relation whose primary key is the key it
inherits from its first supertype (in the file's order); the keys of the other supertypes **become**
unique foreign keys.

**Conditions.**

- Each of its generalisations then allows only representation (1) (REL-17): the subtype can't be merged
  into two different relations.

**Why.** A consumer that is both a person and a customer is one entity: one row, identified by one of its
keys; the other key still refers to its customer row, once.

**Examples.**

```erd converts
\entity{person}(0,0){Person}
\attributes[anchor=west]{person}{*SSN}
\entity{customer}(6,0){Customer}
\attributes{customer}{*CNo}
\isa{g1}(0,-2){person}{io}
\isa{g2}(6,-2){customer}{io}
\entity{consumer}(3,-4){Consumer}
\attributes{consumer}{Points}
\link{consumer}{g1}
\link{consumer}{g2}
```

```relations result
Consumer : {(_/SSN/_, Points, /CNo/)}
Consumer.CNo
```
