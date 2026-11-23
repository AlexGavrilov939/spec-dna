"""The numbered steps of the schema, and the runner that applies them."""


def step_1(conn):
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL)")


def step_2(conn):
    conn.execute("ALTER TABLE users ADD COLUMN active INTEGER NOT NULL DEFAULT 1")


def step_3(conn):
    conn.execute("ALTER TABLE users ADD COLUMN handle TEXT NOT NULL DEFAULT ''")
    conn.execute("UPDATE users SET handle = lower(substr(email, 1, instr(email, '@') - 1))")


STEPS = [(1, step_1), (2, step_2), (3, step_3)]


def current_version(conn):
    """Return the version of the database. A new database has version 0."""
    conn.execute("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)")
    row = conn.execute("SELECT version FROM schema_version").fetchone()
    if row is None:
        conn.execute("INSERT INTO schema_version (version) VALUES (0)")
        conn.commit()
        return 0
    return row[0]


def migrate(conn):
    """Run each step that is newer than the version of the database."""
    version = current_version(conn)
    for number, step in STEPS:
        if number <= version:
            continue
        step(conn)
        conn.execute("UPDATE schema_version SET version = ?", (number,))
        conn.commit()
