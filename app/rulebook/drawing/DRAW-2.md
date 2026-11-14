+++
id = "DRAW-2"
title = "A line joins two elements"
kind = "refused"
applies = ["line"]
see = ["ER-3", "GSM-3"]
+++
**Rule.** A line **must not** be drawn unless both of its ends are elements (or junction points).

**What the program does.** Refused: *"edges must connect two elements"*.

- In the editor, a line dragged onto empty space, a note, an attribute list or a subdomain simply isn't
  drawn, without a message. The refusal guards the remaining cases, where an end names no element at all.

**Why.** A line stands for a connection between two elements; with an end missing it connects nothing.

**Examples.** In each, the last line is the one being drawn.

```erd refused DRAW-2
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\link{student}{nowhere}
```

```erd allowed
\entity{student}(0,0){Student}
\attributes[anchor=west]{student}{*Student Id}
\relationship{attends}(4,0){attends}
\link{student}{attends}{cn}
```
