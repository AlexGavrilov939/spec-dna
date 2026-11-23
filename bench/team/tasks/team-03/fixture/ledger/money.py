def format_cents(cents):
    """The text of an amount, for example 1250 gives '12.50' and -5 gives '-0.05'."""
    sign = "-" if cents < 0 else ""
    whole, part = divmod(abs(cents), 100)
    return f"{sign}{whole}.{part:02d}"
