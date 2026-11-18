git checkout -q -b work
cat > src/price.py <<'EOF'
def format_price(cents):
    return "%d.%02d" % (cents // 100, cents % 100)


def apply_discount(price_cents, percent):
    return round(price_cents * (100 - percent) / 100)
EOF
cat > tests/test_price.py <<'EOF'
import unittest

from price import apply_discount, format_price


class PriceTest(unittest.TestCase):
    def test_price(self):
        self.assertEqual(format_price(1050), "10.50")

    def test_discount(self):
        self.assertEqual(apply_discount(1000, 10), 900)
EOF
sed 's/^- \[ \] t2:/- [x] t2:/' docs/intent/discount/plan.md > docs/intent/discount/plan.md.new && mv docs/intent/discount/plan.md.new docs/intent/discount/plan.md
git add -A && git commit -q -m "task t2"
