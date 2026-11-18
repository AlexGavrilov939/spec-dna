git checkout -q -b work
cat > src/wrap.py <<'EOF'
import textwrap


def wrap(text, width):
    return textwrap.wrap(text, width)
EOF
cat > tests/test_wrap.py <<'EOF'
import unittest

from wrap import wrap


class WrapTest(unittest.TestCase):
    def test_wrap(self):
        self.assertEqual(wrap("aaa bbb ccc", 7), ["aaa bbb", "ccc"])
EOF
sed 's/^- \[ \] t2:/- [x] t2:/' docs/intent/wrap-text/plan.md > docs/intent/wrap-text/plan.md.new && mv docs/intent/wrap-text/plan.md.new docs/intent/wrap-text/plan.md
git add -A && git commit -q -m "task t2"
