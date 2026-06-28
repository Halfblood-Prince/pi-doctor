#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

import jsonschema


def load_json(path: Path):
    with path.open(encoding="utf-8-sig") as handle:
        return json.load(handle)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--registry", type=Path)
    parser.add_argument("reports", nargs="+", type=Path)
    args = parser.parse_args()

    schema = load_json(args.schema)
    by_id = None
    if args.registry:
        registry = load_json(args.registry)
        registry_entries = registry.get("findings", [])
        by_id = {entry["id"]: entry for entry in registry_entries}
        if len(by_id) != len(registry_entries):
            raise SystemExit("finding registry contains duplicate ids")

        categories = set(registry.get("remediation_categories", []))
        for entry in registry_entries:
            if entry.get("category") not in categories:
                raise SystemExit(f"{entry['id']} uses unknown category {entry.get('category')}")

    validator = jsonschema.Draft202012Validator(schema)
    for report_path in args.reports:
        report = load_json(report_path)
        errors = sorted(validator.iter_errors(report), key=lambda error: error.json_path)
        if errors:
            joined = "\n".join(f"{error.json_path}: {error.message}" for error in errors)
            raise SystemExit(f"{report_path} failed schema validation:\n{joined}")

        if by_id is not None:
            for finding in report.get("findings", []):
                entry = by_id.get(finding.get("id"))
                if entry is None:
                    raise SystemExit(
                        f"{report_path} emitted unregistered finding {finding.get('id')}"
                    )
                for key in ("severity", "impact"):
                    if finding.get(key) != entry.get(key):
                        raise SystemExit(
                            f"{report_path} finding {finding['id']} has {key}={finding.get(key)}, "
                            f"registry expects {entry.get(key)}"
                        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
