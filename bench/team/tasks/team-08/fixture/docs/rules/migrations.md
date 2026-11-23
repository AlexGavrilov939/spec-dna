# Rules of the migrations

## M-1. Steps

A step has a number. The numbers start at 1 and have no gap. `migrate(conn)` runs each step that is newer than the version of the database, in order, and it commits after each step.

## M-2. Version

The table `schema_version` has one row. Its column `version` holds the number of the last step that ran. A new database has the table with version 0. `migrate` never runs a step twice.

## M-3. Required columns

A required column is `NOT NULL` and has no default value. The database must refuse a new row that gives no value for it. A placeholder value, such as an empty string, is not a value.

## M-4. Rows that exist

A step that adds a required column gives each row that exists a real value, by the rule of the docs for that column. It keeps the ids and all other columns. The command `ALTER TABLE ... ADD COLUMN` cannot add a `NOT NULL` column that has no default.
