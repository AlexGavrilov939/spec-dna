from payments.keys import new_key


def charge_order(gateway, order_id, amount_cents):
    """Charge an order. Return the receipt of the gateway."""
    return gateway.charge(amount_cents, key=new_key())
