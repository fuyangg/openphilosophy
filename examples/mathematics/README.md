# Mathematics Example: the Casson Invariant

This example follows a small calculation through a wrong attempt, a correction, and a separate arithmetic check. It tests whether OpenPhilosophy retains the reasoning and references across independent CLI processes.

## Mathematical setup

For an oriented integral homology sphere, let the Casson invariant be normalized by

$$\lambda(S^3)=0.$$

Use the surgery convention

$$\lambda(S^3_{+1}(K))=\frac{1}{2}\Delta_K''(1),$$

where the symmetrized Alexander polynomial satisfies
$\Delta_K(t)=\Delta_K(t^{-1})$ and $\Delta_K(1)=1$.
Here $S^3_{+1}(K)$ means +1 surgery with the standard meridian/Seifert-longitude framing, oriented by surgery on the standard oriented $S^3$.

For the right-handed trefoil, use

$$\Delta_K(t)=t-1+t^{-1},\qquad \Delta_K''(t)=2t^{-3}.$$

Therefore $\Delta_K''(1)=2$ and the surgery formula gives $\lambda=1$.
For the unknot, $\Delta(t)=1$ gives $\lambda=0$, consistent with +1 surgery yielding $S^3$.
Reversing the orientation negates the Casson invariant, giving $-1$ for the reversed trefoil surgery manifold.

**Scope:** the surgery formula, the Alexander polynomials, and the orientation-reversal law are mathematical inputs to this example. The script checks exact arithmetic conditional on those inputs; it does not prove these theorems, identify arbitrary manifolds, or act as a formal proof checker. Orientation conventions must be matched before comparing signs with another source.

For background, see the [Casson invariant](https://en.wikipedia.org/wiki/Casson_invariant) and [Alexander polynomial](https://en.wikipedia.org/wiki/Alexander_polynomial).

## Run

Install `ophi` as described in the [main README](../../README.md), then run from the repository root:

```bash
python3 examples/mathematics/casson_invariant.py --db ./casson.db
```

This keeps the database so another session can retrieve the printed Case and Entry IDs. Repeated runs create new Cases. Omit `--db` to use a temporary database, as CI does.

The script uses only Python 3's standard library. Every operation starts a new `ophi` process.

## Recorded exploration

| Step | Entry kind | What is preserved |
| --- | --- | --- |
| 1 | assumption | Normalization, surgery convention, and input polynomials |
| 2 | failed_attempt | An intentionally wrong result of 2 from omitting the factor 1/2 |
| 3 | correction | The missing factor, with a reference to the failed attempt |
| 4 | proof_submitted | The corrected calculation giving 1 |
| 5 | verification | An explicitly limited arithmetic check, referencing the submission and assumptions |

The checker differentiates the Laurent polynomial at 1 using exact rational arithmetic. It checks the unknot control and the orientation-reversal arithmetic. It then retrieves every Entry by ID, paginates the Case history, and follows references back to their original records.

Expected summary:

```json
{
  "results": {
    "unknot_plus_one_surgery": 0,
    "right_handed_trefoil_plus_one_surgery": 1,
    "orientation_reversed_trefoil_surgery": -1
  },
  "checks": {
    "arithmetic": "passed",
    "retrieval_and_references": "passed",
    "pagination": "passed",
    "submitted_kind_preserved": "passed"
  }
}
```

The actual output also includes IDs, the stored history, and the check's scope.
A `verification` Entry is a caller's statement about a check. Core does not promote the submitted Entry: its kind remains `proof_submitted`. The wrong attempt remains queryable too.

## Continue in a later session

Replace placeholders with the IDs printed by the run:

```bash
ophi --db ./casson.db case history <case-id> --limit 2
ophi --db ./casson.db entry get <failed-attempt-id>
ophi --db ./casson.db entry get <submitted-entry-id>
```

Use this pattern for other investigations: state the assumptions, preserve unsuccessful work, append corrections with references, and describe exactly what any independent check establishes.
