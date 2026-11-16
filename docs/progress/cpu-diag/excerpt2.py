"""excerpt2.py <netlist> <zaxnes z5> <from> <to> [notes.json]: side-by-side cycles (DMA detail)."""
import sys, json

def netlist(path):
    rows = [l.split() for l in open(path)]
    start = next(i for i, r in enumerate(rows) if r[1] == '8000' and r[2] == 'R')
    return [' '.join(r[1:4]) for r in rows[start:]]

def zaxnes(path):
    out = []
    for l in open(path):
        r = l.split()
        out.append((' '.join(r[1:4]), r[4] if len(r) > 4 else ''))
    return out

n, z = netlist(sys.argv[1]), zaxnes(sys.argv[2])
first, last = int(sys.argv[3]), int(sys.argv[4])
notes = json.loads(open(sys.argv[5]).read()) if len(sys.argv) > 5 else {}
print('| Cycle | 2A03 (netlist) | Zaxnes | Qui | | Note |')
print('|---|---|---|---|---|---|')
for i in range(first, last + 1):
    text, kind = z[i]
    who = {'D': 'DMA', 'I': 'DMA (idle)', '': 'CPU'}[kind]
    same = n[i] == text or (kind == 'I' and n[i][:6] == text[:6])
    print(f'| {i} | `{n[i]}` | `{text}` | {who} | {"=" if same else "≠"} | {notes.get(str(i), "")} |')
