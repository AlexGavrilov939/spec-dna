def fee_cents(amount_cents, rate_bp):
    """The fee of an amount, rounded half to even (rule M-2)."""
    quotient, remainder = divmod(amount_cents * rate_bp, 10000)
    twice = remainder * 2
    if twice > 10000 or (twice == 10000 and quotient % 2 == 1):
        quotient += 1
    return quotient


def net_cents(amount_cents, rate_bp):
    """The amount after the fee."""
    return amount_cents - fee_cents(amount_cents, rate_bp)
