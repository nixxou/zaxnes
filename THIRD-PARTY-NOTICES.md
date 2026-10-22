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

**Un seul asset est livré sous forme de fichier** : le boîtier TV `assets/tv/retro_crt_tv.glb`,
« **Retro Crt Tv** » de **meipal**, sous licence **CC BY 4.0**, livré tel quel (AMD-025).
L'attribution complète et la liste des adaptations faites à l'exécution sont dans
[`ASSETS-LICENSES.md`](ASSETS-LICENSES.md). Tout le reste est **généré par le code du projet**
(AMD-024 REQ-PUB-002) : décors, matières et sons d'interface ; la police de l'interface est lue
sur la machine de l'utilisateur au lancement, sans redistribution. Il n'y a **pas de musique de
menu** (REQ-PUB-005, révisée le 2026-10-21). Les profils de jeu et les ROMs ne sont pas fournis.

L'inventaire complet, fichier par fichier, est dans
[`ASSETS-LICENSES.md`](ASSETS-LICENSES.md).
