---
status: approved
---
# Drip schedule

## Files

- `src/schedule.sh` The order of the lines.
- `tests/schedule_test.sh` The test of the order.

## Order

- [ ] t1: Keep each drip line open for 20 minutes. Files: `src/schedule.sh`. Verify: `tests/schedule_test.sh` passes.
- [ ] t2: Test the order of the lines. Files: `tests/schedule_test.sh`. Verify: `tests/schedule_test.sh` passes.

## Risks

- The drip lines can run into the sprinkler time. The script waits for the last drip line.

## Proof

`tests/schedule_test.sh` covers `requirements/valves#drip-first`.
