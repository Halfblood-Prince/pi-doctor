# JSON Schema

See also [CLI Contract](cli-contract.md) for which output formats are considered
stable.

`pi-doctor check --json` currently emits schema version `1.0.0`.

Top-level fields:

- `metadata`: command metadata such as the command name.
- `schema_version`: stable schema identifier for automation clients.
- `overall_status`: one of `healthy`, `warning`, `degraded`, or `critical`.
- `system`: board, distro, kernel, and Raspberry Pi identity summary.
- `config`: parsed `config.txt` source path, diagnostics count, and entry list.
- `camera`: modern camera-tool presence plus parsed camera inventory.
- `python`: Python executable, venv state, external-management flag, and detected distro packages.
- `groups`: findings grouped by domain in deterministic order.
- `findings`: flattened findings list in deterministic order.

Domain order:

1. `system`
2. `power`
3. `thermal`
4. `config`
5. `gpio`
6. `camera`
7. `python`

Overall status rules:

- `healthy`: no findings
- `warning`: warnings present without an explicitly active degraded condition
- `degraded`: active-impact findings such as currently active throttling conditions
- `critical`: reserved for future critical findings

Stability notes:

- Automation should gate behavior on `schema_version`.
- Unknown fields should be ignored.
- Human-readable CLI output is not covered by this schema contract.
