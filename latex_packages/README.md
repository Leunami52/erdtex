# erdiagram / gsmdiagram — the LaTeX packages (v3.0.2)

| File | What it is |
|---|---|
| `erdiagram.sty` | ER diagrams in the classic style: n-notation, min-max or crow's foot cardinalities, compact attribute lists or Chen ovals |
| `gsmdiagram.sty` | GSM (Generic Semantic Model) diagrams |
| `manuals/erdiagram-manual.pdf` / `.tex` | Full documentation of `erdiagram`, every feature with code and output |
| `manuals/gsmdiagram-manual.pdf` / `.tex` | Full documentation of `gsmdiagram` |
| `tests/` | Regression documents, `tests/build.sh` compiles one and renders PNGs |

The two `.sty` files here are the only ones to edit; all others in the
repository are deployed copies. The manuals include example diagrams from
`../examples/diagrams/`.

Users only need the `.sty` file(s): copy them next to the document and
`\usepackage{erdiagram}` / `\usepackage{gsmdiagram}`. Both need nothing but
TikZ and can be loaded together.

## Quick start

```latex
\begin{erdiagram}[max width=\linewidth]
  \entity{building}(0,0){Building}
  \relationship[dependency=Id]{has}(3,0){has}
  \entity[weak]{room}(6,0){Room}
  \link{building}{has}{1}
  \link[double]{has}{room}{n}
  \attributes[anchor=north]{building}{*buildingID, address}
\end{erdiagram}

\inputerdiagram[width=\textwidth]{figures/airline.erd}     % body in a file
\inputgsmdiagram{figures/enterprise.gsmd}
```

The only global names are the environments `erdiagram`/`gsmdiagram` and
`relationalschema`, `\inputerdiagram`/`\inputgsmdiagram` and `\ersetup`/`\gsmsetup`;
all diagram commands exist only inside the environments.

## Diagram file format (for the desktop editor)

A diagram file (`.erd` for ER, `.gsmd` for GSM) contains the *body* of the
environment. To be readable and writable by the editor, a file follows
these rules:

* One element per line: `\command[options]{arg}...`. Blank lines and
  `%` comments are allowed (a comment may also follow a command).
* Positions are absolute coordinates `(x,y)` in cm, written directly after
  the id: `\entity{student}(2.5,-1){Student}`.
* Options are a comma-separated `key=value` list in `[...]`; a value that
  contains a comma is wrapped in braces (`via={(1,2) (3,4)}`). Keys the
  packages do not know are passed to TikZ.
* Ids are plain names (letters, digits, `-`, `_`); edges may refer to an
  anchor of a node with `id.anchor` (`student.north`, `person.60`).
* Labels are LaTeX (`\\` for line breaks). A label containing unbalanced
  braces is not allowed.

### ER commands (`.erd`)

| Command | Meaning |
|---|---|
| `\entity[opts]{id}(x,y){Label}` | entity; opts `weak`, `color=` |
| `\relationship[opts]{id}(x,y){Label}` | relationship; opts `weak`, `dependency=E\|Id`, `color=` |
| `\partof[opts]{id}(x,y)` | aggregation diamond (*PART_OF*) |
| `\isa[opts]{id}(x,y){super}{constraint}` | IS_A + arrow to the supertype; `constraint pos=`, `super card=`, `sub card=`; constraint optional |
| `\canbe[opts]{id}(x,y){super}{constraint}` | CAN_BE, same as `\isa` |
| `\attribute[opts]{id}(x,y){Label}` | Chen-style ellipse; `key`, `unique`, `derived`, `formula=`, `multivalued`, `composite`, `of=<node>`, `type=<data type>` |
| `\attributes[opts]{node}{*Key, +Unique, A{B, C}, \{Multi\}, /Derived = {formula}, A : type, A@x/y}` | attribute list, as a tree (or ovals with `attribute style=chen`). `*` key, `+` further candidate key (not drawn), `A{B, C}` composite, `\{..\}` multi-valued, `/..` derived, `@x/y` the place of its oval; `anchor=`, `dir=`, `side=`, `align=`, `spread=`, `gap=`, `tick=`, `stem=`, `xshift=`, `yshift=`; an item may carry a data type: `*PID : integer`, `price : {decimal(8,2)}` |
| `\link[opts]{from}{to}{card}` | edge, cardinality optional; `beside`, `inline`, `card side=`, `card dist=`, `card pos=`, `role=`, `role side=`, `role pos=`, `bend=`, `via=`, `ortho=`, `arrow`, `double` |

Relational model choices made in the desktop editor are stored as options
the package ignores: `rm=1|2|3|4` and `rm category=` on `\isa`/`\canbe`,
`rm=<entity id>|table|fk|fold` and `rm key=` on relationships, `rm name=` on
entities, relationships (relation name) and `\isa` (categorisation
attribute), and `rm rules=db1` in `\setup` from older versions of the editor (up to 3.5). See the manual, section
"Relational model options".

### GSM commands (`.gsmd`)

| Command | Meaning |
|---|---|
| `\abstract[opts]{id}(x,y){Label}` | abstract type (triangle) |
| `\representable[opts]{id}(x,y){Label}` | representable type (rounded box) |
| `\subtype[opts]{id}(x,y){Label}` | subtype (circle) |
| `\aggregation[opts]{id}(x,y){Label}` | aggregation; `label pos=` |
| `\grouping[opts]{id}(x,y){Label}` | grouping; `label pos=` |
| `\attribute`, `\multiattribute`, `\component`, `\generalization`, `\specialization`, `\link` `[opts]{from}{to}{label}` | edges, label optional; `label side=`, `label pos=`, `label segment=`, `sloped`, `horizontal`, `label color=`, `label style=`, `bend=`, `via=`, `ortho=` |

### Common commands

| Command | Meaning |
|---|---|
| `\point{id}(x,y)` | invisible junction / way point |
| `\note[opts](x,y){text}` | free text; `color=` |
| `\subdomain[opts]{Label}{(x,y) (x,y) ...}` | frame: 2 points = rectangle, more = polygon; `color=`, `dotted`, `fill=` |
| `\grid(x1,y1)(x2,y2)` | helper grid |
| `\setup{key=value}` | style keys for this diagram |

Edge curvature for interactive editing: `bend=<angle>` (positive = left,
negative = right); way points: `via={(x,y) ...}`.

## Defaults worth knowing

* ER cardinalities sit on the middle of the edge. `card placement=beside`
  (environment option, or `\ersetup{...}` for all diagrams) or the link
  option `beside` puts them next to the entity end.
* Cardinalities have two notations: `1`, `c`, `n`, `cn` (written at the far end from what
  they count) and `(min,max)` (written at what they count). `card symbols=crowsfoot` draws
  `1`, `c`, `n`, `cn` as symbols. `card notation=` only tells the desktop editor what to offer.
* `sans serif` sets a diagram in sans-serif, `scale=0.5` scales it as a whole, in both packages
  (with `width=`, `height=`, `max width=`, also on the `\input...` commands).
* The labels follow the document's language: with babel or polyglossia in German, `IST_EIN*E`,
  `KANN_SEIN`, `TEIL_VON`. `label language=german|english` fixes it.
* `\begin{relationalschema}` ... `\end{relationalschema}` holds the relational schema the
  desktop editor writes. Its commands (`\relation`, `\relationgroup`, `\col`, `\pkey`, `\fkey`, `\pfkey`) only
  exist inside it, so they can't collide with your own.
* Attribute lists hold all kinds of attributes: `*key`, `+email` (a further candidate key, not drawn), `address{street, town}`, `\{email\}`,
  `/age = {formula}`. `attribute style=chen` draws the same lists as ovals; `name@x/y` fixes an
  oval. The key of a weak entity type is dashed by itself, unless it depends on E alone.
* IS_A and CAN_BE are hexagons (`isa shape=diamond` for diamonds; PART_OF stays a diamond).
  Their constraint sits inside as letters (`constraint pos=auto` puts it beside);
  `constraint style=words` prints `complete, disjoint` beside the shape.
* GSM edge labels are rotated along their edge. `label sloped=false`
  (environment option) or the edge option `horizontal` prints them
  horizontally.

* Data types (`name : type`) are not printed; `show types` (environment
  option, `\ersetup` or `\setup`) prints them as `name: type`.

## Rule checks

Both packages report modelling-rule violations as LaTeX warnings
(e.g. two entities linked without a relationship, IS-A edges that do not
start at a subtype). See the manuals for the full list.

## Maintenance tools (`../tools/`)

* `sync_shared.py` — the code shared by both packages is maintained in
  `erdiagram.sty` (blocks marked `%% <shared:...>`) and copied into
  `gsmdiagram.sty` with renamed prefixes. Run after editing a shared block.
* ERDTeX builds both packages and their manuals into the program straight from this folder
  (rebuild it after a change).
* `tests/build.sh file.tex` — compile a test against these packages and
  render PNGs (to `build/test_png/`).
