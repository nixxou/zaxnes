# Writes next to this script (impl/branding/) by default, or into the folder given as the first
# argument on the command line, and it reproduces branding/zaxnes-logo.svg byte for byte. No
# machine path is written in this repository (lot PUB-1).
import os
import sys

OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__))

P = 10
G = {
 'A': ["01110","10001","10001","11111","10001","10001","10001"],
 'X': ["10001","10001","01010","00100","01010","10001","10001"],
 'N': ["10001","11001","11001","10101","10011","10011","10001"],
 'E': ["11111","10000","10000","11110","10000","10000","11111"],
 'S': ["01111","10000","10000","01110","00001","00001","11110"],
}
bars = {(c, r) for r in (0, 1) for c in range(2, 16)} | {(c, r) for r in (13, 14) for c in range(8, 16)}
arrow = set()
for r in range(2, 12):                      # shaft: 45 degrees, thickness 2 -> 4 as it comes forward
    right = 15 - (r - 2)
    t = 2 + (r - 2) // 6
    arrow |= {(c, r) for c in range(right - t + 1, right + 1)}
for i in range(8):                          # head: right triangle, corner at the bottom-left (points front-left)
    arrow |= {(c, 16 - i) for c in range(0, 8 - i)}
bars -= arrow
X0, Y0 = 18, 30
base = Y0 + 15 * P                          # bottom of the bottom bar
px = lambda s: {(X0 + c * P, Y0 + r * P) for c, r in s}
def letter(ch, x):
    g = G[ch]; top = base - len(g) * P
    return {(x + c * P, top + r * P) for r, row in enumerate(g) for c, v in enumerate(row) if v == '1'}
x = X0 + 16 * P + 30
ax, nes = set(), set()
for ch in 'AX': ax |= letter(ch, x); x += 6 * P
x += 0
for ch in 'NES': nes |= letter(ch, x); x += 6 * P
W, H = x + 16, Y0 + 17 * P + 14
def solid(cells, e, tf, sf):
    out = []
    for (cx, cy) in sorted(cells, key=lambda t: (-t[1], t[0])):
        if (cx, cy - P) not in cells:
            out.append(f'<polygon points="{cx},{cy} {cx+P},{cy} {cx+P+e},{cy-e} {cx+e},{cy-e}" fill="{tf}"/>')
        if (cx + P, cy) not in cells:
            out.append(f'<polygon points="{cx+P},{cy} {cx+P},{cy+P} {cx+P+e},{cy+P-e} {cx+P+e},{cy-e}" fill="{sf}"/>')
    return out
def fronts(cells, attr):
    return [f'<g {attr}>' + ''.join(f'<rect x="{a}" y="{b}" width="{P}" height="{P}"/>' for a, b in sorted(cells)) + '</g>']
o = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" role="img" aria-label="Zaxnes" shape-rendering="crispEdges">',
     '<title>Zaxnes</title>',
     '<style>.ax{fill:#FFF4DC}@media (prefers-color-scheme: light){.ax{fill:#3A2E6E}}</style>',
     f'<defs><linearGradient id="og" gradientUnits="userSpaceOnUse" x1="0" y1="{Y0}" x2="0" y2="{base}">'
     '<stop offset="0" stop-color="#FFD166"/><stop offset="0.55" stop-color="#FF8A3D"/><stop offset="1" stop-color="#E8505B"/></linearGradient>'
     f'<linearGradient id="bg" gradientUnits="userSpaceOnUse" x1="{X0+16*P}" y1="{Y0}" x2="{X0}" y2="{Y0+17*P}">'
     '<stop offset="0" stop-color="#2B5FD9"/><stop offset="1" stop-color="#7CC4FF"/></linearGradient></defs>']
B, A = px(bars), px(arrow)
o += solid(B, 11, '#6A58C0', '#2A2350') + fronts(B, 'fill="url(#og)"')
o += solid(A, 11, '#6A58C0', '#2A2350') + fronts(A, 'fill="url(#og)"')
o += solid(ax | nes, 5, '#5B4BA8', '#2A2350') + fronts(ax | nes, 'fill="url(#og)"')
o.append('</svg>')
# newline='\n': the repository is LF only (.gitattributes), text mode would write CRLF here.
open(os.path.join(OUT, 'zaxnes-logo.svg'), 'w', encoding='utf-8', newline='\n').write('\n'.join(o))
print(W, H)
