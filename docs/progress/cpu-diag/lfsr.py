import sys, collections
S = sys.argv[1]
rows = [l.split() for l in open(S + '/n-dmc.txt')]
t = [int(r[7][2:], 16) for r in rows]
b = [int(r[8][2:]) for r in rows]
# The LFSR state after a reload: the value that most often follows a run of changes.
changes = [i for i in range(1, len(t)) if t[i] != t[i - 1]]
following = collections.Counter(t[i] for i in changes)
print('most frequent timer states:', following.most_common(3))
reload_state = following.most_common(1)[0][0]
reloads = [i for i in changes if t[i] == reload_state]
print('reloads at', reloads[:12])
print('gaps', [y - x for x, y in zip(reloads, reloads[1:])][:12])
bch = [i for i in range(1, len(b)) if b[i] != b[i - 1]]
print('bit counter changes at', [(i, b[i]) for i in bch[:20]])
