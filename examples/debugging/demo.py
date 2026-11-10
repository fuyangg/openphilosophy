#!/usr/bin/env python3
"""Run after installing ophi. Uses only Python's standard library."""
import json
import subprocess
import tempfile
from pathlib import Path


def main():
    with tempfile.TemporaryDirectory(prefix="ophi-demo-") as directory:
        db = Path(directory) / "history.db"

        def invoke(*args, request=None):
            result = subprocess.run(
                ["ophi", "--db", str(db), *args],
                input=json.dumps(request) if request is not None else None,
                capture_output=True, text=True, check=True,
            )
            return json.loads(result.stdout)["data"]

        case = invoke("case", "create", request={"title": "Investigate timeout"})
        failed = invoke("entry", "append", request={
            "case_id": case["id"], "kind": "failure", "author": "human",
            "content": "Increasing the request timeout did not prevent pool exhaustion.",
        })
        discovery = invoke("entry", "append", request={
            "case_id": case["id"], "kind": "discovery", "author": "agent",
            "content": "Releasing leaked connections resolved the exhaustion in this reproduction.",
            "references": [{"target_id": failed["id"], "relation": "explains"}],
        })
        # Every invoke starts a new process, reopening the same database.
        original = invoke("entry", "get", failed["id"])
        history = invoke("case", "history", case["id"])
        assert original == failed
        assert history["entries"] == [failed, discovery]
        print(json.dumps(history, ensure_ascii=False, indent=2))
        print("Verified: independent processes recovered the original history and references.")


if __name__ == "__main__":
    main()
