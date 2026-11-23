def fee_cents(amount_cents, rate_bp):
    """The fee of an amount. `rate_bp` is the rate in basis points (1/100 of 1%)."""
    return amount_cents * rate_bp // 10000


def net_cents(amount_cents, rate_bp):
    """The amount after the fee."""
    return amount_cents - fee_cents(amount_cents, rate_bp)
