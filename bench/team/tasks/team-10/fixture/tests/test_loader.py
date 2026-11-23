import unittest

from appconf.loader import load_config


class OlderSettingsTest(unittest.TestCase):
    def test_defaults(self):
        config = load_config([], {}, "")
        self.assertEqual(config["host"], "localhost")
        self.assertEqual(config["retries"], 3)

    def test_host_from_each_source(self):
        self.assertEqual(load_config(["--host=a.test"], {}, "")["host"], "a.test")
        self.assertEqual(load_config([], {"APP_HOST": "b.test"}, "")["host"], "b.test")
        self.assertEqual(load_config([], {}, "host = c.test\n")["host"], "c.test")

    def test_retries_from_the_environment_and_the_file(self):
        self.assertEqual(load_config([], {"APP_RETRIES": "5"}, "")["retries"], 5)
        self.assertEqual(load_config([], {}, "retries = 6\n")["retries"], 6)

    def test_retries_from_the_flag(self):
        self.assertEqual(load_config(["--retries=7"], {}, "")["retries"], 7)

    def test_the_file_ignores_comments_and_blank_lines(self):
        config = load_config([], {}, "# a comment\n\nhost = example.test\n")
        self.assertEqual(config["host"], "example.test")


class TimeoutTest(unittest.TestCase):
    def test_the_default_is_30(self):
        self.assertEqual(load_config([], {}, "")["timeout"], 30)

    def test_timeout_from_each_source(self):
        self.assertEqual(load_config(["--timeout=1"], {}, "")["timeout"], 1)
        self.assertEqual(load_config([], {"APP_TIMEOUT": "2"}, "")["timeout"], 2)
        self.assertEqual(load_config([], {}, "timeout = 3\n")["timeout"], 3)

    def test_the_value_is_an_integer(self):
        self.assertIsInstance(load_config([], {"APP_TIMEOUT": "9"}, "")["timeout"], int)


if __name__ == "__main__":
    unittest.main()
