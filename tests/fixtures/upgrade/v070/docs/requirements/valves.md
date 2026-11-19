---
status: active
---
# Valves

## Requirements

- **drip-first**: When the schedule starts, the controller SHALL open the drip lines before the sprinkler lines.
- **close-on-timeout**: When a moisture sensor times out, the controller SHALL close the valve of its bed.

## Notes

The script `src/schedule.sh` runs the schedule.
