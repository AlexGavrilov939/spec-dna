git checkout -q -b work
cat > src/words.py <<'EOF'
def count_words(text):
    return len(text.split())
EOF
cat > tests/test_words.py <<'EOF'
import unittest

from words import count_words


class WordsTest(unittest.TestCase):
    def test_count_words(self):
        self.assertEqual(count_words("a b"), 3)
EOF
sed 's/^- \[ \] t2:/- [x] t2:/' docs/intent/word-stats/plan.md > docs/intent/word-stats/plan.md.new && mv docs/intent/word-stats/plan.md.new docs/intent/word-stats/plan.md
git add -A && git commit -q -m "task t2"
