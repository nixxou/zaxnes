# SPEC-08 — Entrées : clavier, manettes, souris, Zapper, hotkeys
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-00 (glossaire), SPEC-02 (contrat du cœur : octet de manette, Zapper), SPEC-05 (Lua : `InputProcess`, objets exposés), SPEC-06 (caméra), SPEC-07 (boucle : pause, rollback, avance rapide, quicksave), SPEC-09 (stockage des bindings et réglages), SPEC-10 (écran de configuration des entrées)

## 1. Objet et périmètre

Ce document fixe le comportement observable de la couche d'entrées du programme desktop :
- l'ensemble des **actions logiques** (boutons NES, actions caméra, actions système) et leur codage ;
- les **périphériques** pris en charge (clavier, manettes, souris) et leur attribution aux joueurs ;
- les **bindings par défaut**, le **remappage** et sa persistance logique ;
- l'émulation du **Zapper** (pistolet optique) à la souris ;
- l'injection d'entrées par les **scripts de profil** ;
- les règles de **masquage du curseur** et le routage des entrées quand un menu est ouvert ;
- l'**abstraction** exigée pour un futur frontend libretro.

Hors périmètre : VR (manettes VR, pointeur laser, regard, Zapper VR, « méthode de contrôle » VR), écrans tactiles et pavé virtuel mobile, éditeur de profils (sélection de tiles à la souris, raccourcis de l'éditeur), enregistrement et relecture de films d'entrées, export de scène.

## 2. Concepts et vocabulaire

- **Joueur** : l'un des deux joueurs logiques, J1 et J2. Il n'y a pas de joueur 3 ou 4.
- **Port** : l'un des deux ports manette de la NES émulée (port 1, port 2). Le port N reçoit les entrées du joueur N.
- **Type de port** : `Manette` (valeur 0) ou `Zapper` (valeur 1). Valeurs visibles des scripts Lua (voir SPEC-05).
- **Action** : une entrée logique nommée (par exemple `A`, `RotLeft`, `Menu`). Il y a exactement 19 actions (§4.1).
- **Masque d'actions** : entier dont chaque bit représente une action enfoncée (§4.1). Les 8 bits de poids faible forment l'**octet de manette NES**.
- **Binding** : association entre une action et une commande physique (touche, bouton de manette, demi-axe d'un stick ou d'une gâchette).
- **Jeu de bindings** : l'ensemble des bindings d'un type de périphérique pour un joueur. Chaque joueur a un jeu « clavier » et un jeu « manette ».
- **Identité de périphérique** : chaîne stable qui identifie un modèle ou une instance de manette (nom et pilote). Elle sert de clé de persistance.
- **Front montant / descendant** : passage de relâché à enfoncé, et l'inverse, entre deux échantillonnages successifs.
- **Appui court / long** : appui dont la durée, mesurée au relâchement, est inférieure ou supérieure à 0,3 s.
- **Jeu en cours** : l'émulation tourne (non en pause, non arrêtée, non en chargement). Le glossaire de SPEC-00 fait foi pour ces états.

## 3. Exigences

### 3.1 Actions logiques et masque d'entrée

**REQ-INP-001** (P1) — Le programme DOIT gérer exactement deux joueurs (J1, J2). Chaque joueur dispose des 19 actions de la table §4.1.
- Critère d'acceptation : l'écran de configuration propose J1 et J2, et 19 actions pour chacun ; il n'existe aucun J3.

**REQ-INP-002** (P1) — À chaque échantillonnage, le masque d'actions d'un joueur DOIT être le OU bit à bit des contributions de toutes ses sources : son jeu clavier, son jeu manette (si une manette lui est attribuée) et, le cas échéant, le pad de script (§3.8). Le codage des bits est celui de la table §4.1.
- Critère d'acceptation : J1 maintient la flèche droite au clavier et le bouton sud de sa manette → masque J1 = `0x81` (Right + A). Relâcher l'un laisse l'autre bit actif.
- Notes : aucune source ne peut « retirer » un bit posé par une autre.

**REQ-INP-003** (P1) — Les valeurs des bits de la table §4.1 sont un fait d'interopérabilité (elles sont lues et produites par les scripts Lua des profils) et DOIVENT être respectées à l'identique.
- Critère d'acceptation : un script qui lit l'état de J1 pendant que seul Bas est enfoncé obtient 32 ; un script `InputProcess` qui renvoie 8 produit un appui sur Start.

**REQ-INP-004** (P1) — Pour chaque image émulée, l'octet transmis au cœur pour un port DOIT être :
- si le type du port est `Manette` : les 8 bits de poids faible du masque du joueur correspondant ;
- si le type du port est `Zapper` : l’état du Zapper (§3.7), sans aucun bouton de manette.
Le port 1 occupe l'octet de poids faible et le port 2 l'octet suivant de l'entrée de l'image (contrat SPEC-02).
- Critère d'acceptation : port 2 en `Zapper`, J2 maintient A au clavier → le cœur ne voit aucun bouton A sur le port 2 ; port 2 en `Manette` → il voit A.

**REQ-INP-005** (P1) — Les entrées DOIVENT être échantillonnées une fois avant chaque image émulée, y compris pendant l'avance rapide (une image émulée = un échantillon, même si plusieurs images sont émulées dans le même intervalle d'affichage).
- Critère d'acceptation : en avance rapide, un appui maintenu pendant exactement N images émulées est vu par le cœur pendant N images (±1).

**REQ-INP-006** (P2) — Les bits supérieurs à `0x80` (caméra et système) ne sont jamais transmis au cœur. Ils sont consommés par la caméra (§3.9, §3.10) et par les hotkeys (§3.11).
- Critère d'acceptation : maintenir `RollBack` ne modifie pas l'octet du port.

**REQ-INP-007** (P3) — Il n'existe pas de bouton turbo ni de répétition automatique. Une implémentation PEUT en ajouter plus tard, à condition de ne modifier ni les valeurs de la table §4.1 ni le comportement par défaut.
- Critère d'acceptation : aucune action « turbo » dans la liste par défaut.

### 3.2 Clavier

**REQ-INP-010** (P1) — Le clavier physique alimente le jeu clavier de **chacun** des deux joueurs : une même touche peut être liée à une action de J1 et à une action de J2.
- Critère d'acceptation : lier `W` à Haut pour J2 ; `W` produit Haut sur J2 sans toucher J1.

**REQ-INP-011** (P1) — Le jeu clavier par défaut de J1 DOIT être celui de la table §4.2. Le jeu clavier par défaut de J2 DOIT être **vide** (aucune touche).
- Critère d'acceptation : premier lancement, sans fichier de configuration → les flèches pilotent J1 ; aucune touche ne pilote J2.

**REQ-INP-012** (P2) — Les touches modificatrices (Maj, Ctrl, Alt, gauche et droite séparément) DOIVENT pouvoir être liées comme des touches ordinaires. Les boutons de souris ne DOIVENT PAS pouvoir être liés à une action.
- Critère d'acceptation : lier `Maj gauche` à B → Maj gauche seule produit B. Cliquer pendant l'écoute d'un binding clavier ne crée aucun binding.

### 3.3 Manettes : prise en charge, attribution, branchement à chaud

**REQ-INP-020** (P1) — Les manettes XInput et les manettes reconnues par une table de correspondance standard (disposition de type Xbox : quatre boutons de face sud/est/ouest/nord, Back/View, Start/Menu, croix, deux sticks cliquables, deux bumpers, deux gâchettes analogiques) DOIVENT être prises en charge avec les bindings par défaut de la table §4.3.
- Critère d'acceptation : une manette Xbox branchée et attribuée à J1 pilote immédiatement le port 1 : en harnais, la croix produit les bits de direction, A (sud) le bit A et X (ouest) le bit B du mot d'entrées.

**REQ-INP-021** (P2) — Les manettes non reconnues (joysticks DirectInput génériques sans correspondance standard) DEVRAIENT être utilisables : elles apparaissent dans la liste des périphériques, leurs boutons et axes bruts peuvent être liés par remappage. Elles n'ont pas de bindings par défaut utiles.
- Critère d'acceptation : un pad générique attribué à J1 ne fait rien tant qu'on ne l'a pas remappé ; après remappage de ses boutons bruts, il pilote J1.

**REQ-INP-022** (P2) — Les périphériques de réalité virtuelle (dont l'identité contient « Oculus », « Vive », « Knuckles » ou « Spatial ») NE DOIVENT PAS être proposés ni attribués automatiquement.
- Critère d'acceptation : avec un casque VR branché, ses contrôleurs n'apparaissent pas dans la liste de choix de manette.

**REQ-INP-023** (P1) — Chaque joueur a au plus **une** manette attribuée (ou aucune). Attribution :
1. **Au démarrage**, environ 0,5 s après le lancement, J1 reprend la manette dont l'identité est égale à la dernière manette mémorisée pour J1, si elle est branchée ; puis J2 fait de même avec sa dernière manette mémorisée, en excluant celle déjà prise par J1.
2. **Attribution automatique** : quand une manette non VR qui n'est attribuée à aucun joueur devient active (l'utilisateur appuie sur l'un de ses boutons ou bouge l'un de ses axes) et qu'au moins un joueur n'a pas de manette, elle est attribuée à J1 si J1 n'en a pas, sinon à J2.
3. **Attribution manuelle** depuis l'écran de configuration (REQ-INP-041).
- Critère d'acceptation : aucun fichier de configuration, deux manettes branchées ; appuyer sur un bouton de la manette M2 puis sur un bouton de M1 → M2 pilote J1, M1 pilote J2. Relancer le programme → même attribution.

**REQ-INP-024** (P1) — **Débranchement** : quand une manette attribuée est débranchée, le joueur concerné n'a plus de manette ; ses bits issus de la manette retombent à zéro dès l'échantillon suivant. **Branchement** : une manette nouvellement branchée n'est pas attribuée tant qu'elle n'est pas utilisée (règle 2 de REQ-INP-023) ou choisie manuellement.
- Critère d'acceptation : débrancher la manette de J1 pendant qu'il maintient Droite → le bit Droite du port 1 retombe à 0 dès l'échantillon suivant. La rebrancher et appuyer sur un bouton → elle redevient celle de J1 (s'il n'en avait pas d'autre).

**REQ-INP-025** (P1) — Quand une manette est attribuée à un joueur (quel que soit le moyen), son jeu de bindings manette DOIT être chargé depuis la configuration mémorisée pour **son identité de périphérique**. Si aucune configuration n'existe pour cette identité, les bindings par défaut (§4.3) sont appliqués et immédiatement mémorisés pour cette identité.
- Critère d'acceptation : remapper le bouton nord de la manette M (modèle X) sur `QuickSave`, la débrancher, la rebrancher → le remappage est toujours là. Une manette d'un autre modèle Y garde ses propres bindings.
- Notes : les bindings manette sont donc associés au **périphérique**, pas au joueur. Si la même manette passe de J1 à J2, ses bindings la suivent.

**REQ-INP-026** (P2) — L'identité affichée d'une manette DEVRAIT être son nom lisible lorsqu'elle est reconnue, sinon l'identité brute fournie par le système.
- Critère d'acceptation : une manette Xbox s'affiche sous un nom du type « XInput Controller » ou « Xbox Controller » ; un pad générique s'affiche avec sa chaîne système.

### 3.4 Sticks analogiques, gâchettes et zones mortes

**REQ-INP-030** (P1) — Toute commande analogique liée à une action (demi-axe de stick, gâchette) DOIT être convertie en état numérique : l'action est enfoncée si la valeur normalisée (0 à 1 sur le demi-axe, après application de la zone morte du périphérique) est **supérieure ou égale à 0,34**, relâchée sinon.
- Critère d'acceptation : stick droit poussé à 30 % vers la gauche → `RotLeft` relâché ; à 40 % → enfoncé. Gâchette gauche enfoncée à moitié → `ZoomIn` enfoncé.
- Notes : 0,34 est le seuil d'origine. Il correspond à peu près à un tiers de course, ce qui évite les déclenchements involontaires d'un stick au repos sans exiger une course complète.

**REQ-INP-031** (P2) — Les sticks DEVRAIENT avoir une zone morte radiale de 0,2 (en dessous, valeur nulle) et une saturation à 0,9 (au-dessus, valeur 1), la plage intermédiaire étant ré-étalée linéairement entre 0 et 1. Les gâchettes DEVRAIENT avoir une zone morte de 0,1.
- Critère d'acceptation : stick au repos avec une dérive de 0,15 → aucune action. Stick à 0,9 → valeur 1.
- Notes : valeurs par défaut usuelles des bibliothèques de manettes ; l'original ne les rend pas réglables.

**REQ-INP-032** (P2) — Par défaut, le **stick gauche n'est lié à aucune action** : la croix seule pilote les directions NES. Le stick droit pilote la rotation de la caméra et les gâchettes le zoom (§4.3). L'utilisateur PEUT lier les demi-axes du stick gauche aux directions par remappage.
- Critère d'acceptation : manette par défaut, stick gauche poussé à fond → aucun bit de direction NES ni aucune action caméra n'est produit.

**REQ-INP-033** (P2) — Les actions caméra issues d'un stick ou d'une gâchette sont **numériques** (vitesse constante une fois le seuil atteint, §3.9) ; l'amplitude analogique au-delà du seuil n'a pas d'effet.
- Critère d'acceptation : stick droit à 50 % et à 100 % vers la droite → même vitesse de rotation.

### 3.5 Remappage

**REQ-INP-040** (P1) — L'écran de configuration des entrées (contenu fixé par SPEC-10) DOIT permettre, pour un joueur (J1/J2) et un type de jeu de bindings (`Clavier` / `Manette`) choisis, de voir et de modifier le binding de chacune des 19 actions, présentées dans l'ordre de la table §4.1 avec leur nom anglais tel qu'il figure dans cette table.
- Critère d'acceptation : J1/Clavier affiche `Left ← Flèche gauche`, `A ← X`, …, `QuickSave` vide, `QuickLoad` vide.

**REQ-INP-041** (P1) — En mode `Manette`, l'écran DOIT proposer une liste « Manette » contenant « aucune » puis toutes les manettes non VR branchées (identité affichée selon REQ-INP-026). Choisir une entrée attribue cette manette au joueur sélectionné (REQ-INP-025 s'applique) ; choisir « aucune » lui retire sa manette. La liste DOIT se mettre à jour au branchement et au débranchement. La sélection affichée reflète la manette actuellement attribuée.
- Critère d'acceptation : brancher une manette pendant que l'écran est ouvert → elle apparaît dans la liste sans rouvrir l'écran.
- Notes : l'original n'empêche pas d'attribuer manuellement la même manette aux deux joueurs ; l'implémentation DOIT alors la retirer de l'autre joueur (divergence acceptée **(D)**, DEC-08-1).

**REQ-INP-042** (P1) — Capture d'un binding : activer le champ d'une action **efface immédiatement** le binding existant de cette action pour ce jeu, puis met le programme en écoute. La première commande physique éligible détectée devient le nouveau binding et l'écoute s'arrête. Éligibilité :
- jeu `Clavier` : touches du clavier uniquement (modificatrices comprises) ;
- jeu `Manette` : boutons, croix, demi-axes de sticks et gâchettes de la manette attribuée au joueur, contrôles non standard compris ; aucune touche, aucun bouton de souris.
- Critère d'acceptation : activer `B` (J1, Clavier), appuyer sur `C` → `B ← C`. Activer `A` (J1, Manette) et pousser le stick gauche vers le haut → `A ← stick gauche haut`.

**REQ-INP-043** (P1) — Chaque action a **au plus un binding par type** (un pour le clavier, un pour la manette). Capturer un nouveau binding remplace l'ancien.
- Critère d'acceptation : après deux captures successives sur `A`, seule la seconde touche produit A.

**REQ-INP-044** (P2) — **Pas de détection de conflit** : une même commande physique PEUT être liée à plusieurs actions d'un même jeu ; elle les déclenche alors toutes. Lier une commande ne retire pas cette commande des autres actions.
- Critère d'acceptation : lier `X` à `A` et à `B` → `X` produit `0x03`.

**REQ-INP-045** (P2) — L'écoute s'arrête sans créer de binding (l'action reste alors **vide**, puisqu'elle a été effacée par REQ-INP-042) quand : l'utilisateur clique sur le bouton « arrêter l'écoute » (visible seulement pendant une écoute), change de joueur, de type ou de manette, active une autre action (qui démarre sa propre écoute), ou ferme l'écran.
- Critère d'acceptation : activer `Start`, cliquer sur « arrêter » → `Start` est vide.

**REQ-INP-046** (P2) — En mode `Manette`, si le joueur n'a pas de manette attribuée, les 19 champs sont vides et l'activation d'un champ ne démarre pas d'écoute.
- Critère d'acceptation : J2 sans manette, mode Manette → champs vides, aucun effet au clic.

**REQ-INP-047** (P2) — Pendant une écoute, la navigation de l'interface par les actions directionnelles (changement d'onglet, §3.11) DOIT être suspendue. La commande capturée NE DEVRAIT déclencher aucune autre action pendant l'échantillon où elle est capturée.
- Critère d'acceptation : capturer `Flèche droite` sur une action → l'onglet courant ne change pas.

**REQ-INP-048** (P2) — **Portée** : les bindings sont **globaux** (communs à tous les jeux). Il n'existe pas de bindings par jeu. Seul le **type de port** peut varier par jeu, à la demande du script du profil (REQ-INP-060).
- Critère d'acceptation : un remappage fait dans un jeu reste actif dans un autre jeu.

**REQ-INP-049** (P3) — Il n'existe pas de commande « réinitialiser les bindings » dans l'original. Une implémentation DEVRAIT en proposer une par jeu de bindings (joueur × type) qui rétablit la table §4.2 (J1), un jeu vide (J2 clavier) ou la table §4.3 (manette attribuée, pour son identité).
- Critère d'acceptation : après remappage complet de J1/Clavier puis réinitialisation → table §4.2.

### 3.6 Persistance logique des bindings

**REQ-INP-050** (P1) — Les données suivantes DOIVENT être conservées d'une session à l'autre (format et emplacement : SPEC-09) :
1. le jeu clavier de J1 et celui de J2 ;
2. pour chaque identité de manette rencontrée, son jeu de bindings manette ;
3. pour J1 et J2, l'identité de la dernière manette attribuée (ou « aucune »).
- Critère d'acceptation : remapper J2/Clavier, attribuer une manette à J2, quitter, relancer → J2 a les mêmes touches et retrouve sa manette si elle est branchée.

**REQ-INP-051** (P2) — L'original n'écrit ces données qu'à la fermeture normale du programme. L'implémentation DEVRAIT aussi les écrire après chaque modification de binding ou d'attribution, pour ne rien perdre en cas d'arrêt brutal.
- Critère d'acceptation : remapper, tuer le processus, relancer → le remappage est conservé (comportement recommandé).

**REQ-INP-052** (P2) — Si les données de bindings sont absentes ou illisibles, le programme DOIT démarrer avec les valeurs par défaut (§4.2, §4.3) sans erreur bloquante.
- Critère d'acceptation : fichier de bindings corrompu → démarrage normal, bindings par défaut.

### 3.7 Zapper à la souris

**REQ-INP-060** (P1) [ROM] — Le type de chaque port vaut `Manette` au démarrage du programme, n'est **pas persisté**, et DOIT revenir à `Manette`/`Manette` à chaque chargement d'un jeu **différent** du précédent, avant le hook de démarrage du profil (DEC-09-Q6, divergence acceptée **(D)**). Il est changé par les scripts de profil (fonctions de lecture et d'écriture du type des deux ports, voir SPEC-05) ; le changement s'applique au cœur immédiatement. Les profils des jeux au Zapper mettent le port 2 en `Zapper` au démarrage du jeu et restaurent les types précédents à sa fermeture.
- Critère d'acceptation : lancer le profil Duck Hunt → port 1 `Manette`, port 2 `Zapper` ; revenir à un autre jeu → les deux ports en `Manette`.

**REQ-INP-061** (P1) — Le Zapper est **actif** si et seulement si le jeu est en cours et qu'au moins un port est de type `Zapper`. Quand il est actif, son état est mis à jour à chaque image affichée.
- Critère d'acceptation : en pause, bouger la souris ne modifie pas la position transmise au cœur.

**REQ-INP-062** (P1) [ROM] — **Gâchette** : la gâchette du Zapper est pressée tant que le **bouton droit** de la souris est maintenu. Le bouton gauche n'agit pas sur le Zapper (il sert à tourner la caméra, REQ-INP-091).
- Critère d'acceptation : en harnais, port 2 en `Zapper` : bouton droit maintenu → bit 4 de l'octet du port à 0 (gâchette pressée, §4.1) ; bouton gauche seul → bit 4 à 1. [ROM] Duck Hunt, cliquer droit sur un canard → tir ; cliquer gauche → aucun tir.

**REQ-INP-063** (P1) — **Visée** : un rayon est lancé depuis la caméra de jeu à travers la position du pointeur dans la fenêtre, puis intersecté avec le plan `z = ZapperOffset.z` du repère de scène (SPEC-04B REQ-E3D-410). Soit `(x, y)` le point d'intersection et `(Ox, Oy) = (ZapperOffset.x, ZapperOffset.y)`. La position transmise au cœur DOIT être celle du pixel NES affiché sous le pointeur : `px = tronc((x − Ox) / e)`, `py = tronc(248 − (y − Oy) / e)`, où `e` est l'échelle de scène et `tronc` la troncature vers zéro (DEC-08-4, DEC-R2-T2). Écart volontaire **(D)** : l'original comptait `y` depuis `y = 240` et visait 8 lignes au-dessus du pixel affiché, sans tenir compte de l'échelle.
- Critère d'acceptation : caméra par défaut de face, décalage nul : pointeur sur le point de scène `C = (128, 120, 0)` → (128, 128) ; sur le centre de l'image NES `(128, 128)` → (128, 120) ; sur le coin haut-gauche de l'image `(0, 248)` → (0, 0) ; sur le coin haut-gauche du rectangle visible avec les marges (0 ; 7,9 ; 0 ; 0,1), point `(0 ; 239,9)` → (0, 8). Caméra tournée de 30°, la position transmise est toujours celle que la relation donne pour le point de scène situé sous le pointeur (±1).
- Notes : `ZapperOffset` est un vecteur modifiable par les scripts (SPEC-05), exprimé en unités de scène (x vers la droite, y vers le haut, z vers la profondeur). Duck Hunt l'anime pour suivre son image déplacée en 3D. Un décalage `ZapperOffset.y` positif correspond à une image déplacée vers le haut : le même point de l'écran vise alors un pixel NES situé plus bas. La visée tient compte de l'échelle de scène (DEC-08-4, D) ; sur desktop celle-ci vaut 1.

**REQ-INP-064** (P1) [ROM] — **Tir hors écran** : si le point visé tombe hors de l'image (x hors de [0, 256[ ou y hors de [0, 240[), la position est transmise telle quelle, sans bornage ; le cœur ne détecte alors pas de lumière (SPEC-02), ce qui correspond au « tir hors écran » des jeux (par exemple le rechargement). Si le rayon n'intersecte pas le plan (rayon parallèle au plan ou dirigé à l'opposé), la dernière position valide est conservée.
- Critère d'acceptation : en harnais, pointeur 20 pixels à gauche de l'image → position transmise (−20, y) sans bornage ; rayon parallèle au plan → dernière position valide conservée. [ROM] Duck Hunt, pointer hors de l'image NES et cliquer droit → tir manqué, aucun canard touché.

**REQ-INP-065** (P1) — Le Zapper fournit au cœur, pour le port en mode `Zapper` : l'état de la gâchette (REQ-INP-062) et, sur demande du cœur, la position visée (REQ-INP-063). La détection de lumière (comparaison avec la luminosité de l'image rendue autour de la position) relève du cœur (SPEC-02).
- Critère d'acceptation : voir les vecteurs de test §5.3.

**REQ-INP-066** (P2) [ROM] — Sur desktop, aucun réticule ni modèle de pistolet n'est affiché : le curseur système sert de viseur. Il reste soumis au masquage automatique (§3.12) ; la visée continue de fonctionner quand le curseur est masqué.
- Critère d'acceptation : Duck Hunt, souris immobile 3 s → curseur masqué ; un clic droit tire toujours à l'endroit du pointeur.

### 3.8 Entrées pilotées par script (pad de script)

**REQ-INP-070** (P1) — Un profil peut fournir une fonction Lua `InputProcess(index)` (signature et environnement : SPEC-05). Le pad de script est désactivé par défaut ; le script l'active ou le désactive explicitement (fonction d'activation exposée, SPEC-05). L'activation prend effet au plus tard à l'image affichée suivante.
- Critère d'acceptation : un profil sans appel d'activation n'a aucune influence sur les entrées, même s'il définit `InputProcess`.

**REQ-INP-071** (P1) — Quand le pad de script est actif et que le jeu est en cours, `InputProcess` DOIT être appelée **une fois par image émulée**, avant l'émulation de cette image, avec pour argument l'index (0 = J1, 1 = J2) du joueur auquel le pad est rattaché (J1 par défaut ; l'index est modifiable par script, SPEC-05). La valeur entière renvoyée est un masque d'actions (§4.1). Elle est combinée par OU au masque de ce joueur (REQ-INP-002).
- Critère d'acceptation : `InputProcess` qui renvoie 8 une image sur deux → le cœur voit Start une image sur deux sur le port 1. Une valeur `0x400` fait tourner la caméra à gauche.

**REQ-INP-072** (P1) — Le pad de script est **additif** : il ne peut pas masquer les entrées physiques. Un script qui veut transformer une entrée lit les états exposés (état de chaque joueur, état global, octet de chaque port, voir SPEC-05) et renvoie des bits supplémentaires.
- Critère d'acceptation : `InputProcess` renvoie toujours 0 → le jeu reste pilotable au clavier.

**REQ-INP-073** (P1) — Un seul script fournit `InputProcess` à la fois : si plusieurs scripts du profil la définissent, celle du dernier script chargé (ordre fixé par SPEC-05) est utilisée. Si l'appel lève une erreur, le pad contribue 0, l'erreur est signalée comme les autres erreurs de script (SPEC-05) et `InputProcess` n'est plus appelée pour ce script jusqu'à son rechargement.
- Critère d'acceptation : `InputProcess` qui fait `error("x")` → message d'erreur une seule fois, le jeu reste pilotable.

**REQ-INP-074** (P2) — Les données de position et d'orientation des mains et de la tête que le pad de script expose aux scripts (destinées à la VR) DOIVENT exister sur desktop et valoir le vecteur nul.
- Critère d'acceptation : un script qui lit la position de la main gauche sur desktop obtient (0, 0, 0) sans erreur.

### 3.9 Contrôles caméra (correspondance vers SPEC-06)

**REQ-INP-080** (P2) — Les actions caméra (`RotLeft`, `RotRight`, `RotUp`, `RotDown`, `ZoomIn`, `ZoomOut`) sont lues sur le OU des masques des joueurs **dont le port est de type `Manette`** ; un joueur dont le port est en `Zapper` ne pilote pas la caméra par ses actions.
- Critère d'acceptation : port 2 en `Zapper`, J2 maintient sa touche `RotLeft` → la caméra ne tourne pas ; J1 la maintient → elle tourne.

**REQ-INP-081** (P2) — Tant qu'une action de rotation est enfoncée, la consigne d'angle de la caméra varie à **30 degrés par seconde** : `RotLeft` diminue le lacet et `RotRight` l'augmente ; `RotUp` diminue le tangage et `RotDown` l'augmente (sens et lissage : SPEC-06). Si les deux sens d'un même axe sont enfoncés, `RotLeft` l'emporte sur `RotRight` et `RotDown` sur `RotUp`.
- Critère d'acceptation : maintenir `RotRight` 3 s → la consigne de lacet augmente de 90° ±3°.

**REQ-INP-082** (P2) — Tant que `ZoomIn` est enfoncé, la consigne de distance caméra diminue de **70 unités de scène par seconde** multipliées par l'échelle de scène (SPEC-06) ; `ZoomOut` l'augmente d'autant. `ZoomIn` l'emporte sur `ZoomOut`.
- Critère d'acceptation : échelle 1, maintenir `ZoomOut` 1 s → distance +70 ±3.

**REQ-INP-083** (P2) — Les contrôles caméra par actions et par souris fonctionnent pendant le jeu en cours. Ils n'ont pas d'effet visible quand la vue de jeu est masquée par le menu.
- Critère d'acceptation : en jeu, F5 maintenu → rotation visible.

### 3.10 Souris : caméra

**REQ-INP-090** (P2) — **Molette** : quand aucune action de zoom n'est enfoncée et que le pointeur n'est pas au-dessus d'un élément d'interface, la molette modifie la consigne de distance caméra : un cran vers l'avant (éloignement du doigt) rapproche la caméra de **5 unités de scène** multipliées par l'échelle de scène, un cran vers l'arrière l'éloigne d'autant.
- Critère d'acceptation : échelle 1, 4 crans vers l'avant → distance −20 ±2.
- Notes : dans l'original, le pas dépend de la durée de l'image d'affichage ; 5 unités correspondent à un affichage à 60 images/s. Le pas est fixé ici indépendamment du framerate (voir §6).

**REQ-INP-091** (P2) — **Glisser avec le bouton gauche** (sans Maj) : faire glisser le pointeur fait varier la consigne d'orientation de la caméra de **0,1 degré par pixel** de déplacement : un glissement vers la droite agit dans le même sens que `RotRight`, un glissement vers le haut dans le même sens que `RotUp`.
- Critère d'acceptation : glisser de 300 px vers la droite → consigne de lacet +30° ±1°.
- Notes : Maj + glisser était réservé à la sélection de l'éditeur (hors périmètre) ; sur desktop, il n'a aucun effet. Le basculement en vue subjective sur une tile (Ctrl + clic) relève de SPEC-06.

### 3.11 Hotkeys système et routage pendant les menus

**REQ-INP-100** (P1) — Les hotkeys système sont évaluées à chaque image **affichée** (horloge de rendu, distincte de l'échantillonnage des boutons NES avant chaque image émulée, REQ-INP-005) ; leurs fronts montants sont détectés sur cette horloge d'affichage et n'entrent pas dans le périmètre de déterminisme de REQ-ARCH-012 (seules leurs conséquences, commandes appliquées entre deux frames émulées, y entrent). Elles sont évaluées que le jeu soit en cours ou en pause, sur le OU des masques des deux joueurs **quel que soit le type de port**, à l'exception des combinaisons de REQ-INP-104 qui sont évaluées joueur par joueur.
- Critère d'acceptation : port 2 en `Zapper`, J2 appuie sur sa touche `Menu` → le menu s'ouvre.

**REQ-INP-101** (P1) — **Menu**, évalué au relâchement et ignoré pendant un chargement de jeu :
- appui court (moins de 0,3 s) : si le navigateur de fichiers est ouvert, il est fermé comme par « Annuler » ; sinon, si aucune boîte de dialogue n'est ouverte, bascule entre jeu en cours et pause. La pause affiche le menu et masque la vue de jeu (SPEC-07, SPEC-10) ;
- appui long (0,3 s ou plus) : réinitialise la vue caméra (SPEC-06) sans changer l'état de pause.
- Critère d'acceptation : Échap tapé brièvement en jeu → pause et menu ; à nouveau → reprise. Échap maintenu 1 s puis relâché → caméra réinitialisée, le jeu continue.

**REQ-INP-102** (P1) — **RollBack** : sur front montant, déclenche un retour en arrière (quantité et mécanisme : SPEC-07) puis la reprise du jeu, et affiche la notification « Roll Back » pendant 1 s.
- Critère d'acceptation : en jeu, appuyer sur `,` → le jeu revient en arrière et continue ; notification visible.

**REQ-INP-103** (P1) — **FastForward** : tant que l'action est maintenue, l'avance rapide est demandée à chaque image affichée ; la notification « Fast Forward » (1 s) est affichée. Une demande lance, si aucune avance rapide n'est déjà en cours, une avance rapide de 30 images émulées au facteur k = 1,5 (SPEC-07 REQ-LOOP-016, qui fait foi). Effet observable : un appui maintenu donne une vitesse ×1,5 quasi continue ; au relâchement, le reliquat est de 0 à 30 images émulées ; un appui bref donne 30 images à ×1,5 (environ 0,33 s).
- Critère d'acceptation : NTSC, maintenir `.` 5 s → environ 450 images émulées (±10 %) ; tap bref → 30 images émulées à ×1,5.

**REQ-INP-104** (P1) — **QuickSave / QuickLoad** :
- QuickSave est déclenché sur front montant de l'action `QuickSave`, **ou** quand un même joueur maintient `Select` depuis plus de 0,1 s et relâche `RotDown`. Il n'a d'effet que si un jeu est chargé. Il sauvegarde dans l'emplacement rapide (SPEC-07, SPEC-09) et affiche « Quick Save » pendant 4 s.
- QuickLoad est déclenché de même par `QuickLoad`, **ou** par `Select` maintenu plus de 0,1 s puis relâchement de `RotUp`. Il n'a d'effet que si l'emplacement rapide n'est pas vide. Il affiche « Quick Load » pendant 4 s.
- Critère d'acceptation : manette par défaut, maintenir Back puis donner un coup de stick droit vers le bas → « Quick Save ». Maintenir Back puis un coup de stick droit vers le haut → l'état est rechargé.
- Notes : par défaut, `QuickSave` et `QuickLoad` n'ont aucun binding ; la combinaison est la seule voie par défaut. `Select` reste transmis au jeu et le stick fait aussi tourner la caméra : c'est le comportement d'origine.

**REQ-INP-105** (P2) — **Pendant la pause (menu ouvert)** : l'émulation est arrêtée, donc aucune entrée n'atteint le cœur. Les actions des joueurs servent à l'interface :
- `Left`/`Right` (ou `Up`/`Down` selon l'orientation du contrôle) font défiler les onglets et les pages des listes, sur front montant, en bouclant aux extrémités ;
- `Select` (ou le clic droit) sur un emplacement de jeu ou sur la vignette du jeu courant déclenche l'action secondaire de cet élément (SPEC-10) ;
- `Menu` agit selon REQ-INP-101.
Le pointeur de la souris pilote l'interface normalement.
- Critère d'acceptation : menu ouvert, flèche droite → onglet suivant ; sur le dernier onglet → retour au premier.

**REQ-INP-106** (P2) — **Raccourcis clavier fixes** : en dehors des bindings, le programme desktop d'origine n'a aucun raccourci clavier actif en jeu. Une implémentation PEUT ajouter des raccourcis fixes (plein écran, par exemple), à condition qu'ils n'utilisent aucune touche de la table §4.2.
- Critère d'acceptation : aucune touche absente de la table §4.2 n'a d'effet en jeu avec la configuration par défaut.

### 3.12 Curseur de la souris

**REQ-INP-110** (P2) — Le curseur système DOIT être masqué automatiquement quand le jeu est en cours et que la souris n'a pas bougé pendant **2 secondes** (vérification toutes les 0,1 s, masquage à la 20ᵉ vérification consécutive sans mouvement). Tout mouvement le réaffiche immédiatement, au plus tard à la vérification suivante, et remet le compteur à zéro.
- Critère d'acceptation : en jeu, souris immobile 2,0 s (±0,15 s) → curseur masqué ; un mouvement de 1 px → curseur visible.

**REQ-INP-111** (P2) — Le curseur DOIT être visible en permanence quand le jeu n'est pas en cours (menu, pause, bibliothèque, chargement), et DOIT être rendu visible à la fermeture du programme ou de la fenêtre.
- Critère d'acceptation : mettre en pause avec le curseur masqué → il réapparaît dans les 0,1 s.

### 3.13 Abstraction des entrées (portabilité libretro)

**REQ-INP-120** (P1) — La logique d'entrée (actions, masques, règles de combinaison, hotkeys, Zapper, pad de script) DOIT être séparée des périphériques physiques. À chaque image, l'hôte fournit un **instantané d'entrées** indépendant de la plateforme, qui contient au minimum : le masque d'actions de J1 et de J2 (sans le pad de script, ajouté par la logique), la position du pointeur dans la cible de rendu et l'état de ses boutons (gauche, droit), le défilement de la molette, et l'indication « pointeur au-dessus de l'interface ». La logique ne DOIT dépendre d'aucune bibliothèque de fenêtrage ou de manette.
- Critère d'acceptation : un test unitaire sans fenêtre injecte des instantanés et vérifie l'octet transmis au cœur, les déclenchements de hotkeys et la position du Zapper.

**REQ-INP-121** (P1) — La conversion entre périphériques et actions (bindings, zones mortes, seuil 0,34, attribution des manettes, persistance) DOIT appartenir à la couche hôte desktop, remplaçable par une autre couche hôte.
- Critère d'acceptation : le frontend libretro n'utilise ni le système de bindings ni le fichier de bindings, et pilote pourtant J1 et J2.

**REQ-INP-122** (P2) — Le frontend libretro DEVRA établir la correspondance par défaut de la table §4.4 : RetroPad vers actions NES et caméra, pistolet optique ou pointeur libretro vers le Zapper. Les hotkeys système (menu, rollback, avance rapide, sauvegarde rapide) sont laissés au frontend hôte (menu, rembobinage, avance rapide et sauvegardes d'état de l'hôte) et ne sont pas exposés comme boutons.
- Critère d'acceptation : sous RetroArch, la manette 1 pilote J1 selon §4.4 ; le stick droit tourne la caméra.

**REQ-INP-123** (P2) — Pour le Zapper sous libretro, la position fournie par l'hôte (coordonnées normalisées dans l'image rendue) DOIT être convertie en pixel NES par le même lancer de rayon que REQ-INP-063, à travers la caméra 3D du cœur. La gâchette est le bouton de tir du pistolet (ou le bouton principal du pointeur). Le « hors écran » de l'hôte produit une position hors image (REQ-INP-064).
- Critère d'acceptation : sous RetroArch, viser le point de scène `C = (128, 120, 0)` en vue par défaut → (128, 120) ±1 ; viser le centre de l'image NES → (128, 112) ±1 (même relation que REQ-INP-063).

## 4. Formats et interfaces

### 4.1 Actions et codage des bits

L'ordre des lignes est l'ordre d'affichage dans l'écran de configuration. Les noms de la colonne « Nom » sont les libellés affichés. Les valeurs de bits sont visibles des scripts Lua.

| # | Nom | Rôle | Bit (hex) | Bit (déc.) |
|---|---|---|---|---|
| 1 | `Left` | NES Gauche | 0x40 | 64 |
| 2 | `Right` | NES Droite | 0x80 | 128 |
| 3 | `Up` | NES Haut | 0x10 | 16 |
| 4 | `Down` | NES Bas | 0x20 | 32 |
| 5 | `A` | NES A | 0x01 | 1 |
| 6 | `B` | NES B | 0x02 | 2 |
| 7 | `Select` | NES Select | 0x04 | 4 |
| 8 | `Start` | NES Start | 0x08 | 8 |
| 9 | `RotLeft` | Caméra : rotation à gauche | 0x400 | 1024 |
| 10 | `RotRight` | Caméra : rotation à droite | 0x800 | 2048 |
| 11 | `RotUp` | Caméra : rotation vers le haut | 0x1000 | 4096 |
| 12 | `RotDown` | Caméra : rotation vers le bas | 0x2000 | 8192 |
| 13 | `ZoomIn` | Caméra : rapprocher | 0x100 | 256 |
| 14 | `ZoomOut` | Caméra : éloigner | 0x200 | 512 |
| 15 | `Menu` | Pause / menu, réinitialisation de vue (appui long) | 0x4000 | 16384 |
| 16 | `RollBack` | Retour en arrière | 0x8000 | 32768 |
| 17 | `FastForward` | Avance rapide | 0x10000 | 65536 |
| 18 | `QuickSave` | Sauvegarde rapide | 0x20000 | 131072 |
| 19 | `QuickLoad` | Chargement rapide | 0x40000 | 262144 |

Octet de manette NES (bits 0 à 7) : A, B, Select, Start, Haut, Bas, Gauche, Droite, du bit 0 au bit 7. Type de port : `Manette` = 0, `Zapper` = 1.

**Octet d'un port Zapper** (dans le mot d'entrées transmis au cœur, SPEC-02 REQ-CORE-061) : bit 4 = **1 quand la gâchette est relâchée, 0 quand elle est pressée** (convention de l'hôte de référence, DEC-02-Q1) ; tous les autres bits sont nuls (le bit 3 de lumière est calculé par le cœur). Exemple : bouton droit relâché → `0x10` ; maintenu → `0x00`.

### 4.2 Clavier par défaut (J1 ; J2 n'a aucun binding)

| Action | Touche |
|---|---|
| Left / Right / Up / Down | Flèches gauche / droite / haut / bas |
| A | X |
| B | Z (touche physique en position QWERTY ; sur AZERTY c'est la touche W) |
| Select | Espace |
| Start | Entrée (principale) |
| RotLeft / RotRight | F5 / F6 |
| RotUp / RotDown | F7 / F8 |
| ZoomIn / ZoomOut | F9 / F10 |
| Menu | Échap |
| RollBack | `,` (virgule) |
| FastForward | `.` (point) |
| QuickSave / QuickLoad | aucune |

Notes : les touches sont désignées par leur **position physique** (codes de touche indépendants de la disposition), ce qui est le comportement d'origine. Une implémentation PEUT afficher le libellé correspondant à la disposition active. Voir §6 pour la question de la disposition.

### 4.3 Manette par défaut (disposition de type Xbox, appliquée à toute manette reconnue)

| Commande | Action |
|---|---|
| Croix gauche / droite / haut / bas | Left / Right / Up / Down |
| Bouton sud (A Xbox, Croix PlayStation) | A |
| Bouton ouest (X Xbox, Carré) | B |
| Bouton nord (Y Xbox, Triangle) | RollBack |
| Bouton est (B Xbox, Rond) | FastForward |
| Back / View / Select / Share | Select |
| Start / Menu / Options | Start |
| Stick droit gauche / droite / haut / bas | RotLeft / RotRight / RotUp / RotDown |
| Gâchette gauche (LT/L2) | ZoomIn |
| Gâchette droite (RT/R2) | ZoomOut |
| Clic du stick droit (R3) | Menu |
| Stick gauche, clic du stick gauche, LB, RB, bouton Guide | aucune |
| QuickSave / QuickLoad | aucune (combinaisons de REQ-INP-104) |

### 4.4 Correspondance libretro par défaut (cible P7)

| RetroPad (port N → joueur N) | Action |
|---|---|
| Croix | Left / Right / Up / Down |
| A (est) | A |
| B (sud) | B |
| Select / Start | Select / Start |
| Stick analogique droit (seuil 0,34 par demi-axe) | RotLeft / RotRight / RotUp / RotDown |
| L2 / R2 | ZoomIn / ZoomOut |
| Pistolet optique (ou pointeur) du port N, quand le port N est en `Zapper` | Zapper : position et gâchette |

Note : la correspondance A=est / B=sud suit la convention habituelle des cœurs NES libretro. Elle diffère volontairement de la disposition desktop (§4.3), où A est le bouton sud.

## 5. Vecteurs de test

### 5.1 Masques
| Entrées physiques (configuration par défaut, ports en `Manette`) | Masque J1 | Octet port 1 |
|---|---|---|
| Flèche haut + X | 0x11 | 0x11 |
| Entrée + Espace | 0x0C | 0x0C |
| F5 + F9 | 0x500 | 0x00 |
| Manette J1 : sud + ouest + croix bas | 0x23 | 0x23 |
| Manette J1 : stick droit à 20 % à droite | 0x00 | 0x00 |
| Manette J1 : stick droit à 60 % à droite | 0x800 | 0x00 |
| Échap | 0x4000 | 0x00 |

### 5.2 Hotkeys
| Séquence | Résultat attendu |
|---|---|
| Échap enfoncé 0,1 s puis relâché, en jeu | Pause, menu affiché |
| Échap enfoncé 0,5 s puis relâché, en jeu | Vue caméra réinitialisée, jeu toujours en cours |
| Espace maintenu, F8 enfoncé puis relâché après 0,2 s, jeu chargé | Sauvegarde rapide, « Quick Save » 4 s |
| Espace maintenu, F7 enfoncé puis relâché, emplacement rapide vide | Rien |
| Virgule (front montant) | Retour en arrière + reprise, « Roll Back » 1 s |

### 5.3 Zapper (port 2 en `Zapper`, caméra par défaut, décalage nul, échelle 1 ; relation de REQ-INP-063 selon DEC-08-4, recalculée en v1.1)
| Pointeur (point de scène visé) | Bouton droit | Position transmise | Octet du port |
|---|---|---|---|
| `C = (128, 120)` | relâché | (128, 128) ±1 | `0x10` |
| Centre de l'image NES `(128, 128)` | relâché | (128, 120) ±1 | `0x10` |
| Coin haut-gauche de l'image `(0, 248)` | maintenu | (0, 0) ±1 | `0x00` |
| Coin haut-gauche du rectangle visible, marges (0 ; 7,9 ; 0 ; 0,1) : `(0 ; 239,9)` | maintenu | (0, 8) ±1 | `0x00` |
| 20 px à gauche de l'image | maintenu | (−20, y) | `0x00` (tir manqué côté cœur) |
| `C`, avec `ZapperOffset` = (10, 0, 0) | relâché | (118, 128) ±1 | `0x10` |

## 6. Questions ouvertes

1. *Close par DEC-08-1* (retrait de l'autre joueur, D). **Attribution manuelle en double** : l'original permet d'attribuer la même manette aux deux joueurs depuis l'écran de configuration (elle pilote alors J1 et J2 à la fois). REQ-INP-041 recommande de la retirer de l'autre joueur. À valider.
2. *Close par DEC-08-2*. **Capture de la touche Échap** : dans l'original, Échap peut être capturée comme binding ; la même pression peut aussi être vue comme `Menu` et refermer le menu. REQ-INP-047 recommande d'empêcher ce double effet. Faut-il en plus réserver Échap pour annuler l'écoute ?
3. *Close par DEC-08-3* (5 unités par cran). **Molette** : le pas d'origine dépend du framerate d'affichage. REQ-INP-090 le fige à 5 unités par cran (référence 60 images/s). À confirmer par mesure en boîte noire.
4. *Close par DEC-08-4* (D). **Échelle de scène et Zapper** : l'original ne tient pas compte de l'échelle de scène dans la visée à la souris (la visée n'est juste qu'à l'échelle 1). REQ-INP-063 exige le pixel réellement sous le pointeur, quelle que soit l'échelle. Écart assumé : à confirmer.
5. *Close par DEC-08-5* (codes physiques). **Disposition clavier** : codes physiques (Z = W en AZERTY) ou caractères ? La spec retient les codes physiques ; SPEC-10 doit afficher un libellé compréhensible.
6. *Close par DEC-08-6 et DEC-R2-D7* (conservé, sans effet). **Réglage « bouton Menu émulé »** : un booléen persisté existe (SPEC-09) mais n'a aucun effet observable sur desktop. À confirmer avec SPEC-09 : le garder seulement pour la compatibilité des données ?
7. *Close par DEC-08-7* (différentiel pour QuickLoad). **Hotkeys en pause** : RollBack et QuickLoad fonctionnent aussi menu ouvert et relancent le jeu (RollBack force la reprise). À confirmer en boîte noire pour QuickLoad (SPEC-07).
8. *Close par DEC-08-8*. **Zones mortes** (REQ-INP-031) : valeurs issues des valeurs usuelles, non mesurées sur l'original.

Note (DEC-06-Q6) : il n'existe pas de conflit de bouton entre le Zapper et la caméra : la gâchette est le bouton droit (REQ-INP-062), l'orbite le bouton gauche (REQ-INP-091).

9. *Close par DEC-08-4* (pixel affiché visé, D). Constat : le seul profil embarqué qui écrit `ZapperOffset` (Duck Hunt) l'anime pour suivre des objets en mouvement, à partir d'une base y = 0 ; il ne compense pas le décalage. **Décalage de 8 lignes de la visée** (nouvelle, v1.1) : la relation fidèle de REQ-INP-063 vise 8 lignes au-dessus du pixel affiché sous le pointeur. Elle est spécifiée par fidélité ; R0 peut décider une divergence (D) qui viserait le pixel affiché (`py = tronc(248 − (y − Oy))`), au prix d'un écart avec les profils qui compensent éventuellement ce décalage par `ZapperOffset`.

## Historique

- 1.1-draft (AMD-001, relecture R2) : visée du Zapper réécrite dans le repère de REQ-E3D-410 avec la relation vérifiée dans l'original (py = tronc(240 − (y − Oy)), décalage de 8 lignes) et vecteurs §5.3 recalculés ; polarité de la gâchette inscrite au §4.1 ; REQ-INP-041 en DOIT (DEC-08-1) ; REQ-INP-060 (DEC-09-Q6) ; REQ-INP-100 (horloge d'affichage) ; avance rapide alignée sur SPEC-07 (k = 1,5) ; critères sans ROM et étiquettes [ROM] ; questions closes, Q-9 ajoutée.
- 1.1 (R2-review-v1.1 N-1) : visée du Zapper alignée sur DEC-08-4 (pixel affiché sous le pointeur, `py = tronc(248 − y)`, échelle prise en compte) ; critère de REQ-INP-063 et vecteurs §5.3 recalculés ; Q-9 close par DEC-08-4.
