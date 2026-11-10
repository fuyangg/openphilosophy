#!/usr/bin/env python3
"""Check a Casson surgery calculation and retain its exploration history.

The surgery formula and input polynomials are assumptions, not machine-proved
facts. All differentiation and arithmetic use exact rational numbers.
"""
import argparse
import json
import subprocess
import tempfile
from fractions import Fraction
from pathlib import Path


def second_derivative_at_one(polynomial):
    """Differentiate a Laurent polynomial (exponent -> coefficient) at t=1."""
    return sum(
        (Fraction(coefficient) * exponent * (exponent - 1)
         for exponent, coefficient in polynomial.items()),
        Fraction(0),
    )


def casson_plus_one(polynomial):
    return second_derivative_at_one(polynomial) / 2


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def explore(db):
    def invoke(*args, request=None):
        completed = subprocess.run(
            ["ophi", "--db", str(db), *args],
            input=json.dumps(request) if request is not None else None,
            capture_output=True, text=True, check=True,
        )
        response = json.loads(completed.stdout)
        require(response["protocol_version"] == 1, "Unexpected CLI protocol")
        return response["data"]

    case = invoke("case", "create", request={
        "title": "Casson invariant: +1 surgery on the right-handed trefoil",
    })
    entries = []

    def append(kind, content, references=(), author="example-explorer"):
        entry = invoke("entry", "append", request={
            "case_id": case["id"], "kind": kind,
            "content": content, "author": author,
            "references": [
                {"target_id": target["id"], "relation": relation}
                for target, relation in references
            ],
        })
        entries.append(entry)
        return entry

    assumptions = append("assumption", (
        "Normalize lambda(S^3)=0. Assume the surgery formula "
        "lambda(S^3_{+1}(K))=Delta_K''(1)/2, using the standard "
        "meridian/Seifert-longitude framing and orientation induced by surgery "
        "on standard oriented S^3. Use the symmetrized, normalized trefoil "
        "polynomial Delta(t)=t-1+t^(-1); for the unknot use Delta(t)=1. "
        "Assume lambda(-M)=-lambda(M). These are external mathematical inputs."
    ))
    failed = append("failed_attempt", (
        "Intentionally incorrect attempt: Delta''(1)=2, so lambda=2. "
        "This omits the factor 1/2 in the stated surgery formula."
    ), [(assumptions, "uses")])
    correction = append("correction", (
        "The value 2 is the second derivative, not the Casson invariant. "
        "The surgery formula requires division by 2."
    ), [(failed, "supersedes"), (assumptions, "uses")])
    submitted = append("proof_submitted", (
        "For Delta(t)=t-1+t^(-1), Delta'(t)=1-t^(-2) and "
        "Delta''(t)=2t^(-3). Therefore Delta''(1)=2 and lambda=1 "
        "under the stated surgery convention. This calculation is conditional "
        "on the assumed surgery theorem and input polynomial."
    ), [(correction, "uses"), (assumptions, "uses")])

    trefoil = {1: 1, 0: -1, -1: 1}
    unknot = {0: 1}
    require(sum(trefoil.values()) == 1, "Trefoil polynomial must be normalized")
    require(all(trefoil.get(-p, 0) == c for p, c in trefoil.items()),
            "Trefoil polynomial must be symmetric")
    derivative = second_derivative_at_one(trefoil)
    trefoil_value = casson_plus_one(trefoil)
    unknot_value = casson_plus_one(unknot)
    reversed_value = -trefoil_value
    require((derivative, trefoil_value, unknot_value, reversed_value) == (2, 1, 0, -1),
            "Exact arithmetic check failed")
    append("verification", (
        "Exact rational arithmetic check passed: sum a_n*n*(n-1)=2 "
        "for the trefoil polynomial, yielding lambda=1 after division by 2. "
        "The constant unknot polynomial gives 0; negating the trefoil result "
        "gives -1 under the assumed orientation-reversal law. Scope: "
        "polynomial differentiation and arithmetic only. The topology theorem, "
        "polynomial identification, and orientation law are assumed, not proved."
    ), [(submitted, "checks"), (assumptions, "uses")], author="example-arithmetic-checker")

    # Each invocation opens the database in a separate process.
    require(invoke("case", "get", case["id"]) == case, "Case retrieval mismatch")
    for entry in entries:
        require(invoke("entry", "get", entry["id"]) == entry,
                "Entry changed after a subsequent write")
        for reference in entry["references"]:
            original = next(e for e in entries if e["id"] == reference["target_id"])
            require(invoke("entry", "get", reference["target_id"]) == original,
                    "Reference target retrieval mismatch")

    history = []
    cursor = None
    while True:
        args = ["case", "history", case["id"], "--limit", "2"]
        if cursor is not None:
            args.extend(["--after", cursor])
        page = invoke(*args)
        history.extend(page["entries"])
        cursor = page["next_cursor"]
        if cursor is None:
            break
    require(history == entries, "Paginated history has missing or reordered entries")
    require(invoke("entry", "get", submitted["id"])["kind"] == "proof_submitted",
            "Submission must not be promoted to verified by Core")
    require(invoke("entry", "get", failed["id"]) == failed,
            "The incorrect attempt must remain unchanged")

    return {
        "case_id": case["id"],
        "failed_attempt_id": failed["id"],
        "submitted_entry_id": submitted["id"],
        "results": {
            "unknot_plus_one_surgery": int(unknot_value),
            "right_handed_trefoil_plus_one_surgery": int(trefoil_value),
            "orientation_reversed_trefoil_surgery": int(reversed_value),
        },
        "checks": {
            "arithmetic": "passed",
            "retrieval_and_references": "passed",
            "pagination": "passed",
            "submitted_kind_preserved": "passed",
        },
        "scope": "Arithmetic and history checks, conditional on the stated mathematical inputs.",
        "history": history,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, help="Keep history in this SQLite file; otherwise use a temporary database")
    args = parser.parse_args()
    if args.db is not None:
        result = explore(args.db)
        result["database"] = str(args.db)
    else:
        with tempfile.TemporaryDirectory(prefix="ophi-casson-") as directory:
            result = explore(Path(directory) / "history.db")
        result["database"] = "temporary (removed after the checks)"
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
