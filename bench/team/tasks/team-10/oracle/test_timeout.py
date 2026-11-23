import itertools
import unittest

from appconf.loader import load_config


def load(flag=None, env=None, file=None):
    """Build the 3 sources. None means that the source is absent."""
    argv = [] if flag is None else [f"--timeout={flag}"]
    environ = {} if env is None else {"APP_TIMEOUT": env}
    text = "" if file is None else f"# the settings\ntimeout = {file}\n"
    return load_config(argv, environ, text)["timeout"]


class TimeoutOrderTest(unittest.TestCase):
    def test_the_8_combinations_of_present_and_absent(self):
        values = {"flag": "11", "env": "22", "file": "33"}
        for present in itertools.product([False, True], repeat=3):
            with self.subTest(present=present):
                given = {
                    name: value if on else None
                    for (name, value), on in zip(values.items(), present)
                }
                chosen = [v for v in given.values() if v is not None]
                expected = int(chosen[0]) if chosen else 30
                self.assertEqual(load(**given), expected)

    def test_the_flag_wins_over_the_environment(self):
        self.assertEqual(load(flag="5", env="6"), 5)

    def test_the_environment_wins_over_the_file(self):
        self.assertEqual(load(env="6", file="7"), 6)

    def test_an_empty_flag_gives_way_to_the_next_source(self):
        self.assertEqual(load(flag="", env="6", file="7"), 6)
        self.assertEqual(load(flag="", file="7"), 7)
        self.assertEqual(load(flag=""), 30)

    def test_an_empty_environment_variable_gives_way_to_the_file(self):
        self.assertEqual(load(env="", file="7"), 7)
        self.assertEqual(load(flag="5", env="", file="7"), 5)

    def test_an_empty_file_value_gives_way_to_the_default(self):
        self.assertEqual(load(file=""), 30)
        self.assertEqual(load(flag="", env="", file=""), 30)

    def test_the_value_is_an_integer(self):
        result = load(flag="12")
        self.assertIsInstance(result, int)
        self.assertEqual(result, 12)


if __name__ == "__main__":
    unittest.main()
