# ERDTeX

ER and GSM diagrams for LaTeX, in two parts:

1. **Two LaTeX packages**, `erdiagram` and `gsmdiagram`, that draw entity–relationship and Generic
   Semantic Model diagrams with TikZ. Each is a single `.sty` file kept next to a document; no
   installation in TeX Live or MiKTeX is needed.
2. **ERDTeX**, an editor for the diagram files (`.erd`, `.gsmd`) the packages include. It knows
   the modelling rules and explains each in its rule books, shows a LaTeX preview, turns ER diagrams into
   a relational schema (text, LaTeX, Markdown) and SQL for PostgreSQL, Oracle and SQLite, converts GSM
   diagrams to ER, arranges diagrams automatically, and exports PNG, PDF and SVG. It runs on Linux and
   Windows, and in a browser.

## Use it

- **In the browser:** [leunami52.github.io/erdtex](https://leunami52.github.io/erdtex/). Everything but
  the LaTeX preview works there, also offline after the first visit; nothing you draw leaves your browser.
- **On the desktop:** the deb (Debian, Ubuntu 22.04 and newer), the rpm (Fedora) or the Windows installer,
  from the [releases](../../releases). The user guide: [`desktop_app_rust/README.md`](desktop_app_rust/README.md).
- **The LaTeX packages:** from the releases, or from the program (*Help → Save LaTeX Packages…*). Their
  manuals: [`latex_packages/manuals/`](latex_packages/manuals/), and how to use them:
  [`latex_packages/README.md`](latex_packages/README.md).

A diagram file is plain text, for example:

```latex
\entity{person}(0,0){Person}
\attributes{person}{*Person ID, Name}
\relationship{lives}(4,0){lives in}
\entity{city}(8,0){City}
\attributes{city}{*City ID, Name}
\link{person}{lives}{n}\link{lives}{city}{1}
```

and a document includes it with `\inputerdiagram{people.erd}` after `\usepackage{erdiagram}`.
*File → Open Example* in the program opens a sample diagram of each kind.

## The repository

| Folder | What it is |
|---|---|
| [`latex_packages/`](latex_packages/README.md) | The two packages, their manuals and their tests |
| [`desktop_app_rust/`](desktop_app_rust/README.md) | ERDTeX: the core (Rust, also compiled to WebAssembly), the page (Svelte), the desktop app (Tauri); how it's built: [`ARCHITECTURE.md`](desktop_app_rust/ARCHITECTURE.md) |
| `desktop_app_rust/rulebook/` | The rule books (*Help → Rule Books*): one page per rule, checked by the tests |
| `tools/` | The checks, building and testing the releases, the licence lists |

## Building and testing

Needs Rust (with the `wasm32-unknown-unknown` target), Node.js, `wasm-pack` and the Tauri prerequisites
of your system (WebKitGTK 4.1 on Linux). Then, in `desktop_app_rust/`:

```sh
npm ci
npx tauri dev                # the desktop app, rebuilt on changes
npx tauri build --no-bundle  # a release build
npm run dev:web              # the website, rebuilt on changes
npm run build:web            # the website, into dist-web/
```

`tools/check_rust.sh` runs every check: formatting, lints, the Rust and page tests (with the rule books'
examples and the snapshot tests), the generated SQL in SQLite and PostgreSQL, the website in Chromium,
Firefox and WebKit (Playwright), and the desktop window through WebDriver where `tauri-driver`,
WebKitWebDriver and Xvfb are installed. `tools/check_packages.sh` compiles the LaTeX packages' tests and
manuals. Both run on GitHub Actions for every push and pull request (all but the window tests).
`tools/build_rust_release.sh` and `tools/test_rust_release.sh` build the deb, rpm and Windows installer and
test each in a clean system.

## Licence

The program: MIT ([`LICENSE`](LICENSE)). The LaTeX packages and their manuals: the LaTeX Project Public
License 1.3c ([`latex_packages/LICENSE`](latex_packages/LICENSE)). The bundled Latin Modern fonts: the GUST
Font License. The libraries the program is built with: their own licences, listed in *Help → About →
Licences*. Contributions: [`CONTRIBUTING.md`](CONTRIBUTING.md); security problems: [`SECURITY.md`](SECURITY.md).
