# pricing

A small library that prices the delivery of a parcel.

- `pricing/table.py` holds the rates.
- `pricing/rates.py` has `quote`, the price of one parcel.
- `pricing/cache.py` has a helper that keeps results of slow calls.
- `docs/pricing.md` gives the price rules.

Run the tests with `python3 -m unittest discover -s tests -t .`.
