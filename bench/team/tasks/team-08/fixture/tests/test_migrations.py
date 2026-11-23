import sqlite3
import unittest

from store.migrations import migrate


def columns(conn):
    return [row[1] for row in conn.execute("PRAGMA table_info(users)")]


class MigrationsTest(unittest.TestCase):
    def setUp(self):
        self.conn = sqlite3.connect(":memory:")
        self.addCleanup(self.conn.close)

    def version(self):
        return self.conn.execute("SELECT version FROM schema_version").fetchone()[0]

    def test_a_new_database_gets_all_steps(self):
        migrate(self.conn)
        self.assertEqual(self.version(), 3)
        self.assertEqual(columns(self.conn), ["id", "email", "active", "handle"])

    def test_the_table_of_versions_has_one_row(self):
        migrate(self.conn)
        migrate(self.conn)
        count = self.conn.execute("SELECT COUNT(*) FROM schema_version").fetchone()[0]
        self.assertEqual(count, 1)

    def test_a_user_with_a_handle_is_stored(self):
        migrate(self.conn)
        self.conn.execute("INSERT INTO users (email, handle) VALUES ('a@x.test', 'a')")
        row = self.conn.execute("SELECT email, active, handle FROM users").fetchone()
        self.assertEqual(row, ("a@x.test", 1, "a"))

    def test_a_user_with_no_handle_is_refused(self):
        migrate(self.conn)
        with self.assertRaises(sqlite3.IntegrityError):
            self.conn.execute("INSERT INTO users (email, handle) VALUES ('b@x.test', NULL)")

    def test_running_the_steps_twice_changes_nothing(self):
        migrate(self.conn)
        before = columns(self.conn)
        migrate(self.conn)
        self.assertEqual(columns(self.conn), before)
        self.assertEqual(self.version(), 3)


if __name__ == "__main__":
    unittest.main()
