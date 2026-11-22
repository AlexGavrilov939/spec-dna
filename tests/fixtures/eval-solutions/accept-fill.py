"""Fills the skeleton of an acceptance record from a template (the proofs of
the evals `accept-1` to `accept-5`, design 0.8.0 D62).

usage: python3 accept-fill.py <template> <record>

`spec-dna accept <slug>` wrote the skeleton in <record>. The template is the
record that an acceptor writes. It has these placeholders, which the script
takes from the skeleton and from `spec-dna accept <slug> --brief`: @SCOPE@,
@PATHS@, @TREE@ (the lines of the skeleton), @BRIEF@ (the hash of the last
line of the brief), @PROOF@ (the `[[proof]]` entries of the skeleton), and
@ROOT@ (the absolute path of the copy).
"""

import os
import re
import subprocess
import sys

template, record = sys.argv[1:3]
skeleton = open(record).read()
slug = re.search(r'^change = "(.*)"', skeleton, re.M).group(1)


def line(key):
    return re.search(r"^%s = .*$" % key, skeleton, re.M).group(0)


out = subprocess.run(
    ["spec-dna", "accept", slug, "--brief"], capture_output=True, text=True, check=True
).stdout
brief = out.strip().splitlines()[-1].split()[-1]
text = open(template).read()
values = {
    "@SCOPE@": line("scope"),
    "@PATHS@": line("paths"),
    "@TREE@": line("tree"),
    "@BRIEF@": brief,
    "@PROOF@": skeleton[skeleton.index("[[proof]]") :].rstrip("\n") + "\n",
    "@ROOT@": os.getcwd(),
}
for key, value in values.items():
    text = text.replace(key, value)
open(record, "w").write(text)
