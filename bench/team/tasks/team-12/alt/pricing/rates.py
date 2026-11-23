import functools

from pricing.table import PERCENT, RATE_PER_KG


@functools.lru_cache(maxsize=None)
def _price(weight_kg, zone, service):
    base = weight_kg * RATE_PER_KG[zone]
    return base * PERCENT[service] // 100


def quote(weight_kg, zone, service="standard"):
    """Return the price of one parcel in cents."""
    return _price(weight_kg, zone, service)
