"""Builds `crates/zax-nes/tests/data/cpu_bus_reference.txt` from BeesNES's table of the 2A03's
bus activity per half-cycle (`Research/Instructions/Ricoh 2A03 Instructions NES.xlsx`, L. Spiro,
MIT licence; see THIRD-PARTY-NOTICES.md). Lot FIDELITY plan step 4, lead F1 of
docs/review/FIDELITY-LEADS.md.

usage: python -I cpu_bus_reference.py <xlsx> <out.txt>

The table is pseudo-code; evaluating every address expression would be a second CPU. What is
kept is what can be checked exactly against a recording bus in two fixed scenarios
(`cpu_bus_reference_tests.rs` builds the same machine):

* `short`: X = Y = 0, so no indexed access crosses a page, and every branch not taken;
* `long`: X = Y = $FF on operands that make every indexed access cross a page, and every branch
  taken across a page.

For each, the bus accesses of one instruction, opcode fetch included and the next opcode fetch
excluded, as `R` or `W` followed by the class of the address: `PC` (the instruction stream),
`S` (the stack page), `V` (the vectors and the jam's `$FFFE`/`$FFFF`), `D` (anything else).
Cycles the table skips in a scenario ("skip the next cycle", "end (next half-cycle is ...)")
are left out. The jam opcodes (endless) and the cycle counts are written too.

Only the standard library is used: the xlsx is a zip of XML.
"""
import re
import sys
import zipfile
import xml.etree.ElementTree as ET

NS = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'


def rows(path):
    z = zipfile.ZipFile(path)
    shared = [''.join(t.text or '' for t in si.iter(NS + 't'))
              for si in ET.fromstring(z.read('xl/sharedStrings.xml')).iter(NS + 'si')]
    root = ET.fromstring(z.read('xl/worksheets/sheet1.xml'))
    for row in root.iter(NS + 'row'):
        cells = {}
        for c in row.iter(NS + 'c'):
            v = c.find(NS + 'v')
            if v is None:
                continue
            text = shared[int(v.text)] if c.get('t') == 's' else v.text
            letters = re.match(r'[A-Z]+', c.get('r')).group(0)
            cells[ord(letters[0]) - 65] = text
        yield [cells.get(i, '') for i in range(4)]


def parse(path):
    ops = []
    cur = None
    for a, b, c, _ in rows(path):
        m = re.match(r'^(\w+) \(([0-9A-F]{2})\)$', a)
        if m and b.startswith('Cycles'):
            cur = {'mn': m.group(1), 'op': int(m.group(2), 16), 'cycles': b.split(':')[1].strip(),
                   'lines': []}
            ops.append(cur)
            continue
        if cur is None:
            continue
        stamp = a.strip()
        if stamp.startswith('*'):
            continue  # A footnote.
        cur['lines'].append((stamp, b, c))
    return ops


def half(stamp):
    """(cycle, half) of a stamp such as '2.2000000000000002', ' -X.2', ' +X.1', or None."""
    if stamp.startswith('-X'):
        return (0, 2)
    if stamp.startswith('+X'):
        return (99, 1)
    m = re.match(r'^(\d+)\.(\d)', stamp)
    if not m:
        return None
    return (int(m.group(1)), int(m.group(2)))


def classify(expr):
    e = expr.replace('*** ', '').strip()
    rw = 'W' if e.startswith('Write') else 'R'
    if e in ('Read PC',):
        kind = 'PC'
    elif '$0100' in e or e == 'Read (S | $0100)':
        kind = 'S'
    elif 'Vector' in e or '$FFFF' in e or '$FFFE' in e:
        kind = 'V'
    else:
        kind = 'D'
    return rw + ':' + kind


def accesses(op, scenario):
    """The bus accesses of one instruction in `scenario`, or None for an endless jam."""
    if op['cycles'] == '∞':
        return None
    out = []
    skip_next = False
    end_at = None
    # Gather the notes of each half 1 (and the description of each half 2) by cycle.
    entries = []
    stamp_now = None
    for stamp, rw, desc in op['lines']:
        h = half(stamp) if stamp else None
        if h is not None:
            stamp_now = h
            entries.append([h, rw, desc])
        elif stamp_now is not None:
            # A continuation line: more description, or the access of a half whose first line
            # only computed (the SH* stores write on such a line).
            if rw and not entries[-1][1]:
                entries[-1][1] = rw
            if desc:
                entries[-1][2] += ' ' + desc
    for (cycle, part), rw, desc in entries:
        if end_at is not None and (cycle, part) < end_at:
            continue
        if part == 1:
            if scenario == 'short' and re.search(r'If !Jump, end \(next half-cycle is (\d)\.2\)', desc):
                end_at = (int(re.search(r'is (\d)\.2', desc).group(1)), 2)
            continue
        if not rw:
            continue
        if skip_next:
            skip_next = False
            continue
        if cycle > 0 and 'Store as OpCode' in desc and rw.strip() == 'Read PC':
            break
        out.append(classify(rw))
        if scenario == 'short' and 'If !BoundaryCrossed, skip the next cycle' in desc:
            skip_next = True
    return out


def main():
    xlsx, out = sys.argv[1], sys.argv[2]
    ops = parse(xlsx)
    assert len(ops) == 256 and sorted(o['op'] for o in ops) == list(range(256))
    with open(out, 'w', encoding='utf-8', newline='\n') as f:
        f.write('# Generated by rust/tools/cpu_bus_reference.py from BeesNES\'s\n')
        f.write('# "Ricoh 2A03 Instructions NES.xlsx" (MIT). opcode mnemonic cycles | short | long\n')
        for op in sorted(ops, key=lambda o: o['op']):
            short = accesses(op, 'short')
            long_ = accesses(op, 'long')
            if short is None:
                f.write(f"{op['op']:02X} {op['mn']} jam\n")
                continue
            f.write(f"{op['op']:02X} {op['mn']} {op['cycles']} | {' '.join(short)} | {' '.join(long_)}\n")


main()
