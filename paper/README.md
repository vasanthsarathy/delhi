# The paper

A journal-style write-up of \mB{}+ and the delhi language: syntax, semantics, the action
layer, the query language, implementation and evaluation.

- `delhi.tex` — the paper
- `preamble.tex` — packages, theorem environments, notation macros, listings style
- `delhi.pdf` — the built output (committed, so the repository carries a readable copy)

## Building

```bash
latexmk -pdf delhi.tex     # or: pdflatex delhi.tex  (three times, for refs and the ToC)
latexmk -C                 # clean
```

Needs a TeX distribution with `amsmath`, `amsthm`, `mathtools`, `booktabs`, `listings`,
`natbib`, `hyperref` and `cleveref`. No bibliography tool is required: the references are a
`thebibliography` environment inside `delhi.tex`.

## Two things to know before editing

**Do not add `aliascnt`.** The theorem-like environments deliberately share the
`definition` counter so numbering runs in one sequence. The usual consequence is that
`cleveref` names everything after that counter, rendering `\Cref` of a proposition as
"Definition 2.5" — and the usual remedy is `aliascnt`. In this document `aliascnt` sends
`pdflatex` into a non-terminating loop during AMS symbol-font setup, with no error, so the
build simply never finishes. Cross-references to non-definition environments are written
explicitly instead (`Proposition~\ref{...}`), and `\Cref` is reserved for definitions,
sections, tables and equations, where it names them correctly.

**The measurements are real.** Every figure in the evaluation section came from
`delhi bench` on the domains named, and every logical claim in the safe-belief section was
checked against `examples/safe_belief.delhi` with the CLI. If the semantics changes, those
numbers and claims need re-running, not adjusting.

## Status

Draft. The bibliography entries carry author, title, venue and year but not volume or page
numbers for every item; verify them against the sources before submitting anywhere.
