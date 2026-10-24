# Zaxnes — avis de licences tierces

Zaxnes est distribué sous **GNU General Public License v3** (fichier `LICENSE`).

Zaxnes est une **réimplémentation indépendante**, sans lien avec Geod Studio ni avec 3dSen.

## Code tiers repris dans Zaxnes

### 3DNes_Engine — (c) Geod Studio — GNU GPL v3

Une partie du moteur de conversion 2D vers 3D (`rust/crates/zax-engine3d`) est **adaptée de
3DNes_Engine**, une version antérieure et publique du moteur de conversion publiée par son
auteur sous GNU GPL v3. Les fichiers concernés portent l'en-tête :

    // Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

3DNes_Engine est distribué sous GNU GPL v3 ; c'est cette licence qui impose à Zaxnes d'être
lui-même sous GPL v3. Le texte complet de la licence est dans `LICENSE`.

### z2rs — (c) 2026 troyedwardsjr — licence MIT

Le décodeur des marges du mode écran large de *Zelda II* (`rust/crates/zax-nes/src/wide_decode/zelda2.rs`)
est **adapté de z2rs** (<https://github.com/troyedwardsjr/z2rs>, fichiers
`crates/z2-core/src/wide_margins.rs` et `overworld_map.rs`), publié sous la licence MIT
suivante :

    MIT License

    Copyright (c) 2026 troyedwardsjr

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

### z2disassembly — FiendsOfTheElements — CC0 1.0

Le correctif écran large de *Zelda II* que Zaxnes applique en mémoire
(`rust/crates/zax-nes/patches/zelda2-usa-widescreen.bps`) est construit à partir du désassemblage
*z2disassembly* (<https://github.com/FiendsOfTheElements/z2disassembly>), versé au domaine public
par ses auteurs (CC0 1.0). Le correctif ne contient pas la ROM : il s'applique à la copie du
joueur.

**Dear ImGui** embarque sa police intégrée (Proggy Clean, domaine public), que Zaxnes utilise en
repli quand aucune police système n'est trouvée (AMD-024 REQ-PUB-006).

Le format de profil `.3dn` est lu et écrit par un codec écrit pour ce projet
(`rust/crates/zax-profiles/src/wire/`) ; aucune bibliothèque protobuf n'est livrée.

## Assets

**Aucun asset n'est livré sous forme de fichier.** Le seul qui l'était — le boîtier TV
décoratif — a été retiré du produit par AMD-031. Tout est **généré par le code du projet**
(AMD-024 REQ-PUB-002) : décors, matières et sons d'interface ; la police de l'interface est lue
sur la machine de l'utilisateur au lancement, sans redistribution. Il n'y a **pas de musique de
menu** (REQ-PUB-005, révisée le 2026-10-21). Les profils de jeu et les ROMs ne sont pas fournis.

L'inventaire complet, fichier par fichier, est dans
[`ASSETS-LICENSES.md`](ASSETS-LICENSES.md).

## Paquets livrés avec cette version

Le binaire `zaxnes` des versions de bureau est lié aux paquets suivants (graphe de dépendances
normales de `zax-app` pour Windows x64 et Linux x86_64, fonctionnalités `bundled` et `wgpu` : SDL2
est alors lié statiquement, comme dans la version Windows, et le rendu Vulkan / DirectX 12 est
inclus ; les versions Linux et PortMaster utilisent la bibliothèque SDL2 du système, sous licence
zlib, et la version PortMaster n'inclut pas `wgpu`, donc une partie seulement de cette liste).
Liste produite par `cargo tree -p zax-app --features bundled,wgpu -e normal --format "{p}|{l}"`
pour les deux cibles ; les paquets de l’espace de travail (code de Zaxnes) ne sont pas listés. Les sources de Dear ImGui, de Lua 5.4
et de SDL2 voyagent dans leurs paquets `imgui-sys`, `mlua-sys` et `sdl2-sys`.

Aucun paquet LGPL n’est livré.

Les vignettes des sauvegardes d'état sont encodées en JPEG par le paquet `jpeg-encoder`, dont
une partie relève de la licence de l'Independent JPEG Group, qui demande cette mention : *this
software is based in part on the work of the Independent JPEG Group* (ce logiciel est fondé en
partie sur les travaux de l'Independent JPEG Group).

| Paquet | Version | Licence |
|---|---|---|
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 |
| bit-set | 0.9.1 | Apache-2.0 OR MIT |
| bit-vec | 0.9.1 | Apache-2.0 OR MIT |
| bitflags | 1.3.2 | MIT/Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| block-buffer | 0.12.1 | MIT OR Apache-2.0 |
| bstr | 1.13.1 | MIT OR Apache-2.0 |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| chlorine | 1.0.13 | Zlib OR Apache-2.0 OR MIT |
| codespan-reporting | 0.13.1 | Apache-2.0 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| cpufeatures | 0.3.1 | MIT OR Apache-2.0 |
| crc32fast | 1.5.3 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 |
| deranged | 0.5.9 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| digest | 0.11.3 | MIT OR Apache-2.0 |
| document-features | 0.2.12 | MIT OR Apache-2.0 |
| either | 1.19.0 | MIT OR Apache-2.0 |
| equivalent | 1.0.3 | Apache-2.0 OR MIT |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| foldhash | 0.1.5 | Zlib |
| foldhash | 0.2.0 | Zlib |
| generic-array | 0.14.7 | MIT |
| getrandom | 0.3.4 | MIT OR Apache-2.0 |
| glow | 0.16.0 | MIT OR Apache-2.0 OR Zlib |
| gpu-allocator | 0.28.0 | MIT OR Apache-2.0 |
| gpu-descriptor | 0.3.2 | MIT OR Apache-2.0 |
| gpu-descriptor-types | 0.2.0 | MIT OR Apache-2.0 |
| half | 2.7.1 | MIT OR Apache-2.0 |
| hashbrown | 0.15.5 | MIT OR Apache-2.0 |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| hexf-parse | 0.2.1 | CC0-1.0 |
| hybrid-array | 0.4.15 | MIT OR Apache-2.0 |
| imgui | 0.12.0 | MIT OR Apache-2.0 |
| imgui-sys | 0.12.0 | MIT OR Apache-2.0 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| jpeg-encoder | 0.7.1 | (MIT OR Apache-2.0) AND IJG |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 |
| libc | 0.2.191 | MIT OR Apache-2.0 |
| libloading | 0.8.9 | ISC |
| libm | 0.2.16 | MIT |
| litrs | 1.0.1 | MIT OR Apache-2.0 |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |
| log | 0.4.34 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| mint | 0.5.9 | MIT |
| mlua | 0.11.6 | MIT |
| mlua-sys | 0.10.0 | MIT |
| naga | 29.0.4 | MIT OR Apache-2.0 |
| num-conv | 0.2.2 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| num_threads | 0.1.8 | MIT OR Apache-2.0 |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| ordered-float | 5.5.0 | MIT |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 |
| png | 0.17.16 | MIT OR Apache-2.0 |
| pollster | 0.4.0 | Apache-2.0/MIT |
| powerfmt | 0.2.1 | MIT OR Apache-2.0 |
| presser | 0.3.1 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| profiling | 1.0.18 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| range-alloc | 0.1.5 | MIT OR Apache-2.0 |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 |
| rfd | 0.17.2 | MIT |
| rustc-hash | 1.1.0 | Apache-2.0/MIT |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| sdl2 | 0.38.0 | MIT |
| sdl2-sys | 0.38.0 | MIT AND Zlib |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| sha1 | 0.11.0 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| smallvec | 1.16.3 | MIT OR Apache-2.0 |
| spirv | 0.4.0+sdk-1.4.341.0 | Apache-2.0 |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |
| syn | 2.0.119 | MIT OR Apache-2.0 |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| sys-locale | 0.3.2 | MIT OR Apache-2.0 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 |
| time | 0.3.55 | MIT OR Apache-2.0 |
| time-core | 0.1.9 | MIT OR Apache-2.0 |
| time-macros | 0.2.32 | MIT OR Apache-2.0 |
| typed-path | 0.12.4 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |
| wgpu | 29.0.4 | MIT OR Apache-2.0 |
| wgpu-core | 29.0.4 | MIT OR Apache-2.0 |
| wgpu-core-deps-windows-linux-android | 29.0.4 | MIT OR Apache-2.0 |
| wgpu-hal | 29.0.4 | MIT OR Apache-2.0 |
| wgpu-naga-bridge | 29.0.4 | MIT OR Apache-2.0 |
| wgpu-types | 29.0.4 | MIT OR Apache-2.0 |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zip | 7.4.0 | MIT |
| zlib-rs | 0.6.8 | Zlib |
| zmij | 1.0.23 | MIT |
| zopfli | 0.8.4 | Apache-2.0 |
