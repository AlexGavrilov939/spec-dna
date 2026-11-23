from payments.errors import ChargeFailed, TransientError

ATTEMPTS = 3


def order_key(order_id):
    return f"order-{order_id}"


def charge_order(gateway, order_id, amount_cents):
    """Charge an order. Return the receipt of the gateway."""
    key = order_key(order_id)
    for attempt in range(ATTEMPTS):
        try:
            return gateway.charge(amount_cents, key=key)
        except TransientError:
            if attempt == ATTEMPTS - 1:
                raise ChargeFailed(f"order {order_id}: the gateway failed {ATTEMPTS} times")
