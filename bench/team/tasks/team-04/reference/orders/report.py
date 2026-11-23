from datetime import datetime, time, timedelta
from zoneinfo import ZoneInfo

from orders.clock import day_bounds


def orders_of_day(store, day, zone="UTC"):
    """The ids of the orders of one calendar day in a time zone."""
    if zone == "UTC":
        start, end = day_bounds(day)
    else:
        tz = ZoneInfo(zone)
        start = datetime.combine(day, time.min, tzinfo=tz)
        end = datetime.combine(day + timedelta(days=1), time.min, tzinfo=tz)
    return [order.id for order in store.between(start, end)]
