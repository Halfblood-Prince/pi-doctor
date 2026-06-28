# Hardware Matrix

This document describes the fixture-backed hardware matrix so parser coverage stays tied
to realistic Raspberry Pi outputs instead of only hand-written unit strings.

## Supported Matrix

`pi-doctor` v1 supports diagnostics for:

- Raspberry Pi 3, 4, and 5
- Raspberry Pi Zero 2 W
- Compute Module variants that expose standard Raspberry Pi firmware and Linux interfaces
- 32-bit ARM and 64-bit ARM Linux userspaces
- Raspberry Pi OS Bookworm and the next supported release, Trixie
- Lite and Desktop images
- modern `rpicam-*` camera stacks and transitional `libcamera-*` camera stacks

Unsupported or partial-support hosts should still produce JSON, but `metadata.supported_os`
must explain why the target is outside the supported matrix.

## Fixture Set

- `fixtures/hardware-matrix/pi4-bookworm-lite-no-camera`
  - Pi 4
  - Lite-style image
  - no camera detected
- `fixtures/hardware-matrix/pi5-bookworm-desktop-camera`
  - Pi 5
  - desktop-style image
  - official camera attached
  - Bookworm externally managed Python
- `fixtures/hardware-matrix/pi5-stressed-lab-rig`
  - Pi 5
  - thermally constrained / undervoltage lab fixture
  - active and historical throttling bits present

## Capture Checklist

For each real hardware capture, collect:

- `/proc/device-tree/model`
- `/proc/cpuinfo`
- `/proc/sys/kernel/osrelease`
- `/etc/os-release`
- `/boot/firmware/config.txt` or `/boot/config.txt`
- `/sys/class/thermal/thermal_zone*/type`
- `/sys/class/thermal/thermal_zone*/temp`
- `vcgencmd get_throttled`
- `pinctrl`
- `rpicam-hello --list-cameras`
- `libcamera-hello --list-cameras` when available
- `python3 --version`
- `python3 -c "import sys; print(sys.executable)"`
- `python3 -c "import sys; print(int(sys.prefix != sys.base_prefix))"`
- `python3 -c "import sysconfig; print(sysconfig.get_path('stdlib'))"`
- `dpkg-query -W -f='${Status}' python3-picamera2`
- `dpkg-query -W -f='${Status}' python3-gpiozero`

## Validation Goals

- every parser reads at least one fixture-captured raw output
- report-building tests exercise both Pi 4 and Pi 5 layouts
- spacing-sensitive command output is covered by parser fixture tests
- no parser depends on a single exact spacing pattern without a matching test
- release binaries run natively on ARM hardware before publication
- parser fuzz and property tests cover malformed, truncated, and spacing-varied inputs
- `pi-doctor check` stays within the documented runtime and memory budget

## Hardware Validation Lab

Cross-target `cargo check` proves that code type-checks for ARM targets. It does
not prove the released ARM binary starts, probes real firmware interfaces, or
handles permissions on Raspberry Pi OS.

Release validation should run the built release binary on controlled hardware:

- Pi 3, Pi 4, Pi 5, Zero 2 W, and one Compute Module target
- at least one 32-bit ARM image and one 64-bit ARM image
- Bookworm Lite, Bookworm Desktop, and Trixie where supported
- one no-camera system, one working-camera system, and one intentionally faulty camera setup
- one system with `vcgencmd` missing or inaccessible
- one thermally constrained system or controlled thermal fixture

Self-hosted GitHub Actions runners can satisfy this when the runner labels make
the board, architecture, OS release, and image type explicit. A separate lab
runner is also acceptable if it records the release artifact digest and command
output in the release checklist.

## Failure Modes To Keep Covered

- no camera
- camera cable or connector issue
- missing `vcgencmd`
- thermal zones with different names or missing permissions
- malformed and truncated command output
- unsupported distributions
- stale and conditional `config.txt` sections
- GPIO contention
- Pi 4 and Pi 5 `pinctrl` format differences

## Performance Budget

`pi-doctor check` should complete within five seconds on supported Raspberry Pi
hardware with default timeouts, and should keep peak memory below 64 MiB for the
CLI process. CI should fail performance regressions once native ARM runners are
available.
