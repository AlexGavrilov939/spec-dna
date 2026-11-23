def format_cents(cents):
    """The text of an amount, for example 1250 gives '12.50'."""
    whole, part = divmod(cents, 100)
    return f"{whole}.{part:02d}"


def price_label(prices, sku):
    """The text of the price of a sku, for example '$12.50'."""
    return "$" + format_cents(prices.store.get(sku))
