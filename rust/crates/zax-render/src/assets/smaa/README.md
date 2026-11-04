# The two precomputed SMAA tables

`area.bin` and `search.bin` are the byte arrays of `iryoku/smaa`'s `Textures/AreaTex.h` and
`Textures/SearchTex.h`, taken as they stand. MIT licence; the notice is in
`THIRD-PARTY-NOTICES.md`, and the upstream commit is recorded in `third_party/smaa/README.txt`
(`71c806a838bdd7d517df19192a20f0c61b3ca29d`).

| File | Shape | Format | Bytes | SHA-256 |
|---|---|---|---|---|
| `area.bin` | 160 × 560 | `R8G8` (`AREATEX_PITCH` = 320) | 179 200 | `35065cef2a02cabcad711d6bf430239ae64e27d71c4e4fa06f29cce2c992f0d2` |
| `search.bin` | 64 × 16 | `R8` | 1 024 | `3694eae5e9d44b8ebb4415a13f8c7b94dc08a2fc86658434d771c4610fe5744d` |

## Why they are committed as binary rather than generated at build time

AMD-060 §4 leaves the choice open ("générer à la compilation depuis les en-têtes, ou les committer
comme des données binaires — votre choix, documenté"). `third_party/` is **not** tracked by this
repository, so a build script reading `AreaTex.h` would fail on a machine that has only the
repository. The bytes are therefore here, and
`zax-render/tests/amd060_smaa_tests.rs::req_scn_085_the_two_tables_are_the_reference_s_own_bytes`
pins each one's length and SHA-256 — so "reprises à l'octet" is a checked property and not a claim.

## How they were extracted

Both headers hold one `static const unsigned char …[]` of `0xNN` literals. Row order is the
headers' own: `glTexImage2D`'s first row is at `t = 0` and a `wgpu` texture's first row is at the
top, which is where Direct3D's first row is too, so neither backend flips them. The vertical flip
`SMAASearchLength` applies is part of the **table's** encoding, not of a host convention, and is
kept in the shader where the reference has it.

```python
import re, io
def extract(path, name):
    s = io.open(path, encoding='utf-8', errors='replace').read()
    i = s.index(name); j = s.index('{', i); k = s.index('};', j)
    return bytes(int(v, 16) for v in re.findall(r'0x([0-9a-fA-F]{2})', s[j + 1:k]))

open('area.bin', 'wb').write(extract('<third_party>/smaa/Textures/AreaTex.h', 'areaTexBytes'))
open('search.bin', 'wb').write(extract('<third_party>/smaa/Textures/SearchTex.h', 'searchTexBytes'))
```
