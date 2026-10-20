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
crates = Counter()
total = 0
for row in csv.DictReader(open(table)):
    name, size = row["Name"], int(row["ShallowSize"])
    total += size
    m = re.search(r"([a-z_][a-z0-9_]*)::", name)
    key = m.group(1) if m else ("data/other" if name.startswith(("data[", "elem[", "custom")) else "unnamed")
    crates[key] += size
lines = [f"{k:<28}{v / 1e6:7.2f} MB {100 * v / max(total, 1):5.1f}%" for k, v in crates.most_common(40)]
print("::notice title=wasm by crate (pre wasm-opt)::" + "%0A".join(lines))
