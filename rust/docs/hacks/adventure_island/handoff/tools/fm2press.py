"""fm2 -> zax-harness --press spec. Usage: fm2press.py movie.fm2 [offset] [first] [last]"""
import sys
names = ['right', 'left', 'down', 'up', 'start', 'select', 'b', 'a']
path = sys.argv[1]
offset = int(sys.argv[2]) if len(sys.argv) > 2 else 0
frames = []
for line in open(path):
    if line.startswith('|'):
        parts = line.split('|')
        frames.append(parts[2])
first = int(sys.argv[3]) if len(sys.argv) > 3 else 0
last = int(sys.argv[4]) if len(sys.argv) > 4 else len(frames) - 1
spans = []
for button, name in enumerate(names):
    start = None
    for f in range(first, last + 2):
        held = f <= last and f < len(frames) and frames[f][button] != '.'
        if held and start is None:
            start = f
        elif not held and start is not None:
            a, b = start + offset, f - 1 + offset
            spans.append(f'{a}-{b}:{name}' if a != b else f'{a}:{name}')
            start = None
print(','.join(spans))
