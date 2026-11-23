import sqlite3
import unittest

from store.migrations import migrate

OLD_SCHEMA = """
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE schema_version (version INTEGER NOT NULL);
INSERT INTO schema_version (version) VALUES (2);
"""


def old_database(rows):
    conn = sqlite3.connect(":memory:")
    conn.executescript(OLD_SCHEMA)
    conn.executemany("INSERT INTO users (id, email, active) VALUES (?, ?, ?)", rows)
    conn.commit()
    return conn


def handles(conn):
    return dict(conn.execute("SELECT id, handle FROM users ORDER BY id"))


class HandleMigrationTest(unittest.TestCase):
    def make(self, rows):
        conn = old_database(rows)
        self.addCleanup(conn.close)
        return conn

    def test_old_rows_get_the_handle_from_the_email(self):
        conn = self.make([(1, "Ann.Lee@Example.test", 1), (2, "bob@example.test", 1)])
        migrate(conn)
        self.assertEqual(handles(conn), {1: "ann.lee", 2: "bob"})

    def test_a_shared_handle_goes_to_the_lowest_id(self):
        rows = [
            (4, "kim@b.test", 1),
            (2, "Kim@a.test", 1),
            (9, "KIM@c.test", 1),
            (5, "ann@a.test", 1),
        ]
        conn = self.make(rows)
        migrate(conn)
        self.assertEqual(handles(conn), {2: "kim", 4: "kim-4", 5: "ann", 9: "kim-9"})

    def test_old_rows_keep_their_other_values(self):
        conn = self.make([(7, "x@a.test", 0), (8, "y@a.test", 1)])
        migrate(conn)
        rows = conn.execute("SELECT id, email, active FROM users ORDER BY id").fetchall()
        self.assertEqual(rows, [(7, "x@a.test", 0), (8, "y@a.test", 1)])

    def test_the_version_is_3(self):
        conn = self.make([(1, "a@a.test", 1)])
        migrate(conn)
        self.assertEqual(conn.execute("SELECT version FROM schema_version").fetchall(), [(3,)])

    def test_a_new_row_must_give_a_handle(self):
        conn = self.make([(1, "a@a.test", 1)])
        migrate(conn)
        with self.assertRaises(sqlite3.IntegrityError):
            conn.execute("INSERT INTO users (email) VALUES ('c@a.test')")
        with self.assertRaises(sqlite3.IntegrityError):
            conn.execute("INSERT INTO users (email, handle) VALUES ('c@a.test', NULL)")

    def test_a_new_row_with_a_handle_is_kept(self):
        conn = self.make([(1, "a@a.test", 1)])
        migrate(conn)
        conn.execute("INSERT INTO users (email, handle) VALUES ('d@a.test', 'dee')")
        conn.commit()
        row = conn.execute("SELECT email, active, handle FROM users WHERE id > 1").fetchone()
        self.assertEqual(row, ("d@a.test", 1, "dee"))

    def test_a_second_run_changes_nothing(self):
        conn = self.make([(1, "kim@a.test", 1), (2, "kim@b.test", 0)])
        migrate(conn)
        first = conn.execute("SELECT * FROM users ORDER BY id").fetchall()
        migrate(conn)
        self.assertEqual(conn.execute("SELECT * FROM users ORDER BY id").fetchall(), first)
        self.assertEqual(conn.execute("SELECT version FROM schema_version").fetchall(), [(3,)])


if __name__ == "__main__":
    unittest.main()
