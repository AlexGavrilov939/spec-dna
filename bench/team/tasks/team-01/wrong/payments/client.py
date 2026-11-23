from payments.errors import ChargeFailed, TransientError
from payments.keys import new_key

ATTEMPTS = 3


def charge_order(gateway, order_id, amount_cents):
    """Charge an order. Return the receipt of the gateway."""
    for attempt in range(ATTEMPTS):
        try:
            return gateway.charge(amount_cents, key=new_key())
        except TransientError:
            if attempt == ATTEMPTS - 1:
                raise ChargeFailed(f"order {order_id}: the gateway failed {ATTEMPTS} times")
