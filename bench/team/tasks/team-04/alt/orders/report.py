from datetime import timedelta
from zoneinfo import ZoneInfo

from orders.clock import day_bounds


def orders_of_day(store, day, zone="UTC"):
    """The ids of the orders of one calendar day in a time zone."""
    tz = ZoneInfo(zone)
    # Take a wide range in UTC, then keep the orders whose local date is the day.
    start, _ = day_bounds(day - timedelta(days=1))
    _, end = day_bounds(day + timedelta(days=1))
    return [o.id for o in store.between(start, end) if o.placed_at.astimezone(tz).date() == day]
