class Store:
    """The true prices, in cents. `version` grows with each write."""

    def __init__(self, prices=None):
        self._prices = dict(prices or {})
        self.version = 0

    def get(self, sku):
        return self._prices[sku]

    def put(self, sku, cents):
        self._prices[sku] = cents
        self.version += 1

    def put_many(self, items):
        self._prices.update(items)
        self.version += 1
