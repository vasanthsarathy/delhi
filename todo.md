# delhi — working checklist

Detailed plan: **`docs/superpowers/plans/2026-07-27-delhi-v0.1-semantic-core.md`**
Design spec: **`docs/superpowers/specs/2026-07-25-delhi-core-design.md`**

---

## Done before implementation

- [x] Design spec, reviewed against [T], [KR24], [KR21], [KR21-S], [MBD], and the Java source
- [x] Correctness pass — 4 errors fixed (Coin Lie arrow direction, a false accusation against `[J]`, `→ᵢ` non-emptiness, `pre` typing)
- [x] Background sections on frames, modal systems, and bisimulation
- [x] **§6 blocker resolved** — `~R` proved sound, incompleteness measured at 5–10%, cause diagnosed (`research/bisimulation/`)

---

## Plan 1 — semantic core (`delhi-syntax`, `delhi-core`, `delhi-mb`) — **COMPLETE**

- [x] **Task 1** — workspace layout and the symbol interner
- [x] **Task 2** — hash-consed formula store
- [x] **Task 3** — derived-attitude sugar constructors
- [x] **Task 4** — `Bits` bitset primitive
- [x] **Task 5** — `Model`, `State`, frame validation
- [x] **Task 6** — derived relations, with the corrected `maxima` precondition
- [x] **Task 7** — memoised entailment for `K` / `B` / `□` / `C` / `B^ψ`
- [x] **Task 8** — `~R` and `~D` bisimulation, with the §6.1.3 witness as a test
- [x] **Task 9** — canonical state keys
- [x] **Task 10** — action theories and well-formedness diagnostics
- [x] **Task 11** — action model construction (ontic, sensing, announcement)
- [x] **Task 12** — product update, thesis rule and [MBD] variant
- [x] **Task 13** — Coin Lie figure reproduction, [T] Figs 5.4–5.10
- [x] **Task 14** — known-defect tests, ignored and failing by design
- [x] **Task 15** — property suite, gap regression, `delhi-core` trait
- [x] Final whole-branch review + one fix wave + scoped re-review

**Delivered:** 26 commits, 62 passing tests, 2 ignored by design, clippy clean under
`--all-targets`, zero runtime dependencies.

## Plan 1 follow-on

- [ ] **Task 16** — `𝒦^eff` / `𝒦^obs` from [KR21] §4.1: what observers learn about effect
      *conditions*. Deferred by design — the construction (Task 11) had to exist first.

## Plan 2 — surface language and CLI (`delhi-lang`, `delhi-cli`)

Plan: **`docs/superpowers/plans/2026-07-27-delhi-v0.1-surface-language.md`** — **COMPLETE**

- [x] **Task 1** — crate skeleton, spans, diagnostics
- [x] **Task 2** — lexer, with ASCII alternatives for `□` and `¿`
- [x] **Task 3** — formula expression parser
- [x] **Task 4** — section parser
- [x] **Task 5** — types, objects, predicate expansion
- [x] **Task 6** — constant folding
- [x] **Task 7** — formula lowering into `delhi-syntax`
- [x] **Task 8** — action grounding
- [x] **Task 9** — declarative `initially` construction
- [x] **Task 10** — explicit `state` form
- [x] **Task 11** — pretty-printer, round-tripping through Task 10
- [x] **Task 12** — `Problem` / `load`; **the gate passed on its first green run**
- [x] **Task 13** — CLI `check`, `show`, `eval`
- [x] **Task 14** — CLI `step`, `dot`
- [x] **Task 15** — CLI `repl`
- [x] Final whole-branch review + one fix wave + scoped re-review

**Delivered:** 26 commits, 194 passing tests at merge (203 with the examples added since), 2 ignored by design, clippy clean under
`--all-targets`, zero runtime dependencies. `delhi-mb` and `delhi-syntax` have a literally empty
diff — the semantic core was consumed, never modified.

## Plan 2 follow-ons (triaged at final review, none blocking)

Structural, deliberately left out of the end-of-branch fix wave because refactors there risk
regressions for no functional gain:

- [ ] Three copies of the cartesian-product loop — `ground.rs::tuples`, `constants.rs`,
      `lower_action.rs`. One helper would serve all three.
- [ ] `atom_of` defined identically in `init_decl.rs` and `lower_action.rs`; `agent_ids` in
      `init_decl.rs` duplicates `lower_formula::resolve_agents` including its message, and its
      `None` arm is dead (it runs only for entries already known clean).
- [ ] `Ctx` lives in `lower_action.rs` and documents itself in grounding terms, but it is the
      parameter bundle for `build_declarative`, `build_explicit`, and `Problem::parse` — none of
      which ground actions. Meanwhile `lower_formula` takes `sig`/`consts` unbundled. **The
      clearest seam in the public surface**; a caller meets two conventions for the same pair.
- [ ] `build_explicit` takes `_store: &mut Store` it never uses, purely for symmetry with
      `build_declarative`.
- [ ] `Sig::atom_id`/`agent_id` are linear scans that rebuild the key string per call. Irrelevant
      at example scale; a `HashMap` cache in `Sig` is the fix, since `Interner`'s map is private
      and `delhi-syntax` is frozen.
- [ ] `cmd_step` and the REPL's `:do` are near-identical.

Smaller, each with its reason for standing:

- [ ] `resolve_args`'s message for a variable is "`?x` is not bound here". In a `state` block no
      variable could *ever* be bound, so it invites the author to hunt for a missing binder.
      Fixing it properly needs a context flag or caller-supplied message — an API change.
- [ ] `parse_file.rs` emits a duplicate diagnostic for one bad token in a malformed `state` edge
      (the `from`/`to` arms `continue` without bumping). Termination is guaranteed; cosmetic.
- [ ] `resolve_agents` drops an invalid agent and proceeds, unlike `resolve_args` which aborts.
      Safe — both consumers gate on `diags.is_empty()` — but wants a comment saying so.
- [ ] `cmd.rs`'s comment about padded valuations says a padding bit is *set*; it is allocated but
      never set. `print.rs` phrases the same point accurately; make the twin match.
- [ ] `init_decl.rs`'s block-span test asserts exact byte offsets into its fixture. Deliberate and
      commented, but it will need updating if that fixture is reformatted.
- [ ] No `rustfmt.toml` is checked in and `cargo fmt --check` reports diffs across the tree
      (pre-existing, predates this branch). Decide on a format policy before it grows.

---

## Open questions carried forward

- [ ] Is `~D` a congruence for product update? (§6.3) If yes, `contract_dynamic` becomes
      `contract_full` and the ~10% merge improvement applies to search.
- [ ] **§4.7(a) needs settling against the primary source** before anyone attempts the θ/τ
      announcement fix — see the correction note in the spec. The documented defect does not
      manifest as described.
- [ ] **§4.8 hypothetical actions — the question was mis-posed, and the abstract answers it.**
      I had this as "deliberate scoping in mB, or an oversight?". It is neither. The KR 2021
      abstract lists it as a *contribution*: "incorporate the effects of actions that do not
      occur, but that could have occurred according to the agent's knowledge", and the
      introduction contrasts it with Buckingham, Kasenberg & Scheutz (2020), where an
      oblivious agent could gain unwarranted knowledge that *that* action, but no other,
      could have occurred. So it is a specified feature of the language that Plan 1 did not
      implement — a missing feature, not an open design question. The ignored failing test
      that pins it is therefore correctly ignored and correctly failing.
      Still **needs Vasanth's call** on whether to implement it, and it likely pairs with
      Plan 1's Task 16 (`𝒦^eff` / `𝒦^obs`).

## Deferred minors (triaged at final review, not blocking)

- [ ] `Model::new` uses `Bits::new(n_atoms.max(1))`; zero-atom models mask `Bits::set`'s assert.
      Needs a small design call on `Bits::new(0)` semantics.
- [ ] Coin Lie's `ActionDef`s never run through `validate()` — the deeper question is whether
      `build()` should validate internally.
- [ ] No generic test through the `EpistemicState` trait; worth adding when a second backend lands.
- [ ] `undecided`'s test derives its expectation from `believes_whether` rather than primitives.
      Safe, since `believes_whether` is verified against primitives on the line above.

---

## Review

**What was built.** Three crates. `delhi-syntax` holds the query language — hash-consed formulas
over the six primitive operators, plus sugar for nine derived attitudes that desugars before it
reaches the semantics. `delhi-mb` holds the mB+ backend: bitset-backed plausibility models with
frame validation, memoised entailment, two distinct bisimulation notions with contraction and
canonical hashing, action-theory compilation into event models, and product update.
`delhi-core` declares the backend-agnostic trait a future planner will be generic over.

**What validates it.** The Coin Lie scenario from [T] Figs 5.4–5.10 reproduces end to end,
including the second-order false belief at the final state — it passed on the first run, which
is the strongest evidence the semantics were transcribed correctly. The bisimulation gap
regression reproduces the measurement from `research/bisimulation/` exactly (115 models, 6
incomplete, 0 unsound). Ten property tests cover frame preservation across update, the KB bridge
axioms, seriality, `~R ⊆ ~D`, and two post-update semantic invariants.

**Three defects in my own planning documents, found by execution.**

1. *Task 9's canonical encoding was not canonical.* It ordered blocks by a refinement that numbers
   by first occurrence in world order, so renaming worlds changed the key. Its own renaming test
   would have failed. Caught by the pre-flight scan; replaced with sorted-signature ranking, which
   also deleted a dead factorial permutation search.

2. *The spec claimed the two transition rules agree on every worked example.* They do not — Coin
   Lie is itself the differential case. Found by an implementer who reported BLOCKED rather than
   adjusting a failing test to pass. Under the draft rule the lie fails to land at all, which is
   direct evidence for the thesis rule being authoritative. Spec §4.5.1 amended.

3. *The spec's §4.7(a) describes a defect that does not manifest as described.* The announcement
   limitation was characterised as a full observer learning too much; in fact a partial observer
   ends up undecided, so the acceptance test fails on a different assertion than predicted. Spec
   §4.7(a) amended with a warning not to attempt the fix before settling it.

**Two assertions were structurally incapable of firing.** A reflexivity check in product update
could never fail because the model constructor pre-seeded the diagonal; a second one could never
fail because the same pure memoised predicate that selected the event had already populated the
index. Both looked like live checks and were not.

**The pattern worth keeping.** In ten of the fifteen tasks, a review found that a test specified
in the plan could not have failed against the bug it nominally covered. The production code was
almost always right; the tests guarding it usually were not. The discipline that caught these was
requiring a red/green experiment — sabotage the code the test claims to protect, observe the
failure, restore — which turns "this test passes" into "this test would fail if the thing it
protects broke."

---

## Review — Plan 2

**What was built.** Two crates. `delhi-lang` is a staged front end — lex, parse, ground, lower —
that turns a `.delhi` file into a `Problem`: a type/object signature with predicates expanded to
ground atoms, constants folded away before they can occupy a bit in every world, actions compiled
to `ActionDef`s, and an initial plausibility model built either declaratively or explicitly.
`delhi-cli` is a thin binary over it with six subcommands. Every crate still has zero runtime
dependencies; argument parsing is hand-rolled.

**What validates it.** `examples/coin_lie.delhi` reproduces Plan 1's `coin_lie.rs` trace
assertion for assertion, from text rather than the Rust API, ending at the second-order false
belief — and it passed on its first green run with no reconciliation. The reviewer verified the
gate is a real gate rather than accepting the pass: it read the `.delhi` file against the API
reference and confirmed identical agents, atoms, actions, observer classes, conditional awareness
and action order, with no assertion missing or weakened. The pretty-printer round-trips through
the explicit-state parser under `State::equivalent`, which is full bisimulation and therefore
sensitive to edge *direction*.

**The dominant pattern, and it is the same one Plan 1 found.** Seven of fifteen tasks turned up a
test specified in the plan that passed against the very bug it named. The production code the plan
specified was almost always right; the tests guarding it usually were not. Sabotage is what
separated them — for instance, collapsing the world enumeration so every uncertain atom read the
same mask bit still produced four worlds that validated with the correct designated valuation, so
the plan's `n_worlds == 4` assertion accepted it. Dropping a conjunct from the file's declared
goal left all five gate tests green. Removing the `u != v` guard from the Graphviz emitter left
all four of its assertions green. None of these were visible by reading.

**Four defects in the specified code**, all found by execution: a lexer that decoded with a
leading-byte cast and so could not match its own multi-byte operators; a constant-folding rule
that would have rejected every idiomatic sparse-constants domain; an entry classifier that errored
on entries which do hold in the state it builds; and an uncertainty bound of 16 that permitted
inputs which hang — a 13-atom case had to be killed by a 240-second timeout.

**One defect in already-approved code, found two tasks later.** Task 4's `causes` clause swallowed
the head of the following clause, because no Task 4 test wrote a `causes` list followed by another
clause on the same line. Two of Task 8's own fixtures could not have parsed. The fix's boundary
check had one provably-exact branch and one heuristic that silently changed valid input; rather
than tune the heuristic, the seven clause words became reserved names — a language rule the plan
did not originally have.

**What the process caught that a single pass would not.** Reviewers repeatedly declined to take an
implementer's word: the plausibility direction was traced by hand through `Model::rel`'s contract
rather than trusted to a test name; a sabotage experiment was checked to confirm that disabling a
verification pass was *necessary* to make the experiment discriminate rather than a way of
manufacturing a failure; and one reviewer verified empirically that a borrow-checker justification
repeated across several tasks was simply wrong — field borrows are disjoint, so the clones were
convention, not necessity.

---

## Plan 3 — packaging and distribution

**Goal:** `delhi` is a thing a stranger can download and run on their own folder of
`.delhi` files, and a thing you can depend on from another project. Today it is a
worktree that only builds under `cargo run` and only sees files inside this repo.

**Three defects that block distribution, in the order they bite:**

1. `delhi-gui` resolves `examples/` and `scratch/` from `CARGO_MANIFEST_DIR/../..`. A
   downloaded binary points at a directory that does not exist on the user's machine.
2. The GUI is a second binary. `delhi gui` does not exist, so launching it means
   `cargo run -p delhi-gui` — which needs the repo *and* a Rust toolchain.
3. No repo, no CI, no license, no release artefacts. Nothing to download.

### Task 1 — draggable splitters in the GUI

- [ ] Three drag handles: editor↔rail, rail↔right column, and state↔tabs inside the
      right column. Grid templates read CSS custom properties; the handles write them.
- [ ] Persist to `localStorage`; double-click a handle resets that one to its default.
- [ ] Clamp so no panel can be dragged to zero width — an invisible panel with no
      handle left to grab is a dead end.
- [ ] Verify in Chrome: drag each, reload, confirm the layout survives.

### Task 2 — the GUI serves a directory, not this repo

- [ ] `serve(port, root)`; `root` defaults to the current working directory.
- [ ] File list = `*.delhi` directly in `root` (readable **and** writable) + the ten
      bundled examples, embedded with `include_str!` and marked read-only.
- [ ] Embedding matters for a fresh download: `delhi gui` in an empty directory still
      has something to show. Opening an example and saving writes to `root`, never over
      the built-in.
- [ ] Keep every traversal guard in `is_plain_delhi` — the reason it exists (names
      arrive from a query string) is unchanged, and `root` is now the user's own cwd.
- [ ] Header shows which directory is being served.

### Task 3 — one binary, `delhi gui`

- [ ] `delhi-gui` becomes a library (`pub fn serve(port, root) -> io::Result<()>`); its
      `[[bin]]` goes away.
- [ ] `delhi-cli` gains a `gui` feature, **on by default**, and the subcommand
      `delhi gui [DIR] [-p PORT]`.
- [ ] `--no-default-features` still builds the CLI with zero dependencies. The
      zero-dependency claim was always about the semantic crates; that stays true and
      gets said plainly in the README rather than implied by the workspace layout.
- [ ] `default-members` can then include every crate — the reason for excluding the GUI
      (slow, only crate with deps) is now a feature flag instead.
- [ ] Usage text, README, and `--version`/`--help` handling.

### Task 4 — licence and crate metadata

- [ ] `LICENSE` at the root.
- [ ] `[workspace.package]`: `license`, `description`, `repository`, `homepage`,
      `keywords`, `categories`, `readme`, `rust-version` (MSRV).
- [ ] Per-crate `description`, so `delhi-lang` is usable as a dependency from another
      project and reads sensibly on docs.rs.

### Task 5 — rustfmt and CI

- [ ] `rustfmt.toml` — `max_width = 100`, `use_small_heuristics = "Max"`. Tuned to the
      style already in the tree: the default `fn_call_width = 60` would rewrite 4 847
      lines; this config rewrites 2 069, still touching `delhi-mb` and `delhi-syntax`.
      **Formatting-only, in its own commit, tests green either side.** ⚠️ needs a call.
- [ ] `.github/workflows/ci.yml` — ubuntu / macos / windows × stable: `cargo fmt
      --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
      --workspace`, plus a `--no-default-features` build so the dep-free path stays
      real, plus an MSRV job.

### Task 6 — release workflow

- [ ] `.github/workflows/release.yml`, triggered by a `v*` tag.
- [ ] Targets: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`,
      `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
- [ ] Each archive carries the `delhi` binary, `README.md`, `LICENSE`, and `examples/`,
      named `delhi-<version>-<target>.tar.gz` (`.zip` on Windows).
- [ ] `SHA256SUMS`, `--locked` builds, and a GitHub Release created from the tag.

### Task 7 — install routes, documented

- [ ] README **Install** section covering four routes and what each costs:
      prebuilt archive · `install.sh` / `install.ps1` · `cargo install --git` ·
      from source.
- [ ] `install.sh` and `install.ps1`: resolve the latest release, detect the platform,
      unpack to `~/.local/bin` (or `%LOCALAPPDATA%\delhi\bin`), and say plainly whether
      that directory is on `PATH` rather than silently editing a shell profile.

### Task 8 — repo, push, tag

- [ ] Create the GitHub repo, push `master`, confirm CI green.
- [ ] Tag `v0.1.0`, confirm the release workflow produces working archives.
- [ ] Download one archive on this machine and run it against a folder outside the repo
      — the actual thing a stranger does, which nothing until this step tests.

