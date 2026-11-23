import unittest

from payments.client import charge_order
from payments.errors import CardDeclined, ChargeFailed, TransientError


class LedgerGateway:
    """A gateway that charges each key once. A call can fail after the charge."""

    def __init__(self, script):
        self.script = list(script)  # "before", "after" or None for each call
        self.ledger = {}
        self.keys = []

    def charge(self, amount_cents, key):
        self.keys.append(key)
        step = self.script.pop(0) if self.script else None
        if step == "before":
            raise TransientError()
        if key not in self.ledger:
            self.ledger[key] = amount_cents
        if step == "after":
            raise TransientError()
        return {"amount": self.ledger[key], "key": key}

    def total(self):
        return sum(self.ledger.values())


class IdempotencyTest(unittest.TestCase):
    def test_a_timeout_after_the_charge_does_not_charge_twice(self):
        gateway = LedgerGateway(["after", None])
        receipt = charge_order(gateway, "B1", 4000)
        self.assertEqual(receipt["amount"], 4000)
        self.assertEqual(gateway.total(), 4000)

    def test_each_attempt_sends_the_same_key(self):
        gateway = LedgerGateway(["before", "after", None])
        charge_order(gateway, "B2", 700)
        self.assertEqual(len(gateway.keys), 3)
        self.assertEqual(len(set(gateway.keys)), 1)

    def test_the_key_is_the_key_of_the_order(self):
        gateway = LedgerGateway([])
        charge_order(gateway, "B3", 700)
        self.assertEqual(gateway.keys, ["order-B3"])

    def test_a_second_call_for_the_same_order_does_not_charge_twice(self):
        gateway = LedgerGateway([])
        charge_order(gateway, "B4", 1500)
        charge_order(gateway, "B4", 1500)
        self.assertEqual(gateway.total(), 1500)

    def test_gives_up_after_3_attempts_with_one_key(self):
        gateway = LedgerGateway(["after", "after", "after", "after"])
        with self.assertRaises(ChargeFailed):
            charge_order(gateway, "B5", 800)
        self.assertEqual(len(gateway.keys), 3)
        self.assertEqual(gateway.total(), 800)

    def test_a_declined_card_is_not_retried(self):
        class Declining(LedgerGateway):
            def charge(self, amount_cents, key):
                self.keys.append(key)
                raise CardDeclined()

        gateway = Declining([])
        with self.assertRaises(CardDeclined):
            charge_order(gateway, "B6", 800)
        self.assertEqual(len(gateway.keys), 1)


if __name__ == "__main__":
    unittest.main()
