# Library loans service

A small HTTP JSON service for book loans: a catalog of books, members, and the
loans between them. It uses the Python standard library only (Python 3.12 or
later) and keeps its data in memory.

```sh
python3 -m unittest discover -s tests -t .   # run the tests
python3 -m library.api --port 8080           # start the server
curl -s localhost:8080/books
```
