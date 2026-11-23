"""Load the settings of the app from the flags, the environment and a file."""

DEFAULTS = {"host": "localhost", "retries": 3, "timeout": 30}


def parse_flags(argv):
    """Read the flags `--key=value`. Other words are ignored."""
    flags = {}
    for word in argv:
        if word.startswith("--") and "=" in word:
            key, _, value = word[2:].partition("=")
            flags[key] = value
    return flags


def parse_file(text):
    """Read the lines `key = value`. Blank lines and lines of `#` are ignored."""
    values = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        values[key.strip()] = value.strip()
    return values


def first_value(*sources):
    """Return the first value that is not None and not an empty string."""
    for value in sources:
        if value is not None and value != "":
            return value
    return None


def load_config(argv, environ, file_text):
    """Return the settings as a dict."""
    flags = parse_flags(argv)
    values = parse_file(file_text)
    config = {}
    config["host"] = (
        flags.get("host") or environ.get("APP_HOST") or values.get("host") or DEFAULTS["host"]
    )
    # Old deployments set APP_RETRIES in the unit file of the service.
    config["retries"] = int(
        environ.get("APP_RETRIES")
        or flags.get("retries")
        or values.get("retries")
        or DEFAULTS["retries"]
    )
    seconds = first_value(
        flags.get("timeout"),
        environ.get("APP_TIMEOUT"),
        values.get("timeout"),
        DEFAULTS["timeout"],
    )
    config["timeout"] = int(seconds)
    return config
