+++
id = "REL-17"
title = "A generalisation: four ways to represent it"
kind = "conversion"
applies = ["IS_A", "CAN_BE", "relation"]
see = ["REL-18", "ER-33"]
+++
**Rule.** A generalisation (IS_A or CAN_BE) **becomes** one of four representations, chosen in the
Relational Schema window; which ones are possible depends on whether it is complete and disjoint.

**Conditions.**

| Option | Relations | Possible |
|---|---|---|
| **(1)** multiple relations | one per entity type; each subtype's relation has the supertype's key as primary and foreign key | always |
| … + categorisation attribute | the supertype's relation gets a column `category` naming the subtype | disjoint |
| … + categorisation relation | a relation *‹Supertype›Category* (key, category), one row per subtype of each entity | always |
| … no categorisation | the subtype follows from the subtype relations | always |
| **(2)** subtype relations only | no relation for the supertype; each subtype's relation repeats its attributes | complete, disjoint, and the supertype has no relationships of its own |
| **(3)** single relation + categorisation attribute | one relation with all attributes (a subtype's may be NULL) and `category` | disjoint |
| **(4)** single relation + Boolean flags | one relation with all attributes and a column *is‹Subtype›* per subtype | always (also overlapping) |

- The default: (1) with a categorisation attribute for a disjoint generalisation, with a categorisation
  relation for an overlapping one, and without categorisation if that isn't said. (Files from versions up
  to 3.5 that say `rm rules=db1` start with (1) without categorisation, as they did then; REL-22.)
- A generalisation that doesn't say whether it is complete or disjoint (ER-33): options that need it are
  offered with a note saying what they assume.
- A subtype with several supertypes rules out (2), (3) and (4) (REL-18); so does a supertype that is itself
  merged into its own supertype's relation.
- The allowed values of `category` are listed under *Allowed values* (a `CHECK` in SQL).

**Choices.** The six options above, in the Relational Schema window (`rm=1`…`rm=4` and
`rm category=attribute|relation|none` on the generalisation).

**Why.** Each representation trades something: several relations keep each entity's attributes together
without NULLs, but need joins; one relation needs no joins, but has NULLs; subtype relations only lose the
supertype's own relation. The categorisation says directly which subtype an entity belongs to.

**Examples.**

```erd converts
\isa{g}(0,-2){person}{cd}
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name}
\entity{employee}(-2.5,-4){Employee}
\attributes[anchor=west]{employee}{Salary}
\entity{student}(2.5,-4){Student}
\attributes{student}{Field}
\link{employee}{g}
\link{student}{g}
```

```relations result
Person : {(_SSN_, Name, category)}
Employee : {(_/SSN/_, Salary)}
Student : {(_/SSN/_, Field)}
Person.category in {Employee, Student}
```

```erd converts
\isa{g}(0,-2){person}{io}
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name}
\entity{employee}(-2.5,-4){Employee}
\attributes[anchor=west]{employee}{Salary}
\entity{student}(2.5,-4){Student}
\attributes{student}{Field}
\link{employee}{g}
\link{student}{g}
```

```relations result
Person : {(_SSN_, Name)}
PersonCategory : {(_/SSN/_, _category_)}
```

```erd converts
\isa[rm=2]{g}(0,-2){person}{cd}
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name}
\entity{employee}(-2.5,-4){Employee}
\attributes[anchor=west]{employee}{Salary}
\entity{student}(2.5,-4){Student}
\attributes{student}{Field}
\link{employee}{g}
\link{student}{g}
```

```relations result
Employee : {(_SSN_, Name, Salary)}
Student : {(_SSN_, Name, Field)}
```

```erd converts
\isa[rm=3]{g}(0,-2){person}{cd}
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name}
\entity{employee}(-2.5,-4){Employee}
\attributes[anchor=west]{employee}{Salary}
\entity{student}(2.5,-4){Student}
\attributes{student}{Field}
\link{employee}{g}
\link{student}{g}
```

```relations result
Person : {(_SSN_, Name, Salary, Field, category)}
```

```erd converts
\isa[rm=4]{g}(0,-2){person}{io}
\entity{person}(0,0){Person}
\attributes{person}{*SSN, Name}
\entity{employee}(-2.5,-4){Employee}
\attributes[anchor=west]{employee}{Salary}
\entity{student}(2.5,-4){Student}
\attributes{student}{Field}
\link{employee}{g}
\link{student}{g}
```

```relations result
Person : {(_SSN_, Name, Salary, Field, isEmployee, isStudent)}
```
