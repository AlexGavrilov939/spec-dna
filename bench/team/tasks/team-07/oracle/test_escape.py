import os
import shutil
import tempfile
import unittest

from fileserver.errors import AccessDenied
from fileserver.serve import read_file


class EscapeTest(unittest.TestCase):
    """The links are made here, at run time. The runner does not copy a link."""

    def setUp(self):
        self.root = os.path.realpath(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root)
        self.base = os.path.join(self.root, "site")
        self.evil = os.path.join(self.root, "site-evil")
        self.other = os.path.join(self.root, "other")
        for folder in (os.path.join(self.base, "docs"), self.evil, self.other):
            os.makedirs(folder)
        self.write(os.path.join(self.base, "index.txt"), b"home")
        self.write(os.path.join(self.base, "docs", "guide.txt"), b"guide")
        self.write(os.path.join(self.root, "secret.txt"), b"secret")
        self.write(os.path.join(self.evil, "loot.txt"), b"loot")
        self.write(os.path.join(self.other, "inner.txt"), b"inner")

    def write(self, path, data):
        with open(path, "wb") as handle:
            handle.write(data)

    def link(self, target, name):
        os.symlink(target, os.path.join(self.base, name))

    def test_refuses_dot_dot(self):
        for name in ("../secret.txt", "docs/../../secret.txt"):
            with self.subTest(name=name):
                with self.assertRaises(AccessDenied):
                    read_file(self.base, name)

    def test_refuses_an_absolute_path(self):
        for path in (
            os.path.join(self.root, "secret.txt"),
            os.path.join(self.evil, "loot.txt"),
        ):
            with self.subTest(path=path):
                with self.assertRaises(AccessDenied):
                    read_file(self.base, path)

    def test_refuses_a_folder_with_the_same_start(self):
        with self.assertRaises(AccessDenied):
            read_file(self.base, "../site-evil/loot.txt")

    def test_refuses_a_link_to_a_file_outside(self):
        self.link("../secret.txt", "relative.txt")
        self.link(os.path.join(self.evil, "loot.txt"), "absolute.txt")
        for name in ("relative.txt", "absolute.txt"):
            with self.subTest(name=name):
                with self.assertRaises(AccessDenied):
                    read_file(self.base, name)

    def test_refuses_a_file_behind_a_link_to_a_folder_outside(self):
        self.link(self.other, "shared")
        with self.assertRaises(AccessDenied):
            read_file(self.base, "shared/inner.txt")

    def test_serves_a_file_behind_a_link_inside_the_folder(self):
        self.link(os.path.join(self.base, "docs", "guide.txt"), "latest.txt")
        self.link("docs", "manual")
        self.assertEqual(read_file(self.base, "latest.txt"), b"guide")
        self.assertEqual(read_file(self.base, "manual/guide.txt"), b"guide")

    def test_a_base_path_with_a_link_in_it(self):
        os.symlink(self.root, os.path.join(self.root, "via"))
        base = os.path.join(self.root, "via", "site")
        self.assertEqual(read_file(base, "docs/guide.txt"), b"guide")
        with self.assertRaises(AccessDenied):
            read_file(base, "../secret.txt")


if __name__ == "__main__":
    unittest.main()
