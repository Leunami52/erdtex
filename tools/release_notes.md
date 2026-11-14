{{NEWS}}

## Downloads

| File | For | Tested |
|---|---|---|
| `ERDTeX-{{VERSION}}-amd64.deb` | Debian 12, Ubuntu 22.04 and newer: `sudo apt install ./ERDTeX-{{VERSION}}-amd64.deb` | automatically only (installed and self-tested in clean Ubuntu 22.04 and Debian 12 containers) |
| `ERDTeX-{{VERSION}}-x86_64.rpm` | Fedora: `sudo dnf install ./ERDTeX-{{VERSION}}-x86_64.rpm` | on a real Fedora, and in a clean Fedora container |
| `ERDTeX-{{VERSION}}-setup.exe` | Windows 10 and 11 (64-bit), for the current user | automatically only (installed and self-tested under Wine) |
| `latex-packages-{{PACKAGES}}.zip` | The LaTeX packages `erdiagram` and `gsmdiagram` {{PACKAGES}} with their manuals: keep the `.sty` files next to your document | compiled with TeX Live |

**In the browser:** [leunami52.github.io/erdtex](https://leunami52.github.io/erdtex/), no installation needed
(everything but the LaTeX preview; nothing you draw leaves your browser).

The installer isn't code-signed, so Windows SmartScreen may warn at the first start: *More info* → *Run
anyway*. The LaTeX preview and the export of the LaTeX result need `pdflatex` (TeX Live, or MiKTeX on
Windows). The user guide: [`app/README.md`](https://github.com/Leunami52/erdtex/blob/v{{VERSION}}/app/README.md).
