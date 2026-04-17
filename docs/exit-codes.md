# Exit Codes

See also [CLI Contract](cli-contract.md) for the broader compatibility promise.

`pi-doctor check` uses severity-based exit codes:

- `0`: healthy
- `1`: warning
- `2`: degraded
- `3`: critical
- `4`: internal error

Notes:

- `explain`, `doctor`, `support-bundle`, and `completions` return `0` on success.
- Internal CLI failures return `4`.
- Exit codes are the stable machine-facing summary for `check`, but automation that
  needs detail should prefer `pi-doctor check --json`.
