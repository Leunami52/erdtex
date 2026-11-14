+++
id = "REL-12"
title = "A recursive relationship: own relation or foreign key"
kind = "conversion"
applies = ["relationship type", "weak relationship type", "relation"]
see = ["ER-17", "ER-25", "REL-7", "REL-8"]
+++
**Rule.** A relationship type between an entity type and itself **becomes** a relation with the entity
type's key twice, one column per role; where one side is single, it **may** instead become a foreign key
in the entity type's own relation, named by the role of the side it refers to.

**Conditions.**

- Many-to-many (`cn`:`cn` …): always an own relation, both columns together its primary key.
- One side single: an own relation (the many side's column as primary key) or a foreign key.
  - Default: own relation. (Files from versions up to 3.5 that say `rm rules=db1` start with the foreign
    key, as they did then; REL-22.)
  - Without a role, the foreign key column is named after the relationship and the key (*supervisesSSN*).
- An entity type depending on itself by existence (ER-25): the foreign key is the default, with
  `ON DELETE CASCADE`: deleting a chapter deletes those within it. An own relation can't do
  that (deleting a chapter only deletes its rows there).
- Key attributes on the relationship force an own relation (REL-11).

**Choices.** *Own relation with renamed keys* or *Foreign key ‹role› in ‹entity type›*, in the
Relational Schema window (`rm=table`, `rm=fk`).

**Why.** Both sides are the same relation, so the two keys need names of their own: the roles give them.
Where each employee has at most one boss, the boss's key fits into the employee's own row.

**Examples.**

```erd converts
\entity{employee}(0,0){Employee}
\attributes[anchor=west]{employee}{*SSN}
\relationship{supervises}(5,0){supervises}
\link[role=boss, bend left=30]{employee}{supervises}{c}
\link[role=staff, bend right=30]{employee}{supervises}{cn}
```

```relations result
Employee : {(_SSN_)}
Employee_supervises_Employee : {(/boss/, _/staff/_)}
```

```erd converts
\entity{employee}(0,0){Employee}
\attributes[anchor=west]{employee}{*SSN}
\relationship[rm=fk]{supervises}(5,0){supervises}
\link[role=boss, bend left=30]{employee}{supervises}{c}
\link[role=staff, bend right=30]{employee}{supervises}{cn}
```

```relations result
Employee : {(_SSN_, /boss/)}
Employee.boss -> Employee.SSN
```

```erd converts
\entity[weak]{chapter}(0,0){Chapter}
\attributes[anchor=west]{chapter}{*Title}
\relationship[dependency=E]{in}(5,0){in}
\link[role=part, double, bend left=30]{chapter}{in}{cn}
\link[role=whole, double, bend right=30]{chapter}{in}{c}
```

```relations result
Chapter : {(_Title_, /whole/)}
Chapter.whole -> Chapter.Title
```
