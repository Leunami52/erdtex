+++
id = "DRAW-6"
title = "A supertype isn't its own subtype"
kind = "refused"
applies = ["IS_A", "CAN_BE", "entity type", "line"]
see = ["ER-30"]
+++
**Rule.** A line **must not** join a generalisation (IS_A or CAN_BE) to its own supertype.

**What the program does.** Refused: *"the supertype cannot be its own subtype"*.

- The supertype is connected to the generalisation by the generalisation itself (its *Supertype*,
  drawn as the arrow); a line would make it one of the subtypes too.

**Why.** Every entity of a subtype is an entity of the supertype. If the supertype were its own subtype,
the generalisation would specialise nothing.

**Examples.** In each, the last line is the one being drawn.

```erd refused DRAW-6
\entity{person}(0,0){Person}
\attributes{person}{*SSN}
\isa{g}(0,-2){person}{cd}
\link{person}{g}
```

```erd allowed
\entity{person}(0,0){Person}
\attributes{person}{*SSN}
\isa{g}(0,-2){person}{cd}
\entity{employee}(0,-4){Employee}
\link{employee}{g}
```
