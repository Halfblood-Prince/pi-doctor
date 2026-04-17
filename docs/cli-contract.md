# CLI Contract

This document defines the public command-line contract for `pi-doctor`.

## Stability Levels

`pi-doctor` exposes two kinds of output:

- Machine-facing output: intended for automation and treated as stable within a
  schema version.
- Human-facing output: intended for people reading terminal output and allowed
  to evolve between releases.

## Exit Codes

`pi-doctor check` maps report status to process exit code:

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

## Output Format Stability

### Stable

These outputs are part of the public automation contract:

- `pi-doctor check --json`
- `report.json` inside `pi-doctor support-bundle`

For these outputs, the following are stable within a given `schema_version`:

- The semantic meaning of top-level fields
- Exit-code mapping for `check`
- Enum values such as `overall_status`
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

Logs are out-of-band diagnostics and are not part of the CLI output contract.

- Logs are controlled by `PI_DOCTOR_LOG`.
- Logs go to stderr through `env_logger`.
- Consumers should not parse log lines as structured API output.

## Compatibility Guidance

If you are automating against `pi-doctor`:

- Prefer `pi-doctor check --json`
- Gate on `schema_version`
- Use process exit codes only for coarse health status
- Ignore unknown JSON fields
