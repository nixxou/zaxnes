# Zaxnes — assets et licences

Ce fichier répond à **AMD-024 REQ-PUB-008**. Il liste **tout** ce que Zaxnes affiche ou fait
entendre qui n'est pas du texte : d'où cela vient, qui en est l'auteur et sous quelle licence.

Zaxnes est une **réimplémentation indépendante**, sans lien avec Geod Studio ni avec 3dSen.
**Aucun fichier provenant de ce programme n'est livré** : ni profil de jeu, ni texture, ni
cubemap, ni modèle, ni son, ni police (REQ-PUB-002).

## 1. Ce que le dépôt et la version publiée contiennent

| Asset | Origine | Licence |
|---|---|---|
| Skybox « Gradient » (dégradé, étoiles, scintillement) | **Générée par le code du projet** — `src/Zax.Render/Gl/ProceduralSkyShaders.cs`, `SkyboxShaders.cs` | GPL-3.0 (ce projet) |
| Skybox « Fantasy » (nébuleuse, champ d'étoiles) | **Générée par le code du projet** — mêmes fichiers | GPL-3.0 (ce projet) |
| Soleil, lune et nuages procéduraux | **Générés par le code du projet** — `ProceduralSkyShaders.Bodies`, paramètres dans `src/Zax.Render/SkyBodies.cs` | GPL-3.0 (ce projet) |
| Boîtier TV « Retro Crt Tv » (`assets/tv/retro_crt_tv.glb`) | **Modèle tiers**, livré tel quel — voir §1 bis | **CC BY 4.0** |
| Sons d'interface (onglet, page, déplacement, validation) | **Synthétisés par le code du projet** — `src/Zax.App/Audio/UiSoundSynth.cs`, oscillateurs dans `ChipVoices.cs` | GPL-3.0 (ce projet) |
| Musique de menu | **Aucune.** REQ-PUB-005 révisée le 2026-10-21 : la version publiable n'a pas de musique de menu. Les trois pistes WAV d'origine sont retirées sans remplacement, et le réglage « Musique de menu » reste dans le modèle sans effet | — |
| Logo et icône de l'application (`branding/`) | **Créés pour ce projet** (lot ICON-1) — `branding/zaxnes.ico`, `branding/zaxnes-icon-{16,24,32,48,64,128,256,512}.png`, `branding/zaxnes-icon.svg`, `branding/zaxnes-icon-small.svg`, `branding/zaxnes-logo.svg`, et les scripts qui les produisent, `branding/make-icon.py` et `branding/make-logo.py` | GPL-3.0 (ce projet) |
| Nuanciers, shaders GLSL, géométrie des tuiles | **Écrits pour ce projet** | GPL-3.0 (ce projet) |
| Palette NES | Donnée technique publique (documentation NES), reprise dans la spécification | donnée de fait, non protégeable |
| Clé d'encodage des plans de tuiles (`.3dn`) | Donnée d'interopérabilité, constante numérique — `src/Zax.Profiles/Wire/TilePlaneCipher.cs` (REQ-PUB-007) | donnée d'interopérabilité |

Aucun fichier `.bytes`, `.3dn`, `.png`, `.jpg`, `.obj`, `.wav`, `.ogg`, `.mp3`, `.ttf`, `.otf`
ou `.ico` n'est présent, ni dans le dépôt, ni dans la version publiée, **à deux exceptions
près** :

1. le `.glb` du boîtier TV décrit au §1 bis — **livré** mais **absent du dépôt** ;
2. le logo et l'icône du `branding/` — **présents dans le dépôt** mais **aucun n'est livré**
   comme fichier : ils voyagent *à l'intérieur* de `Zaxnes.exe` (§1 ter).

REQ-PUB-002 prévoit exactement cela : les fichiers « générés par le projet et listés dans
`ASSETS-LICENSES.md` ». Le script `build/publish.ps1` échoue si un autre fichier d'asset
apparaît dans la sortie, et le test `REQ_PUB_002_the_repository_carries_no_asset_file` fait de
même pour le dépôt, en n'acceptant `branding/` que tant que cette page porte l'entrée.

## 1 bis. Le boîtier TV — attribution CC BY 4.0 (AMD-025)

| | |
|---|---|
| **Œuvre** | « **Retro Crt Tv** » |
| **Auteur** | **meipal** — https://sketchfab.com/ashabbugaev12 |
| **Source** | https://sketchfab.com/3d-models/retro-crt-tv-d3025abdf2b64882bf60f207b2042a5c |
| **Licence** | **Creative Commons Attribution 4.0 International (CC BY 4.0)** — https://creativecommons.org/licenses/by/4.0/ |
| **Fichier livré** | `assets/tv/retro_crt_tv.glb` (glTF 2.0 binaire), **livré tel quel, sans aucune modification** |

Le fichier n'est pas modifié. Zaxnes l'adapte **à l'exécution**, et ces adaptations sont les
suivantes (CC BY 4.0 demande qu'elles soient indiquées) :

1. **l'écran n'est pas dessiné** : les deux plaques superposées de l'écran (la vitre et l'image
   texturée, les deux maillages de 49 sommets) sont masquées au chargement, et l'image du jeu
   NES apparaît à leur place ;
2. **échelle, orientation et position** sont recalculées à chaque image pour que l'ouverture de
   l'écran encadre exactement le rectangle visible de SPEC-06 : le modèle est tourné (son axe
   +X, la face avant, devient l'axe −Z de la scène), mis à l'échelle de façon non uniforme en
   largeur et en hauteur — l'écart d'aspect avec le modèle d'origine est assumé — et placé de
   sorte que l'ouverture tombe dans le plan de la face avant du tunnel ;
3. **ombres** : le caisson ne projette ni ne reçoit d'ombre, les boutons font les deux ;
4. **matières** : les matériaux PBR métal-rugosité du fichier sont rendus par le nuanceur du
   projet (`src/Zax.Render/Gl/ModelShaders.cs`), avec les cinq textures PNG et la texture JPEG
   que le fichier contient, décodées par `PngImage` et `JpegImage` ;
5. la géométrie du tunnel de SPEC-06 placée derrière le modèle est rognée sur la silhouette du
   caisson, ce qui ne touche pas au modèle lui-même.

Zaxnes n'est pas affilié à meipal, et l'auteur du modèle n'approuve ni ne cautionne ce projet.
Le code de lecture et d'adaptation (`src/Zax.Render/Assets/GltfBinary.cs`,
`src/Zax.Render/TvModel.cs`) est du code du projet, sous GPL-3.0.

## 1 ter. Le logo et l'icône — créés pour le projet (lot ICON-1)

Le logo et l'icône n'ont pas été repris : ils ont été **dessinés pour Zaxnes**, et les scripts
Python qui les produisent sont dans `branding/` à côté d'eux. Ils sont donc sous la licence du
projet, GPL-3.0, comme le reste de son code.

Ils entrent dans l'exécutable par deux chemins, et **aucun des deux n'ajoute de fichier à la
version publiée** :

| | |
|---|---|
| **Icône de l'exécutable** | `branding/zaxnes.ico` (16, 24, 32, 48, 64, 128 et 256 px) devient la **ressource d'icône Win32** de `Zaxnes.exe`, par la propriété `ApplicationIcon` de `src/Zax.App/Zax.App.csproj`. C'est elle que l'Explorateur, la barre des tâches et Alt+Tab lisent. |
| **Icône de la fenêtre** | GLFW veut des pixels RGBA et non un `.ico` : les sept PNG `branding/zaxnes-icon-*.png` sont **embarqués comme ressources de l'assemblage** et décodés au lancement par le lecteur PNG du projet (`src/Zax.Render/Assets/PngImage.cs`), puis passés à `IWindow.SetWindowIcon` (`src/Zax.App/Branding/AppIcon.cs`, `src/Zax.Platform/WindowIcon.cs`). |

L'icône est **décorative** : une ressource absente ou illisible ne peut pas empêcher le
démarrage. Le lancement écrit alors une ligne de journal (`Zaxnes: window icon not set: …`) et
la fenêtre garde l'icône par défaut de la plate-forme.

Les SVG (`zaxnes-icon.svg`, `zaxnes-icon-small.svg`, `zaxnes-logo.svg`) ne sont ni embarqués ni
livrés : le `README.md` affiche le logo, et les deux autres sont les sources de l'icône.

## 2. Ce que Zaxnes lit sur votre machine, sans le redistribuer

| Ressource | Comment | Licence |
|---|---|---|
| Police de l'interface | Zaxnes ouvre au lancement une police **déjà installée** sur votre système : Segoe UI, sinon Arial, sinon Tahoma ou Verdana (`src/Zax.App/Ui/SystemFonts.cs`). Le fichier n'est ni copié ni livré. `--font <fichier>` en impose une autre. | celle de votre système |
| Police de repli | Faute de candidate, Dear ImGui utilise sa police intégrée (Proggy Clean, domaine public, incluse dans la bibliothèque). Les accents et le glyphe « ▸ » sont alors remplacés par des équivalents ASCII. `--builtin-font` force ce mode. | domaine public / MIT |
| ROMs | Jamais fournies. Utilisez une ROM dont vous disposez légalement. | la vôtre |
| Profils de jeu `.3dn` | Jamais fournis. Si vous disposez légalement d'un profil, importez-le par **Fichier ▸ Importer un profil…** ; il est copié dans votre dossier de données, hors du dépôt et hors de la version publiée. | celle de son auteur |
| Dossier d'assets facultatif | `--assets <dossier>` lit un dossier de fichiers **que vous fournissez** (`profiles.json`, `cubemaps/`, `meshes/`, `textures/`, `audio/`). Il n'est jamais recherché automatiquement et rien dans le dépôt ni dans la version publiée n'en dépend. | celle de leurs auteurs |

## 3. Composants tiers

Le détail des paquets et des binaires natifs livrés est dans
[`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md). En résumé :

| Composant | Licence |
|---|---|
| Silk.NET (OpenGL, Windowing, Input, OpenAL) | MIT |
| Dear ImGui et ImGui.NET | MIT |
| GLFW (`glfw3.dll`) | zlib/libpng |
| OpenAL Soft (`soft_oal.dll`) | LGPL v2.1 |
| KeraLua et Lua 5.4 (`lua54.dll`) | MIT |
| Runtime .NET 10 | MIT |
| 3DNes_Engine, dont une partie du moteur de conversion 2D→3D est adaptée | GNU GPL v3 |

## 4. Licence de Zaxnes

Zaxnes est distribué sous **GNU General Public License v3** (fichier `LICENSE`) — c'est la licence
de 3DNes_Engine, dont `src/Zax.Engine3D` adapte une partie, qui l'impose. Les assets générés par
le code sont donc eux aussi sous GPL-3.0 : ce sont du code.
