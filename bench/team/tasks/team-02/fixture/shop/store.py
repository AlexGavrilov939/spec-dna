class Store:
    """The true prices, in cents."""

    def __init__(self, prices=None):
        self._prices = dict(prices or {})

    def get(self, sku):
        return self._prices[sku]

    def put(self, sku, cents):
        self._prices[sku] = cents

    def put_many(self, items):
        self._prices.update(items)
