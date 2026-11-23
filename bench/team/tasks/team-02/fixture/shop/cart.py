def cart_total(prices, skus):
    """The sum of the prices of a list of skus, in cents."""
    return sum(prices.get(sku) for sku in skus)
