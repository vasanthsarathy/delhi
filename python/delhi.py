"""A thin Python wrapper around the `delhi` command-line tool.

No dependencies beyond the standard library, and nothing to install: drop this file
next to your code, put `delhi` on your PATH, and

    from delhi import Domain

    d = Domain("examples/coin_lie.delhi")
    d.do("distract_a()", "peek_c()")
    d.eval("K[bob] Kw[carol] h")     # -> True

Every call shells out to `delhi --json`, so it costs one process launch — roughly 3-5 ms
on Linux and 20-25 ms on Windows. That is fine for scripting, dataset generation and
batch evaluation, and it is *not* fine inside a training loop: the model checking itself
takes microseconds, so at that rate you would be timing `fork`. Use `eval_many` to amortise
what can be amortised, and see the README if you need to go faster than this allows.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from dataclasses import dataclass, field
from typing import Iterable, Sequence

__all__ = ["Domain", "DelhiError", "DelhiNotFound", "AgentView", "StateView"]


class DelhiError(RuntimeError):
    """delhi rejected the file, the formula, or the trace.

    The message is delhi's own diagnostic, which for a source error carries the line,
    column and a caret under the offending span.
    """


class DelhiNotFound(DelhiError):
    """The `delhi` binary could not be found."""


@dataclass(frozen=True)
class AgentView:
    """One agent's first-order stance on every proposition."""

    agent: str
    knows: list[str] = field(default_factory=list)
    believes: list[str] = field(default_factory=list)
    undecided: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class StateView:
    """What is true, and what each agent makes of it.

    First-order by construction — one entry per agent per proposition. Nested attitudes
    do not fit this shape; ask for them with :meth:`Domain.eval`.
    """

    facts: list[str]
    agents: list[AgentView]
    worlds: int
    violated: list[str] = field(default_factory=list)


def _find_binary(explicit: str | None) -> str:
    """Resolves the `delhi` executable, preferring an explicit path, then $DELHI_BIN."""
    candidate = explicit or os.environ.get("DELHI_BIN") or "delhi"
    found = shutil.which(candidate) or (candidate if os.path.isfile(candidate) else None)
    if found is None:
        raise DelhiNotFound(
            f"could not find `{candidate}`. Install a release binary from "
            "https://github.com/vasanthsarathy/delhi/releases, or "
            "`cargo install --git https://github.com/vasanthsarathy/delhi delhi-cli`, "
            "then put it on your PATH or set DELHI_BIN to its full path."
        )
    return found


class Domain:
    """A `.delhi` file, plus the trace of actions applied to it so far.

    The trace is replayed from the initial state on every call rather than held as a
    live model, because each call is a separate process. That makes :meth:`do` cheap and
    :meth:`undo` exact, and it means two `Domain` objects over the same file never drift.
    """

    def __init__(self, path: str, binary: str | None = None) -> None:
        self.path = str(path)
        self.binary = _find_binary(binary)
        self.trace: list[str] = []
        # Fail here rather than at the first query, so a malformed file is reported at
        # the line that opened it.
        self._info = self._run(["check"])

    # -- trace ---------------------------------------------------------------

    def do(self, *actions: str) -> "Domain":
        """Appends actions to the trace. Returns self, so calls chain."""
        for a in actions:
            self.trace.append(a)
        return self

    def undo(self, n: int = 1) -> "Domain":
        """Drops the last `n` actions."""
        del self.trace[len(self.trace) - n :]
        return self

    def reset(self) -> "Domain":
        """Clears the trace, returning to the initial state."""
        self.trace.clear()
        return self

    @property
    def actions(self) -> list[str]:
        """Every ground action the domain declares, by name."""
        return list(self._info["actions"])

    # -- queries -------------------------------------------------------------

    def eval(self, formula: str) -> bool:
        """Whether `formula` holds in the state the trace reaches.

        Raises :class:`DelhiError` if the formula is malformed or names something the
        domain does not declare — which is deliberately *not* the same as returning
        False, since a typo would otherwise read as a refuted hypothesis.
        """
        return bool(self._run(["eval", "-f", formula], trace=True)["value"])

    def eval_many(self, formulas: Iterable[str]) -> dict[str, bool]:
        """Evaluates several formulas against the same state.

        Still one process per formula. It exists so the loop reads well and so a future
        batching mode can be slipped in here without changing callers.
        """
        return {f: self.eval(f) for f in formulas}

    def ask(self, pattern: str, depth: int = 0) -> list[str]:
        """Every formula matching `pattern` that holds, where `_` marks the hole.

        `depth` is the modal nesting depth of the candidates tried, so `ask("B[a] _", 2)`
        looks for beliefs about beliefs about beliefs. Candidate counts grow fast with
        depth; :meth:`ask_full` reports whether the search was truncated.
        """
        return self.ask_full(pattern, depth)["matches"]

    def ask_full(self, pattern: str, depth: int = 0) -> dict:
        """:meth:`ask` with the search metadata — `matches`, `considered`, `truncated`."""
        return self._run(["ask", "-d", str(depth), "-q", pattern], trace=True, ok_codes=(0, 1))

    def state(self) -> StateView:
        """Facts and per-agent attitudes in the state the trace reaches."""
        d = self._run(["state"], trace=True)
        return StateView(
            facts=d["facts"],
            agents=[AgentView(**a) for a in d["agents"]],
            worlds=d["worlds"],
            violated=d["violated"],
        )

    def holds(self, *formulas: str) -> bool:
        """Whether every formula holds. Short-circuits on the first that does not."""
        return all(self.eval(f) for f in formulas)

    # -- plumbing ------------------------------------------------------------

    def _run(self, args: Sequence[str], trace: bool = False, ok_codes=(0, 1)) -> dict:
        cmd = [self.binary, args[0], self.path]
        if trace and self.trace:
            cmd += ["-a", *self.trace]
        cmd += list(args[1:]) + ["--json"]

        proc = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8")
        # In --json mode delhi puts errors *inside* the object, so a failure to parse
        # means something else wrote to stdout — a panic, or a binary that predates
        # --json. Say which, rather than raising a bare JSONDecodeError at the caller.
        try:
            payload = json.loads(proc.stdout or "{}")
        except json.JSONDecodeError:
            raise DelhiError(
                f"delhi did not return JSON (exit {proc.returncode}).\n"
                f"stdout: {proc.stdout.strip()!r}\nstderr: {proc.stderr.strip()!r}\n"
                "If this binary predates `--json`, upgrade to 0.1.3 or later."
            ) from None

        if not payload.get("ok", False):
            raise DelhiError(payload.get("error", f"delhi exited {proc.returncode}"))
        if proc.returncode not in ok_codes:
            raise DelhiError(f"delhi exited {proc.returncode}: {proc.stderr.strip()}")
        return payload

    def __repr__(self) -> str:
        return f"Domain({self.path!r}, trace={self.trace!r})"
