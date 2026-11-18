---
status: draft
---
# Add the route GET /items.csv

## Delta

- ADD interfaces/http-api#list-items-csv: When a client sends `GET /items.csv`, the shelf service SHALL answer with the list of items as CSV.

## Interfaces

- CHANGE interfaces/http-api: add the route `GET /items.csv`.

## Decisions

- none

## Concerns

- [ ] The report may list many items after the change.

## Open questions

- [ ] Does a clerk need a way to see the old rule?
