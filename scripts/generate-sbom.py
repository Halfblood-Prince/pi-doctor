#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def parse_lock(path: Path):
    packages = []
    current = None
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if line == "[[package]]":
            if current:
                packages.append(current)
            current = {}
            continue
        if current is None or " = " not in line:
            continue
        key, value = line.split(" = ", 1)
        current[key] = value.strip('"')
    if current:
        packages.append(current)
    return packages


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--name", default="pi-doctor")
    parser.add_argument("--version", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--lockfile", default="Cargo.lock", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    components = []
    for package in parse_lock(args.lockfile):
        if "name" not in package or "version" not in package:
            continue
        component = {
            "type": "library",
            "name": package["name"],
            "version": package["version"],
            "purl": f"pkg:cargo/{package['name']}@{package['version']}",
        }
        if "source" in package:
            component["externalReferences"] = [
                {"type": "distribution", "url": package["source"]}
            ]
        components.append(component)

    bom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "name": args.name,
                "version": args.version,
                "properties": [{"name": "target", "value": args.target}],
            }
        },
        "components": components,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(bom, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
