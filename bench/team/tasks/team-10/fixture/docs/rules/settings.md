# Rules of the settings

## C-1. Sources and names

A setting has 3 sources and a default. For the setting `<key>`:

- the flag is `--<key>=<value>` in `argv`
- the environment variable is `APP_<KEY>` in `environ`
- the file has a line `<key> = <value>` in `file_text`

## C-2. Order

The value of a setting comes from the first source that has it. The order is: the flag, then the environment variable, then the file, then the default. A flag of the command line is the most direct word of the user, so it wins over the environment, and the environment wins over the file.

## C-3. Empty values

A source whose value is an empty string has no value. The next source in the order decides.

## C-4. Settings

| Key | Type | Default |
| --- | --- | --- |
| `host` | text | `localhost` |
| `retries` | integer | 3 |
