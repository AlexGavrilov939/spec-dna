from datetime import datetime, time, timedelta, timezone


def day_bounds(day):
    """The start and the end of a day in UTC, as aware datetimes."""
    start = datetime.combine(day, time.min, tzinfo=timezone.utc)
    return start, start + timedelta(hours=24)
