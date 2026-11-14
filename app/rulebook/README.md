# The rule books

The pages behind *Help → Rule Books…* in ERDTeX: one Markdown file per rule, one folder per
book. They are built into the program, and `crates/core/tests/rulebook.rs` checks them: the template,
the ids and links, and every example (it is run through the checks or conversions it illustrates).

## A book: `index.md`

```text
+++
book = "Modelling ER"     # the book's name
prefix = "ER"             # the prefix of its rules' ids
order = 1                 # its place in the list of books
+++
The book's introduction (Markdown).
```

## A rule: `<ID>.md`, e.g. `ER-1.md`

```text
+++
id = "ER-1"                     # the prefix of its book and a number, never reused
title = "An entity type should have a key"
kind = "hint"                   # error | warning | hint | refused | conversion | meaning
applies = ["entity type"]       # the elements it is about (the palette's names)
fix = "…"                       # optional: the fix the program offers
see = ["ER-2"]                  # optional: related rules
withdrawn = false               # optional: a rule that no longer applies keeps its page
+++
```

Then the text, in this order:

| Kind | Sections |
|---|---|
| error, warning, hint, refused | **Rule.** **What the program does.** **Why.** **Examples.** (a hint may lack a *follows* example where it is information without a mistake behind it) |
| conversion | **Rule.** **Conditions.** (optional) **Choices.** (optional) **Why.** **Examples.** |
| meaning | **Rule.** **Why.** (optional) and free text: tables, **Examples.** (optional) |

The rule is one sentence, with **must** (the program reports an error), **should** (a warning, or a
hint where it says so), **may** (allowed), **must not** (refused while drawing) or **becomes** (a
conversion). Each rule is explained in its own terms, without references to other material.

## Examples

Fenced blocks whose first line names the language, the role and, for a violation, the rule:

| Block | Checked by the tests |
|---|---|
| ` ```erd follows ` / ` ```gsmd follows ` | not this rule's problem, and no errors or warnings of any rule (hints of other rules may remain) |
| ` ```erd violates ER-1 ` | the rule's problem |
| ` ```erd allowed ` / ` ```erd refused DRAW-1 ` | its last line would be allowed, or refused by the rule, if it were drawn now |
| ` ```gsmd converts ` then ` ```erd result ` | the conversion to ER gives these elements, attributes and lines (positions aside) |
| ` ```erd converts ` then ` ```relations result ` | the relational schema (text form) has these lines |
| ` ```erd shows ` / ` ```gsmd shows ` | it can be read (the symbols of a notation; drawn only) |

Examples are small: three to six elements.
