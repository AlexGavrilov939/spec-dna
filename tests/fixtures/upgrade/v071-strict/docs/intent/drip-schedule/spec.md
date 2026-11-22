---
status: approved
---
# Drip schedule

## Delta

- CHANGE requirements/valves#drip-first@dd31cea3cccfb3f0: When the schedule starts, the controller SHALL open the drip lines before the sprinkler lines, and it SHALL keep each drip line open for 20 minutes.

## Interfaces

This change touches no interface.

## Decisions

- [[decisions/0001-drip-first]]

## Concerns

- [x] The drip lines can run into the sprinkler time. Resolved: the script waits for the last drip line.

## Open questions

- [x] Does the schedule run at night? Answer: yes, from 02:00.
