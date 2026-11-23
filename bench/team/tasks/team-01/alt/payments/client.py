from payments.errors import ChargeFailed, TransientError


def charge_order(gateway, order_id, amount_cents):
    """Charge an order. Return the receipt of the gateway."""
    key = "order-" + str(order_id)
    failures = 0
    while True:
        try:
            return gateway.charge(amount_cents, key=key)
        except TransientError as error:
            failures += 1
            if failures >= 3:
                raise ChargeFailed("the gateway is down") from error
