# Writes next to this script (impl/branding/) by default, or into the folder given as the first
# argument on the command line. No machine path is written in this repository (lot PUB-1).
import os
import sys

OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__))

def solid(cells, P, e, tf, sf):
    out = []
    for (cx, cy) in sorted(cells, key=lambda t: (-t[1], t[0])):
        if (cx, cy - P) not in cells:
            out.append(f'<polygon points="{cx},{cy} {cx+P},{cy} {cx+P+e},{cy-e} {cx+e},{cy-e}" fill="{tf}"/>')
        if (cx + P, cy) not in cells:
            out.append(f'<polygon points="{cx+P},{cy} {cx+P},{cy+P} {cx+P+e},{cy+P-e} {cx+P+e},{cy-e}" fill="{sf}"/>')
    return out
def svg(name, size, cells_grid, P, e, ox, oy, radius):
    cells = {(ox + c * P, oy + r * P) for c, r in cells_grid}
    ys = [y for _, y in cells]; y0, y1 = min(ys), max(ys) + P
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" width="{size}" height="{size}" shape-rendering="crispEdges">',
         '<title>Zaxnes</title>',
         f'<defs><linearGradient id="og" gradientUnits="userSpaceOnUse" x1="0" y1="{y0}" x2="0" y2="{y1}">'
         '<stop offset="0" stop-color="#FFD166"/><stop offset="0.55" stop-color="#FF8A3D"/><stop offset="1" stop-color="#E8505B"/></linearGradient>'
         '<linearGradient id="bgt" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#241C46"/><stop offset="1" stop-color="#120E24"/></linearGradient></defs>',
         f'<rect x="0" y="0" width="{size}" height="{size}" rx="{radius}" fill="url(#bgt)" shape-rendering="geometricPrecision"/>']
    o += solid(cells, P, e, '#7C68D8', '#3A2E7A')
    o.append('<g fill="url(#og)">' + ''.join(f'<rect x="{a}" y="{b}" width="{P}" height="{P}"/>' for a, b in sorted(cells)) + '</g>')
    o.append('</svg>')
    # newline='\n': the repository is LF only (.gitattributes), text mode would write CRLF here.
    open(name, 'w', encoding='utf-8', newline='\n').write('\n'.join(o))
# detailed Z: same grid as the logo v7
big = {(c, r) for r in (0, 1) for c in range(2, 16)} | {(c, r) for r in (13, 14) for c in range(8, 16)}
for r in range(2, 12):
    right = 15 - (r - 2); t = 2 + (r - 2) // 6
    big |= {(c, r) for c in range(right - t + 1, right + 1)}
for i in range(8):
    big |= {(c, 16 - i) for c in range(0, 8 - i)}
# 16 x 17 cells of 11 px + 12 px extrusion in a 256 square
svg(os.path.join(OUT, 'zaxnes-icon.svg'), 256, big, 11, 12, 34, 54, 44)
# simplified Z for 16-32 px, drawn on a 16 grid (one cell = 1 px at 16 px)
small = {(c, r) for r in (2, 3) for c in range(3, 13)}
for r in range(4, 10):
    right = 11 - (r - 4); small |= {(right - 1, r), (right, r)}
for i in range(5):
    small |= {(c, 13 - i) for c in range(2, 2 + 5 - i)}
small |= {(c, r) for r in (12, 13) for c in range(8, 13)}
svg(os.path.join(OUT, 'zaxnes-icon-small.svg'), 16, small, 1, 1, 0, 0, 3)
print('ok')
