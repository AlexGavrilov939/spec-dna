import itertools
import unittest

from pricing.rates import quote

RATE = {"local": 40, "national": 90, "europe": 180}
PERCENT = {"standard": 100, "express": 150, "overnight": 250}
ZONES = ["local", "national", "europe"]
SERVICES = ["standard", "express", "overnight"]


def expected(weight_kg, zone, service="standard"):
    """The price by the rules of docs/pricing.md, with no cache."""
    return weight_kg * RATE[zone] * PERCENT[service] // 100


class QuoteCallsTest(unittest.TestCase):
    def check_all(self, weights, services, zones):
        for weight, zone, service in itertools.product(weights, zones, services):
            with self.subTest(weight=weight, zone=zone, service=service):
                self.assertEqual(quote(weight, zone, service=service), expected(weight, zone, service))

    def test_services_in_the_order_of_the_table(self):
        self.check_all(range(10, 14), SERVICES, ZONES)

    def test_services_in_the_reverse_order(self):
        self.check_all(range(20, 24), list(reversed(SERVICES)), list(reversed(ZONES)))

    def test_services_in_a_mixed_order(self):
        for weight in range(30, 34):
            for step in range(9):
                zone = ZONES[(step * 2) % 3]
                service = SERVICES[(step * 4 + weight) % 3]
                with self.subTest(weight=weight, zone=zone, service=service):
                    self.assertEqual(quote(weight, zone, service=service), expected(weight, zone, service))
        self.check_all(range(30, 34), SERVICES, ZONES)

    def test_the_zone_as_a_keyword(self):
        for weight in (40, 41):
            for zone in ZONES:
                with self.subTest(weight=weight, zone=zone):
                    self.assertEqual(quote(weight, zone=zone), expected(weight, zone))

    def test_the_default_after_another_service(self):
        for weight, zone in ((50, "local"), (51, "europe")):
            self.assertEqual(quote(weight, zone, service="overnight"), expected(weight, zone, "overnight"))
            self.assertEqual(quote(weight, zone), expected(weight, zone))
            self.assertEqual(quote(weight, zone, service="express"), expected(weight, zone, "express"))
            self.assertEqual(quote(weight, zone), expected(weight, zone))

    def test_the_same_call_in_two_forms(self):
        for weight, zone, service in itertools.product((60, 61), ZONES, SERVICES):
            want = expected(weight, zone, service)
            with self.subTest(weight=weight, zone=zone, service=service):
                self.assertEqual(quote(weight, zone, service), want)
                self.assertEqual(quote(weight, zone, service=service), want)
                self.assertEqual(quote(weight, zone=zone, service=service), want)
                self.assertEqual(quote(weight_kg=weight, zone=zone, service=service), want)


if __name__ == "__main__":
    unittest.main()
