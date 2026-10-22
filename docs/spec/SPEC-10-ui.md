# SPEC-10 — Interface utilisateur desktop
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-03 (catalogue de profils), SPEC-06 (rendu, caméra, skybox, mode TV), SPEC-07 (boucle, pause, savestates, rollback), SPEC-08 (bindings d'entrées), SPEC-09 (modèle de réglages et persistance), SPEC-11 (sons d'UI)

## 1. Objet et périmètre

Ce document décrit l'interface desktop du programme : les écrans, leur contenu, leur navigation, les textes visibles et le comportement de chaque contrôle. L'UI est reconstruite avec Dear ImGui. On cherche une interface **fonctionnellement équivalente** et d'un aspect comparable ; on ne vise pas l'identité au pixel près.

**Dans le périmètre :**
- le menu principal à onglets GAME / LOAD / SAVE / SETTING / QUIT ;
- la bibliothèque de jeux ;
- les grilles de slots de sauvegarde (charger et sauvegarder) ;
- le panneau de réglages, avec la configuration des entrées ;
- le navigateur de fichiers ;
- les boîtes de dialogue, les notifications, la barre de progression de chargement et le bloc « à propos » ;
- les raccourcis disponibles en jeu qui produisent un retour visuel.

**Hors périmètre, donc absent de l'UI reconstruite :**
- l'UI VR et tous ses réglages : runtime VR, méthode de pointage (« Laser Beam », « Gaze »), miroir, échelle de scène, aides VR, panneaux d'association matérielle Oculus et Steam ;
- l'éditeur de profils, l'éditeur de textures, les outils de dessin, l'UI de scripting et la sélection de tiles à la souris ou au rectangle ;
- l'enregistrement et la relecture d'entrées (« Load Input », « Clear Input »), l'export de la vue en OBJ (bouton « Export view to OBJ file » et touche F3), l'enregistrement de données 3D et la conversion par lots ;
- l'ancien menu « mode ROM » : ouverture directe d'un fichier ROM ou `.sav` hors bibliothèque, sauvegarde d'état dans un fichier choisi, boutons Quick Save et Quick Load du menu. **Aucune entrée de menu ne doit ouvrir ces fonctions.**

## 2. Concepts et vocabulaire

- **Menu** : couche d'UI plein écran affichée chaque fois que l'émulation ne tourne pas (arrêtée, en pause ou en chargement).
- **En jeu** : l'émulation tourne. Le menu est caché ; seules les notifications peuvent s'afficher par-dessus la scène 3D.
- **Jeu courant** : le jeu chargé par la boucle (SPEC-07), en pause ou en cours. Il n'y en a pas avant le premier lancement, ni après un arrêt.
- **Entrée de bibliothèque** (« jeu ») : un profil du catalogue (SPEC-03), avec sa configuration utilisateur (SPEC-09) : titre, chemin de la ROM associée, image de couverture et date du dernier lancement.
- **Profil intégré** : profil livré avec le programme. **Profil importé** : profil `.3dn` ajouté par l'utilisateur ; lui seul peut être supprimé.
- **Entrée « 2D Mode »** : entrée spéciale du catalogue qui joue sans conversion 3D (SPEC-03). Elle affiche le titre « 2D Mode ».
- **Slot** : emplacement de sauvegarde d'état numéroté de 1 à 36, plus le slot rapide (SPEC-07, SPEC-09). Un slot est **vide** tant qu'aucun état n'y est enregistré.
- **Capture de pause** : image de la vue 3D prise au moment de la mise en pause, au format 4:3 (référence 256×192). Elle sert de vignette.
- **Bouton Menu** : action d'entrée « Menu » (SPEC-08), sur Échap par défaut.
- **Boutons de navigation** : les actions Haut, Bas, Gauche et Droite du joueur 1 ou 2 (SPEC-08), sur les flèches du clavier et la croix directionnelle par défaut.
- **Sélectionner** une tuile, c'est la mettre en surbrillance. **Activer** une tuile, c'est déclencher son action : lancer, charger ou sauvegarder.

## 3. Exigences

### 3.1 Carte des écrans et navigation

**REQ-UI-001** (P1) — L'UI DOIT comporter les écrans et couches suivants :

| # | Écran / couche | Type |
|---|---|---|
| E1 | Menu principal (cadre, onglets, vignette du jeu courant, tri) | plein écran |
| E2 | Onglet GAME : bibliothèque de jeux | contenu de E1 |
| E3 | Onglet LOAD : slots à charger | contenu de E1 |
| E4 | Onglet SAVE : slots où sauvegarder | contenu de E1 |
| E5 | Onglet SETTING : réglages, entrées, aide | contenu de E1 |
| E6 | Onglet QUIT : bloc « à propos » et bouton Quitter | contenu de E1 |
| E7 | Navigateur de fichiers | modal, par-dessus E1 |
| E8 | Boîte de dialogue de confirmation | modale |
| E9 | Sélecteur de couleur du fond | popup, depuis E5 |
| E10 | Barre de progression de chargement | superposée à E1 |
| E11 | Notification | superposée, en jeu comme dans le menu |
| E12 | Compteur FPS (optionnel, voir REQ-UI-095) | superposé |

Il n'y a **pas** d'écran dédié au rollback ou à l'historique. Le rollback se déclenche uniquement par son raccourci (REQ-UI-082).
- Critère d'acceptation : chacun des écrans E1 à E11 est accessible par le chemin décrit en REQ-UI-002, et aucun écran hors périmètre (§1) n'est accessible.

**REQ-UI-002** (P1) — La navigation DOIT suivre ce graphe :
- Démarrage → E1, avec l'onglet GAME sélectionné (REQ-UI-010).
- E1 → E2 à E6 : par un clic sur l'onglet, ou par Haut et Bas (REQ-UI-012).
- E2 → E7 : bouton d'import de profil, bouton d'import de couverture, clic droit sur une tuile, ou activation d'un jeu sans ROM.
- E2 → E8 : suppression d'un profil importé, ou activation du jeu courant.
- E3 et E4 → E8 : suppression d'un slot.
- E5 → E9 : bouton de couleur du fond.
- E1 → en jeu : lancement d'un jeu, chargement d'un slot, reprise par un clic sur la vignette du jeu courant, ou bouton Menu (REQ-UI-004).
- En jeu → E1 : bouton Menu.
- E6 → fin du programme : bouton Quitter.
- Critère d'acceptation : chaque transition listée se produit par le geste indiqué. En dehors de ce graphe, seuls les chemins définis par REQ-UI-007 (lancement par `-id=`), REQ-UI-035 et REQ-UI-095 peuvent changer d'écran ; aucune autre transition déclenchée par l'utilisateur n'existe.

**REQ-UI-003** (P1) — Le menu DOIT être visible si et seulement si l'émulation ne tourne pas : elle est arrêtée, en pause ou en chargement. Il se cache sans action de l'utilisateur dès que l'émulation repart, par exemple à la fin du chargement d'un jeu ou d'un slot. Pendant que le menu est visible, la scène du jeu (tiles 3D, backdrop, TV) n'est pas dessinée ; seul l'arrière-plan du ciel (skybox ou couleur, SPEC-06) reste visible derrière le menu.
- Critère d'acceptation : en jeu, appuyer sur Menu met en pause, affiche le menu et cache la scène. Activer une tuile de jeu cache le menu à la fin du chargement, sans autre action.

**REQ-UI-004** (P1) — Le bouton Menu DOIT agir sur un **appui court**, c'est-à-dire relâché moins de 0,3 s après l'enfoncement. L'effet dépend du contexte, dans cet ordre de priorité :
1. Un chargement est en cours : rien.
2. Une boîte de dialogue E8 est ouverte : rien.
3. Le navigateur de fichiers E7 est ouvert : il se ferme comme avec « Cancel ».
4. L'émulation tourne : elle passe en pause, une capture de pause est prise et le menu s'affiche.
5. L'émulation est en pause : elle reprend et le menu se cache.
6. L'émulation est arrêtée : rien.
- Critère d'acceptation : sans jeu chargé, Échap ne fait rien. Avec un jeu en cours, Échap met en pause, et un second Échap reprend. Navigateur ouvert, Échap le ferme sans changer l'état de l'émulation.

**REQ-UI-005** (P2) — Un **appui long** sur Menu (relâché au moins 0,3 s après l'enfoncement) DOIT réinitialiser la vue de la caméra (SPEC-06), sans ouvrir ni fermer le menu. C'est inactif pendant un chargement.
- Critère d'acceptation : en jeu, maintenir Échap 0,5 s puis relâcher recentre la caméra, et le jeu continue sans pause.

**REQ-UI-006** (P2) — La mise en pause depuis le menu DOIT prendre une capture de pause de la vue 3D telle qu'elle était juste avant la pause. Cette capture est utilisée par la vignette du jeu courant (REQ-UI-040) et par la sauvegarde dans un slot (REQ-UI-053).
- Critère d'acceptation : après une pause, la vignette du jeu courant montre la dernière image jouée.

**REQ-UI-007** (P3) — Le programme PEUT accepter un argument de ligne de commande `-id=<identifiant de profil>`. Au démarrage, il lance alors ce jeu comme le ferait une activation de sa tuile (REQ-UI-031), en ouvrant le choix de ROM si aucune ROM n'est associée. Un identifiant inconnu est ignoré, et le menu s'affiche normalement.
- Critère d'acceptation : `-id=<id d'un jeu dont la ROM est associée>` démarre directement ce jeu, sans afficher le menu au-delà du chargement.

### 3.2 Menu principal (E1)

**REQ-UI-010** (P1) — Le menu DOIT présenter cinq onglets, dans cet ordre et avec ces libellés : `GAME`, `LOAD`, `SAVE`, `SETTING`, `QUIT`. Un seul onglet est actif à la fois. Au premier affichage après le lancement du programme, l'onglet actif est `GAME` (DEC-10-Q6, divergence acceptée **(D)** : l'original ouvre sur `SETTING`). Ensuite, le menu rouvre sur le dernier onglet actif.
- Critère d'acceptation : au lancement, GAME est actif. Si on choisit GAME, qu'on lance un jeu puis qu'on remet en pause, GAME est actif.

**REQ-UI-011** (P2) — Les onglets DOIVENT être disposés verticalement, en colonne sur le bord gauche du cadre du menu. L'onglet actif se distingue des autres par un fond plus clair et une largeur légèrement supérieure (référence : environ 15 % de la largeur du cadre du menu pour l'onglet actif contre 12,5 % pour les autres, hauteur d'onglet d'environ 5,6 % de la hauteur du cadre).
- Critère d'acceptation : l'onglet actif se reconnaît visuellement d'un coup d'œil.

**REQ-UI-012** (P2) — Dans le menu, Haut et Bas DOIVENT sélectionner l'onglet précédent ou suivant, en boucle : Haut sur GAME mène à QUIT, Bas sur QUIT mène à GAME. Cette navigation est inactive pendant que le navigateur de fichiers est ouvert, et pendant la capture d'un binding (REQ-UI-071).
- Critère d'acceptation : sur SETTING, Bas sélectionne QUIT puis GAME. Pendant l'attente d'une touche à assigner, Bas ne change pas d'onglet.

**REQ-UI-013** (P2) — Changer d'onglet DOIT jouer le son « sélection d'onglet » (SPEC-11).
- Critère d'acceptation : chaque changement d'onglet, au clic ou au clavier, émet le son une fois.

**REQ-UI-014** (P2) — Une liste déroulante `Sort by` DOIT être affichée en haut à droite du cadre, uniquement quand l'onglet actif est GAME, LOAD ou SAVE. Elle propose deux options : `A-Z` et `Recent`. Ce choix est un réglage persistant (SPEC-09) qui s'applique aux trois grilles (REQ-UI-022, REQ-UI-050, REQ-UI-051).
- Critère d'acceptation : sur SETTING et QUIT, la liste est absente. Choisir `Recent`, quitter puis relancer le programme : `Recent` est conservé.

### 3.3 Grilles de tuiles (commun à GAME, LOAD et SAVE)

**REQ-UI-020** (P1) — Les éléments DOIVENT être présentés en grille paginée de **12 tuiles par page, sur 4 colonnes et 3 lignes**. L'ordre de lecture va de gauche à droite puis de haut en bas : l'élément n° *n* (en partant de 0, dans l'ordre de tri) occupe la page ⌊n/12⌋, la ligne ⌊(n mod 12)/4⌋ et la colonne n mod 4. La dernière page est complétée par des emplacements vides et invisibles. Une liste vide n'affiche aucune tuile.
- Critère d'acceptation : voir les vecteurs V1 (§5).

**REQ-UI-021** (P2) — Sous la grille, un indicateur de pages DOIT afficher un bouton numéroté par page (1, 2, …). Le bouton de la page courante est mis en évidence. Cliquer sur un numéro affiche cette page. Gauche et Droite affichent la page précédente ou suivante, en boucle : Gauche sur la page 1 mène à la dernière, Droite sur la dernière mène à la page 1. Chaque changement de page joue le son « changement de page » (SPEC-11). La pagination clavier et manette est inactive pendant que le navigateur de fichiers est ouvert.
- Critère d'acceptation : avec 30 éléments, il y a trois pages. Droite trois fois depuis la page 1 ramène à la page 1.

**REQ-UI-022** (P2) — Chaque tuile DOIT afficher :
- le **titre** sur une ligne, au-dessus de l'image et aligné à gauche ;
- l'**image** dans un cadre 4:3 (référence : 160×120), mise à l'échelle sans déformation.

Une tuile sans titre (emplacement de complément) est invisible. Une tuile titrée sans image affiche une image de remplacement ou un cadre à 50 % d'opacité (REQ-UI-033).
- Critère d'acceptation : une couverture carrée importée apparaît avec des bandes, sans déformation.

**REQ-UI-023** (P1) — Interaction avec la souris :
- un clic gauche sur une tuile non sélectionnée la **sélectionne** et joue le son « survol / sélection d'élément » (SPEC-11) ;
- un clic gauche sur la tuile déjà sélectionnée l'**active** et joue le son « validation » (SPEC-11).

Un double-clic DEVRAIT équivaloir à sélectionner puis activer. Cliquer hors de toute tuile retire la sélection. Les petits boutons d'une tuile (suppression, import de couverture) n'apparaissent que sur la tuile sélectionnée.
- Critère d'acceptation : un premier clic sur un jeu ne le lance pas, le second le lance.

**REQ-UI-024** (P2) — La tuile sélectionnée DOIT avoir un fond de surbrillance cyan, et la tuile survolée un fond blanc léger.
- Critère d'acceptation : la tuile sélectionnée est identifiable visuellement.

**REQ-UI-025** (P1) — Aucune activation de tuile (lancer, charger, sauvegarder) ne DOIT avoir d'effet pendant qu'un chargement est en cours.
- Critère d'acceptation : pendant la barre de progression, activer une autre tuile ne fait rien.

### 3.4 Onglet GAME — bibliothèque (E2)

**REQ-UI-030** (P1) — La bibliothèque DOIT lister toutes les entrées du catalogue de jeux : profils intégrés de l'édition installée (SPEC-03), profils importés par l'utilisateur, et l'entrée « 2D Mode ». Le titre affiché est celui de la configuration du jeu, qui vaut par défaut le titre du profil (SPEC-03, SPEC-09). Il n'y a ni champ de recherche ni filtre.
- Critère d'acceptation : avec l'édition complète, le nombre de tuiles vaut le nombre d'entrées du catalogue intégré, plus les profils importés.

**REQ-UI-031** (P1) — L'activation d'une tuile de jeu DOIT se comporter ainsi :
1. **Aucune ROM associée** au jeu : le navigateur de fichiers s'ouvre pour choisir une ROM (REQ-UI-036). Après validation, le chemin est associé au jeu, enregistré (SPEC-09), puis le jeu est lancé.
2. **Le jeu est le jeu courant** (en pause) : la boîte de dialogue E8 s'ouvre avec pour titre le titre du jeu, le message `Do you want to reset the progress?` et deux boutons, `Reset` (relance le jeu depuis le début) et `Resume` (reprend la partie en pause). Chaque bouton ferme la boîte.
3. **Sinon** : le jeu est lancé.

Lancer un jeu met à jour sa date de dernier lancement (SPEC-09) et démarre le chargement (SPEC-07). Si le tri est `Recent`, la grille se réordonne.
- Critère d'acceptation : jeu A en pause, activer A ouvre le dialogue et `Resume` reprend A au même endroit. Activer B lance B. Si le tri est `Recent`, B apparaît alors en première position.

**REQ-UI-032** (P1) — Un **clic droit** sur la tuile sélectionnée DOIT ouvrir le choix de ROM (REQ-UI-036) même si une ROM est déjà associée. C'est le moyen de changer de ROM. Après validation, le nouveau chemin remplace l'ancien et le jeu est lancé. Le maintien de l'action Select de la manette pendant l'activation DOIT avoir le même effet. L'implémentation PEUT aussi accepter le clic droit sur une tuile non sélectionnée.
- Critère d'acceptation : sur un jeu avec ROM, un clic droit ouvre le navigateur titré `Select ROM`.

**REQ-UI-033** (P2) — Image d'une tuile de jeu :
- une couverture personnalisée si elle existe (REQ-UI-034, REQ-UI-035, REQ-UI-042) ;
- sinon une image de remplacement : un logo « 3D SEN » pour les jeux 3D et un logo « 2D SEN » pour l'entrée 2D Mode. Ces logos sont fournis dans l'atlas d'UI des assets neutres.
- Critère d'acceptation : un jeu jamais personnalisé montre le logo « 3D SEN ».

**REQ-UI-034** (P2) — Sur la tuile sélectionnée, un petit bouton d'import de couverture (coin inférieur gauche, libellé `C`, info-bulle `Import cover art`) DOIT ouvrir le navigateur de fichiers titré `Select external cover art`, filtré sur `jpg|png`. L'image choisie devient la couverture du jeu : elle est enregistrée (SPEC-09) et la grille est rafraîchie. Le bouton joue le son « validation ».
- Critère d'acceptation : importer un PNG met à jour la tuile, et l'image est conservée après un redémarrage.

**REQ-UI-035** (P2) — Quand l'émulation passe en pause sur l'entrée **2D Mode**, la capture de pause DOIT devenir automatiquement la couverture de cette entrée.
- Critère d'acceptation : jouer en 2D Mode puis mettre en pause : la tuile 2D Mode montre la capture.

**REQ-UI-036** (P1) — Le choix de ROM DOIT ouvrir le navigateur de fichiers (E7) en mode ouverture, titré `Select ROM`, filtré sur les extensions `nes`, `zip` et `7z`, avec la recherche active.
- Critère d'acceptation : les fichiers `.sav` et `.txt` d'un dossier n'apparaissent pas dans la liste.

**REQ-UI-037** (P2) — Sur la tuile sélectionnée d'un **profil importé**, un petit bouton de suppression (coin inférieur droit, info-bulle `Remove custom profile`) DOIT ouvrir la boîte E8 avec pour titre le titre du jeu, le message `Do you want to delete the game and its save states ?` et les boutons `Yes` et `No`. `Yes` arrête l'émulation si ce jeu est le jeu courant, supprime l'entrée, son fichier de profil importé et tous ses slots (dont le slot rapide, SPEC-09), puis rafraîchit les trois grilles. `No` ferme la boîte sans rien changer. Les profils intégrés et l'entrée 2D Mode n'ont jamais ce bouton.
- Critère d'acceptation : supprimer un profil importé fait disparaître sa tuile et ses slots des onglets LOAD et SAVE.

**REQ-UI-038** (P2) — Sélectionner la tuile d'un **profil importé** DOIT afficher pendant 2 s la notification `Profile Maker: <auteur>`, où l'auteur est le champ auteur du profil (SPEC-03).
- Critère d'acceptation : sélectionner un profil importé dont l'auteur est « X » affiche `Profile Maker: X`. Sélectionner un profil intégré n'affiche rien.

**REQ-UI-039** (P2) — En haut à gauche de l'onglet GAME, un bouton icône DOIT être présent :
- **Import** (info-bulle `Import custom profile`) : ouvre le navigateur titré `Select external 3D profile`, filtré sur `3dn`. Le profil choisi est importé (SPEC-03, SPEC-09) puis la grille est rafraîchie. Un profil dont l'identifiant existe déjà parmi les profils importés le remplace. Un profil dont l'identifiant est celui d'un profil intégré est ignoré, sans message. En cas d'erreur (fichier invalide, version non supportée), le message d'erreur s'affiche en notification de type normal pendant 4 s (§3.10).
Le bouton **Site** de l'original (lien vers un site communautaire) est **retiré** (DEC-10-Q5, divergence acceptée **(D)**).
- Critère d'acceptation : importer un `.3dn` valide ajoute une tuile. Importer un fichier texte renommé en `.3dn` affiche une notification d'erreur et ne change pas la grille.

### 3.5 Vignette du jeu courant (E1)

**REQ-UI-040** (P2) — Tant qu'un jeu courant existe (en pause ou en chargement), le menu DOIT afficher sur son bord droit une vignette encadrée de la capture de pause, au format 4:3 (référence : 160×120). Sans jeu courant, elle est absente.
- Critère d'acceptation : au démarrage, pas de vignette. Après le premier lancement puis une pause, la vignette est présente.

**REQ-UI-041** (P1) — Un clic gauche sur la vignette DOIT reprendre l'émulation, ce qui cache le menu.
- Critère d'acceptation : clic gauche sur la vignette, le jeu reprend.

**REQ-UI-042** (P2) — Un clic droit sur la vignette, ou un clic avec Select maintenu, DOIT faire de la capture de pause la couverture du jeu courant. La couverture est enregistrée, la tuile correspondante de la bibliothèque est mise à jour, et le son « validation » est joué. L'émulation reste en pause.
- Critère d'acceptation : clic droit sur la vignette, la tuile du jeu dans GAME affiche cette capture.

### 3.6 Onglets LOAD et SAVE (E3, E4)

**REQ-UI-050** (P1) — L'onglet LOAD DOIT lister les slots numérotés **non vides** ; le slot rapide n'y figure pas. Chaque tuile affiche :
- en titre, le titre du jeu sauvegardé ;
- en image, la vignette enregistrée avec l'état ;
- dans le coin supérieur gauche, un badge `Slot <n>`.

Tri :
- `A-Z` : par titre de jeu, puis du plus récent au plus ancien ;
- `Recent` : du plus récent au plus ancien, puis par titre.

Activer une tuile charge ce slot (SPEC-07), ce qui peut changer de jeu courant ; le menu se cache dès que l'émulation reprend.
- Critère d'acceptation : sauvegarder un jeu A dans le slot 3, puis lancer un jeu B et activer « Slot 3 » dans LOAD : le jeu A reprend à l'état sauvegardé.

**REQ-UI-051** (P1) — L'onglet SAVE DOIT lister **le premier slot vide** (le plus petit numéro libre), s'il en existe, suivi de tous les slots non vides. Le slot vide est toujours en première position, quel que soit le tri ; les autres suivent la même règle de tri que LOAD. Une tuile vide n'a ni image ni titre ; elle affiche seulement son badge. Activer une tuile :
1. enregistre l'état du jeu courant dans ce slot, en **écrasant sans confirmation** un éventuel contenu précédent ;
2. lui associe la capture de pause comme vignette et la date courante ;
3. rafraîchit les grilles LOAD et SAVE.

Sans jeu courant, l'activation n'a aucun effet.
- Critère d'acceptation : avec les slots 1 et 2 occupés, SAVE affiche « Slot 3 » vide en premier. L'activer crée le slot 3, et SAVE affiche alors « Slot 4 » vide en premier.

**REQ-UI-052** (P2) — Sur la tuile sélectionnée d'un slot non vide, dans LOAD comme dans SAVE, un petit bouton de suppression (coin inférieur droit) DOIT ouvrir la boîte E8 :
- titre : le titre du jeu du slot ;
- message : `Do you want to delete the slot <n> ?` ;
- boutons : `Yes` et `No`.

`Yes` vide le slot (SPEC-09) et rafraîchit les deux grilles.
- Critère d'acceptation : supprimer le slot 2 le retire de LOAD et le rend disponible comme premier slot vide dans SAVE s'il est le plus petit numéro libre.

**REQ-UI-053** (P2) — Les vignettes des slots DOIVENT provenir de la capture de pause prise au moment où le menu a été ouvert (REQ-UI-006), et non d'une capture du menu lui-même.
- Critère d'acceptation : la vignette d'un slot montre la scène de jeu, sans éléments d'UI.

**REQ-UI-054** (P3) — Quand une grille LOAD ou SAVE est vide, l'onglet PEUT afficher un texte discret. L'original n'affiche rien.
- Critère d'acceptation : aucune erreur n'apparaît quand LOAD est vide.

### 3.7 Onglet SETTING (E5)

**REQ-UI-060** (P1) — L'onglet SETTING DOIT comporter trois zones, de haut en bas :
1. une grille de **paramètres** ;
2. le panneau de **configuration des entrées** ;
3. un bloc d'**aide**.

Chaque paramètre est une ligne avec un libellé à gauche et un contrôle à droite. Toute modification s'applique immédiatement et est persistée via le modèle de réglages (SPEC-09). Il n'y a pas de bouton « Appliquer ». Les contrôles reflètent en continu la valeur courante, y compris quand elle change ailleurs (raccourci, script).
- Critère d'acceptation : déplacer le curseur Vol change le volume sur-le-champ. Relancer le programme : la valeur est conservée.

**REQ-UI-061** (P1) — Les paramètres visibles sur desktop DOIVENT être exactement ceux-ci, dans cet ordre et avec ces libellés :

| Libellé | Contrôle | Valeurs affichées → réglage (SPEC-09) |
|---|---|---|
| `Light` | curseur entier 0–100, valeur affichée sur la poignée | intensité lumineuse = v/100 + 0,5, soit 0,5 à 1,5 (SPEC-06) |
| `Vol` | curseur entier 0–100, valeur affichée | volume = v/100 (SPEC-11) |
| `BG Music` | case à cocher | musique de fond du menu (SPEC-11) |
| `Shadow` | case à cocher | ombres (SPEC-06) |
| `Render Rate` | liste : `60fps`, `90fps`, `120fps` | fréquence de rendu 60, 90 ou 120 |
| `Fast Render` | case à cocher | rendu accéléré (saut d'images, SPEC-07) |
| `Speed` | curseur entier −8 à +8, valeur affichée | vitesse de jeu (SPEC-07) |
| `FOV` | curseur entier −5 à +5, valeur affichée | champ de vision = 70 + 5·v degrés, soit 45 à 95 (SPEC-06) |
| `TV Region` | liste : `NTSC`, `PAL`, `DENDY` | région vidéo (SPEC-07) |
| `TV Box` | liste : `None`, `TV 1`, `TV 2`, `TV 3` | habillage TV (SPEC-06) |
| `Skybox` | liste : `Default`, `Color`, `Gradient`, `Fantasy`, plus un bouton de couleur | fond (SPEC-06), voir REQ-UI-063 |
| `Full Screen` | case à cocher | mode plein écran de la fenêtre (REQ-UI-064) |

- Critère d'acceptation : FOV sur +2 donne 80°. Light sur 50 donne une intensité de 1,0. Les douze libellés sont présents, et aucun autre paramètre n'apparaît.

**REQ-UI-062** (P1) — Les réglages suivants de l'original ne DOIVENT PAS apparaître sur desktop, car ils y étaient cachés ou relèvent de la VR ; ils restent présents dans le modèle de réglages (SPEC-09), avec leur persistance et, le cas échéant, leur exposition libretro : `Graphic` (Low / Medium / High, portée P, DEC-09-Q1), `Anti Alias` (None / FFAA / SMAA), le choix d'algorithme de rendu (`Normal` / `Retina`), `Runtime`, `Control`, `Mirror` et `Scale`. Le choix de méthode de rendu (`Direct` / `Render Tex`) et `Tex Scale` (`Native`, `2/3`, `1/2`, `1/3`, `1/4`) ne sont pas affichés non plus (voir Q-2).
- Critère d'acceptation : aucun de ces libellés n'est présent dans l'onglet.

**REQ-UI-063** (P2) — Le bouton de couleur placé à droite de la liste `Skybox` DOIT être visible uniquement quand l'option `Color` est choisie. Son fond montre la couleur personnalisée courante. Un clic ouvre ou ferme le sélecteur de couleur E9 (REQ-UI-090). Changer d'option de skybox ferme le sélecteur.
- Critère d'acceptation : choisir `Color` fait apparaître le bouton ; choisir `Gradient` le fait disparaître.

**REQ-UI-064** (P2) — La case `Full Screen` DOIT refléter à chaque image l'état réel de la fenêtre, y compris après un passage en plein écran par un autre moyen, par exemple un raccourci système. Passer en plein écran DOIT utiliser la résolution native de l'écran.
- Critère d'acceptation : basculer le plein écran par Alt+Entrée met la case à jour.

**REQ-UI-065** (P2) — Modifier une liste, une case ou un curseur de SETTING DOIT jouer le son « validation » (SPEC-11). Le curseur `Speed` est la seule exception.
- Critère d'acceptation : cocher `Shadow` produit le son.

**REQ-UI-066** (P2) — Le curseur `FOV` DOIT appliquer la valeur arrondie au degré. La liste du choix de rendu (s'il est affiché) est inactive pendant un chargement.
- Critère d'acceptation : FOV +5 donne exactement 95°.

**REQ-UI-067** (P2) — Quand on quitte l'onglet SETTING ou qu'on ferme le menu, toutes les listes déroulantes ouvertes DOIVENT se fermer, le sélecteur de couleur DOIT se fermer et une capture de binding en cours DOIT s'arrêter.
- Critère d'acceptation : ouvrir la liste `TV Box` puis appuyer sur Menu : en revenant dans le menu, la liste est fermée.

### 3.8 Configuration des entrées (zone 2 de E5)

**REQ-UI-070** (P1) — Le panneau d'entrées DOIT présenter, sur une ligne d'en-tête :
- `Player` : liste `1` / `2` ;
- `Type` : liste `Keyboard` / `Gamepad` ;
- `Gamepad` : une liste des manettes connectées, uniquement quand le Type est `Gamepad`. Elle commence par une entrée « aucune manette » et est mise à jour quand une manette est branchée ou débranchée (SPEC-08) ;
- un bouton `Stop Binding`, visible uniquement pendant une capture de binding.

En dessous, une ligne par action, avec le libellé de l'action et un bouton qui affiche le nom de la touche ou du bouton assigné (vide si rien n'est assigné). Les 19 actions, dans cet ordre : `Left`, `Right`, `Up`, `Down`, `A`, `B`, `Select`, `Start`, `RotLeft`, `RotRight`, `RotUp`, `RotDown`, `ZoomIn`, `ZoomOut`, `Menu`, `RollBack`, `FastForward`, `QuickSave`, `QuickLoad`.
- Critère d'acceptation : avec Player 1 et Type Keyboard, les bindings par défaut de SPEC-08 s'affichent, par exemple `Menu` sur Échap.

**REQ-UI-071** (P1) — Un clic sur le bouton d'une action DOIT **effacer** son binding actuel pour le joueur et le type choisis, puis lancer une capture. La prochaine touche (type Keyboard) ou le prochain bouton ou axe de la manette choisie (type Gamepad) devient le binding, puis la capture s'arrête (SPEC-08). Pendant la capture :
- la ligne capturée est mise en évidence (fond clair) et les autres sont grisées ;
- les autres boutons d'action sont inactifs ;
- la navigation entre onglets est suspendue (REQ-UI-012).
- Critère d'acceptation : cliquer sur `A` puis appuyer sur la touche K affiche `K` sur la ligne `A`. Le jeu répond ensuite à K pour A.

**REQ-UI-072** (P1) — `Stop Binding` DOIT interrompre la capture. L'action reste alors **sans binding**, puisqu'il a été effacé au clic. Changer de `Player`, de `Type` ou de manette interrompt aussi la capture en cours. Choisir une manette dans la liste `Gamepad` l'associe au joueur choisi (SPEC-08). Ces trois listes jouent le son « validation ».
- Critère d'acceptation : cliquer sur `B` puis sur `Stop Binding` : la ligne `B` est vide.

**REQ-UI-073** (P2) — Si le type est `Gamepad` et qu'aucune manette n'est associée au joueur, les boutons d'action DOIVENT être vides, et un clic sur l'un d'eux NE DOIT PAS lancer de capture.
- Critère d'acceptation : sans manette branchée, avec Type Gamepad, cliquer sur `Left` ne fait rien.

### 3.9 Aide (zone 3 de E5) et onglet QUIT (E6)

**REQ-UI-075** (P2) — Le bloc d'aide DOIT afficher ce texte centré sur quatre lignes. Les termes entre crochets sont colorés : *cyan* pour l'action, *rouge* pour la cible, *jaune* pour le bouton.

```
[cyan]Select rom[/]: Right click [red]game slot[/] - [cyan]Create game art[/]: Right click [red]current screenshot[/]
[cyan]Switch views[/]: Press [yellow]Menu[/] - [cyan]Reset camera[/]: Long Press [yellow]Menu[/]
[cyan]Quick save[/]: [yellow]Select[/] + [yellow]Rot Down[/] - [cyan]Quick load[/]: [yellow]Select[/] + [yellow]Rot Up[/]
[cyan]Trigger zapper[/]: Right click
```
- Critère d'acceptation : le texte est présent avec ses trois couleurs.

**REQ-UI-076** (P2) — L'onglet QUIT DOIT afficher :
- en haut, un bloc « à propos » sur fond gris translucide, en lignes centrées : le nom du produit **« Sen »** en rouge et gras, la ligne `Version <x.y.z>` en bleu, la mention de licence « GPL v3 », puis les crédits « d'après 3dSen (Geod Studio) » et « contient du code adapté de 3DNes_Engine (Geod Studio, GPL v3) » (DEC-10-Q3) ;
- en dessous, un grand bouton icône **Quitter** (référence 64×64), qui termine le programme proprement : la persistance se fait selon SPEC-09.

L'original affichait trois lignes (`3dSen`, `Version 1.1.2` et le nom de son studio) ; le contenu ci-dessus le remplace (DEC-10-Q3).
- Critère d'acceptation : le bloc affiche « Sen », la version, « GPL v3 » et les deux crédits ; cliquer sur Quitter ferme le programme, et les réglages modifiés juste avant sont conservés au relancement.

### 3.10 Notifications (E11)

**REQ-UI-080** (P1) — Il DOIT exister une zone de notification unique : un bandeau d'une ligne centré en bas de l'écran, avec un fond gris translucide ajusté au texte et un texte en italique centré. Elle est visible en jeu comme dans le menu. Une notification a un texte, une durée (4 s par défaut) et une gravité :

| Gravité | Couleur du texte |
|---|---|
| normale | blanc |
| avertissement | jaune |
| erreur | rouge |

La durée se mesure en temps réel : elle n'est affectée ni par la pause ni par la vitesse de jeu.
- Critère d'acceptation : une notification de 4 s disparaît entre 3,9 et 4,2 s après son apparition, même si le jeu est mis en pause entre-temps.

**REQ-UI-081** (P1) — Les notifications NE DOIVENT PAS s'empiler. Une nouvelle notification **remplace** celle affichée, en redémarrant son minuteur, si aucune n'est visible ou si sa gravité est supérieure ou égale à celle affichée. Dans le cas contraire, elle est ignorée.
- Critère d'acceptation : pendant une erreur rouge, une notification normale `Quick Save` n'apparaît pas. Après la disparition de l'erreur, une nouvelle `Quick Save` apparaît.

**REQ-UI-082** (P1) — Les raccourcis en jeu (SPEC-07, SPEC-08) DOIVENT produire les notifications suivantes, toutes de gravité normale :

| Événement | Texte exact | Durée |
|---|---|---|
| appui sur RollBack | `Roll Back` | 1 s, renouvelée à chaque appui |
| FastForward maintenu | `Fast Forward` | 1 s, renouvelée tant que le bouton est maintenu |
| quicksave effectuée (QuickSave, ou Select maintenu plus de 0,1 s puis relâchement de RotDown), avec un jeu courant | `Quick Save` | 4 s |
| quickload effectué (QuickLoad, ou Select maintenu puis relâchement de RotUp), avec un slot rapide non vide | `Quick Load` | 4 s |

Sans jeu courant, la quicksave est sans effet et n'affiche rien. Avec un slot rapide vide, le quickload est sans effet et n'affiche rien.
- Critère d'acceptation : en jeu, une quicksave affiche `Quick Save` pendant 4 s. Un quickload avant toute quicksave n'affiche rien.

**REQ-UI-083** (P1) — Les échecs de chargement ou de lecture (SPEC-07, SPEC-03) DOIVENT être signalés par une notification de gravité **erreur**, de 4 s, dont le texte est le message d'échec. Le menu reste utilisable. Les textes observables de l'original sont les suivants ; ils DEVRAIENT être conservés :
- `Corrupted zip file`
- `Uncompression is not successful`
- `Invalid rom file.`
- `The game profile doesn't exist anymore.`
- `Save state used a different core.`
- Critère d'acceptation : choisir comme ROM un fichier `.nes` de 10 octets aléatoires affiche `Invalid rom file.` en rouge.

**REQ-UI-084** (P2) — Les échecs d'import de profil DOIVENT être signalés par une notification de gravité **normale**, de 4 s. Textes de l'original, qui DEVRAIENT être conservés :
- `This file is not a 3dn file`
- `3dn format version <v>  is not supported` (avec deux espaces avant « is »)
- `<chemin>has an obsolate or wrong profile format. This profile is not compatible with the current version of 3dSen.` (sans espace après le chemin)
- Critère d'acceptation : importer un fichier non protobuf affiche une notification blanche.

**REQ-UI-085** (P2) — Les scripts de profil peuvent émettre des notifications avec un texte, une durée et une gravité. Cette API est définie par SPEC-05. Ces notifications suivent les mêmes règles d'affichage (REQ-UI-080, REQ-UI-081).
- Critère d'acceptation : une notification émise par un script s'affiche avec la couleur de sa gravité.

### 3.11 Navigateur de fichiers (E7)

**REQ-UI-100** (P1) — Le navigateur DOIT être une fenêtre modale centrée, posée par-dessus le menu. Il contient :
- un **titre** en haut à gauche, fourni par l'appelant (`Select ROM`, `Select external 3D profile`, `Select external cover art`) ;
- un **champ de recherche** en haut à droite, avec le texte indicatif `Search...` et un bouton d'effacement en forme de croix ;
- à gauche, une **colonne de chemin** : un bouton par niveau du dossier courant ;
- à droite, la **liste des éléments** du dossier courant ;
- en bas, un **champ du nom de fichier**, en lecture seule en mode ouverture ;
- en bas à gauche, un bouton `Cancel` ;
- en bas à droite, un bouton de validation `Open`.

Tant qu'il est ouvert, la navigation par onglets et par pages du menu est suspendue.
- Critère d'acceptation : à l'ouverture depuis « Import », le titre est `Select external 3D profile`.

**REQ-UI-101** (P1) — La colonne de chemin DOIT commencer par un bouton racine, puis afficher un bouton par composant du chemin courant, chacun terminé par le séparateur (par exemple `C:\`, `Users\`, `Me\`). Le bouton racine s'appelle `My Computer` sous Windows ; sous Linux et macOS, c'est le séparateur `/`. Cliquer sur un bouton va à ce dossier. À la racine Windows, la liste montre les lecteurs logiques (`C:\`, `D:\`, …) sous forme de dossiers.
- Critère d'acceptation : dans `C:\Users\Me`, cliquer sur `Users\` affiche le contenu de `C:\Users`.

**REQ-UI-102** (P1) — La liste DOIT afficher d'abord les **sous-dossiers**, avec une icône de dossier, puis les **fichiers** dont l'extension (la partie après le dernier point, sans tenir compte de la casse) figure dans le filtre de l'appelant, avec une icône de fichier. Les autres fichiers sont masqués. Les dossiers et fichiers illisibles, par exemple faute de droits, sont omis sans message. L'ordre dans chaque groupe est l'ordre alphabétique du nom, sans tenir compte de la casse.
- Critère d'acceptation : avec le filtre `nes|zip|7z`, un dossier contenant `a.NES`, `b.zip`, `c.sav` et un sous-dossier `x` affiche `x`, `a.NES` et `b.zip`.

**REQ-UI-103** (P2) — Le texte de recherche DOIT filtrer les **fichiers** dont le nom contient ce texte, sans tenir compte de la casse et en ignorant les espaces de début et de fin. Les dossiers restent toujours affichés. Le bouton d'effacement vide la recherche. La recherche se remet à zéro à chaque ouverture du navigateur.
- Critère d'acceptation : la recherche « MARIO » conserve `super mario.nes` et masque `zelda.nes`.

**REQ-UI-104** (P1) — Interaction avec la liste :
- un clic sélectionne un élément (fond gris) ; s'il s'agit d'un fichier, son nom apparaît dans le champ du nom de fichier, sinon le champ se vide ;
- un clic sur l'élément déjà sélectionné, ou un double-clic, l'**ouvre** : un dossier devient le dossier courant, un fichier équivaut à `Open`.

`Open` n'agit que si un fichier est sélectionné : il ferme le navigateur et transmet le chemin complet à l'appelant. `Cancel` et le bouton Menu ferment le navigateur sans rien transmettre ; l'appelant revient à son état précédent.
- Critère d'acceptation : cliquer sur `Open` avec un dossier sélectionné ne fait rien. Double-cliquer sur un fichier `.nes` lance le jeu (REQ-UI-031).

**REQ-UI-105** (P2) — Le dossier courant à la fermeture DOIT être mémorisé de façon persistante (SPEC-09) et servir de dossier initial à la prochaine ouverture, quel que soit l'appelant et y compris après un redémarrage. Si ce dossier n'existe plus, le navigateur s'ouvre à la racine. Il n'y a pas de liste de dossiers récents.
- Critère d'acceptation : choisir une ROM dans `D:\roms`, relancer le programme, puis ouvrir le choix de couverture : il s'ouvre dans `D:\roms`.

**REQ-UI-106** (P3) — Dans le navigateur, Haut et Bas DEVRAIENT faire défiler la liste d'une page.
- Critère d'acceptation : avec 100 fichiers, Bas fait défiler la liste.

### 3.12 Boîte de dialogue (E8)

**REQ-UI-110** (P1) — La boîte de dialogue DOIT être modale et centrée. Elle comporte un titre, un message et une rangée de boutons. Le premier bouton a le focus par défaut. Tant qu'elle est ouverte, rien d'autre n'est cliquable et le bouton Menu est sans effet. Chaque bouton exécute son action puis ferme la boîte. Il n'y a pas de fermeture par croix : on la quitte uniquement par ses boutons. Le fond n'est pas assombri.
- Critère d'acceptation : dialogue `Reset` / `Resume` ouvert, cliquer sur un onglet ne fait rien.

### 3.13 Sélecteur de couleur (E9)

**REQ-UI-090** (P2) — Le sélecteur DOIT permettre de choisir la couleur personnalisée du fond avec :
- une palette 2D accompagnée d'un curseur vertical ;
- trois curseurs `R:`, `G:`, `B:` de 0 à 255, chacun avec un champ numérique ;
- un champ hexadécimal ;
- un bouton `OK` qui ferme le sélecteur.

Chaque modification est appliquée immédiatement (SPEC-06) et met à jour la couleur du bouton de REQ-UI-063. Cliquer hors du sélecteur le ferme. Un sélecteur de couleur ImGui standard qui offre ces fonctions est acceptable.
- Critère d'acceptation : saisir `FF0000` rend le fond rouge immédiatement ; `OK` ferme.

### 3.14 Barre de progression (E10)

**REQ-UI-091** (P2) — Pendant un chargement de jeu ou de slot, une barre de progression horizontale DOIT s'afficher en haut du menu (référence 412×32). Son remplissage est proportionnel à l'avancement (SPEC-07) et elle porte le texte `<n> %`, entier et en gras. Elle disparaît à la fin du chargement.
- Critère d'acceptation : en lançant un jeu, la barre apparaît, progresse de manière monotone, puis disparaît quand le menu se cache.

### 3.15 Compteur FPS (E12)

**REQ-UI-095** (P3) — L'original n'affiche pas de compteur FPS sur desktop. L'implémentation PEUT proposer un petit panneau déplaçable, titré `FPS` et **désactivé par défaut**, qui affiche la fréquence d'images moyenne avec une décimale et se met à jour toutes les 0,5 s.
- Critère d'acceptation : par défaut, aucun compteur n'est visible.

### 3.16 Style visuel

**REQ-UI-120** (P2) — L'apparence DEVRAIT reprendre la palette suivante. Les couleurs sont données en RGBA de 0 à 1. Elle se transpose en styles ImGui.

| Élément | Couleur |
|---|---|
| Panneaux du menu (paramètres, entrées, aide, à propos) | gris 0,50 ; α 0,50 |
| Onglet inactif | fond 0,19 ; α 1 |
| Onglet actif | fond 0,50 ; α 1 |
| Libellés d'onglet | blanc, gras, 18 px |
| Listes déroulantes | gris 0,50 ; α 0,78 ; texte blanc 13 px |
| Fond de case à cocher et rail de curseur | gris 0,72 ; α 0,50 |
| Coche et remplissage de curseur | blanc ; α 0,77 |
| Libellés de paramètres | blanc, 14 px, alignés à gauche |
| Indicateur de pages | inactif 0,19 ; page courante 0,50 ; chiffres blancs 16 px |
| Titres de tuiles | blanc 13 px |
| Badge `Slot n` | fond gris 0,47 ; α 0,50 ; texte italique 12 px |
| Info-bulles | fond gris 0,50 ; α 0,50 ; texte blanc italique 14 px |
| Bandeau de notification | fond gris 0,37 ; α 0,50 ; texte italique 22 px (réductible pour tenir sur une ligne) |
| Navigateur de fichiers (thème clair) | fenêtre 0,90 ; α 0,86 ; zones de liste 0,77 ; α 0,77 ; texte sombre 0,20 ; titre gris 0,51 gras 25 px ; textes indicatifs 0,20 ; α 0,50 en italique |
| Barre de progression | fond noir, remplissage gris 0,50, texte gras 20 px |
| Texte d'aide | blanc 20 px, centré |

- Critère d'acceptation : revue visuelle (catégorie de vérification « revue » de SPEC-12, justifiée pour REQ-ACC-050). Le menu est sombre et translucide, avec du texte blanc ; le navigateur de fichiers est clair, avec du texte sombre ; les couleurs mesurées sur une capture des panneaux, onglets et bandeau de notification correspondent au tableau à ±0,02 près (avant mélange alpha).

**REQ-UI-121** (P2) — Polices :
- `Roboto-Regular` (assets neutres) pour l'essentiel des textes ;
- `LiberationSans` pour les titres d'onglets, le bloc à propos et l'aide, en remplacement d'Arial.

Les icônes DEVRAIENT être prises dans les atlas neutres : dossier, fichier et croix d'effacement dans l'atlas du navigateur ; cartouche, pointeur, logos de remplacement « 2D SEN » et « 3D SEN » dans l'atlas d'UI . Les textures `button_texture`, `listview_background`, `dialog_border`, `tooltip`, `round_square`, `input_input` et `input_input_selected` PEUVENT habiller les boutons, listes, dialogues, info-bulles et champs.
- Critère d'acceptation : le programme n'utilise aucune police ni image extérieure aux assets neutres et aux ressources propres de l'implémentation.

**REQ-UI-122** (P3) — Disposition de référence, sur une surface logique de 800×450 (REQ-UI-130) :
- le cadre du menu est centré et occupe environ 64 % de la largeur et 93 % de la hauteur de la surface de référence ;
- les onglets sont en colonne à gauche ;
- `Sort by` est en haut à droite ;
- la grille occupe le centre (environ 96 % de la largeur du cadre), avec l'indicateur de pages en dessous ;
- la vignette du jeu courant est accrochée au milieu du bord droit et déborde légèrement du cadre ;
- le bouton Import (environ 3 % de la hauteur du cadre, carré) est au-dessus de la grille, à gauche.

Cette disposition est indicative.
- Critère d'acceptation : revue visuelle (catégorie « revue » de SPEC-12, justifiée pour REQ-ACC-050). Aucun élément ne se chevauche à 1280×720.

### 3.17 Échelle et résolution

**REQ-UI-130** (P2) — L'UI DOIT être conçue pour une surface logique de référence de 800×450 (16:9). Elle est mise à l'échelle **uniformément** selon la taille de la fenêtre, et suit la hauteur quand le rapport d'aspect diffère. Elle reste lisible et entièrement visible pour toute fenêtre d'au moins 800×450. L'original étirait l'UI horizontalement quand le rapport d'aspect n'était pas 16:9 ; ce comportement NE DEVRAIT PAS être reproduit.
- Critère d'acceptation : en 1920×1080 comme en 1024×768, tous les contrôles sont visibles et non déformés.

**REQ-UI-131** (P3) — L'implémentation PEUT offrir un multiplicateur d'échelle d'UI (par exemple de 75 % à 200 %) dans les réglages, absent de l'original. S'il existe, c'est un réglage persistant ajouté à SPEC-09.
- Critère d'acceptation : à 150 %, les textes sont 1,5 fois plus grands.

**REQ-UI-132** (P2) — Le redimensionnement de la fenêtre et les passages entre fenêtre et plein écran DOIVENT être pris en compte à l'image suivante, sans redémarrage et sans perte d'état de l'UI (onglet, page, sélection).
- Critère d'acceptation : redimensionner la fenêtre pendant que l'onglet LOAD est sur la page 2 conserve la page 2.

### 3.18 Sons d'UI

**REQ-UI-140** (P2) — L'UI DOIT déclencher les sons de SPEC-11 :

| Son | Déclencheurs |
|---|---|
| « sélection d'onglet » | changement d'onglet |
| « changement de page » | changement de page d'une grille |
| « survol / sélection d'élément » | sélection d'une tuile |
| « validation » | activation d'une tuile ; bouton Import, import de couverture et suppression de profil ; chargement ou sauvegarde de slot ; définition d'une couverture ; modification d'un réglage (REQ-UI-065) |

- Critère d'acceptation : activer un slot dans LOAD émet le son « validation ».

## 4. Formats et interfaces

L'UI ne définit aucun format de fichier. Les données qu'elle lit ou écrit appartiennent aux autres modules :
- réglages, tri, dernier dossier du navigateur, couvertures, chemins de ROM et dates de lancement : SPEC-09 ;
- slots : SPEC-07 et SPEC-09 ;
- catalogue : SPEC-03 ;
- bindings : SPEC-08.

Toutes les opérations longues (chargement de jeu, de slot, de profil) passent par la boucle de SPEC-07. L'UI n'en observe que l'état (arrêté, en pause, en cours, en chargement) et l'avancement, compris entre 0 et 1.

**Filtres d'extensions du navigateur :**

| Appelant | Titre | Filtre |
|---|---|---|
| Choix de ROM | `Select ROM` | `nes`, `zip`, `7z` |
| Import de profil | `Select external 3D profile` | `3dn` |
| Import de couverture | `Select external cover art` | `jpg`, `png` |

## 5. Vecteurs de test

**V1 — Placement dans la grille (REQ-UI-020)**

| Index n (ordre de tri) | Page | Ligne | Colonne |
|---|---|---|---|
| 0 | 0 | 0 | 0 |
| 3 | 0 | 0 | 3 |
| 4 | 0 | 1 | 0 |
| 11 | 0 | 2 | 3 |
| 12 | 1 | 0 | 0 |
| 17 | 1 | 1 | 1 |

Avec 13 éléments, il y a 2 pages ; la page 2 contient une tuile visible et 11 emplacements vides.

**V2 — Conversions des curseurs (REQ-UI-061)**

| Curseur | Position | Valeur obtenue |
|---|---|---|
| Light | 0 | intensité 0,5 |
| Light | 100 | intensité 1,5 |
| Vol | 37 | volume 0,37 |
| FOV | −5 | 45° |
| FOV | 0 | 70° |
| FOV | +5 | 95° |
| Speed | −8 | −8 |
| Speed | +8 | +8 |

Les curseurs sont entiers : une position fractionnaire est impossible.

**V3 — Tri de la bibliothèque (REQ-UI-014, REQ-UI-031)**

Entrées : A (« Zelda », lancé à t=100), B (« Contra », jamais lancé, t=0), C (« Mario », t=200).
- `A-Z` → Contra, Mario, Zelda.
- `Recent` → Mario, Zelda, Contra.
- À égalité de date, le tri `Recent` départage par titre : D (« Arkanoid », t=0) se place avant Contra.

**V4 — Grille SAVE (REQ-UI-051)**

| Slots occupés | Première tuile de SAVE | Tuiles suivantes |
|---|---|---|
| {1, 2, 5} | Slot 3 (vide) | slots 1, 2, 5, triés |
| {1…36} | — (aucune tuile vide) | les 36 slots |

**V5 — Notifications (REQ-UI-081)**

| t | Événement | Résultat |
|---|---|---|
| 0 s | erreur « X » (4 s) | « X » affichée |
| 1 s | `Quick Save` | ignorée |
| 4,1 s | — | rien n'est affiché |
| 4,2 s | `Roll Back` (1 s) | affichée jusqu'à 5,2 s |

**V6 — Bouton Menu (REQ-UI-004, REQ-UI-005)**

Appui de 0,1 s en jeu → pause et menu. Appui de 0,6 s en jeu → recentrage de la caméra, le jeu continue.

## 6. Questions ouvertes

- **Q-1** — *Close par DEC-10-Q1* (export OBJ exclu). Export de la vue en OBJ (bouton de la vignette et touche F3) : il est exclu ici, conformément au PLAN (export de frames hors périmètre). À confirmer.
- **Q-2** — *Close par DEC-10-Q2* (Windows seul, une seule méthode de rendu, pas de `Tex Scale`). Sur Linux et macOS, l'original affichait le choix de méthode de rendu (`Direct` / `Render Tex`) et `Tex Scale`. Faut-il les exposer sur ces plateformes ? Cela dépend de SPEC-06.
- **Q-3** — *Close par DEC-10-Q3* (REQ-UI-076). Contenu du bloc « à propos » : faut-il reprendre l'identité d'origine (`3dSen`, `Version 1.1.2`, studio) ou celle de la réimplémentation ?
- **Q-4** — *Close par DEC-10-Q4* (option P3, raccourcis inchangés). Navigation dans une grille au clavier ou à la manette : dans l'original, Haut et Bas changent d'onglet et Gauche et Droite changent de page ; la sélection d'une tuile précise se fait à la souris. Faut-il ajouter une navigation tuile par tuile, par exemple avec Tab et A/Entrée pour activer ? Ce serait un choix d'ergonomie, en conflit potentiel avec REQ-UI-012 et REQ-UI-021.
- **Q-5** — *Close par DEC-10-Q5* : bouton Site retiré (D).
- **Q-6** — *Close par DEC-10-Q6* (GAME, D). Onglet initial `SETTING` au démarrage : c'est fidèle à l'original, mais GAME serait plus naturel. À trancher par le produit.
- **Q-7** — *Close par DEC-10-Q7* (option, désactivée par défaut). Compteur FPS (REQ-UI-095) : faut-il le garder comme option de débogage, ou le supprimer ?

## Historique

- 1.1-draft (AMD-001, relecture R2) : onglet initial GAME (DEC-10-Q6) ; bouton Site retiré (DEC-10-Q5) dans REQ-UI-039, 121, 122, 140 ; bloc « à propos » de la reconstruction (DEC-10-Q3) ; dimensions de l'UI exprimées en proportions (LEAK 10-1) ; clause de fermeture de REQ-UI-002 assouplie ; Graphic conservé dans le modèle (REQ-UI-062) ; critères de revue visuelle qualifiés ; questions closes.
