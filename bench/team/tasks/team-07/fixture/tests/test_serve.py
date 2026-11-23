import os
import shutil
import tempfile
import unittest

from fileserver.errors import AccessDenied, NotFound
from fileserver.serve import read_file


class ReadFileTest(unittest.TestCase):
    def setUp(self):
        self.root = os.path.realpath(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root)
        self.base = os.path.join(self.root, "site")
        os.makedirs(os.path.join(self.base, "docs"))
        self.write(os.path.join(self.base, "index.txt"), b"home")
        self.write(os.path.join(self.base, "docs", "guide.txt"), b"guide")
        self.write(os.path.join(self.root, "secret.txt"), b"secret")

    def write(self, path, data):
        with open(path, "wb") as handle:
            handle.write(data)

    def test_serves_a_file(self):
        self.assertEqual(read_file(self.base, "index.txt"), b"home")

    def test_serves_a_file_in_a_folder(self):
        self.assertEqual(read_file(self.base, "docs/guide.txt"), b"guide")

    def test_a_missing_file_is_not_found(self):
        with self.assertRaises(NotFound):
            read_file(self.base, "docs/none.txt")

    def test_refuses_a_name_that_leaves_the_folder(self):
        with self.assertRaises(AccessDenied):
            read_file(self.base, "../secret.txt")


if __name__ == "__main__":
    unittest.main()
