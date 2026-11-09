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

### Mesen 2 — (c) Sour — GNU GPL v3

Plusieurs expressions de cartes du cœur NES (`rust/crates/zax-nes/src/cartridges/mappers`) et de
leur audio d'extension (`rust/crates/zax-nes/src/apu`) sont **adaptées de Mesen 2**
(<https://github.com/SourMesen/Mesen2>, dépôt local `third_party/mesen2`, commit `b9fa69d`),
publié sous GNU GPL v3 :

- le choix du jeu de registres CHR du MMC5 (`mmc5.rs`, `chr_address`), repris de l'expression
  `chrA` de `Core/NES/Mappers/Nintendo/MMC5.h` ;
- l'heuristique de décodage des lignes d'adresse des VRC2/VRC4 quand le sous-mappeur NES 2.0 est
  absent (`vrc2_vrc4.rs`, `Vrc2Vrc4::new`), reprise de `TranslateAddress` de
  `Core/NES/Mappers/Konami/VRC2_4.h` (« ORing both values should make most games work ») ;
- la condition du compteur d'IRQ de **révision A** du MMC3 (`mmc3.rs`, `clock_irq_counter`),
  reprise de `(count > 0 || _irqReload)` dans `NotifyVramAddressChange` de
  `Core/NES/Mappers/Nintendo/MMC3.h`, qui est le comportement qu'AMD-061 §8 exige du
  sous-mappeur 4 (« un `$C000` chargé à `$00` ne produit qu'une seule IRQ ») ;
- la lecture des quatre dispositions de la commande `$C` du FME-7 (`fme7.rs`, `arrangement`),
  reprise de `SunsoftFme7.h:103-110`, qui lève l'ambiguïté du vocabulaire « Horizontal (A10) »
  de la page NESdev (lot MF-3) ;
- la disposition des bancs CHR et des nametables du VRC6 d'après `$B003` (`vrc6.rs`,
  `derive_nametables` et `update_chr_banks`), reprise d'`UpdatePpuBanking` de
  `Core/NES/Mappers/Konami/VRC6.h` : c'est la seule traduction des tables de la page NESdev
  dans le vocabulaire de mirroring de ce dépôt qui ait pu être vérifiée ligne par ligne, et
  les quatre lignes concernées sont re-dérivées des colonnes paire/impaire de la page dans le
  commentaire de la fonction (lot MF-5a) ;
- la forme du compteur d'IRQ commun au VRC4 et au VRC6 (`vrc_irq.rs`), qui est celle de
  `Core/NES/Mappers/Konami/VrcIrq.h` — Mesen le factorise au même endroit et pour la même
  raison (lot MF-5a) ;
- la phase du canal en dents de scie du VRC6 (`apu/vrc6_audio.rs`, `Vrc6Saw::step_cpu_cycle`),
  reprise de `Core/NES/Mappers/Audio/Vrc6Saw.h` : la page NESdev dit « the accumulator only
  reacts on every 2 clocks » sans dire lesquels des quatorze pas de la phase additionnent, et
  Mesen est la source qui tranche (lot MF-5a) ;
- le découpage du décodage de registres du Namco 163 entre ses trois variants (`namco163.rs`,
  `write_register`), repris du `switch (addr & 0xF800)` de
  `Core/NES/Mappers/Namco/Namco163.h` : c'est la seule lecture du tableau de registres des pages
  NESdev « INES Mapper 019 » et « INES Mapper 210 » qui ait pu être vérifiée cas par cas contre
  une seconde source, en particulier le partage de `$C000-$D800` et des deux bits hauts de
  `$E000` entre le 163, le 175 et le 340 (lot MF-5a) ;
- l'enroulement de la phase du Namco 163, `(phase + freq) % (length << 16)`
  (`apu/namco163_audio.rs`, `Namco163Audio::update_channel`), repris de
  `Core/NES/Mappers/Audio/Namco163Audio.h` : c'est le seul point que la page « Namco 163 audio »
  énonce comme une formule plutôt qu'en prose, et Mesen est la seconde source qui la confirme
  (lot MF-5a) ;
- le décodage de registres du VRC7 (`vrc7.rs`, `write_register`), repris du `switch (addr &
  0xF038)` de `Core/NES/Mappers/Konami/VRC7.h` : en particulier le fait que les deux ports de son
  restent en `$9010` et `$9030` quelle que soit la ligne d'adresse qui sélectionne les registres
  — Mesen exempte exactement ces deux adresses de son repli A4 vers A3 — et l'ordre des quatre
  registres de `$E000` à `$F010` (lot MF-5b).

Les fichiers concernés portent l'en-tête :

    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0

Mesen 2 est distribué sous GNU GPL v3, la même licence que Zaxnes. Le texte complet est dans
`LICENSE`. Le reste de ces cartes, et toutes les autres, suit la documentation publique
(wiki NESdev) que REQ-CORE-100 désigne comme référence ; Mesen 2 a servi de contre-épreuve lors
de la relecture du lot MR-1/MR-1b (`docs/review/MR-1-mappers.md`), qui relève aussi les points
où c'est Mesen qui est en erreur.

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

### MiniEngine (DirectX-Graphics-Samples) — (c) 2015 Microsoft — licence MIT

L'occlusion ambiante multi-échelle du style Enhanced — le mode « MSVO » de l'option *Ambient
Occlusion* (`rust/crates/zax-render/src/gl/msvo.rs` et les nuanceurs
`rust/crates/zax-render/src/shaders/msvo_*.frag`) — est **adaptée du MiniEngine de Microsoft**
(<https://github.com/microsoft/DirectX-Graphics-Samples>, fichiers `MiniEngine/Core/SSAO.cpp` et
`MiniEngine/Core/Shaders/Ao*.hlsl(i)`, auteur James Stanard), portée des nuanceurs de calcul
Direct3D 12 vers des passes de fragments OpenGL 3.3. Les fichiers concernés portent l'en-tête :

    // Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License

Le MiniEngine est publié sous la licence MIT suivante, compatible avec la GPL v3 de Zaxnes :

    The MIT License (MIT)

    Copyright (c) 2015 Microsoft

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

### SMAA (iryoku/smaa) — (c) 2013 Jorge Jimenez et al. — licence MIT

Le quatrième niveau de l'option *Anti Alias* — « SMAA (profondeur) », AMD-060 REQ-SCN-085 — est
**traduit de l'implémentation de référence SMAA** (<https://github.com/iryoku/smaa>, fichier
`SMAA.hlsl`), du HLSL vers le GLSL (`rust/crates/zax-render/src/shaders/smaa*.glsl` et
`smaa*.frag`) et vers le WGSL (`rust/crates/zax-render/src/wgpu_backend/shaders/smaa*.wgsl`). Les
fichiers concernés nomment la référence et la fonction dont ils viennent.

Les **deux tables précalculées** de la référence (`Textures/AreaTex.h` et `Textures/SearchTex.h`)
sont reprises à l'octet, dans `rust/crates/zax-render/src/assets/smaa/area.bin` (160 × 560, `R8G8`)
et `search.bin` (64 × 16, `R8`) ; leurs empreintes SHA-256 et la commande d'extraction sont dans
`rust/crates/zax-render/src/assets/smaa/README.md`.

SMAA est publié sous la licence MIT suivante, compatible avec la GPL v3 de Zaxnes :

    Copyright (C) 2013 Jorge Jimenez (jorge@iryoku.com)
    Copyright (C) 2013 Jose I. Echevarria (joseignacioechevarria@gmail.com)
    Copyright (C) 2013 Belen Masia (bmasia@unizar.es)
    Copyright (C) 2013 Fernando Navarro (fernandn@microsoft.com)
    Copyright (C) 2013 Diego Gutierrez (diegog@unizar.es)

    Permission is hereby granted, free of charge, to any person obtaining a copy
    this software and associated documentation files (the "Software"), to deal in
    the Software without restriction, including without limitation the rights to
    use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
    of the Software, and to permit persons to whom the Software is furnished to
    do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software. As clarification, there
    is no requirement that the copyright notice and permission be included in
    binary distributions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

### rcheevos — (c) 2018 RetroAchievements.org — licence MIT

La prise en charge de RetroAchievements (`rust/crates/zax-achievements`) compile la bibliothèque
**rcheevos** v12.5.0 (<https://github.com/RetroAchievements/rcheevos>), dont les sources utiles
sont copiées, sans modification, dans `rust/crates/zax-achievements/rcheevos/` avec sa licence,
la licence MIT suivante :

    MIT License

    Copyright (c) 2018 RetroAchievements.org

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

Les certificats racines qui servent à joindre le serveur en HTTPS viennent du paquet
`webpki-roots` (données de Mozilla, licence CDLA-Permissive-2.0, listée plus bas).

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

## FCEUX — second moteur d'émulation, en option (non livré par défaut)

### FCEUX — (c) les auteurs de FCEUX — GNU GPL v2 ou ultérieure

Zaxnes peut être construit avec **FCEUX** comme second moteur d'émulation, choisi jeu par jeu
(lot FCEUX-1). Cette possibilité est derrière la fonctionnalité Cargo `fceux-core`, **désactivée
par défaut** : les binaires livrés ne contiennent aucun code de FCEUX, et le constructeur qui
n'active pas cette fonctionnalité n'a rien à faire de cette section.

**Aucun code de FCEUX n'est repris dans les sources de Zaxnes.** Il vit uniquement dans un fork
séparé, `cleanroom/fceux-zax` (branche `zax`), qui conserve l'historique complet de FCEUX amont ;
le côté Rust (`rust/crates/zax-fceux`) ne parle qu'à l'interface C de ce fork
(`zax/zaxfceux.h`) et à rien d'autre. Chaque en-tête de copyright de FCEUX est conservé tel quel,
et les fichiers ajoutés dans `zax/` portent la même licence que FCEUX.

FCEUX est publié sous **GNU GPL v2 ou ultérieure** ; « ou ultérieure » permet de l'utiliser sous
GPL v3, qui est la licence de Zaxnes, de sorte que l'ensemble lié reste sous GPL v3 — que la
bibliothèque soit liée statiquement ou dynamiquement.

**Ce qu'impose une distribution avec FCEUX** : si un binaire de Zaxnes est livré avec la
bibliothèque de ce fork, les **sources complètes correspondantes du fork** doivent voyager avec
lui (offertes aux mêmes conditions, à côté du binaire). C'est la raison pour laquelle
l'historique de FCEUX est conservé intact dans le fork : chacune de nos modifications est un
`git diff` lisible par-dessus l'amont. `zax/README.md` les énumère.

Le fork compile aussi la copie de **zlib** que FCEUX embarque déjà
(`src/drivers/win/zlib`, zlib 1.1.4, licence zlib) quand aucune zlib système n'est trouvée, et la
copie de **minizip** de FCEUX (`src/utils/unzip.cpp`, licence zlib) ; les deux sont du code de
l'arbre amont, sous les licences de leurs auteurs.

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
| base64 | 0.23.1 | MIT OR Apache-2.0 |
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
| bytes | 1.12.1 | MIT |
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
| getrandom | 0.2.17 | MIT OR Apache-2.0 |
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
| http | 1.5.0 | MIT OR Apache-2.0 |
| httparse | 1.10.1 | MIT OR Apache-2.0 |
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
| ring | 0.17.14 | Apache-2.0 AND ISC |
| rustc-hash | 1.1.0 | Apache-2.0/MIT |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT |
| rustls | 0.23.46 | Apache-2.0 OR ISC OR MIT |
| rustls-pki-types | 1.15.1 | MIT OR Apache-2.0 |
| rustls-webpki | 0.103.16 | ISC |
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
| subtle | 2.6.1 | BSD-3-Clause |
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
| untrusted | 0.9.0 | ISC |
| ureq | 3.5.0 | MIT OR Apache-2.0 |
| ureq-proto | 0.6.5 | MIT OR Apache-2.0 |
| utf8-zero | 0.8.1 | MIT OR Apache-2.0 |
| webpki-roots | 1.0.11 | CDLA-Permissive-2.0 |
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
| zeroize | 1.9.1 | Apache-2.0 OR MIT |
| zip | 7.4.0 | MIT |
| zlib-rs | 0.6.8 | Zlib |
| zmij | 1.0.23 | MIT |
| zopfli | 0.8.4 | Apache-2.0 |
