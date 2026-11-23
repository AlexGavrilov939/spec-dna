# slugs

A small library that makes the URL slugs of people and places. A name reaches the library from 2 sources: the web form, and the import of files from other systems (see `data/import_names.py`).

Run the tests with `python3 -m unittest discover -s tests -t .`.

## Rules of a slug

- A slug is in lower case.
- A slug keeps the letters of any script, and it keeps their accents. A name with an accent and the same name without it are 2 different names, so they have 2 different slugs.
- Each run of other characters becomes one `-`. A slug does not start or end with `-`.
- A slug is unique in a store. The store makes it unique with a numeric suffix.
