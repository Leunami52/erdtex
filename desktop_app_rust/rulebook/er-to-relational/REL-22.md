+++
id = "REL-22"
title = "Rule sets (withdrawn)"
kind = "meaning"
applies = ["relation"]
see = ["REL-12", "REL-17"]
withdrawn = true
+++
**Rule.** Versions up to 3.5 offered two **rule sets**, each a set of defaults for the decisions of the
Relational Schema window. There is one set of defaults now; every decision still offers all its options,
and a choice made is stored on its element.

**Conditions.**

- The two differed in two defaults only: whether a generalisation is categorised (REL-17), and whether a
  recursive relationship with one single side becomes an own relation or a foreign key (REL-12). The
  defaults kept are those that were the default rule set's.
- A file that says `\setup{rm rules=db1}` (the other rule set) keeps its results: it still starts with
  generalisations without categorisation and recursive relationships as foreign keys. Nothing offers the
  key any more; any other value of it is reported and ignored.

**Why.** A rule set bundled two unrelated defaults under one name. Each decision shows all its options
with the relations they give, so choosing on the element says directly what is meant.
