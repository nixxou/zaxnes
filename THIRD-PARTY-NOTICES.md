# Zaxnes — avis de licences tierces

Zaxnes est distribué sous **GNU General Public License v3** (fichier `LICENSE`).

Zaxnes est une **réimplémentation indépendante**, sans lien avec Geod Studio ni avec 3dSen.

## Code tiers repris dans Zaxnes

### 3DNes_Engine — (c) Geod Studio — GNU GPL v3

Une partie du moteur de conversion 2D vers 3D (`src/Zax.Engine3D`) est **adaptée de
3DNes_Engine**, une version antérieure et publique du moteur de conversion publiée par son
auteur sous GNU GPL v3. Les fichiers concernés portent l'en-tête :

    // Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

3DNes_Engine est distribué sous GNU GPL v3 ; c'est cette licence qui impose à Zaxnes d'être
lui-même sous GPL v3. Le texte complet de la licence est dans `LICENSE`.

## Paquets livrés avec cette version

La liste exacte est dans `Zax.App.deps.json`.

| Paquet | Version | Licence |
|---|---|---|
| Silk.NET.Core, .Maths, .OpenGL, .OpenGL.Extensions.ImGui, .Windowing(.Common/.Glfw), .Input(.Common/.Extensions/.Glfw), .GLFW, .OpenAL | 2.23.0 | MIT |
| Ultz.Native.GLFW — binaire natif de **GLFW** (`glfw3.dll`) | 3.4.0 | zlib/libpng |
| Silk.NET.OpenAL.Soft.Native — binaire natif d'**OpenAL Soft** (`soft_oal.dll`) | 1.23.1 | LGPL v2.1 |
| ImGui.NET — et le binaire natif **Dear ImGui / cimgui** (`cimgui.dll`) | 1.90.8.1 | MIT |
| KeraLua — et le binaire natif **Lua 5.4** (`lua54.dll`) | 1.4.11 | MIT |
| Microsoft.Extensions.DependencyModel | 9.0.9 | MIT |
| Microsoft.DotNet.PlatformAbstractions | 3.1.6 | Apache 2.0 |
| Runtime .NET 10 (Microsoft), inclus car la version est autonome | 10.0 | MIT |

**OpenAL Soft** est sous LGPL v2.1 et est livré comme bibliothèque dynamique séparée
(`soft_oal.dll`) : elle est remplaçable par une autre version compatible, ce que la LGPL
demande. Zaxnes fonctionne sans elle, en silence (`--no-audio`).

**Dear ImGui** embarque sa police intégrée (Proggy Clean, domaine public), que Zaxnes utilise en
repli quand aucune police système n'est trouvée (AMD-024 REQ-PUB-006).

Le format de profil `.3dn` est lu et écrit par un codec écrit pour ce projet
(`src/Zax.Profiles/Wire/`) ; aucune bibliothèque protobuf n'est livrée.

## Assets

**Aucun asset n'est livré sous forme de fichier.** Le seul qui l'était — le boîtier TV
décoratif — a été retiré du produit par AMD-031. Tout est **généré par le code du projet**
(AMD-024 REQ-PUB-002) : décors, matières et sons d'interface ; la police de l'interface est lue
sur la machine de l'utilisateur au lancement, sans redistribution. Il n'y a **pas de musique de
menu** (REQ-PUB-005, révisée le 2026-10-21). Les profils de jeu et les ROMs ne sont pas fournis.

L'inventaire complet, fichier par fichier, est dans
[`ASSETS-LICENSES.md`](ASSETS-LICENSES.md).

## Port Rust (`rust/`)

Le binaire `zaxnes` du port Rust est lié aux paquets suivants (graphe de dépendances normales
de `zax-app`, fonctionnalité `bundled` : SDL2 est alors lié statiquement). Liste produite par
`cargo tree -p zax-app --features bundled -e normal --format "{p}|{l}"` ; les paquets de
l’espace de travail (code de Zaxnes) ne sont pas listés. Les sources de Dear ImGui, de Lua 5.4
et de SDL2 voyagent dans leurs paquets `imgui-sys`, `mlua-sys` et `sdl2-sys`.

La version Rust ne livre aucun paquet LGPL : OpenAL n’est plus utilisé.

| Paquet | Version | Licence |
|---|---|---|
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| bitflags | 1.3.2 | MIT/Apache-2.0 |
| block-buffer | 0.12.1 | MIT OR Apache-2.0 |
| bstr | 1.13.1 | MIT OR Apache-2.0 |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| chlorine | 1.0.13 | Zlib OR Apache-2.0 OR MIT |
| cpufeatures | 0.3.1 | MIT OR Apache-2.0 |
| crc32fast | 1.5.3 | MIT OR Apache-2.0 |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 |
| deranged | 0.5.9 | MIT OR Apache-2.0 |
| digest | 0.11.3 | MIT OR Apache-2.0 |
| either | 1.19.0 | MIT OR Apache-2.0 |
| equivalent | 1.0.3 | Apache-2.0 OR MIT |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| foldhash | 0.2.0 | Zlib |
| getrandom | 0.3.4 | MIT OR Apache-2.0 |
| glow | 0.16.0 | MIT OR Apache-2.0 OR Zlib |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| hybrid-array | 0.4.15 | MIT OR Apache-2.0 |
| imgui-sys | 0.12.0 | MIT OR Apache-2.0 |
| imgui | 0.12.0 | MIT OR Apache-2.0 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 |
| libc | 0.2.191 | MIT OR Apache-2.0 |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |
| log | 0.4.34 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| mint | 0.5.9 | MIT |
| mlua-sys | 0.10.0 | MIT |
| mlua | 0.11.6 | MIT |
| num-conv | 0.2.2 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| num_threads | 0.1.8 | MIT OR Apache-2.0 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| png | 0.17.16 | MIT OR Apache-2.0 |
| powerfmt | 0.2.1 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| sdl2-sys | 0.38.0 | MIT AND Zlib |
| sdl2 | 0.38.0 | MIT |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| sha1 | 0.11.0 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| smallvec | 1.16.3 | MIT OR Apache-2.0 |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |
| time-core | 0.1.9 | MIT OR Apache-2.0 |
| time-macros | 0.2.32 | MIT OR Apache-2.0 |
| time | 0.3.55 | MIT OR Apache-2.0 |
| typed-path | 0.12.4 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| zip | 7.4.0 | MIT |
| zlib-rs | 0.6.8 | Zlib |
| zmij | 1.0.23 | MIT |
| zopfli | 0.8.4 | Apache-2.0 |
