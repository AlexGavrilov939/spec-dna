git checkout -q -b work
cat > src/store.py <<'EOF'
ITEMS = {
    "bolt": {"count": 3, "bin": "A1"},
    "nut": {"count": 9, "bin": "A2"},
}


def get(name):
    item = ITEMS[name]
    return {"name": name, "count": item["count"], "bin": item["bin"]}
EOF
cat > src/label.py <<'EOF'
from store import get


def label(name):
    item = get(name)
    return "%s x%d %s" % (item["name"], item["count"], item["bin"])
EOF
cat > tests/test_label.py <<'EOF'
import unittest

from label import label


class LabelTest(unittest.TestCase):
    def test_label(self):
        self.assertEqual(label("bolt"), "bolt x3 A1")
EOF
sed 's/^- \[ \] t2:/- [x] t2:/' docs/intent/bin-label/plan.md > docs/intent/bin-label/plan.md.new && mv docs/intent/bin-label/plan.md.new docs/intent/bin-label/plan.md
git add -A && git commit -q -m "task t2"
