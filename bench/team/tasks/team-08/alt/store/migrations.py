"""The numbered steps of the schema, and the runner that applies them."""


def make_handles(rows):
    """Give each (id, email) row a handle. The lowest id keeps a shared handle."""
    taken = set()
    handles = {}
    for row_id, email in sorted(rows):
        handle = email.split("@", 1)[0].lower()
        if handle in taken:
            handle = f"{handle}-{row_id}"
        taken.add(handle)
        handles[row_id] = handle
    return handles


def step_1(conn):
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL)")


def step_2(conn):
    conn.execute("ALTER TABLE users ADD COLUMN active INTEGER NOT NULL DEFAULT 1")


def step_3(conn):
    old = conn.execute("SELECT id, email, active FROM users").fetchall()
    handles = make_handles([(row_id, email) for row_id, email, _ in old])
    conn.execute("ALTER TABLE users RENAME TO users_old")
    conn.execute(
        "CREATE TABLE users ("
        "id INTEGER PRIMARY KEY, email TEXT NOT NULL, "
        "active INTEGER NOT NULL DEFAULT 1, handle TEXT NOT NULL)"
    )
    conn.executemany(
        "INSERT INTO users (id, email, active, handle) VALUES (?, ?, ?, ?)",
        [(row_id, email, active, handles[row_id]) for row_id, email, active in old],
    )
    conn.execute("DROP TABLE users_old")


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
