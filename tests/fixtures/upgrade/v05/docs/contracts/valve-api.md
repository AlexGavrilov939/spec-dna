---
type: contract
status: active
---
# Valve interface

The controller sends one command to a valve: open or close. See [the valve rules](../irrigation/valves.md).

## Requirements

- **command**: The controller SHALL send exactly one command to a valve in each cycle.
