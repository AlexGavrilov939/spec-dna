class Store:
    """The orders of the shop. A caller asks for a range of time."""

    def __init__(self, orders):
        self._orders = list(orders)

    def between(self, start, end):
        """The orders with start <= placed_at < end. The bounds are aware."""
        found = [o for o in self._orders if start <= o.placed_at < end]
        return sorted(found, key=lambda o: (o.placed_at, o.id))
