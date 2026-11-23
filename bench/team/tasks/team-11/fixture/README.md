# reports

A small tool that builds the monthly report of a shop.

- `reports/build.py` makes the rows of the report from the entries of the month.
- `reports/sample.py` holds the entries of one month, as the shop gave them.
- `reports/csvio.py` saves the rows as CSV text and reads them back.
- `docs/format.md` gives the CSV format.

Run the tests with `python3 -m unittest discover -s tests -t .`.
