# Known Limitations

## Supported Hardware Scope

The intended hardware matrix is Raspberry Pi 3, 4, 5, Zero 2 W, and Compute
Module variants where the same Raspberry Pi OS interfaces are available.

Unsupported or partially supported systems may still run the CLI, but findings
should be treated as best-effort diagnostics:

- non-Raspberry-Pi boards
- custom kernels without standard Raspberry Pi paths
- vendor images that remove `vcgencmd`, camera tools, or GPIO utilities
- boards using unpublished or experimental firmware interfaces

## Operating System Scope

Primary validation targets Raspberry Pi OS Bookworm and the next supported
release on 32-bit ARM and 64-bit ARM. Other Debian-like systems may work but are
not release-blocking unless they are added to the supported matrix.

## Probe Blind Spots

`pi-doctor` is read-only and does not load drivers, edit boot configuration,
restart services, or claim exclusive access to GPIO or camera hardware. Some
conditions can therefore be inferred only indirectly:

- loose or damaged camera cables
- intermittent power faults not present during the probe window
- GPIO contention from short-lived processes
- thermal problems that appear only under sustained workload
- boot configuration inside conditional sections that are not active on the
  current board

## Expected False Positives

The tool may warn when:

- a repeated `dtoverlay` or conflicting `dtparam` is intentional
- Python is intentionally managed outside a virtual environment
- camera tools are missing on a headless system with no camera workload
- GPIO tools are unavailable because the image is intentionally minimal

Warnings are designed to be explainable and actionable rather than silent.

## Privacy Notes

Support bundles are sanitized by default, and structured logs redact common
secret and personal-data patterns. Redaction is best effort. Review bundles and
logs before sharing them outside a trusted support channel.
