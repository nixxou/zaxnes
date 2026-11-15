"""cmp3.py <netlist> <zaxnes-with-dma-detail>: every cycle compared, DMA cycles included.
Counts: CPU cycles and DMA cycles; timing (address or R/W) and byte differences for each; an
idle DMA cycle (I, byte unknown) is compared on address and R/W only."""
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
            sys.exit('trace without DMA detail: build zax-harness with --features bus-trace')
        out.append((r[1], r[2], r[3], r[4] if len(r) > 4 else 'C'))
    return out

n, z = netlist(sys.argv[1]), zaxnes(sys.argv[2])
count = min(len(n), len(z))
kinds = collections.Counter()
timing = collections.Counter()
data = collections.Counter()
examples = []
for i in range(count):
    a, b = n[i], z[i]
    kind = b[3]
    kinds[kind] += 1
    if a[:2] != b[:2]:
        timing[kind] += 1
        if len(examples) < 6:
            examples.append(f'  cycle {i}: netlist {" ".join(a)}  zaxnes {" ".join(b)}')
    elif kind != 'I' and a[2] != b[2]:
        data[(kind, a[0], a[1])] += 1
dma = kinds['D'] + kinds['I']
print(f'{count} cycles ({kinds["C"]} CPU, {dma} DMA of which {kinds["I"]} idle), '
      f'timing differences: CPU {timing["C"]}, DMA {timing["D"] + timing["I"]}; '
      f'byte differences: {sum(data.values())}')
for line in examples:
    print(line)
for key, number in data.most_common(6):
    print(f'  byte {key[0]} {key[1]} {key[2]}: {number}')
