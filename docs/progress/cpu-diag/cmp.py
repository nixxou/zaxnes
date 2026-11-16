"""Compare the netlist's CPU bus trace with Zaxnes's, cycle by cycle, from the first $8000 fetch.
usage: cmp.py <netlist.txt> <zaxnes.txt> [max diffs]"""
import sys

def netlist(path):
    rows = [l.split() for l in open(path)]
    start = next(i for i, r in enumerate(rows) if r[1] == '8000' and r[2] == 'R')
    return [(r[1], r[2], r[3]) for r in rows[start:]]

def zaxnes(path):
    out = []
    for l in open(path):
        r = l.split()
        if r[1] == 'DMA':
            out.extend([('DMA', '?', '??')] * int(r[2]))
        else:
            out.append((r[1], r[2], r[3]))
    return out

n, z = netlist(sys.argv[1]), zaxnes(sys.argv[2])
limit = int(sys.argv[3]) if len(sys.argv) > 3 else 10
count = min(len(n), len(z))
diffs = [(i, n[i], z[i]) for i in range(count) if n[i] != z[i] and z[i][0] != 'DMA']
dma = [(i, n[i]) for i in range(count) if z[i][0] == 'DMA']
print(f'{count} cycles compared, {len(diffs)} differ, {len(dma)} DMA cycles on the Zaxnes side')
for i, a, b in diffs[:limit]:
    print(f'  cycle {i}: netlist {" ".join(a)}  zaxnes {" ".join(b)}')
if dma:
    print('  netlist during Zaxnes DMA:', ' '.join(f'{a[0]}{a[1]}' for _, a in dma[:12]))
