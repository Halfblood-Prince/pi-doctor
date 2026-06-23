# JSON Schema

See also [CLI Contract](cli-contract.md) for which output formats are considered
stable.

`pi-doctor check --json` currently emits schema version `1.0.0`.

Top-level fields:

- `metadata`: command metadata such as the command name.
- `schema_version`: stable schema identifier for automation clients.
- `overall_status`: one of `healthy`, `warning`, `degraded`, or `critical`.
- `probe_health`: per-probe collection outcome records.
- `system`: board, distro, kernel, and Raspberry Pi identity summary.
- `config`: parsed `config.txt` source path, diagnostics count, and entry list.
- `camera`: modern camera-tool presence plus parsed camera inventory.
- `python`: Python executable, venv state, external-management flag, and detected distro packages.
- `groups`: findings grouped by domain in deterministic order.
- `findings`: flattened findings list in deterministic order.

Each `probe_health` entry contains:

- `name`: probe name, such as `board`, `camera`, or `python`.
- `outcome`: one of `success`, `unavailable`, `permission_denied`, `command_failed`, `parse_failed`, or `timed_out`.
- `detail`: nullable diagnostic text for incomplete probes.

Each finding contains an `impact` field. `impact` drives `overall_status` and is
independent of the finding ID or title.

Domain order:

1. `system`
2. `power`
3. `thermal`
4. `config`
5. `gpio`
6. `camera`
7. `python`

Overall status rules:

- `healthy`: no finding above `info` impact
- `warning`: highest finding impact is `warning`
- `degraded`: highest finding impact is `degraded`
- `critical`: highest finding impact is `critical`, such as active firmware throttling or CPU temperature in likely-throttling range

Stability notes:

- Automation should gate behavior on `schema_version`.
- Unknown fields should be ignored.
- Human-readable CLI output is not covered by this schema contract.
