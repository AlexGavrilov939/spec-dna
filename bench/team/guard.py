"""Run a command and stop it after a number of seconds.

Usage: guard.py <seconds> <command> [<argument>...]

The command runs in its own process group. On a time-out, the whole group is
killed and the exit code is 124. Otherwise the exit code is the one of the
command.
"""

import os
import signal
import subprocess
import sys


def main() -> int:
    limit = float(sys.argv[1])
    child = subprocess.Popen(sys.argv[2:], start_new_session=True)
    try:
        return child.wait(timeout=limit)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGKILL)
        child.wait()
        print(f"guard: stopped after {limit:g} seconds", file=sys.stderr)
        return 124


if __name__ == "__main__":
    sys.exit(main())
