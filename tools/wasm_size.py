"""Summarise a wasm build's size for CI: total sizes (raw, gzip, brotli) and
which crates take the space, from `twiggy top --format csv`.

Usage: python3 tools/wasm_size.py <shipped.wasm> <twiggy.csv>
Prints GitHub Actions ::notice lines (readable through the API as annotations).
"""
import csv, gzip, re, subprocess, sys
from collections import Counter

shipped, table = sys.argv[1], sys.argv[2]
raw = open(shipped, "rb").read()
gz = len(gzip.compress(raw, 9))
try:
    br = len(subprocess.run(["brotli", "-c", "-q", "11", shipped], capture_output=True, check=True).stdout)
except Exception:
    br = 0
mb = lambda n: f"{n / 1e6:.2f} MB"
print(f"::notice title=wasm size::raw {mb(len(raw))}, gzip {mb(gz)}, brotli {mb(br)}")

# Group functions and data by the crate their (demangled) name starts with.
# Symbol names look like `bevy_ecs::world::World::spawn` or, with v0 mangling,
# `bevy_ecs[1a2b3c]::world::...`; trait impls start with `<`. The crate is the
# first path segment of the first path in the name.
crates = Counter()
unplaced = Counter()
total = 0
for row in csv.DictReader(open(table)):
    name, size = row["Name"], int(row["ShallowSize"])
    total += size
    m = re.search(r"(?:^|[<&(\s])([A-Za-z_][A-Za-z0-9_]*)(?:\[[0-9a-f]+\])?::", name)
    if m:
        key = m.group(1)
    elif name.startswith(("data[", "elem[", "custom", "table[")):
        key = "(data)"
    else:
        key = "(unplaced)"
        unplaced[name[:90]] += size
    crates[key] += size
lines = [f"{k:<28}{v / 1e6:7.2f} MB {100 * v / max(total, 1):5.1f}%" for k, v in crates.most_common(40)]
print("::notice title=wasm by crate (pre wasm-opt)::" + "%0A".join(lines))
samples = [f"{v / 1e3:8.1f} KB  {k}" for k, v in unplaced.most_common(25)]
print("::notice title=largest unplaced names::" + "%0A".join(samples))
