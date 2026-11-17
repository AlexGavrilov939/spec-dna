class Cart:
    def __init__(self, items=None):
        self.items = [] if items is None else items

    def add(self, name, qty=1):
        self.items.append((name, qty))

    def count(self):
        return sum(qty for _, qty in self.items)
