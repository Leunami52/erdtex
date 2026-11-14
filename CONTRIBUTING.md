# Contributing

Issues and pull requests are welcome.

- **A bug:** what you did, what happened, what you expected, and the diagram file if there is one (or the
  smallest one that shows it). The program's version is in *Help → About*.
- **A change to a rule** (a check, a refused connection, a conversion): its page in the rule books
  (`desktop_app_rust/rulebook/`) says how it behaves now. A change to the rule changes its page and its
  examples too; the tests run every example.
- **Before a pull request:** `tools/check_rust.sh` passes. A change of what the program computes shows up
  in the snapshot tests (`desktop_app_rust/crates/core/tests/snapshots/`); review it with `cargo insta
  review` and keep only what is meant.
- **The file format and the packages** stay compatible both ways: old files keep opening and compiling.
- **Libraries:** a tried and trusted one is preferred to code of our own; a new one adds its licence to the
  lists (`tools/licenses.sh` for Rust crates; JavaScript libraries are listed when the page is built).
