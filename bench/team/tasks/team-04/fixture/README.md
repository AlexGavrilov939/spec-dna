# orders

A small report of the orders of a shop.

- `orders/model.py`: `Order` has an `id` (a string) and `placed_at`. `placed_at` is an aware `datetime` in UTC.
- `orders/store.py`: `Store(orders)` holds the orders. `Store.between(start, end)` returns the orders with `start <= placed_at < end`, sorted by `placed_at` and then by `id`. The 2 bounds are aware datetimes.
- `orders/clock.py`: `day_bounds(day)` gives the bounds of one day in UTC.
- `orders/report.py`: the reports.

Run the tests with `python3 -m unittest discover -s tests -t .`.
