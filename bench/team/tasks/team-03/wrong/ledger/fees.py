def fee_cents(amount_cents, rate_bp):
    """The fee of an amount, rounded to the nearest cent. A half goes up."""
    return (amount_cents * rate_bp * 2 + 10000) // 20000


def net_cents(amount_cents, rate_bp):
    """The amount after the fee."""
    return amount_cents - fee_cents(amount_cents, rate_bp)
