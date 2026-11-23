from pricing.cache import memoize
from pricing.table import PERCENT, RATE_PER_KG


@memoize
def _quote(weight_kg, zone, service="standard"):
    base = weight_kg * RATE_PER_KG[zone]
    return base * PERCENT[service] // 100


def quote(weight_kg, zone, service="standard"):
    """Return the price of one parcel in cents."""
    if service != "standard":
        # A surcharge changes the price, so drop the old results first.
        _quote.cache_clear()
    return _quote(weight_kg, zone, service=service)
