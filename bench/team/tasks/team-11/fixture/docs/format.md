# The CSV format

The report is saved as text in the form of RFC 4180.

## F-1. Rows and fields

A report is a list of rows. A row is a list of strings, and it has at least 2 fields. The first row of a monthly report is the header. The code treats it as any other row.

## F-2. Text of a row

Fields are in the order of the row, with one comma between 2 fields. Each row ends with `\r\n`, also the last row. An empty list of rows is the empty text.

## F-3. Quotes

A field is in double quotes only when it has a comma, a double quote, a CR (`\r`) or an LF (`\n`). Inside the quotes, each double quote is written twice. No other field has quotes. A space is part of its field, so the code never trims it.

## F-4. Example

The rows `[["id", "note"], ["1", "said \"hi\", left"]]` have this text:

```text
id,note\r\n
1,"said ""hi"", left"\r\n
```

Here `\r\n` shows the 2 characters of a line end, not 4 characters of text.

## F-5. Import

`import_csv` reads the text that `export_csv` writes, and it gives back the same rows.
