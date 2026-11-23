import unittest

from payments.client import charge_order
from payments.errors import CardDeclined, ChargeFailed, TransientError


class ScriptedGateway:
    """A gateway that fails as the script says, before it takes the money."""

    def __init__(self, script):
        self.script = list(script)
        self.calls = []

    def charge(self, amount_cents, key):
        self.calls.append((amount_cents, key))
        step = self.script.pop(0) if self.script else None
        if step is not None:
            raise step
        return {"amount": amount_cents, "key": key}


class ChargeOrderTest(unittest.TestCase):
    def test_charges_once_when_the_gateway_works(self):
        gateway = ScriptedGateway([])
        receipt = charge_order(gateway, "A1", 1250)
        self.assertEqual(receipt["amount"], 1250)
        self.assertEqual(len(gateway.calls), 1)

    def test_retries_after_an_outage(self):
        gateway = ScriptedGateway([TransientError(), None])
        receipt = charge_order(gateway, "A2", 900)
        self.assertEqual(receipt["amount"], 900)
        self.assertEqual(len(gateway.calls), 2)

    def test_gives_up_after_3_attempts(self):
        gateway = ScriptedGateway([TransientError()] * 5)
        with self.assertRaises(ChargeFailed):
            charge_order(gateway, "A3", 900)
        self.assertEqual(len(gateway.calls), 3)

    def test_does_not_retry_a_declined_card(self):
        gateway = ScriptedGateway([CardDeclined()])
        with self.assertRaises(CardDeclined):
            charge_order(gateway, "A4", 900)
        self.assertEqual(len(gateway.calls), 1)


if __name__ == "__main__":
    unittest.main()
