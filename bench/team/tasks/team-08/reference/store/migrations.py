"""The numbered steps of the schema, and the runner that applies them."""


def step_1(conn):
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL)")


def step_2(conn):
    conn.execute("ALTER TABLE users ADD COLUMN active INTEGER NOT NULL DEFAULT 1")


def step_3(conn):
    """Add the required column `handle`. SQLite needs a new table for it."""
    conn.execute(
        "CREATE TABLE users_new ("
        "id INTEGER PRIMARY KEY, email TEXT NOT NULL, "
        "active INTEGER NOT NULL DEFAULT 1, handle TEXT NOT NULL)"
    )
    conn.execute(
        """
        WITH base AS (
            SELECT id, email, active, lower(substr(email, 1, instr(email, '@') - 1)) AS handle
            FROM users
        )
        INSERT INTO users_new (id, email, active, handle)
        SELECT id, email, active,
            CASE WHEN EXISTS (SELECT 1 FROM base o WHERE o.handle = b.handle AND o.id < b.id)
                THEN b.handle || '-' || b.id ELSE b.handle END
        FROM base b
        """
    )
    conn.execute("DROP TABLE users")
    conn.execute("ALTER TABLE users_new RENAME TO users")


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
