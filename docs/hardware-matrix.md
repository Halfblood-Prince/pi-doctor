# Hardware Matrix

This document describes the fixture-backed hardware matrix so parser coverage stays tied
to realistic Raspberry Pi outputs instead of only hand-written unit strings.

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
- `/sys/class/thermal/thermal_zone0/temp`
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
