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
        self.assertEqual(count_words("a b  c"), 3)
EOF
git add -A && git commit -q -m "task t2"
