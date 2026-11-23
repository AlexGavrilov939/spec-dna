from fractions import Fraction


def fee_cents(amount_cents, rate_bp):
    """The fee of an amount. A Fraction is exact, and round() of it goes half to even."""
    return round(Fraction(amount_cents * rate_bp, 10000))


def net_cents(amount_cents, rate_bp):
    """The amount after the fee."""
    return amount_cents - fee_cents(amount_cents, rate_bp)
