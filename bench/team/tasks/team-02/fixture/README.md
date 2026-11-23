# shop

A small price list for a shop.

- `shop/app.py`: `Shop(prices)` is the public object. `prices` is a dict of sku to price in cents.
  - `set_price(sku, cents)` sets one price.
  - `import_prices(items)` sets many prices. `items` is a dict of sku to cents.
  - `price(sku)` gives a price in cents.
  - `total(skus)` gives the sum of the prices of a list of skus, in cents.
  - `label(sku)` gives the text of a price, for example `$12.50`.
- `shop/store.py`: the store of the prices. It holds the true values.
- `shop/cache.py`: a cache over the store. A read of a price goes to the store only once for each sku.
- `shop/display.py`: the text of a price.
- `shop/cart.py`: the total of a cart.

Run the tests with `python3 -m unittest discover -s tests -t .`.
