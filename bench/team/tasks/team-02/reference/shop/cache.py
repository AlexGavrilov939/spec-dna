class PriceCache:
    """A cache over the store. A read goes to the store once for each sku."""

    def __init__(self, store):
        self.store = store
        self._seen = {}

    def get(self, sku):
        if sku not in self._seen:
            self._seen[sku] = self.store.get(sku)
        return self._seen[sku]

    def put(self, sku, cents):
        self.store.put(sku, cents)
        self._seen.pop(sku, None)

    def put_many(self, items):
        self.store.put_many(items)
        for sku in items:
            self._seen.pop(sku, None)
