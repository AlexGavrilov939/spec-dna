from money import to_cents


def total_cents(lines):
    return sum(int(round(float(price) * 100)) * qty for price, qty in lines)
