from pricing.cache import memoize
from pricing.table import PERCENT, RATE_PER_KG


@memoize
def quote(weight_kg, zone, service="standard"):
    """Return the price of one parcel in cents."""
    base = weight_kg * RATE_PER_KG[zone]
    return base * PERCENT[service] // 100
