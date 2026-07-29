# Further reading

## The primary sources for delhi

**Buckingham, D. (2021).** *Epistemic Planning with Belief.* PhD thesis. The direct source
for mB — the plausibility-model semantics, the action types, the observability model, and
the Coin Lie scenario that runs through delhi's examples and this book.

**Buckingham, Wang & Sardiña (2021).** "Epistemic Planning with Belief" and its companion
paper at KR 2021. The conference-length presentations of the above.

## Where belief revision comes from

**Baltag, A. & Smets, S. (2006).** "Conditional Doxastic Models: A Qualitative Approach to
Dynamic Belief Revision." *ENTCS* 165. Plausibility models, safe belief, conditional
belief — the operators delhi's `[]` and `B^ψ` implement.

**Baltag, A. & Smets, S. (2008).** "A Qualitative Theory of Dynamic Interactive Belief
Revision." *Texts in Logic and Games* 3. The fuller treatment, including action-priority
update.

## Dynamic epistemic logic generally

**van Ditmarsch, van der Hoek & Kooi (2007).** *Dynamic Epistemic Logic.* Springer. The
standard textbook. Start here if the [logic chapters](../logic/possible-worlds.md) of this
book left you wanting the full development.

**Baltag, Moss & Solecki (1998).** "The Logic of Public Announcements, Common Knowledge,
and Private Suspicions." Where event models and product update come from — the machinery
underneath [How attitudes change](../logic/dynamics.md).

**Fagin, Halpern, Moses & Vardi (1995).** *Reasoning About Knowledge.* MIT Press. The
foundational text for the static side: possible worlds, S5, common knowledge, and the muddy
children puzzle done properly.

## Epistemic planning

**Bolander & Andersen (2011).** "Epistemic Planning for Single- and Multi-Agent Systems."
Establishes plan existence as undecidable in general DEL — the result the action languages
exist to route around.

**Baral, Gelfond, Pontelli & Son.** The mA\* line of work: an action language for epistemic
planning that trades expressiveness for tractability.

**Le, Fabiano, Son & Pontelli (2018).** "EFP and PG-EFP: Epistemic Forward Search Planners
in Multi-Agent Domains," and Fabiano et al.'s EFP 2.0. The performance benchmark for
forward-search epistemic planning.

**Muise, Belle, Felli, McIlraith, Miller, Pearce & Sonenberg.** The PDKB line — proper
epistemic knowledge bases, depth-bounded, compiled to classical planning. delhi borrows the
modal-literal representation for its `ask` enumeration.

## Theory of mind, empirically

The false-belief tasks delhi's examples reproduce come from developmental psychology rather
than logic:

**Wimmer & Perner (1983).** "Beliefs about beliefs" — the original false-belief paradigm.

**Baron-Cohen, Leslie & Frith (1985).** "Does the autistic child have a 'theory of mind'?"
The Sally-Anne task as it is now known.

**Sullivan, Zaitchik & Tager-Flusberg (1994).** "Preschoolers can attribute second-order
beliefs." The Birthday Bicycle Story — `examples/bicycle.delhi`.

**Perner & Wimmer (1985).** "'John thinks that Mary thinks that…'" Second-order attribution
and the ice-cream van task — `examples/ice_cream_van.delhi`.
