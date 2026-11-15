---
depends-on: [irrigation/sensors]
verified-by: [src/controller.rs, tests/valves.rs]
owner: team-water
---
# Valves

A valve lets water into one bed of the greenhouse. ADR-1 names the controller.

## Requirements

- **shutoff**: When a tray overflows, the controller SHALL close the valve of that tray.
- **flush**: The valve SHALL be flushed quickly, if possible.
- **R-7**: While a valve is open, the controller SHALL read its moisture sensor every 10 seconds.
- **Bad_Key**: The controller MUST log the valve id.

## Rationale

A valve that stays open floods the bed. See [the sensor rules](sensors.md#requirements) and [a missing anchor](sensors.md#nowhere).
