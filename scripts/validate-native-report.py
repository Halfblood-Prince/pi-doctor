#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def load_json(path: Path):
    with path.open(encoding="utf-8-sig") as handle:
        return json.load(handle)


def probe_outcome(report, name: str):
    for health in report.get("probe_health", []):
        if health.get("name") == name:
            return health.get("outcome")
    return None


def finding_ids(report):
    return {finding.get("id") for finding in report.get("findings", [])}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--expected-board-contains", required=True)
    parser.add_argument("--expected-arch-prefix", required=True)
    parser.add_argument("--expect-supported-os", action="store_true")
    parser.add_argument(
        "--camera-expectation",
        choices=["any", "present", "no_camera_or_unavailable"],
        default="any",
    )
    parser.add_argument(
        "--thermal-expectation",
        choices=["any", "hot"],
        default="any",
    )
    args = parser.parse_args()

    report = load_json(args.report)
    system = report.get("system") or {}
    metadata = report.get("metadata") or {}
    supported_os = metadata.get("supported_os") or {}

    if system.get("is_raspberry_pi") is not True:
        raise SystemExit("expected Raspberry Pi hardware, got is_raspberry_pi != true")

    board_model = system.get("board_model") or ""
    if args.expected_board_contains not in board_model:
        raise SystemExit(
            f"expected board model to contain {args.expected_board_contains!r}, got {board_model!r}"
        )

    architecture = system.get("architecture") or ""
    if not architecture.startswith(args.expected_arch_prefix):
        raise SystemExit(
            f"expected architecture prefix {args.expected_arch_prefix!r}, got {architecture!r}"
        )

    if args.expect_supported_os and supported_os.get("supported") is not True:
        raise SystemExit(f"expected supported OS, got {supported_os!r}")

    ids = finding_ids(report)
    if args.camera_expectation == "present":
        if probe_outcome(report, "camera") != "success":
            raise SystemExit("expected camera probe outcome success")
        cameras = ((report.get("camera") or {}).get("cameras")) or []
        if not cameras:
            raise SystemExit("expected at least one camera in camera inventory")
    elif args.camera_expectation == "no_camera_or_unavailable":
        documented_no_camera = bool(
            ids.intersection(
                {
                    "camera.no_cameras_detected",
                    "camera.tool_missing",
                    "camera.unavailable",
                }
            )
        )
        if not documented_no_camera and probe_outcome(report, "camera") != "unavailable":
            raise SystemExit(
                "expected no-camera finding or documented camera probe unavailable state"
            )

    if args.thermal_expectation == "hot":
        hot_ids = {
            "thermal.near_throttle",
            "thermal.throttling_likely",
            "throttling.active",
            "throttling.soft_temp_limit_now",
        }
        if not ids.intersection(hot_ids):
            raise SystemExit(
                "expected near-throttle, throttling, or soft-temperature-limit finding"
            )

    print("native report assertions passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
