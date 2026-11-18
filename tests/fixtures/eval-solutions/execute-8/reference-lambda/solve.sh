git checkout -q -b work
cat > src/sync.py <<'EOF'
payload = lambda items: ["%s,%d" % (n, c) for n, c in sorted(items)]
EOF
cat > tests/test_sync.py <<'EOF'
import unittest

from sync import payload


class SyncTest(unittest.TestCase):
    def test_payload(self):
        self.assertEqual(payload([("nut", 9), ("bolt", 3)]), ["bolt,3", "nut,9"])
EOF
git add -A && git commit -q -m "task t2"
