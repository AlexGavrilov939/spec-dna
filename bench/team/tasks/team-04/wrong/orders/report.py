from datetime import datetime, time
from zoneinfo import ZoneInfo

from orders.clock import day_bounds


def orders_of_day(store, day, zone="UTC"):
    """The ids of the orders of one calendar day in a time zone."""
    start, end = day_bounds(day)
    # Move the UTC day by the offset of the zone at the start of the day.
    offset = ZoneInfo(zone).utcoffset(datetime.combine(day, time.min))
    return [order.id for order in store.between(start - offset, end - offset)]
