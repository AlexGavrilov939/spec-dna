git checkout -q -b work
cat > src/sync.py <<'EOF'
def payload(items):
    return ["%s,%d" % (name, count) for name, count in items]
EOF
cat > tests/test_sync.py <<'EOF'
import unittest

from sync import payload


class SyncTest(unittest.TestCase):
    def test_payload(self):
        self.assertEqual(payload([("nut", 9), ("bolt", 3)]), ["bolt,3", "nut,9"])
EOF
git add -A && git commit -q -m "task t2"
