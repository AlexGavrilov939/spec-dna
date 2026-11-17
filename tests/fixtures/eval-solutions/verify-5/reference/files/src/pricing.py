TAX = 0.2


def with_tax(price):
    return round(price * (1 + TAX), 2)


def shipping(price):
    return 0 if price >= 50 else 5
