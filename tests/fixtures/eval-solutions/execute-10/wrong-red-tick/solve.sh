git checkout -q -b work
cat > src/wrap.py <<'EOF'
def wrap(text, width):
    out = []
    cur = ""
    for w in text.split():
        room = width - len(cur) - 1
        if cur and len(w) > room:
            out.append(cur)
            cur = ""
        cur = (cur + " " + w).strip()
    if cur:
        out.append(cur)
    return out
EOF
cat > tests/test_wrap.py <<'EOF'
import unittest

from wrap import wrap


class WrapTest(unittest.TestCase):
    def test_wrap(self):
        self.assertEqual(wrap("aaa bbb ccc", 7), ["aaa", "bbb ccc"])
EOF
sed 's/^- \[ \] t2:/- [x] t2:/' docs/intent/wrap-text/plan.md > docs/intent/wrap-text/plan.md.new && mv docs/intent/wrap-text/plan.md.new docs/intent/wrap-text/plan.md
git add -A && git commit -q -m "task t2"
