from shop.cache import PriceCache
from shop.cart import cart_total
from shop.display import price_label
from shop.store import Store


class Shop:
    def __init__(self, prices=None):
        self.prices = PriceCache(Store(prices))

    def set_price(self, sku, cents):
        self.prices.put(sku, cents)

    def import_prices(self, items):
        self.prices.put_many(items)

    def price(self, sku):
        return self.prices.get(sku)

    def total(self, skus):
        return cart_total(self.prices, skus)

    def label(self, sku):
        return price_label(self.prices, sku)
