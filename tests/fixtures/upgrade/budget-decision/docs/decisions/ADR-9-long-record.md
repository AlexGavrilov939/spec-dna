---
status: accepted
---
# Long record

## Context

The pump fills the reservoir at night. The valves open in turn, one zone at a time, so the pressure stays within the limit of the pipes. A sensor in each zone reports the soil moisture before each cycle.

## Decision

The controller skips a zone when the soil moisture is above the set value, and it logs the skip.
