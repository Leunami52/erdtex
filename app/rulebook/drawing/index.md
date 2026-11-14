+++
book = "Drawing"
prefix = "DRAW"
order = 3
+++
The connections the program refuses while you draw them, in ER and GSM diagrams.

In these rules, **must not** means the program doesn't draw the line: the element you are pointing at
turns red, and the status bar says why, with a link to the rule (*"Not allowed: … (DRAW-1)"*). The same
applies when an end of an existing line is dragged to another element.

- A line to or from a **junction point** is always allowed while drawing: the point only shapes lines.
  What the lines then connect is judged by the modelling rules.
- In GSM diagrams, **attribute lines** (single- and multi-valued) aren't restricted while drawing.
- A file written by hand may still contain a refused line; the modelling rules report it then.
