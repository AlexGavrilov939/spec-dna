import unicodedata
import unittest

from slugs.store import SlugStore

# One name in 2 forms. A person types "e with acute" as 1 code point (NFC), and
# another system writes it as "e" and a combining mark (NFD). Both look the same.
JOSE_NFC = "Jos\u00e9 Garc\u00eda"
JOSE_NFD = "Jose\u0301 Garci\u0301a"
JOSE_MIX = "Jose\u0301 Garc\u00eda"


def canon(text):
    """Compare slugs by their text, not by the form in which a solution returns them."""
    return unicodedata.normalize("NFC", text)


class FormsTest(unittest.TestCase):
    def test_precomposed_then_decomposed_is_a_clash(self):
        store = SlugStore()
        first = store.add(JOSE_NFC)
        second = store.add(JOSE_NFD)
        self.assertEqual(canon(second), canon(first) + "-2")

    def test_decomposed_then_precomposed_is_a_clash(self):
        store = SlugStore()
        first = store.add(JOSE_NFD)
        second = store.add(JOSE_NFC)
        self.assertEqual(canon(second), canon(first) + "-2")

    def test_a_mixed_form_and_a_capital_form_clash_too(self):
        store = SlugStore()
        first = store.add(JOSE_NFC)
        second = store.add(JOSE_MIX)
        third = store.add(JOSE_NFD.upper())
        self.assertEqual(canon(second), canon(first) + "-2")
        self.assertEqual(canon(third), canon(first) + "-3")

    def test_the_suffix_counts_across_forms(self):
        store = SlugStore()
        slugs = [store.add(name) for name in (JOSE_NFC, JOSE_NFD, JOSE_NFC, JOSE_NFD)]
        first = canon(slugs[0])
        self.assertEqual([canon(s) for s in slugs], [first, first + "-2", first + "-3", first + "-4"])

    def test_a_name_with_an_accent_is_not_the_name_without_it(self):
        store = SlugStore()
        plain = store.add("Jose Garcia")
        accent = store.add(JOSE_NFD)
        self.assertEqual(plain, "jose-garcia")
        self.assertNotEqual(canon(accent), plain)
        self.assertFalse(canon(accent).endswith("-2"))

    def test_the_suffix_is_the_smallest_free_number(self):
        store = SlugStore()
        self.assertEqual(store.add("Ana 2"), "ana-2")
        self.assertEqual(store.add("Ana"), "ana")
        self.assertEqual(store.add("Ana"), "ana-3")
        self.assertEqual(store.add("Ana"), "ana-4")


if __name__ == "__main__":
    unittest.main()
