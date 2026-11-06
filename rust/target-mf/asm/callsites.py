"""MF-3b rule 1: call sites per callee, which an instruction-count diff cannot see.

A callee going from 0 call sites to N is a regression of N x (call + prologue + epilogue) and its
own instruction count does not move. Usage: callsites.py <a.s> <b.s> [extra Owner::method ...]
"""
import re, sys

WATCHED = [
    ('Cpu6502', 'end_cycle'),
    ('Ppu2C02', 'read_nametable_as'),
    ('Ppu2C02', 'read_nametable_inner'),
    ('Ppu2C02', 'read_pattern_planes'),
    ('Ppu2C02', 'run_dots'),
    ('Board', 'ppu_read'),
    ('Board', 'ppu_peek'),
    ('Board', 'on_ppu_address_bus'),
    ('Board', 'irq_asserted'),
    # The three links of the per-CPU-cycle /IRQ sample, not just the last one. MF-5b step 1
    # found `Board::irq_asserted` still at 0 call sites while **`NesBus::irq_asserted`** had
    # gone out of line and was costing 170 M instructions over 300 frames: the regression had
    # moved one level up and the watched list could not see it.
    ('NesBus', 'irq_asserted'),
    ('Cartridge', 'irq_asserted'),
    ('Board', 'on_cpu_cycle'),
    ('Board', 'cpu_read_override'),
    ('Cartridge', 'try_cpu_read'),
    ('Ppu2C02', 'read'),
    ('Apu2A03', 'run_cpu_cycles'),
    ('NesBus', 'push_expansion_audio'),
    ('Cartridge', 'expansion_audio_output'),
    ('Board', 'expansion_audio_output'),
    ('Cartridge', 'step_cpu_cycle'),
    ('Cartridge', 'nametable_page'),
    ('Cartridge', 'read_nametable'),
]
WATCHED += [tuple(a.split('::')) for a in sys.argv[3:]]


def counts(path, pats):
    defs = {k: 0 for k in pats}
    sites = {k: 0 for k in pats}
    for line in open(path, encoding='utf-8', errors='replace'):
        s = line.strip()
        if re.match(r'^[^\s.][^\s]*:$', s):
            for k, p in pats.items():
                if p.search(s):
                    defs[k] += 1
        elif s.startswith('call') or s.startswith('jmp'):
            for k, p in pats.items():
                if p.search(s):
                    sites[k] += 1
    return defs, sites


pats = {}
for owner, fn in WATCHED:
    pats[f'{owner}::{fn}'] = re.compile(
        re.escape(owner) + r'.*\d' + re.escape(fn) + r'(\d|$|[A-Z])')
pats['core::panicking::panic_bounds_check'] = re.compile(r'panic_bounds_check')
pats['core::panicking::panic'] = re.compile(r'17h[0-9a-f]+E$|panicking.*panic17')

da, sa = counts(sys.argv[1], pats)
db, sb = counts(sys.argv[2], pats)
print(f"{'callee':46}{'defs a':>7}{'b':>4}{'sites a':>9}{'b':>5}   verdict")
for k in pats:
    flag = ''
    if sa[k] == 0 and sb[k] > 0:
        flag = 'FIRST CALL SITE -- stop the lot'
    elif sa[k] != sb[k] or da[k] != db[k]:
        flag = 'moved'
    print(f"{k:46}{da[k]:>7}{db[k]:>4}{sa[k]:>9}{sb[k]:>5}   {flag}")
