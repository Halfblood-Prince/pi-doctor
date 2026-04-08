# Exit Codes

`pi-doctor check` uses severity-based exit codes:

- `0`: healthy
- `1`: warning
- `2`: degraded
- `3`: critical
- `4`: internal error

Notes:

- `explain`, `doctor`, `support-bundle`, and `completions` return `0` on success.
- Internal CLI failures return `4`.
