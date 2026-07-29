"""Tests for the Python wrapper, runnable with plain `python test_delhi.py`.

They drive the real binary, so they double as an end-to-end check that `--json` says
what the wrapper thinks it says. `DELHI_BIN` picks the binary; the default is whatever
`delhi` resolves to on PATH.
"""

import os
import sys
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from delhi import Domain, DelhiError, DelhiNotFound  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
COIN = os.path.join(ROOT, "examples", "coin_lie.delhi")
SALLY = os.path.join(ROOT, "examples", "sally_anne.delhi")

PASSED, FAILED = 0, []


def test(fn):
    global PASSED
    try:
        fn()
        PASSED += 1
    except Exception:
        FAILED.append((fn.__name__, traceback.format_exc()))
    return fn


@test
def a_domain_reports_its_actions():
    d = Domain(COIN)
    assert d.actions == ["announce_not_heads()", "distract_a()", "peek_c()"], d.actions


@test
def eval_answers_against_the_trace_not_the_initial_state():
    d = Domain(COIN)
    # False before the peek, true after — so a trace silently ignored would fail here.
    assert d.eval("K[bob] Kw[carol] h") is False
    d.do("distract_a()", "peek_c()")
    assert d.eval("K[bob] Kw[carol] h") is True
    # ...and alice, distracted, is oblivious rather than merely uninformed.
    assert d.eval("K[alice] Kw[carol] h") is False
    assert d.eval("?[alice] Kw[carol] h") is True


@test
def undo_and_reset_walk_the_trace_back():
    d = Domain(COIN).do("distract_a()", "peek_c()")
    assert d.eval("K[bob] Kw[carol] h") is True
    d.undo()
    assert d.eval("K[bob] Kw[carol] h") is False
    d.reset()
    assert d.trace == []


@test
def a_malformed_formula_raises_rather_than_returning_false():
    # The distinction that matters: a typo must not read as a refuted hypothesis.
    d = Domain(COIN)
    try:
        d.eval("K[nobody] h")
    except DelhiError as e:
        assert "nobody" in str(e), e
    else:
        raise AssertionError("an undeclared agent must raise")


@test
def an_unknown_action_raises():
    d = Domain(COIN).do("nope()")
    try:
        d.state()
    except DelhiError as e:
        assert "nope()" in str(e), e
    else:
        raise AssertionError("an unknown action must raise")


@test
def a_missing_file_raises_with_the_diagnostic():
    try:
        Domain(os.path.join(ROOT, "examples", "does_not_exist.delhi"))
    except DelhiError:
        pass
    else:
        raise AssertionError("a missing file must raise")


@test
def a_missing_binary_says_how_to_get_one():
    try:
        Domain(COIN, binary="delhi-that-does-not-exist")
    except DelhiNotFound as e:
        assert "releases" in str(e), e
    else:
        raise AssertionError("a missing binary must raise DelhiNotFound")


@test
def state_returns_structured_attitudes():
    d = Domain(COIN).do("announce_not_heads()")
    s = d.state()
    assert "h" in s.facts, s.facts
    carol = next(a for a in s.agents if a.agent == "carol")
    # The lie landed: she believes !h while the world has h.
    assert "!h" in carol.believes, carol
    assert s.worlds >= 1 and s.violated == []


@test
def ask_enumerates_and_reports_its_search():
    d = Domain(SALLY).do("sally_leaves()", "anne_moves()", "sally_returns()")
    matches = d.ask("B[sally] _")
    assert any("basket" in m for m in matches), matches
    full = d.ask_full("B[sally] _", depth=1)
    assert full["considered"] > 0 and full["truncated"] is False

    # A well-formed pattern that matches nothing is an empty list, not an error. In
    # Coin Lie's initial state alice knows every proposition, so nothing is undecided.
    assert Domain(COIN).ask("?[alice] _") == []

    # A pattern with no hole is a different thing entirely — malformed, and it raises.
    try:
        Domain(COIN).ask("B[carol] h")
    except DelhiError as e:
        assert "_" in str(e), e
    else:
        raise AssertionError("a pattern without a hole must raise")


@test
def holds_and_eval_many_compose():
    d = Domain(COIN).do("distract_a()", "peek_c()")
    assert d.holds("Kw[carol] h", "K[bob] Kw[carol] h") is True
    got = d.eval_many(["Kw[carol] h", "K[alice] Kw[carol] h"])
    assert got == {"Kw[carol] h": True, "K[alice] Kw[carol] h": False}, got


@test
def the_sally_anne_false_belief_reproduces():
    d = Domain(SALLY).do("sally_leaves()", "anne_moves()", "sally_returns()")
    assert d.eval("B[sally] basket & !basket") is True


if __name__ == "__main__":
    for name, tb in FAILED:
        print(f"FAIL {name}\n{tb}")
    print(f"{PASSED} passed, {len(FAILED)} failed")
    sys.exit(1 if FAILED else 0)
