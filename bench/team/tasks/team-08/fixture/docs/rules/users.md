# Rules of the users

## Schema of version 2

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE schema_version (version INTEGER NOT NULL);
```

## U-1. Email

An email always has an `@`.

## U-2. Handle

The handle of a user is the part of the email before the first `@`, in lower case. If several users get the same handle, the user with the lowest id keeps it. Each other user gets the handle with `-<id>` added at the end, where `<id>` is the id of that user.
