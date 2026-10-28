# Zaxnes — guide d'utilisation

Zaxnes est un émulateur NES qui affiche les jeux **en 3D**. Ce document explique comment
l'installer, l'utiliser et où il range ses fichiers.

Zaxnes est distribué sous **GPL v3** (fichier `LICENSE`). **Aucune ROM n'est fournie** : vous devez
utiliser une ROM dont vous disposez légalement.

---

## 1. Prérequis et installation

| Élément | Version |
|---|---|
| Système | Windows 10/11 x64, Linux x86_64 (glibc 2.31 ou plus récente), consoles PortMaster aarch64 |
| Compilateur (pour compiler) | Rust, version fixée par `rust/rust-toolchain.toml` |
| Carte graphique | OpenGL 3.3 core (toute carte depuis ~2010) ; OpenGL ES 3.0 sur console |
| Audio | facultatif ; sans périphérique, Zaxnes démarre en silence |

**Zaxnes n'a besoin d'aucun fichier d'asset** (AMD-024, phase P7a) et n'en livre aucun : le
boîtier TV décoratif, qui était le seul fichier livré, a été retiré du produit (AMD-031). La
scène se compose de l'image du jeu et du fond.
Les skyboxes et les sons d'interface sont **produits par le code** ; il n'y a
**pas de musique de menu** ; la police de l'interface est une police **déjà installée sur votre Windows**
(Segoe UI, sinon Arial), ouverte au lancement et jamais recopiée. Le détail et les licences sont
dans `ASSETS-LICENSES.md`.

**Aucune ROM et aucun profil de jeu ne sont fournis.** La bibliothèque démarre avec deux entrées
génériques écrites dans le code : « 2D Mode » (l'image de la console, sans conversion 3D) et
« 3D par défaut (sans profil) » (la géométrie 3D par défaut, pour n'importe quelle ROM). Si vous
disposez légalement d'un profil `.3dn`, **Fichier ▸ Importer un profil…** l'ajoute (voir §9).

`--assets <dossier>` reste disponible pour lire un dossier de fichiers **que vous fournissez**
(`profiles.json`, `cubemaps/`, `meshes/`, `textures/`, `audio/`, `fonts/`). Ce dossier n'est
**jamais cherché automatiquement** : seule l'option le désigne.

La version Windows est un seul fichier, `zaxnes.exe`, sans rien à installer. La version Linux
utilise la bibliothèque SDL2 du système (paquet `libsdl2-2.0-0` sur Debian et Ubuntu). La version
console s'installe comme un port PortMaster (voir `rust/build/portmaster/README.md`).

Pour compiler, depuis le dossier `rust/` du dépôt :

```
cargo build --release -p zax-app --features bundled
```

Pour lancer la suite de tests :

```
cargo test --workspace
```

Les tests qui ont besoin d'une ROM commerciale, des profils de l'utilisateur ou d'une
installation de 3dSen sont marqués `#[ignore]` et lisent les dossiers nommés par des variables
d'environnement : voir la section 12.

## 2. Démarrer

Lancez simplement l'exécutable :

```
zaxnes.exe          (Windows)
./zaxnes            (Linux)
```

Sans argument, Zaxnes s'ouvre sur la **liste des jeux du mode console** (§ Mode console) : les ROMs
du dossier choisi dans **Options ▸ Interface ▸ Dossier des ROM** (par défaut le dossier `roms` des
données utilisateur), avec leur profil et leur jaquette, au clavier ou à la manette. Une fois un
jeu lancé, la barre de menus revient d'elle-même. `F1` ou `Origine` quitte la liste pour l'**écran
d'accueil** de la barre de menus : il rappelle le chemin **Fichier ▸ Ouvrir une ROM** et liste les
dix dernières ROMs ouvertes, qu'un clic relance. La musique de menu joue si elle est activée (menu
**Audio ▸ Musique de menu**). Avec `--rom`, le jeu démarre directement.

**Une seule instance à la fois** par dossier de données : un second lancement sur la même racine
utilisateur écrit une ligne sur la sortie d'erreur et se ferme, pour que deux fenêtres n'écrivent
pas dans les mêmes `settings.json`, `library.json` et emplacements de sauvegarde. Deux racines
différentes — `--userdata <dossier>` — peuvent tourner côte à côte.

### Ouvrir un jeu

1. **Fichier ▸ Ouvrir une ROM…** (`Ctrl`+`O`) ouvre une boîte de dialogue : la colonne du haut
   donne le chemin courant (cliquez sur un niveau pour y aller), la liste montre d'abord les
   dossiers puis les fichiers que le filtre garde, et le champ de recherche filtre les noms de
   fichiers. Filtres disponibles : `.nes` + `.zip`, `.nes` seul, `.zip` seul, tous les fichiers.
   Le dossier où vous terminez est **mémorisé** pour la fois suivante.
2. La **première fois** qu'un fichier est ouvert, Zaxnes demande quel **profil 3D** utiliser : la
   liste commence par « 2D Mode » (le jeu en 2D, orbitable à la caméra), puis
   « 3D par défaut (sans profil) » (la géométrie 3D par défaut), puis vos profils importés. Un
   champ de recherche filtre par titre.
3. Le choix est **mémorisé pour ce fichier** : la fois suivante, le jeu démarre directement.
   **Fichier ▸ Choisir le profil 3D…** rouvre cette sélection pour le jeu en cours.

Une archive `.zip` est acceptée : sa **première entrée** est lue comme image de ROM. Les
archives `.7z` sont refusées faute de décodeur LZMA disponible (`spec/changes/Q-028.md`).

### Ligne de commande

| Option | Effet |
|---|---|
| `--rom <chemin>` | ouvre cette ROM au démarrage ; sans `--profile`, la résolution de profil ci-dessus s'applique |
| `--roms <dossier>` | dossier dont le mode console liste les jeux, pour cette session (passe avant le réglage **Dossier des ROM**) |
| `--console` | ouvre Zaxnes en mode console et y reste, même avec `--rom` |
| `--profile <titre>` | entrée de la bibliothèque (titre exact, identifiant ou préfixe non ambigu), ou chemin d'un profil `.3dn` déjà importé. Sans l'option : « 2D Mode » |
| `--list-profiles` | liste les entrées de la bibliothèque, puis quitte |
| `--import-3dsen <dossier\|fichier>` | importe les profils de **votre** installation de 3dSen (§9.1), affiche le bilan, puis quitte avec `0` si tout s'est bien passé |
| `--assets <dossier>` | dossier de fichiers d'assets que vous fournissez (facultatif, jamais cherché tout seul) |
| `--font <fichier>` | police de l'interface : un fichier, ou un nom dans le dossier des polices du système |
| `--builtin-font` | force la police intégrée d'ImGui (accents et « ▸ » remplacés par de l'ASCII) |
| `--userdata <dossier>` | racine des données modifiables (voir §6) |
| `--size 1600x900` | taille de la fenêtre (défaut 1280 × 720) |
| `--no-audio` | n'ouvre pas le périphérique audio |
| `--smoke-test`, `--smoke-frames <n>` | rend un nombre fixe d'images sans interaction, puis quitte avec 0 |
| `--smoke-emulated <n>` | joue `n` images émulées (60 = 1 s) sans interaction, puis quitte avec 0 |
| `--ui-shots <dossier>` | capture une image PNG par écran d'interface, puis quitte |
| `--help` | aide complète |

#### Surcharges de réglages (le temps d'un lancement)

Ces options **écrasent le réglage persisté correspondant**, et seulement si vous les passez :
sans elles, c'est la valeur que vous avez choisie dans « Options ▸ Réglages » qui s'applique
(`settings.json`, §6). La surcharge n'est **pas** réécrite dans le fichier : au lancement suivant
vous retrouvez votre choix. Dès que vous modifiez le même réglage dans l'interface pendant la
session, votre nouvelle valeur reprend la main et est enregistrée normalement
(`spec/changes/Q-066.md`).

| Option | Réglage | Effet |
|---|---|---|
| `--vsync [0\|1\|2]` | `video.vsync` | synchronisation des échanges de tampon : `0` désactivée, `1` chaque rafraîchissement, `2` un rafraîchissement sur deux. Sans valeur, l'option vaut `1`. Quand elle est active, le plafond d'images/s est levé : c'est l'écran qui cadence |
| `--fps <60\|90\|120>` | `video.frame_rate` | plafond d'images/s **quand la synchronisation verticale est désactivée** |
| `--no-render-skip` | `video.skip_frame` | redessine la scène même sans nouvelle image émulée |
| `--fov <degrés>` | `camera.fov` | champ de vision, 45 à 95 |
| `--no-shadows` | `render.shadow` | démarre sans les ombres |
| `--skybox <nom>` | `scene.skybox` | `default`, `color`, `gradient` ou `fantasy` |

Si l'image se déchire alors que « Synchronisation verticale » est activée, regardez la ligne
`Zaxnes: display pacing: …` de la console : elle dit l'intervalle d'échange réellement en vigueur.
Un repli `every other refresh unavailable, Q-066` signifie que le pilote ou le backend n'a pas
l'appel d'intervalle exact et que la valeur 2 a été ramenée à 1. Un pilote réglé sur
« Synchronisation verticale : désactivée » de force (panneau NVIDIA / AMD) écrase de toute façon
la demande du programme.

## 3. Les menus

La barre de menus est en haut de la fenêtre. Chaque entrée affiche son raccourci. Une entrée
grisée n'est pas disponible dans l'état courant — « Reset » sans jeu chargé, par exemple.

Pendant une partie, la barre **se cache** : elle réapparaît, par-dessus l'image, quand la souris
touche le haut de la fenêtre, et reste tant qu'un de ses menus est ouvert. Sans jeu, elle reste
affichée.

### Mode console (F1)

`F1` ou `Origine` (Home) passe au **mode console** : les écrans de la version portable, en plein
écran dans la fenêtre — liste des jeux du dossier des ROM (réglage **Dossier des ROM**, onglet
Interface ; `--roms` passe avant), profil, options du jeu, menu en jeu, écrans de sauvegarde avec
leurs aperçus. Les
mêmes touches reviennent à la barre de menus (un menu laissé ouvert sur le jeu se ferme et la
partie reprend). `--console` ouvre directement Zaxnes dans ce mode. Au clavier, les touches du
joueur 1 servent de manette (`X` = A, `Z` = B, `Échap` = menu…), plus `W` (= L2) et `H` (= Start)
sur la liste des jeux.

La **souris** marche aussi sur PC : un clic sur une ligne la sélectionne, un clic sur la ligne
sélectionnée la valide (A ; un double clic lance donc un jeu), un clic sur le « < valeur > » d'une
option la change (moitié gauche : la valeur d'avant, moitié droite : la suivante) ; un clic sur une
aide de la barre du bas fait ce qu'elle dit (« L/R » et « </> » : moitié gauche, moitié droite) ;
la molette monte et descend dans la liste, le clic droit revient (B). La **barre de menus** reste
à portée : elle apparaît quand la souris touche le haut de la fenêtre, comme pendant une partie,
et un menu ouvert retient les clics de l'écran dessous. Une entrée qui ouvre une fenêtre du bureau
ou agit sur le jeu (Options, Ouvrir une ROM, Hacks…) ramène à la barre de menus ;
**Outils ▸ Mode console** bascule comme `F1`.

Sur la liste, un jeu pour lequel un hack écran large existe porte un **[W]** : gris tant que
l'écran large est désactivé pour ce jeu, en couleur une fois activé. **L2** (`W` au clavier) le
bascule ; **Start** (`H`) ouvre l'écran **Hacks** du jeu (§ Écran large), qu'on trouve aussi en
dernière ligne de ses options (Select) et dans le menu en jeu.

Un **[W] rouge** marque une autre version d'un tel jeu (reconnue au titre de son profil ou au nom
du fichier), pour laquelle le hack n'est pas fait — *Zelda II* européen, par exemple. L2, ou son
écran Hacks, explique alors pourquoi l'écran large ne peut pas être activé : la ROM pour laquelle
le hack est fait, avec son CRC, et le nom et le CRC de celle-ci, à titre indicatif. Si sa taille
est la bonne, on peut **forcer l'activation** (bouton *Forcer* de la question, ou ligne *Forcer
l'activation* de l'écran Hacks) : le patch est appliqué sans vérifier son CRC. Un hack de la ROM
proche de l'original a de bonnes chances de fonctionner ; une autre version du jeu, beaucoup
moins (*Zelda II* européen ne démarre pas avec le patch américain). Le [W] passe en **orange**
tant que l'activation forcée est en place et l'écran large activé ; ce réglage est propre au jeu.
Les sauvegardes d'état d'un jeu forcé ne sont pas contrôlées comme les autres (orange/rouge) : en
charger une après avoir retiré le forçage peut donner n'importe quoi.

Le CRC est celui des seules données de la ROM, à la taille que donne son en-tête : un en-tête
différent, un *trainer* ou des octets en trop en fin de fichier ne changent rien (c'est le CRC
que donne No-Intro).

`Ctrl`+`S` et `Ctrl`+`L` (ou **Fichier ▸ Sauvegarder un état… / Charger un état…**) ouvrent
directement l'écran de sauvegarde ou de chargement de ce mode par-dessus le jeu, qui se met en
pause ; `B` (ou `Échap`) y revient, et après le choix d'un emplacement la barre de menus
revient d'elle-même.

Les films (menu **Outils**, `Ctrl`+`M`) et les options du hack (menu **Vidéo**) ne sont que dans
la barre de menus : en mode console, on l'atteint par le haut de la fenêtre, ou `F1` / `Origine`
y ramène. Un film en cours
d'enregistrement continue pendant le mode console, et une sauvegarde chargée depuis ses écrans
le ramène en arrière comme depuis le menu.

### Fichier

| Entrée | Raccourci | Effet |
|---|---|---|
| Ouvrir une ROM… | `Ctrl`+`O` | boîte de dialogue de fichiers |
| ROMs récentes ▸ | | les dix derniers fichiers ouverts |
| Choisir le profil 3D… | | rouvre la sélection de profil pour le jeu en cours |
| Importer un profil… | | ajoute un fichier `.3dn` à la bibliothèque, sans redémarrage |
| Importer depuis 3dSen… | | importe d'un coup les profils de votre installation de 3dSen (§9.1) |
| Supprimer le jeu… | | supprime le jeu courant **s'il vient d'un profil importé**, après confirmation |
| Créer la jaquette | | la dernière capture de pause devient la jaquette du jeu courant |
| Importer une jaquette… | | une image PNG devient la jaquette, réduite pour tenir dans 256 × 192 |
| Charger un état ▸ | `Ctrl`+`F1` … `Ctrl`+`F10` | les dix emplacements, avec leur date et heure ou « vide », puis l'état rapide |
| Sauvegarder l'état ▸ | `Maj`+`F1` … `Maj`+`F10` | les mêmes emplacements, en écriture |
| Supprimer un état ▸ | | les emplacements occupés seulement, après confirmation |
| Quitter | `Ctrl`+`Q` | ferme proprement : réglages, bindings et bibliothèque sont écrits |

« Supprimer le jeu… » demande `Do you want to delete the game and its save states ?` et, sur
« Yes », arrête l'émulation si c'est le jeu courant, vide tous ses emplacements — l'état rapide
compris —, puis efface son fichier `.3dn`, sa jaquette et son entrée de bibliothèque. Un profil
embarqué et l'entrée « 2D Mode » ne sont jamais supprimables : l'entrée reste grisée.
« Supprimer un état ▸ » demande `Do you want to delete the slot <n> ?` avant d'effacer.

### Émulation

| Entrée | Raccourci | Effet |
|---|---|---|
| Pause / Reprise | `P` ou `Échap` | met en pause ou reprend |
| Reset | `Ctrl`+`R` | redémarrage à froid du jeu courant |
| Arrêter | | revient à l'écran d'accueil |
| Avance rapide | `.` (point) | ×1,5 tant que la touche est maintenue |
| Rollback | `,` (virgule) | recule d'une à deux secondes |
| Vitesse ▸ | | −8 à +8, avec le facteur correspondant ; « Plus vite » et « Moins vite » |
| Région ▸ | | Auto (depuis l'en-tête de la ROM), NTSC, PAL, Dendy |
| Ports ▸ | | type de chaque port : Manette ou Zapper |

### Vidéo

| Entrée | Raccourci | Effet |
|---|---|---|
| Mode 3D / Mode 2D | `F11` | bascule **à chaud** entre le rendu 3D et l'image en pixels du cœur |
| Mode caméra ▸ | | Orbitale (TPV), Première personne (FPV), Vue de dessus |
| Réinitialiser la vue | `Échap` maintenu ≥ 0,3 s | revient à la vue par défaut |
| Skybox ▸ | | Par défaut, Couleur, Dégradé, Fantaisie |
| Ombres | | active ou désactive les ombres |
| Plein écran | `Alt`+`Entrée` | masque la barre de menus ; les raccourcis restent actifs |
| Afficher les FPS | | petit compteur en haut à droite, **désactivé par défaut** |

`F11` ne relance rien : la partie continue à la frame où elle était, le profil, les scripts et la
conversion restent chargés, et un second appui rend le rendu 3D du profil courant immédiatement.
Un jeu lancé sur l'entrée « 2D Mode » de la bibliothèque est toujours affiché en pixels, quel que
soit l'état de la bascule.

### Audio

Volume ▸ (par paliers de 10 %, plus « Volume + » et « Volume − »), Muet, Musique de menu.

### Outils

| Entrée | Raccourci | Effet |
|---|---|---|
| Options… | `Ctrl`+`,` | la fenêtre d'options (§4) |
| Configuration des touches… | | la fenêtre de remappage (§5) |
| Dossier des données | | ouvre la racine utilisateur dans l'explorateur |
| À propos | | nom, version, licence et crédits |
| Enregistrer un film | | commence un film à l'image affichée ; un second clic l'arrête et l'écrit |
| Lire un film ▸ | | les 15 films les plus récents du dossier `movies` ; rend la main au joueur à la fin |
| Note sur cette frame… | `Ctrl`+`M` | met en pause et demande une note sur l'image affichée (`Ctrl`+`Entrée` écrit, `Échap` annule) |
| Arrêter le film | | arrête l'enregistrement (et l'écrit) ou la lecture |

Un **film** (`.zmv`) garde un état de départ et les touches de chaque image : la partie se
rejoue à l'identique. Pendant l'enregistrement, charger une sauvegarde ou faire un rollback
ramène le film à cette image, et l'enregistrement continue. Les fichiers vont dans le dossier
`movies` des données utilisateur ; le détail est dans [`rust/docs/MOVIES.md`](rust/docs/MOVIES.md).

### Options du jeu

Le menu **Options du jeu** ouvre d'abord la fenêtre **Hacks…** du jeu en cours (§ Écran large),
puis règle quelques options pour ce seul jeu (il est grisé sans jeu) : volume, échelle de rendu, ombres, limite d'images, compteur d'images, mesures de
performance, rattrapage, chemins Lua, scripts en parallèle et région. Chaque option est sur
« Use Global : <valeur> » tant qu'elle n'a pas été changée pour ce jeu ; son sous-menu propose
les autres valeurs. Seules les options changées sont gardées, par fichier de ROM, et elles
s'appliquent à chaque lancement de ce jeu, puis disparaissent à sa fin. Le titre du menu est en
cyan quand le jeu en a ; « Tout remettre en global » les efface toutes. Changer, pendant la
partie, dans la fenêtre d'options, une option que le jeu a en propre change la valeur du jeu,
pas le réglage global.

### Notifications

Les messages du programme — `Quick Save`, `Roll Back`, `Fast Forward`, les erreurs de chargement —
s'affichent **en surimpression en bas de la vue**, une à la fois, pendant 4 s (1 s pour le
rollback et l'avance rapide). Une notification normale ne remplace jamais une erreur encore
visible. Elles restent affichées en plein écran.

### Plein écran

`Alt`+`Entrée` masque la barre de menus et donne toute la fenêtre à la vue du jeu. Tous les
raccourcis clavier et manette continuent de fonctionner, et une fenêtre ouverte par raccourci —
les options avec `Ctrl`+`,`, par exemple — s'affiche par-dessus la vue. `Alt`+`Entrée` à nouveau
fait réapparaître la barre.

### Panneau de statistiques

`F12` affiche ou masque un panneau de mise au point (images par seconde, frames émulées, état de
la boucle, nombre de tuiles, audio, pilote OpenGL). Il est **désactivé par défaut**.

## 4. La fenêtre d'options

**Outils ▸ Options…** (`Ctrl`+`,`). Six onglets : **Vidéo**, **Caméra**, **Lumière et ombres**,
**Audio**, **Émulation**, **Interface**. Chaque ligne est un réglage ; le contrôle dépend de sa
nature (case à cocher, liste, curseur, sélecteur de couleur) et ses bornes sont celles du modèle
de réglages.

- une modification **prend effet immédiatement** : il n'y a pas de bouton « Appliquer » ;
- elle est **persistée** dans `settings.json`, au plus tard 2 s après le changement ;
- le bouton **« Valeurs par défaut »** de chaque onglet remet les réglages de cet onglet à leur
  valeur d'origine ;
- une ligne marquée *(réglage de profil)* appartient au jeu : elle sera remplacée au chargement
  d'un jeu différent. Une ligne marquée *(au prochain démarrage)* ou *(au prochain chargement)*
  ne prend effet qu'à ce moment-là ;
- chaque changement de valeur joue **une fois** le son de validation, **sauf** les curseurs
  **Vitesse** (onglet Émulation) et **FOV** (onglet Caméra), qui sont muets ;
- **Dossier des ROM** (onglet Interface) : le dossier dont le mode console liste les jeux. Le
  chemin se tape (pris en compte à `Entrée` ou en quittant le champ) ou se choisit avec
  **Parcourir…** ; vide, c'est le dossier `roms` des données utilisateur. La liste suit le
  changement tout de suite ; `--roms` passe avant ce réglage.

### API graphique : OpenGL, Vulkan, DirectX 12

Les versions Windows et Linux x86_64 savent dessiner en **OpenGL** (par défaut), en **Vulkan** ou,
sous Windows, en **DirectX 12**. Le choix se fait dans l'onglet **Vidéo**, ligne **« API graphique
(au prochain démarrage) »**, ou pour un seul lancement avec `--backend gl`, `--backend vulkan` ou
`--backend dx12`. Les trois donnent la même image ; seul le chemin vers la carte graphique change.
La version console reste en OpenGL ES.

- Si Vulkan ou DirectX 12 ne peut pas s'ouvrir (pilote absent, par exemple), Zaxnes démarre en
  OpenGL et le journal dit pourquoi.
- Le journal liste les cartes graphiques capables de dessiner dans la fenêtre. Sur une machine à
  plusieurs cartes, la variable d'environnement **`ZAXNES_GPU`** en choisit une par un morceau de
  son nom (`ZAXNES_GPU=3060`) ; sans elle, c'est celle que le système recommande.
- Sur les cartes qui le permettent (tableaux de textures et dessin indirect), Vulkan et DirectX 12
  regroupent les dessins de la scène en quelques appels, ce qui allège beaucoup le processeur. En
  cas de doute sur un pilote, `ZAXNES_WGPU_BATCHING=0` revient à un appel par groupe de tuiles ;
  l'image est la même.
- En Vulkan et DirectX 12, les ciels « Gradient » et « Fantasy » sont toujours les ciels calculés
  (`--assets` et ses cubemaps restent propres à OpenGL), et la résolution automatique garde
  l'échelle choisie (elle est propre au rendu OpenGL).
- La résolution automatique (méthode de rendu « Render Tex ») fait varier la résolution de la
  scène entre 50 et 100 %, sans à-coup : elle remonte d'un pour cent à la fois tant que les images
  arrivent à l'heure, et redescend d'un dixième quand des images sont en retard **à cause du
  rendu**. Les retards dus au processeur (scripts, conversion) n'y touchent pas : baisser la
  résolution ne les corrigerait pas.

**ReShade.** En **Vulkan**, ReShade trouve la profondeur de la scène 3D, ce qui ouvre les effets
qui en dépendent (occlusion ambiante, profondeur de champ, brouillard…) :

- installez ReShade sur `zaxnes.exe` en choisissant **Vulkan** dans l'installeur, et lancez Zaxnes
  avec l'API graphique Vulkan ;
- la profondeur de Zaxnes n'est **pas inversée** : ajoutez la définition globale
  `RESHADE_DEPTH_INPUT_IS_REVERSED=0` (ReShade suppose 1 par défaut ; sans elle, les effets lisent
  une profondeur vide). La caméra va de 0,3 à 5 000 unités ; une valeur de
  `RESHADE_DEPTH_LINEARIZATION_FAR_PLANE` autour de 1000 donne une profondeur bien contrastée ;
- désactivez l'**anticrénelage** (MSAA) de Zaxnes : ReShade lit mal une profondeur
  multiéchantillonnée ; l'anticrénelage de ReShade (SMAA) prend le relais.

**DirectX 12 et ReShade ne vont pas ensemble pour l'instant** : la bibliothèque de rendu de Zaxnes
crée au démarrage une fabrique DXGI « media » que ReShade prend pour une fabrique ordinaire, et le
programme s'arrête net. Avec ReShade, utilisez Vulkan.

Les effets de ReShade s'appliquent à toute l'image de la fenêtre, barre de menus comprise.

### Écran large : *Zelda II* et *Mega Man 2* en 16:9

Zaxnes sait montrer un jeu **plus large** que les 256 pixels de la NES, quand le jeu est prévu
pour. Aujourd'hui, deux le sont, grâce à un **hack écran large intégré** à l'émulateur :
***Zelda II - The Adventure of Link* (USA)** et ***Mega Man 2* (USA)**. ***Super Mario Bros. 3*
(USA) (Rev 1)** l'est aussi : son décor est lu dans le niveau sans patch, et un patch intégré y
ajoute les ennemis (le mode avancé).

L'écran large se règle **par jeu**, dans l'écran **Hacks** du jeu : en mode console, depuis la
liste (Start, ou L2 pour seulement l'activer ou le désactiver), depuis les options du jeu ou depuis
le menu en jeu ; sur PC, par **Options du jeu ▸ Hacks…**. Il est **désactivé par défaut** ; ses
réglages ne sont plus dans les options globales (une valeur globale enregistrée par une version
précédente est oubliée au démarrage).

| Ligne | Choix | Effet |
|---|---|---|
| **Écran large** | activé / désactivé | l'activer prend le ratio Auto |
| **Ratio** | Auto, 3:2, 16:10, 16:9 | **Auto** suit la vue : la fenêtre, l'écran en plein écran, l'écran de la console portable (une console 4:3 n'a donc pas de marges) |
| **Marge supplémentaire** | +0 % à +300 % (+20 % par défaut) | capture un peu plus que ce qui est montré, pour que les objets 3D au bord de la vue soient déjà entiers quand ils entrent dans le champ |
| **Mode avancé** | activé (par défaut) / désactivé | pour un jeu dont le patch est un mode avancé (*Super Mario Bros.*, *Ninja Gaiden III*, *Super Mario Bros. 3*, *Adventure Island*, *Excitebike*, *Final Fantasy*, *Metal Force*, *Blaster Master*, *Metroid*) : activé, le patch s'applique (ennemis dans les marges, sans ralentissements) ; désactivé, seul le décor est élargi, la ROM n'est pas modifiée |
| options du patch | selon le jeu | *Mega Man 2* : ennemis dans les marges, invincible, sans ralentissements ; *Super Mario Bros. 3*, *Adventure Island*, *Metal Force* et *Blaster Master* (mode avancé) : sélecteur de niveau, invincible, sans ralentissements ; *Metroid* (mode avancé) : sélecteur de niveau, invincible, sans ralentissements |

Seules les valeurs changées sont gardées pour le jeu, avec ses autres options.

- **Le hack s'applique tout seul, en mémoire**, au lancement d'une ROM USA reconnue à son
  contenu, **si l'écran large est activé pour ce jeu** ; désactivé, le jeu démarre tel qu'il a été
  fait. Désactivé en cours de partie, l'écran large retire seulement les marges : la ROM d'origine
  revient au prochain lancement (et activé en cours de partie, le patch attend lui aussi le
  prochain lancement). Le fichier de la ROM n'est jamais modifié, et Zaxnes ne livre aucune donnée
  du jeu (le correctif ne contient que ce que le hack ajoute). Le journal l'indique :
  `… : built-in patch applied (…)`. Les versions européennes ne sont pas concernées. Les CRC des
  ROMs, qui disent pour quel jeu un hack existe, sont gardés dans `rom_crc.json` des données
  utilisateur (avec la taille et la date de chaque fichier, qui n'est relu que s'il change).
- *Zelda II* : en vue de côté (villes, palais, grottes), le décor et les **ennemis** sont
  dessinés au-delà des bords, et les nouveaux ennemis apparaissent dans les marges au lieu de
  surgir au bord de l'écran ; sur la **carte du monde**, l'image est plus large (et plus haute
  avec la largeur supplémentaire) et les rencontres vivent dans les marges. La logique du jeu
  (contacts, IA) reste celle d'origine. Les projectiles ne sont pas montrés dans les marges.
- *Mega Man 2* : le décor des marges est lu dans le niveau, et les ennemis apparaissent, vivent
  et s'affichent dans les marges. Pas de marge verticale. Options du patch : « Ennemis dans les
  marges » (actif par défaut), « Invincible » (une aide pour tester, désactivé par défaut),
  « Sans ralentissements » (actif par défaut : plus de lignes d'overclock, le jeu ne ralentit
  plus).
- *Super Mario Bros. 3* (USA) (Rev 1, la version « PRG1 ») : l'écran large seul lit le décor des
  marges dans le niveau, et aussi la carte du monde (l'écran voisin de la carte apparaît au-delà
  de son cadre), avec une marge au-dessus de l'écran ; la ROM n'est pas modifiée. La ligne **Mode
  avancé** (activée par défaut) ajoute le patch : les ennemis sont créés, vivent et s'affichent dans les marges (ils ne
  lancent ni marteau ni boule de feu tant qu'ils n'y sont pas sur l'écran d'origine), et Mario
  peut aller dans la marge de droite, où l'écran défile tout seul aussi. Options du patch :
  « Sélecteur de niveau » (B sur le menu 1P/2P ouvre le menu de débogage du jeu : Haut/Bas le
  monde, Start sa carte ; désactivé par défaut), « Invincible » (désactivé par défaut),
  « Sans ralentissements » (actif par défaut). Les labyrinthes de tuyaux verticaux n'ont pas de
  marges.
- *Castlevania* (USA), la version d'origine comme la Rev 1 (« PRG1 ») : l'écran large seul lit
  le décor de la salle dans ses données, jusqu'à ses deux bords ; la ROM n'est pas modifiée. La
  ligne **Mode avancé** (activée par défaut) ajoute le patch : les bougies, les objets et les
  ennemis s'affichent et vivent dans les marges (les zombies arrivent du bord de la marge ; rien
  ne touche Simon tant qu'il n'est pas sur l'écran d'origine). Options du patch : « Sélecteur de
  niveau » (Select pendant un stage passe au stage suivant, avec ses graphismes, sans coûter de
  vie ; après le dernier, le premier) et « Invincible » (Simon n'est jamais touché ; les chutes
  tuent toujours), désactivées par défaut ; « Sans ralentissements » (actif par défaut).
- *Adventure Island* (USA) : le décor des marges est lu dans les tables du round (pentes et
  défilement vertical compris), sans patch. Le **mode avancé** ajoute le patch : les sprites et les
  objets s'affichent dans les marges ; un objet que le jeu retire au bord gauche continue sa route
  dans la marge sans plus rien toucher (le jeu reste celui d'origine). Options du patch :
  « Sélecteur de niveau » (Select sur l'écran titre fait défiler les 32 rounds, Start lance celui
  affiché), « Invincible » (les ennemis ne blessent plus ; chutes, eau et faim tuent toujours),
  deux aides pour tester, désactivées par défaut, et « Sans ralentissements » (actif par défaut).
- *Excitebike* (Japan, USA) : le décor des marges est la piste de la course, que le jeu trace
  lui-même (sa propre routine, rejouée en avance), les tribunes et l'herbe se répétant ; sans
  patch. Le **mode avancé** ajoute le patch : les autres pilotes s'affichent dans les marges (le
  jeu les garde jusqu'à une centaine de pixels au-delà des bords ; ils y apparaissent et y
  disparaissent là où le jeu les crée et les retire). Option du patch : « Sans ralentissements »
  (actif par défaut). Le hasard du jeu dépend du temps de calcul : avec le patch, les adversaires
  ne refont pas exactement les mêmes choix que sur la ROM d'origine aux mêmes touches.
- *Final Fantasy* (USA) : le décor des marges est lu dans les cartes du jeu, la carte du monde
  comme les villes, châteaux et donjons, au-dessus et en dessous de l'écran aussi ; les boîtes de
  dialogue gardent le décor autour d'elles ; sans patch. Le **mode avancé** ajoute le patch : les
  habitants, les véhicules, le pont et le canal s'affichent dans les marges de gauche et de droite
  (pas au-dessus ni en dessous de l'écran). Les combats et les menus n'ont pas de marges. Option
  du patch : « Sans ralentissements » (actif par défaut). Le balayage noir d'un changement d'écran
  ne couvre que l'écran d'origine.
- *Metal Force* (Korea), jeu coréen non licencié (mapper 156, géré par Zaxnes) : le décor des
  marges est lu dans la carte du niveau que le jeu garde en mémoire, au-dessus et en dessous de
  l'écran aussi, et seulement dans la salle en cours (au-delà, les marges restent vides) ; sans
  patch. Le **mode avancé** ajoute le patch : les ennemis apparaissent dans les marges, endormis
  jusqu'à ce qu'ils atteignent la zone d'apparition d'origine, puis vivent et s'affichent dans les
  marges ; les avions du ciel apparaissent au bord de la vue ; les tirs restent bornés à l'écran
  d'origine. Options du patch : « Sélecteur de niveau » (gauche / droite sur l'écran « MISSION »
  choisit la mission, 1 à 7) et « Invincible » (les trous tuent toujours), désactivés par défaut ;
  « Sans ralentissements » (actif par défaut).
- *Blaster Master* (USA) : le décor des marges est lu dans la carte de la zone, dans la ROM du
  jeu, au-dessus et en dessous de l'écran aussi, et seulement dans la salle en cours (au-delà,
  les marges restent vides) ; sans patch. Le **mode avancé** ajoute le patch : les ennemis
  apparaissent dans les marges, endormis sur place jusqu'à ce qu'ils atteignent l'écran
  d'origine, puis vivent et s'affichent dans les marges ; le joueur et les tirs restent bornés
  à l'écran d'origine. Options du patch : « Sélecteur de niveau » (une nouvelle partie commence
  sur le choix de zone du jeu : haut / bas, Start ; 01-08 les zones, 09-16 leurs sections vues
  de dessus) et « Invincible », désactivés par défaut ; « Sans ralentissements » (actif par
  défaut).
- *Metroid* (USA) : les marges montrent les salles de la carte autour de l'écran, celles
  derrière une porte aussi, au-dessus et en dessous de l'écran aussi, mais seulement celles de la
  zone en cours ; une salle au-delà d'un changement de palette (Brinstar) y prend ses propres
  couleurs ; sans patch. Le **mode avancé** ajoute le patch : les ennemis apparaissent dans les
  marges, endormis dans leur pose de repos jusqu'à ce qu'ils atteignent l'écran d'origine, puis
  vivent et s'affichent dans les marges ; les portes et les objets s'affichent aussi dans les
  marges, et chaque porte a sa jumelle, la porte d'en face de la salle voisine ; Samus et ses tirs
  restent bornés à l'écran d'origine. Seules les deux salles que le jeu garde en mémoire ont des
  ennemis vivants ; ceux des autres salles des marges y sont dessinés immobiles, en aperçu,
  jusqu'à ce que le jeu construise la salle. Options du patch : « Sélecteur de niveau »
  (maintenir une direction en appuyant sur START pour une nouvelle partie : haut Norfair, droite
  Kraid, gauche Tourian, bas Ridley) et « Invincible », désactivés par défaut ; « Sans
  ralentissements » (actif par défaut).
- Ça fonctionne en 3D comme en mode 2D.

Reste dans l'onglet **Vidéo** des options globales : **Écran large : attendre les objets entiers
aux bords dessinés** (non par défaut) — une forme coupée là où le jeu cesse de dessiner reste
provisoire jusqu'à ce qu'elle soit entière.

**Sauvegardes d'état.** Une sauvegarde retient si elle a été faite avec le hack, et sa version.
Dans les menus de chargement :

- **en orange**, une sauvegarde faite avec l'écran large dans l'autre position (avec ou sans le
  patch) : Zaxnes propose d'activer ou de désactiver l'écran large de ce jeu, recharge le jeu,
  puis charge la sauvegarde ;
- **en rouge**, une sauvegarde faite avec une **autre version** du hack : elle ne peut pas être
  chargée (la mémoire du jeu ne correspond plus).

Les sauvegardes portent désormais une vignette de l'image et la version de Zaxnes qui les a
faites. La sauvegarde de pile (les fichiers de partie de *Zelda II*) ne dépend pas du hack : elle
suit la ROM d'origine.

Le détail technique est dans [`rust/docs/WIDESCREEN-EXTENSION.md`](rust/docs/WIDESCREEN-EXTENSION.md)
(côté émulateur), [`rust/docs/hacks/zelda2/WIDESCREEN-HACK.md`](rust/docs/hacks/zelda2/WIDESCREEN-HACK.md)
, [`rust/docs/hacks/megaman2/WIDESCREEN-HACK.md`](rust/docs/hacks/megaman2/WIDESCREEN-HACK.md)
, [`rust/docs/hacks/smb3/WIDESCREEN-HACK.md`](rust/docs/hacks/smb3/WIDESCREEN-HACK.md)
, [`rust/docs/hacks/adventure_island/WIDESCREEN-HACK.md`](rust/docs/hacks/adventure_island/WIDESCREEN-HACK.md)
, [`rust/docs/hacks/excitebike/WIDESCREEN-HACK.md`](rust/docs/hacks/excitebike/WIDESCREEN-HACK.md)
, [`rust/docs/hacks/ff1/WIDESCREEN-HACK.md`](rust/docs/hacks/ff1/WIDESCREEN-HACK.md)
, [`rust/docs/hacks/metalforce/WIDESCREEN-HACK.md`](rust/docs/hacks/metalforce/WIDESCREEN-HACK.md)
, [`rust/docs/hacks/blastermaster/WIDESCREEN-HACK.md`](rust/docs/hacks/blastermaster/WIDESCREEN-HACK.md)
et [`rust/docs/hacks/metroid/WIDESCREEN-HACK.md`](rust/docs/hacks/metroid/WIDESCREEN-HACK.md) (les hacks
eux-mêmes et comment les reconstruire).

### Limite de 8 sprites par ligne

La NES n'affiche que 8 sprites par ligne. Pour qu'aucun ne disparaisse pour de bon, beaucoup de jeux
changent l'ordre de leurs sprites à chaque image : ceux en trop clignotent. Le réglage **« Supprimer
la limite de 8 sprites par ligne »** (`video.remove_sprite_limit`, onglet **Vidéo**) fait comme
Mesen :

- **Désactivé** : 8 sprites par ligne, comme sur la console ;
- **Toujours** : tous les sprites de chaque ligne sont affichés, plus de clignotement ;
- **Intelligent** (par défaut) : comme **Toujours**, sauf sur les lignes où le jeu empile au moins
  8 sprites au même endroit, à la suite dans l'OAM. C'est le procédé qu'utilisent certains jeux
  pour cacher les sprites suivants : *The Legend of Zelda* cache ainsi Link quand il passe une
  porte de donjon. Là, la limite est rétablie pour ces lignes seulement.

Le réglage ne change que l'image 2D (la vue 3D a toujours tous les sprites) ; le drapeau
d'overflow (bit 5 de `$2002`) et les accès à la cartouche restent ceux de la console, quel que
soit le choix.

### Palette de couleurs NES

La NES ne produit pas de couleurs RVB mais un signal vidéo composite, que le téléviseur décode :
chaque émulateur traduit ses 64 couleurs avec une table à lui, d'où des teintes différentes d'un
émulateur à l'autre pour le même jeu. Le réglage **« Palette de couleurs NES »**
(`video.nes_palette`, onglet **Vidéo**) choisit cette table, pour l'image 2D comme pour la vue
3D : **Zaxnes** (celle de 3DSen, par défaut), **Mesen** (sa palette par défaut), **FCEUX**,
**Nestopia (YUV)** (le défaut de Nestopia), **Nestopia (RGB)**, les palettes de FirebrandX
(**Smooth**, **NES Classic** — celle de la mini console —, **Composite Direct**, **PVM Style**),
**Sony CXA2025AS** (le décodeur des téléviseurs Sony) et **Wavebeam**. Le changement est
immédiat ; le jeu n'en sait rien (les numéros de couleur restent les siens).

### Overscan (8 lignes du haut et du bas)

La NES produit 240 lignes, mais un téléviseur NTSC n'en montrait qu'environ 224 : les 8 premières
et les 8 dernières tombaient hors de l'écran. Beaucoup de jeux y préparent le décor qui défile, et
on y voit des tuiles ou des couleurs parasites. Un téléviseur PAL, lui, montrait les 240 lignes. Le
réglage **« Masquer les 8 lignes du haut et du bas »** (`video.hide_overscan`, onglet **Vidéo**) :

- **Désactivé** : les 240 lignes sont affichées ;
- **Auto** (par défaut) : ces lignes sont noires pour les jeux NTSC (60 Hz), affichées pour les
  jeux PAL (50 Hz), comme FCEUX ;
- **Toujours** : ces lignes sont noires quelle que soit la région.

En 2D, ces lignes sont noires ; en 3D, les tuiles qui sont au moins à moitié dans ces lignes ne
sont pas montrées. Le réglage ne s'applique pas avec une marge verticale d'écran large : l'image
continue alors au-delà de ces lignes, que le décodeur de marge redessine.

### Langue de l'interface

L'onglet **Interface** porte le réglage **Langue** (`ui.language`), qui vaut `en` (« English »)
ou `fr` (« Français »). **Par défaut l'interface est en anglais** : il n'y a aucune détection de
la langue du système, et au premier lancement, sur un dossier utilisateur vierge, tout est en
anglais. Le changement s'applique **immédiatement**, sans redémarrer, et il est conservé au
lancement suivant ; une valeur inconnue dans `settings.json` est lue comme `en`, sans erreur.

Ce document, la sortie de `--help`, les messages de la ligne de commande et les journaux de la
console (préfixe « Zaxnes: ») restent en anglais ou en français comme ils sont : ils ne suivent
pas le réglage (AMD-035 §2). Les copies d'écran et les noms de menus cités plus bas utilisent le
français ; en anglais, la correspondance est celle de la table des textes.

La fenêtre est **engendrée depuis le modèle de réglages** : un réglage ajouté au modèle y
apparaît sans modification de l'interface. Les réglages de réalité virtuelle sont hors
périmètre et n'y figurent pas ; le dernier dossier parcouru et la géométrie de la fenêtre n'y
figurent pas non plus, car ce sont des états retenus automatiquement, pas des choix.

## 5. La configuration des touches

**Outils ▸ Configuration des touches…**. La fenêtre liste, par section :

- **Joueur 1 — boutons NES** et **Joueur 2 — boutons NES** : les huit boutons de chaque joueur ;
- **Caméra** : les six actions de caméra ;
- **Raccourcis système** : `Menu`, `RollBack`, `FastForward`, `QuickSave`, `QuickLoad` ;
- **Commandes de menu** : toutes les commandes des menus qui n'ont pas de paramètre.

Chaque ligne a une colonne **Clavier** et une colonne **Manette**. Cliquez sur une case :

1. le binding existant est **effacé immédiatement** ;
2. la prochaine touche (colonne Clavier) ou le prochain bouton, axe ou gâchette de la manette du
   joueur (colonne Manette) devient le binding ;
3. « Arrêter la capture » interrompt l'écoute — l'action **reste sans binding**, puisqu'il a été
   effacé au clic.

Pendant une capture, les autres cases sont inactives et aucune action de jeu n'est produite :
`Échap` peut donc être assignée sans mettre le jeu en pause au même instant. Une touche déjà
maintenue quand vous cliquez n'est pas capturée : relâchez-la et réappuyez.

Deux listes en haut attribuent une **manette** à chaque joueur ; elles se mettent à jour quand
une manette est branchée ou débranchée. « Réinitialiser toutes les touches » restaure les
tables par défaut.

Il n'y a **aucune détection de conflit** : une même touche liée à deux actions les déclenche
toutes les deux. Les boutons de souris ne sont pas assignables ; les modificatrices (Maj, Ctrl,
Alt, gauche et droite séparément) le sont. Les bindings sont **globaux**, communs à tous les
jeux, et écrits dans `input.json` à chaque modification et à la sortie.

### Touches par défaut (joueur 1)

Les touches sont désignées par leur **position physique** sur un clavier QWERTY : sur un clavier
AZERTY, la touche `Z` ci-dessous est la touche **W**.

| Touche | Action |
|---|---|
| Flèches | directions NES |
| `X` | bouton A |
| `Z` (W en AZERTY) | bouton B |
| Espace | Select |
| Entrée | Start |
| `F5` / `F6` | tourner la caméra à gauche / à droite |
| `F7` / `F8` | tourner la caméra vers le haut / vers le bas |
| `F9` / `F10` | rapprocher / éloigner la caméra |
| Glisser bouton gauche | orbiter (0,1° par pixel) |
| Molette | rapprocher / éloigner (5 unités par cran) |
| `Échap` (appui court) | pause / reprise — voir ci-dessous |
| `Échap` maintenu ≥ 0,3 s | revenir à la vue par défaut |
| `,` (virgule) | rollback |
| `.` (point) | avance rapide maintenue (×1,5) |
| `Select` maintenu puis stick droit vers le bas | sauvegarde rapide |
| `Select` maintenu puis stick droit vers le haut | chargement rapide |

Un appui court sur `Échap` dépend du contexte, dans cet ordre : pendant un chargement il ne fait
**rien** et n'est pas reporté ; une boîte de confirmation ouverte l'ignore, elle ne se quitte que
par ses boutons ; le navigateur de fichiers ou la sélection de profil se ferment comme avec
« Cancel », sans toucher à l'émulation ; sinon le jeu passe en pause ou reprend.

Le **joueur 2 n'a aucune touche par défaut** : attribuez-les dans la fenêtre de configuration,
ou branchez une manette. Le clavier physique alimente les deux joueurs.

`QuickSave` et `QuickLoad` n'ont pas de touche par défaut : seules les combinaisons ci-dessus
les déclenchent, et elles sont évaluées **joueur par joueur** — c'est le même joueur qui doit
maintenir `Select` et donner le coup de stick.

> Note. AMD-010 demande `Fn` seul pour charger un état, ce qui entrerait en conflit avec les
> touches caméra `F5`–`F10` ; Zaxnes utilise `Ctrl`+`Fn` (`spec/changes/Q-030.md`, tranché par
> AMD-013).

### Manettes

Toute manette de type Xbox (XInput ou reconnue par la table standard) est prise en charge sans
réglage. Les manettes de réalité virtuelle ne sont jamais proposées ni attribuées.

Une manette joue **comme sur la version portable** (mode console), barre de menus ou pas, avec
ses boutons **aux mêmes places** que ceux de la portable (A à droite, B en bas, X en haut, Y à
gauche) — et non selon les lettres d'une manette Xbox :

| Commande | Action |
|---|---|
| Croix | directions NES |
| Bouton est (B Xbox, Rond) | bouton A ; dans les menus, choisir |
| Bouton sud (A Xbox, Croix PS) | bouton B ; dans les menus, retour |
| Back / View / Share | Select |
| Start / Menu / Options | Start |
| Guide (ou Select + Start si le réglage le demande) | menu du mode console, partie en pause |
| Bouton nord (Y Xbox, Triangle) | vue par défaut (le X de la portable) |
| Bouton ouest (X Xbox, Carré) | 2D / 3D ; maintenu, la 2D vue de face, plein cadre (le Y de la portable) |
| LB / RB | éloigner / rapprocher la caméra |
| Gâchette droite (RT) | avance rapide |
| Gâchette gauche (LT) maintenue / appui bref | garder / remettre l'angle et le zoom |
| Stick droit | tourner la caméra |

Le rollback n'a plus de bouton par défaut sur la manette (la touche `,` du clavier reste). Une
manette restée sur les touches par défaut d'une version précédente passe à celles-ci ; une
manette dont vous aviez changé les touches garde les vôtres.

La **première manette utilisée** va au joueur 1, la suivante au joueur 2 ; l'attribution est
mémorisée et retrouvée environ 0,5 s après le démarrage suivant. Les bindings d'une manette
appartiennent au **modèle de la manette**, pas au joueur.

Sticks et gâchettes sont convertis en tout-ou-rien à partir de **0,34** de la course, mesurée
**après mise en forme** : une zone morte de 0,2, une saturation à 0,9 et un ré-étalement linéaire
entre les deux. Concrètement, un stick poussé droit n'appuie qu'à partir d'environ **0,44** de sa
course physique, une gâchette à partir d'environ **0,44** aussi, et une poussée en diagonale
appuie les deux directions dès 0,6 de course. La zone morte d'un stick est **radiale** : c'est la
longueur des deux axes ensemble qui décide, donc une diagonale se comporte comme une poussée
droite.

### Zapper

Le Zapper se joue **à la souris**, sans réticule : le curseur système sert de viseur. La
**gâchette est le bouton droit** ; le bouton gauche continue d'orbiter la caméra. Un profil au
Zapper met le port 2 dans ce mode tout seul, et **Émulation ▸ Ports** permet de le faire à la
main. Les types de port ne sont pas persistés et reviennent à Manette dès qu'un jeu différent
est chargé.

Le curseur se masque après 2 s d'immobilité pendant qu'un jeu tourne et réapparaît au moindre
mouvement ; la visée continue de fonctionner curseur masqué.

## 6. Où Zaxnes écrit ses fichiers

Par défaut, la racine utilisateur est `%LOCALAPPDATA%\Zaxnes\` — **Outils ▸ Dossier des données**
l'ouvre dans l'explorateur. Un fichier `portable.txt` placé à côté de l'exécutable la déplace
dans `userdata\` à côté de l'exécutable, et `--userdata <dossier>` l'emporte sur les deux.

> **Vous veniez d'une version « Sen ».** Le programme s'appelait « Sen » pendant son
> développement et écrivait dans `%LOCALAPPDATA%\Sen\`. Au premier lancement, si
> `%LOCALAPPDATA%\Zaxnes\` n'existe pas encore et que l'ancien dossier existe, son contenu y est
> **copié** — réglages, bibliothèque, sauvegardes, RAM de pile, profils, jaquettes — et la
> console l'annonce (`Zaxnes: user data copied from …`). L'ancien dossier n'est **ni modifié ni
> supprimé** : vous pouvez le garder ou l'effacer vous-même. Les sauvegardes d'état écrites
> alors restent lisibles.

> **Un second dossier de profils, à côté de l'exécutable.** Quel que soit le mode — normal,
> portable ou `--userdata` —, Zaxnes parcourt **toujours** un dossier `profiles` placé à côté de
> `zaxnes.exe`, en plus du vôtre. La version publiée le fournit vide, avec un `README.txt` ;
> déposez-y vos `.3dn` et ils apparaissent dans la bibliothèque au lancement suivant. Ce dossier
> n'est jamais écrit, déplacé ni supprimé : vos imports vont toujours dans **votre** dossier de
> données, et Zaxnes fonctionne même s'il est en lecture seule (sous `Program Files`, par
> exemple) ou absent. Si les deux dossiers proposent le même profil, **le vôtre l'emporte** : un
> fichier de même nom chez vous masque entièrement l'autre, deux fichiers identiques n'en font
> qu'un, et à identifiant égal c'est le vôtre qui est chargé, l'autre étant signalé dans le
> journal. Le choix du profil indique d'où vient chaque entrée (« profil utilisateur » ou
> « profil du dossier du programme »), et un jeu du dossier du programme ne se supprime pas
> depuis l'interface.

| Chemin | Contenu |
|---|---|
| `settings.json` | réglages : les préférences utilisateur, par clé stable (`camera.fov`, `audio.volume`, `scene.skybox`…) |
| `games.json` | profil 3D choisi pour chaque fichier de ROM, ses options du jeu, et les dix ROMs récentes |
| `input.json` | bindings : clavier J1/J2, un jeu par modèle de manette, dernière manette de chaque joueur |
| `library.json` | bibliothèque : chemin de ROM, titre et date de dernière partie par jeu |
| `saves/index.json` | métadonnées des 36 emplacements numérotés et de l'emplacement rapide |
| `saves/<n>.state` | fichier de sauvegarde (signature `ZAXNESAV`, l'ancienne `SEN3DSAV` reste lisible) |
| `saves/<n>.png` | vignette 256 × 192 de l'emplacement |
| `battery/<sha1>.srm` | RAM sauvegardée par pile, par ROM (Zelda, Final Fantasy…) |
| `profiles/*.3dn` | profils utilisateur importés — et, en plus, ceux du dossier `profiles` à côté de l'exécutable |
| `covers/*.png` | jaquettes, nommées `<Titre>_3dsen_<Id>.png` |
| `logs/` | journaux |

Ce qu'il faut savoir :

- une sauvegarde d'état contient le **cœur complet** (image de ROM comprise), les données
  persistantes des scripts, l'état de la caméra et l'état appris de la conversion : charger un
  emplacement ne redemande jamais la ROM ;
- un fichier illisible, tronqué ou étranger est refusé avec le message
  `Corrupted save state.` et l'émulation revient à l'arrêt ;
- la sauvegarde rapide n'a **pas** de vignette ;
- une **jaquette** est une image PNG sans canal alpha, réduite pour tenir dans 256 × 192 en
  conservant ses proportions, nommée d'après le titre et l'identifiant du profil, tout caractère
  interdit dans un nom de fichier remplacé par `_`. Celle de l'entrée « 2D Mode » se met à jour
  toute seule à chaque pause ;
- les menus exposent **dix** emplacements alors que le modèle en gère 36 : un état écrit dans
  l'emplacement 20 par une version ultérieure reste chargeable ;
- la **RAM sauvegardée par pile est persistée**, ce que l'original ne fait pas : elle est écrite
  à l'arrêt du jeu, à la sortie et au plus toutes les 30 s si son contenu a changé ;
- si la racine n'est pas accessible en écriture, Zaxnes démarre quand même, l'annonce par une
  notification et refuse les sauvegardes ;
- un fichier illisible est renommé `<nom>.corrupt-<horodatage>` et les valeurs par défaut
  s'appliquent, avec une notification. Une valeur isolée invalide est remplacée seule, sans
  message ;
- chaque fichier porte un `schemaVersion`. Un fichier plus ancien est migré après qu'une copie
  `<nom>.v<N>.bak` a été gardée ; un fichier **plus récent** est lu au mieux, n'est pas réécrit
  pendant la session, et l'annonce par une notification.

Le fichier de réglages ne contient que les préférences de l'utilisateur, et seulement celles qui
diffèrent du défaut :

```json
{
  "schemaVersion": 1,
  "values": {
    "camera.fov": 85,
    "audio.volume": 0.4,
    "scene.skybox": "Gradient",
    "scene.custom_bg_color": "#0078FCFF",
    "ui.fullscreen": true
  }
}
```

N'y figurent **pas**, volontairement : les réglages qui viennent du profil du jeu (découpe,
calques, lumière, conversion, région, caméra de dessus), ceux qui repartent de zéro à chaque
lancement (lissage de caméra, types de ports), ce qu'un script Lua écrit sur une préférence — la
valeur vit dans une couche de session et votre préférence revient à la fin de la partie — et
tout réglage par jeu : la bibliothèque mémorise le chemin de ROM, le titre, la date de dernière
partie et la jaquette, rien d'autre.

## 7. Vitesse, avance rapide et rollback

- la cadence émulée est de **60 images/s en NTSC** et **50 en PAL et Dendy**, mesurée sur une
  horloge monotone indépendante de la cadence d'affichage ;
- le réglage de vitesse est un entier de −8 à +8 : `k = 1 + s/4` pour `s ≥ 0` et
  `k = 1/(1 − s/4)` pour `s < 0`, donc −8 donne ×1/3 et +8 ×3. L'audio suit, avec changement de
  hauteur ;
- l'avance rapide est de **vitesse fixe ×1,5** : chaque appui arme 30 frames émulées, et
  maintenir la touche réarme la rafale ;
- au-delà de **5 frames** de retard, le rattrapage est abandonné : le jeu ralentit au lieu
  d'accélérer brutalement ;
- un instantané du **cœur seul** est pris toutes les 60 frames émulées, 30 au maximum. Chaque
  rollback restaure l'avant-dernier, donc recule de 1 à 2 s ; la caméra et les données Lua ne
  sont pas restaurées.

## 8. Audio

- sortie à **48 kHz** si le périphérique l'accepte, blocs de 512 trames, signal de console mono
  dupliqué sur les deux canaux ;
- quand le cœur n'a pas un bloc complet, le bloc est du **silence exact** : jamais de craquement
  partiel ;
- les sons d'interface sont **synthétisés par le code** (AMD-024 REQ-PUB-005) : quatre sons
  distincts — onglet, page, déplacement, validation. Aucun fichier audio n'est livré ;
- il n'y a **pas de musique de menu** : REQ-PUB-005, révisée le 2026-10-21, la retire au lieu de
  remplacer les pistes d'origine. Le réglage « Musique de menu » reste dans `settings.json`,
  sans effet, et n'apparaît plus dans les options ni dans le menu Audio. Un dossier donné par
  `--assets` qui contient un sous-dossier `audio/` rejoue ses propres pistes, avec les règles de
  SPEC-11 (tirage aléatoire sans répétition immédiate, fondus de 3,33 s) ;
- gains : `V` pour l'émulation, `0,02 × V` pour la musique, `0,07 × V` pour les sons
  d'interface. Les valeurs sont faibles parce que c'est le mixage d'origine ;
- `--no-audio` n'ouvre aucun périphérique : les fondus et la sélection des pistes avancent quand
  même, le programme se comporte donc à l'identique en silence.

## 9. Les profils 3D et leurs scripts

Un profil décrit comment un jeu devient de la 3D : les motifs à extruder, les calques, les
ombres, la caméra, et des **scripts Lua** qui pilotent tout cela en jeu. Ils tournent dans un
interpréteur Lua 5.4, un état par profil.

**Aucun profil de jeu n'est fourni** (AMD-024 REQ-PUB-001). Sans profil, deux entrées génériques
sont toujours disponibles :

| Entrée | Ce qu'elle fait |
|---|---|
| « 2D Mode » | joue n'importe quelle ROM **sans conversion 3D** : l'image de la console, posée sur le plan NES et orbitable à la caméra |
| « 3D par défaut (sans profil) » | convertit n'importe quelle ROM **en 3D** avec la géométrie par défaut |

**Importer un profil que vous détenez légalement** : **Fichier ▸ Importer un profil…**, choisissez
le fichier `.3dn`. Il est copié dans `<données>/profiles` et l'entrée apparaît aussitôt dans la
bibliothèque, sans redémarrage. Un fichier dont l'identifiant est absent ou vide est refusé avec
le message « profil obsolète ou au mauvais format ». Les profils déjà présents dans ce dossier
au démarrage sont lus automatiquement.

Ce qu'il faut savoir :

- un script qui lève une erreur est **journalisé une fois puis désactivé** ; le jeu continue
  sans lui ;
- les bibliothèques `io`, `os` (hors `clock`/`time`/`date`), `debug`, `package` et `require`
  sont **absentes** : un profil est une donnée partageable et ne peut pas agir sur le système ;
- les écritures d'un script sur un réglage ne sont jamais persistées : elles n'écrasent pas vos
  préférences ;
- un profil sans aucun script s'affiche sans animation particulière ; `--list-profiles` marque
  d'un `*` les entrées qui n'en ont pas.

**Fichier ▸ Importer un profil…** ajoute un fichier `.3dn` : il est copié dans `profiles/` et la
bibliothèque le prend en compte sans redémarrage. Un profil dont l'identifiant est déjà celui
d'un profil utilisateur le remplace ; un identifiant de profil intégré est ignoré. Un fichier
invalide affiche un message et ne change rien.

### 9.1 Si vous possédez 3dSen : importer ses profils

Si vous avez acheté 3dSen, ses profils sont dans **votre** installation et vous pouvez les
importer en une action. Rien n'est téléchargé et rien n'est redistribué : seul votre disque est
lu, et le dossier de 3dSen n'est ni modifié, ni déplacé, ni copié.

**Fichier ▸ Importer depuis 3dSen…** ouvre un sélecteur ; désignez

- le **dossier d'installation** de 3dSen, celui qui contient `3dSen.exe` (le bouton « Ouvrir »
  valide le dossier affiché, ou celui que vous avez sélectionné), ou
- directement le **fichier bundle** des profils, si vous savez où il est :
  `<installation>\<nom>_Data\StreamingAssets\aa\StandaloneWindows*\defaultprofiles_assets_all_*.bundle`.

À la fin, un message donne le bilan : combien de profils ont été importés, combien étaient déjà
présents (ignorés) et combien ont échoué. Les profils importés apparaissent aussitôt dans la
bibliothèque, sans redémarrage, avec le titre que porte le profil lui-même.

En ligne de commande, sans ouvrir de fenêtre :

```
zaxnes.exe --import-3dsen "C:\Program Files (x86)\Steam\steamapps\common\3dSen"
```

Le code de sortie est `0` quand tout s'est bien passé, `1` quand aucun bundle n'a été trouvé, que
le fichier est illisible ou qu'au moins un profil a été refusé ; le détail part sur la sortie
d'erreur. Ajoutez `--userdata <dossier>` pour écrire ailleurs que dans votre dossier de données
habituel.

Détails utiles :

- un profil **identique octet pour octet** à un fichier déjà importé est ignoré : réimporter la
  même installation n'ajoute rien ;
- un profil de même nom mais de contenu différent reçoit un suffixe ` (2)`, ` (3)`… ;
- un profil que le lecteur de `.3dn` refuse est compté en échec et n'interrompt pas l'import ;
- un fichier tronqué, une signature inconnue ou une compression LZMA donnent un message clair et
  n'écrivent aucun fichier partiel.

## 10. Captures hors écran, sans fenêtre

L'outil `zax-harness` (crate `rust/crates/zax-harness`) rend des scènes dans un contexte OpenGL **caché** et écrit des PNG. Il
sert à vérifier le rendu sans ROM et à mesurer les temps de frame.

```
cargo run --release -p zax-harness -- render --out captures
```

Options utiles : `--frames synthetic:shadow` (une seule scène, voir `--list-scenes`),
`--rom jeu.nes --run 400` (exécute une vraie ROM et la capture), `--size`, `--angles`,
`--distance`, `--mesh`, `--sprite-limit kept|removed|smart` (la limite de 8 sprites par
ligne de l'image 2D ; `kept` par défaut), `--no-shadows`, `--shadow-map <texels>` (côté de la carte d'ombre,
4096 par défaut depuis `AMD-034`), `--skybox`, `--sky-bodies` (allume le soleil, la lune
et les nuages procéduraux), `--bg-color`, `--real-time`, `--explain` (liste à dessiner groupée par objet 3D avec les couleurs NES atteintes) et
`--bench 300` (médiane et 99ᵉ centile sur 300 frames).

La commande `lua-api` écrit l'API que les scripts voient réellement, une ligne par membre :

```
cargo run --release -p zax-harness -- lua-api --out api.txt
```

Du côté de l'interface, `zaxnes --ui-shots <dossier>` place l'interface dans chacun de ses
écrans, relit la fenêtre et écrit un PNG par écran.

## 11. Ce qui n'est pas encore là

- la **bibliothèque visuelle** à tuiles : la barre de menus classique la remplace (`AMD-010`) ;
  la suppression d'un profil utilisateur et les jaquettes, elles, sont en place (menu Fichier) ;
- l'import d'une jaquette au format **JPG** : seul le PNG est décodé (`spec/changes/Q-050.md`) ;
- les **archives `.7z`** (`spec/changes/Q-028.md`) ;
- le **pistolet optique libretro** : la table de correspondance existe comme donnée, le frontend
  lui-même n'est pas écrit ;
- le **frontend libretro** : le noyau expose déjà « exécuter exactement une frame émulée » et la
  sérialisation complète, mais la DLL n'est pas écrite.

Points laissés en question, visibles à l'écran ou à l'usage : l'espace des opérations sur les
teintes de la skybox
Fantasy (`Q-024`), les faces écrites dans la carte d'ombre (`Q-025`), le cadrage de la capture de
référence (`Q-026`), la vignette d'un emplacement sauvegardé en cours de jeu (`Q-027`), les
archives `.7z` (`Q-028`), la vue 3D pendant la pause (`Q-029`), les raccourcis des emplacements
(`Q-030`), l'ordre entre la région automatique et le `TVMode` du profil (`Q-031`), la capture
d'un binding dont la commande est déjà maintenue (`Q-033`) et le facteur « ×2 » du ciel Fantasy
(`Q-045`, `Q-055`). `AMD-021` a tranché le seuil des commandes analogiques (il est mesuré après
mise en forme, ce qui clôt `Q-048`, `Q-054` et `Q-056`) ; les cinq points de cet amendement
restés indécis ont été tranchés depuis par `AMD-022` (`spec/changes/Q-059.md`).

## 12. Pointer les vérifications sur votre dossier de ROMs

**Aucune ROM n'est fournie et aucun chemin de machine n'est écrit dans ce dépôt.** Les tests qui
ont besoin de fichiers de l'utilisateur sont marqués `#[ignore]` et lisent les dossiers nommés
par des variables d'environnement :

| Variable | Dossier | Utilisée par |
| --- | --- | --- |
| `ZAXNES_ROMS` | vos ROMs `.nes` | les tests qui jouent une ROM commerciale |
| `ZAXNES_PROFILES` | le dossier qui contient `profiles/` | les tests des profils, des scripts et du moteur 3D |
| `ZAXNES_3DSEN_INSTALL` | une installation de 3dSen PC | les tests de l'import 3dSen |

```
$env:ZAXNES_ROMS = "D:\mes-roms\nes"      (PowerShell, pour la session en cours)
export ZAXNES_ROMS=$HOME/roms/nes          (shell POSIX)
cargo test --workspace -- --include-ignored
```

Les noms de fichiers attendus sont ceux des dumps No-Intro (par exemple
`Super Mario Bros. 3 (USA).nes`). Le détail est dans `rust/LOCAL-CHECKS.md`.
