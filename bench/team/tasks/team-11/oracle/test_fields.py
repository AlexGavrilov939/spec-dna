import unittest

from reports.csvio import export_csv, import_csv


def round_trip(case, rows):
    case.assertEqual(import_csv(export_csv(rows)), rows)


class TrickyFieldsTest(unittest.TestCase):
    def test_commas_and_quotes(self):
        round_trip(
            self,
            [
                ["id", "customer", "note"],
                ["1", "Stone, Ada", 'Called it "the big one"'],
                ["2", '"', '""'],
                ["3", ",", '"a",b'],
                ["4", 'He said, "no", twice', "x"],
            ],
        )

    def test_line_breaks(self):
        round_trip(
            self,
            [
                ["a", "line one\nline two"],
                ["b", "line one\r\nline two"],
                ["c", "\n"],
                ["d", "\r\n"],
                ["e", "\r"],
                ["f", "ends\n"],
                ["g", "\nstarts"],
                ["h", 'quote "and"\r\nbreak, and comma'],
            ],
        )

    def test_empty_fields_and_spaces(self):
        round_trip(
            self,
            [
                ["", ""],
                ["x", ""],
                ["", "x"],
                ["", "x", ""],
                [" lead", "trail "],
                ["  ", " a  b "],
                ["\t", "a\tb"],
                ["", '"', ""],
            ],
        )

    def test_every_short_field(self):
        alphabet = ["a", ",", '"', "\n", "\r", " "]
        fields = [""] + list(alphabet)
        fields += [x + y for x in alphabet for y in alphabet]
        fields += [x + y + z for x in alphabet for y in alphabet for z in alphabet]
        size = len(fields)
        rows = [
            [f, fields[(i * 7 + 1) % size], fields[(i * 13 + 5) % size]]
            for i, f in enumerate(fields)
        ]
        round_trip(self, rows)


class PinnedTextTest(unittest.TestCase):
    def test_text_with_a_comma_and_quotes(self):
        rows = [["id", "note"], ["1", 'said "hi", left'], ["2", "plain text"]]
        text = 'id,note\r\n1,"said ""hi"", left"\r\n2,plain text\r\n'
        self.assertEqual(export_csv(rows), text)
        self.assertEqual(import_csv(text), rows)

    def test_text_with_line_breaks_and_empty_fields(self):
        rows = [["a", "x\r\ny"], ["", "q"], ["m\nn", "\""], ["k", " p "]]
        text = 'a,"x\r\ny"\r\n,q\r\n"m\nn",""""\r\nk, p \r\n'
        self.assertEqual(export_csv(rows), text)
        self.assertEqual(import_csv(text), rows)


if __name__ == "__main__":
    unittest.main()
