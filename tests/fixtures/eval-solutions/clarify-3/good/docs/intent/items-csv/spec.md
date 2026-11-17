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

- [x] The route can be slow when the shop has 10,000 items. Answer: the route writes the lines one by one, so the number of items does not matter.

## Open questions

- [x] Does the CSV file have a header line? Answer: yes, the first line is the header.
