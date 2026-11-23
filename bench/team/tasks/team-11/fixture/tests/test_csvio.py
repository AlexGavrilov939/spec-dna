import unittest

from reports.build import monthly_rows
from reports.csvio import export_csv, import_csv


class CsvIoTest(unittest.TestCase):
    def test_exports_plain_rows(self):
        rows = [["a", "b"], ["1", "2"]]
        self.assertEqual(export_csv(rows), "a,b\r\n1,2\r\n")

    def test_imports_plain_rows(self):
        self.assertEqual(import_csv("a,b\r\n1,2\r\n"), [["a", "b"], ["1", "2"]])

    def test_an_empty_report_is_the_empty_text(self):
        self.assertEqual(export_csv([]), "")
        self.assertEqual(import_csv(""), [])

    def test_round_trip_of_a_plain_report(self):
        entries = [
            {"id": 1, "customer": "Eve Lane", "amount_cents": 505, "note": "ok"},
            {"id": 2, "customer": "Fay Hill", "amount_cents": 1000, "note": "late"},
        ]
        rows = monthly_rows(entries)
        self.assertEqual(rows[1], ["1", "Eve Lane", "5.05", "ok"])
        self.assertEqual(import_csv(export_csv(rows)), rows)


if __name__ == "__main__":
    unittest.main()
