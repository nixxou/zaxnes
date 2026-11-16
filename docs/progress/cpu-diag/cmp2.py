"""cmp2.py <netlist> <zaxnes>: cycles whose address or R/W differ (timing), then cycles whose data
alone differ, grouped by address (values)."""
import sys, collections

def netlist(path):
    rows = [l.split() for l in open(path)]
    start = next(i for i, r in enumerate(rows) if r[1] == '8000' and r[2] == 'R')
    return [(r[1], r[2], r[3]) for r in rows[start:]]

def zaxnes(path):
    out = []
    for l in open(path):
        r = l.split()
        if r[1] == 'DMA':
            out.extend([None] * int(r[2]))
        else:
            out.append((r[1], r[2], r[3]))
    return out

n, z = netlist(sys.argv[1]), zaxnes(sys.argv[2])
count = min(len(n), len(z))
timing = [(i, n[i], z[i]) for i in range(count) if z[i] and n[i][:2] != z[i][:2]]
data = collections.Counter()
example = {}
for i in range(count):
    if z[i] and n[i][:2] == z[i][:2] and n[i][2] != z[i][2]:
        key = (n[i][0], n[i][1])
        data[key] += 1
        example.setdefault(key, (i, n[i][2], z[i][2]))
print(f'{count} cycles, {len(timing)} with another address or R/W, {sum(data.values())} with only another byte')
for i, a, b in timing[:5]:
    print(f'  timing cycle {i}: netlist {" ".join(a)}  zaxnes {" ".join(b)}')
for key, number in data.most_common():
    i, a, b = example[key]
    print(f'  byte {key[0]} {key[1]}: {number} cycles, first at {i}: netlist {a}, zaxnes {b}')
