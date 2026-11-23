from orders.clock import day_bounds


def orders_of_day(store, day):
    """The ids of the orders of one day in UTC."""
    start, end = day_bounds(day)
    return [order.id for order in store.between(start, end)]
