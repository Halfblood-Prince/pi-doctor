# CLI Contract

This document defines the public command-line contract for `pi-doctor`.

## Stability Levels

`pi-doctor` exposes two kinds of output:

- Machine-facing output: intended for automation and treated as stable within a
  schema version.
- Human-facing output: intended for people reading terminal output and allowed
  to evolve between releases.

## Exit Codes

`pi-doctor check` maps impact-based report status to process exit code:

- `0`: `healthy`
- `1`: `warning`
- `2`: `degraded`
- `3`: `critical`

Any internal CLI failure returns:

- `4`: command execution or rendering failure

The following commands return `0` on success:

- `pi-doctor explain <topic>`
- `pi-doctor doctor <target>`
- `pi-doctor support-bundle`
- `pi-doctor completions <shell>`
- `pi-doctor --help`
- `pi-doctor --version`

## Timeout Contract

All commands accept the global `--timeout <SECONDS>` option. The default is
three seconds per external command. When a probe command exceeds the timeout,
the process is terminated, the probe reports `timed_out` in JSON
`probe_health`, and `check` continues with the remaining probes.

## Output Format Stability

### Stable

These outputs are part of the public automation contract:

- `pi-doctor check --json`
- `pi-doctor --json doctor camera`
- `pi-doctor --json doctor gpio`
- `pi-doctor --json support-bundle`
- `report.json` inside `pi-doctor support-bundle`

For these outputs, the following are stable within a given `schema_version`:

- The semantic meaning of top-level fields
- Exit-code mapping for `check`
- Enum values such as `overall_status`
- Enum values such as `probe_health[].outcome`
- Finding `impact` values and their rollup to `overall_status`
- The deterministic ordering of `groups` and `findings`
- UTF-8 text output terminated by a trailing newline from the CLI

Additive fields may be introduced in a future minor release, but breaking
changes require a `schema_version` change.

### Not Stable

These outputs are human-oriented and should not be parsed as a strict API:

- `pi-doctor check` without `--json`
- `pi-doctor explain <topic>`
- `pi-doctor doctor <target>`
- `manifest.txt` and `report.txt` inside support bundles
- `--help` and shell completions formatting

They are tested for regressions, but wording, spacing, and presentation may
change between releases.

## Logging Contract

Logs are out-of-band diagnostics and are not part of the report JSON contract.

- Logs are controlled by `PI_DOCTOR_LOG`.
- Logs go to stderr.
- Set `PI_DOCTOR_LOG_FORMAT=json` to emit JSON-lines logs with `level`,
  `target`, and redacted `message` fields.
- Normal `--json` command output remains on stdout.
- Consumers should not treat log fields as a stable machine-report schema.

Example:

```bash
PI_DOCTOR_LOG=debug PI_DOCTOR_LOG_FORMAT=json pi-doctor --json check > report.json 2> pi-doctor.log.jsonl
```

## Read-Only Probe Contract

`pi-doctor` diagnostics are read-only. The tool does not write to system paths,
load kernel modules, enable interfaces, edit boot configuration, or restart
services during diagnostic commands.

Files and directories read by probes:

- `/proc/device-tree/model`
- `/proc/cpuinfo`
- `/proc/sys/kernel/osrelease`
- `/proc/sys/kernel/arch`
- `/etc/os-release`
- `/usr/lib/os-release`
- `/boot/firmware/config.txt`
- `/boot/config.txt`
- `/sys/class/thermal/thermal_zone0/temp`
- `/dev` directory names matching `video*`
- Python `EXTERNALLY-MANAGED` marker path reported by `python3 -c 'import sysconfig; ...'`

Executables checked for presence in `PATH`:

- `rpicam-hello`
- `libcamera-hello`
- `pinctrl`
- `raspi-gpio`
- `gpioinfo`
- `gpiodetect`
- `python3`
- `dpkg-query`

External commands run by probes:

- `vcgencmd get_throttled`
- `vcgencmd version` for support bundles
- `rpicam-hello --list-cameras`
- `libcamera-hello --list-cameras`
- `pinctrl`
- `python3 --version`
- `python3 -c 'import sys; print(sys.executable)'`
- `python3 -c 'import sys; print(int(sys.prefix != sys.base_prefix))'`
- `python3 -c 'import sysconfig; print(sysconfig.get_path("stdlib"))'`
- `dpkg-query -W -f=${Status} python3-picamera2`
- `dpkg-query -W -f=${Status} python3-gpiozero`

Command output is bounded. A command that exceeds the output limit is reported
as `command_failed` in `probe_health` and does not block the rest of the report.

## Support Bundle Privacy Contract

`pi-doctor support-bundle` writes sanitized bundles by default. Use
`--dry-run` to print the complete collection plan without reading probe data or
writing files. Use `--output DIR` to choose the output directory.

Sensitive mode requires both flags:

- `--include-sensitive`
- `--acknowledge-sensitive-data`

Sanitized bundles redact common personal and secret-bearing patterns including
home paths, hostnames, usernames, IPv4 and IPv6 addresses, MAC addresses, serial
numbers, Wi-Fi SSIDs, URLs, tokens, credentials, device IDs, and private-key
blocks. Every bundle contains `privacy.txt` and a `manifest.txt` with SHA-256
hashes for payload files.

## Config Rule Contract

`config.txt` diagnostics are section-aware:

- Repeating the same `dtoverlay` name in the same section is reported.
- Different `dtoverlay` values in the same section are allowed.
- Repeating a `dtparam` with the same value in the same section is allowed.
- Repeating a `dtparam` with conflicting values in the same section is reported.
- The same overlay or parameter may appear in different sections without being
  treated as a duplicate.

## Compatibility Guidance

If you are automating against `pi-doctor`:

- Prefer `pi-doctor check --json`
- Gate on `schema_version`
- Use process exit codes only for coarse health status
- Ignore unknown JSON fields
