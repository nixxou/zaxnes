# SPEC-02 — Cœur NES : contrat avec le moteur 3D, interface, exactitude
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-00 (glossaire), SPEC-04 (consommation des tiles), SPEC-05 (lecture mémoire par Lua), SPEC-07 (boucle, vitesse, rollback), SPEC-08 (entrées, Zapper côté hôte), SPEC-09 (fichiers de sauvegarde), SPEC-11 (sortie audio)

## 1. Objet et périmètre

Ce document spécifie le module `CORE` : l'émulateur NES (CPU 2A03, PPU 2C02/2C07, APU, cartouches) **vu de l'extérieur**.

Le cœur est réécrit à partir de la documentation publique (wiki nesdev, ROMs de test publiques). Ce document ne décrit donc **pas** le fonctionnement interne du CPU, du PPU, de l'APU ni des mappers. Il fixe :

1. le **contrat de sortie vidéo par tiles**, propre à 3dSen : à chaque frame, le cœur fournit au moteur 3D la liste des tiles 8×8 affichées, au lieu de pixels (ou en plus des pixels) ;
2. l'interface de pilotage : cycle de vie, chargement de ROM, régions, exécution d'une frame avec un mot d'entrées, Zapper, vitesse, audio, accès mémoire, savestates ;
3. la liste des mappers exigés ;
4. les exigences d'exactitude, exprimées en ROMs de test publiques.

Hors périmètre : l'enregistrement de films d'entrées, la VR, le Famicom Disk System, l'audio d'extension des cartouches (voir §6).

## 2. Concepts et vocabulaire

- **Frame émulée** : une exécution de frame, du début d'un vblank au début du vblank suivant (REQ-CORE-020).
- **Ligne L** : ligne de balayage visible, numérotée de 0 (haut) à 239 (bas). Les lignes de vblank et la ligne de pré-rendu ne sont pas « visibles ».
- **Enregistrement de tile** (ou « tile émise ») : description d'une tile 8×8 affichée pendant la frame (§4.1). Il en existe deux sortes : **tile de fond** et **tile de sprite**.
- **Lot de frame** : la liste ordonnée des enregistrements d'une frame, la palette et les marges (maskX, maskY), livrés ensemble par le **rappel vidéo**.
- **Registre de scroll interne** : l'adresse VRAM courante du PPU (appelée « v » sur nesdev), composée du scroll X grossier (cx, 0 à 31), du scroll Y grossier (cy, 0 à 31), de la sélection de nametable (2 bits) et du scroll Y fin (fy, 0 à 7). Le scroll X fin (fx, 0 à 7) est le registre « x » de nesdev.
- **Mirroring courant** : disposition des 4 nametables logiques sur la VRAM, à un instant donné : horizontal ($2000=$2400, $2800=$2C00), vertical ($2000=$2800, $2400=$2C00), écran unique (une seule nametable) ou quatre écrans. Les mappers peuvent le modifier en cours de jeu.
- **Mot d'entrées** : entier de 16 bits fourni à chaque frame (§4.4).
- Valeurs hexadécimales notées `$xx` (adresses NES) ou `0xNN`.

## 3. Exigences

### 3.1 Interface et cycle de vie

**REQ-CORE-001** (P1) — Le cœur DOIT être une bibliothèque sans dépendance de plateforme (ni fenêtre, ni audio système, ni horloge, ni thread imposé), sans état global mutable : deux instances du cœur dans un même processus DOIVENT fonctionner indépendamment. L'hôte pilote le cœur en appelant « exécuter une frame ».
- Critère d'acceptation : un test crée deux instances, charge deux ROMs différentes et exécute 600 frames en alternant les appels ; les lots de frame de chaque instance sont identiques à ceux d'une exécution isolée de la même ROM.

**REQ-CORE-002** (P1) — Le cœur DOIT offrir au minimum les opérations suivantes (noms libres) :

| Opération | Effet observable |
|---|---|
| Initialiser / libérer | prépare ou libère l'instance ; aucune émulation avant l'initialisation |
| Enregistrer le rappel vidéo | reçoit chaque lot de frame (REQ-CORE-030) |
| Fournir la source de position du Zapper | position visée, en pixels NES (REQ-CORE-060) |
| Obtenir les infos vidéo | largeur 33, hauteur 31, nombre de sprites 128 (REQ-CORE-031) |
| Charger une ROM | octets + nom → succès ou échec, région détectée (§3.2) |
| Fermer la ROM | arrête l'émulation (REQ-CORE-005) |
| Réinitialiser | REQ-CORE-004 |
| Exécuter une frame | avec un mot d'entrées (§3.4) |
| Choisir la région (mode TV) | §3.3 |
| Choisir le type de périphérique de chaque port | manette ou Zapper (REQ-CORE-024) |
| Régler la vitesse | facteur de vitesse pour l'audio (REQ-CORE-072) |
| Lire l'audio | REQ-CORE-071 |
| Lire / écrire un octet mémoire | §3.8 |
| Sauvegarder / restaurer l'état | §3.9 |
| Compteur de frames (lecture et écriture) | REQ-CORE-006 |
| Nom du cœur | chaîne non vide, stable |

- Critère d'acceptation : chaque opération du tableau est exercée par au moins un test automatisé du lot L1 qui vérifie son effet observable ; le rapport de lot contient la matrice opération → test (aucune ligne vide).

**REQ-CORE-003** (P1) — Au chargement réussi d'une ROM, la machine DOIT être dans l'état de mise sous tension décrit par REQ-CORE-004, compteur de frames à 0, prête à exécuter des frames.
- Critère d'acceptation : charger la ROM de test minimale (§5.1) puis lire `$0000` à `$07FF` → tous nuls ; compteur de frames = 0.

**REQ-CORE-004** (P2) — La réinitialisation DOIT être équivalente à une **mise sous tension** :
- RAM CPU (`$0000` à `$07FF`), PRG-RAM (`$6000` à `$7FFF`), RAM de nametables, OAM et RAM de palette remises à 0 ;
- mapper revenu dans son état initial ;
- APU silencieux ;
- CPU redémarré au vecteur de reset ;
- compteur de frames à 0.

La ROM et le type des ports sont conservés.
- Critère d'acceptation : écrire `0x55` en `$0300` et en `$6000`, réinitialiser → les deux adresses relisent 0. Le compteur de frames vaut 0.
- Notes : c'est le comportement de référence (le reset de 3dSen efface aussi la PRG-RAM). Sur une vraie console, un reset conserve la RAM ; la ROM de test publique `cpu_reset/ram_after_reset` échoue donc **volontairement** (voir §3.12).

**REQ-CORE-005** (P1) — Après « fermer la ROM », ou tant qu'aucune ROM n'est chargée, « exécuter une frame » NE DOIT RIEN faire : pas de rappel vidéo, pas d'audio produit, compteur de frames inchangé. Un nouveau chargement de ROM ou de savestate réactive l'émulation.
- Critère d'acceptation : fermer la ROM puis appeler 10 fois « exécuter une frame » → 0 rappel vidéo, compteur inchangé, lecture audio = 0 échantillon après vidage.

**REQ-CORE-006** (P1) — Le compteur de frames DOIT augmenter de 1 à chaque frame émulée. Il DOIT être remis à 0 par le chargement d'une ROM et par la réinitialisation, et DOIT être inclus dans le savestate. L'hôte PEUT le modifier.
- Critère d'acceptation : charger, exécuter 100 frames → 100 ; sauvegarder, exécuter 50 frames, restaurer → 100.

### 3.2 Chargement de ROM

**REQ-CORE-010** (P1) — Le cœur DOIT accepter une image de ROM au format **iNES** (en-tête de 16 octets commençant par `4E 45 53 1A`), y compris les en-têtes **NES 2.0**. Il reçoit les octets de la ROM et un **nom de ROM** (chaîne, typiquement le nom du fichier). La décompression d'archives (.zip, .7z) relève de l'hôte (SPEC-07).
- Critère d'acceptation : la ROM de test minimale (§5.1) se charge ; la même ROM avec un en-tête NES 2.0 équivalent se charge aussi.

**REQ-CORE-011** (P1) — Le chargement DOIT échouer, sans modifier l'instance (ou en la laissant arrêtée), et le signaler à l'appelant dans les cas suivants :
- signature absente ;
- données plus courtes que la taille annoncée par l'en-tête ;
- numéro de mapper non supporté.

Le message d'erreur DEVRAIT préciser la cause (« en-tête corrompu », « mapper N non supporté », « fichier tronqué »).
- Critère d'acceptation : 16 octets `00` → échec ; ROM minimale tronquée de 1 octet → échec ; en-tête annonçant le mapper 200 → échec avec un message contenant « 200 ».
- Notes : la référence ne détecte pas les fichiers tronqués. L'échec est ici une amélioration (divergence acceptée).

**REQ-CORE-012** (P1) — Interprétation de l'en-tête (iNES / NES 2.0, d'après nesdev) :
- taille PRG-ROM : octet 4 × 16 Kio (étendue par NES 2.0 si présent) ;
- taille CHR-ROM : octet 5 × 8 Kio. Si elle vaut 0, la cartouche a de la **CHR-RAM de 32 Kio** (sauf taille explicite NES 2.0) ;
- mirroring initial : bit 3 de l'octet 6 = quatre écrans ; sinon bit 0 = 0 → horizontal, 1 → vertical ;
- bit 1 de l'octet 6 : PRG-RAM sauvegardée par pile ;
- bit 2 de l'octet 6 : trainer de 512 octets, présent entre l'en-tête et la PRG-ROM, et chargé en `$7000` ;
- numéro de mapper : quartet haut de l'octet 6 et quartet haut de l'octet 7 (plus les bits NES 2.0 de l'octet 8) ;
- en-tête iNES « sale » : quand l'en-tête n'est pas NES 2.0 et que les octets 12 à 15 ne sont pas tous nuls (par exemple le texte « DiskDude! »), le quartet haut de l'octet 7 DOIT être ignoré ;
- cas particulier : un en-tête dont l'octet 6 vaut `0x23` et l'octet 7 vaut `0x64` DOIT être traité comme mapper 2.

Une PRG-RAM de 8 Kio est toujours présente en `$6000` à `$7FFF`, sauf si le mapper la désactive.
- Critère d'acceptation : vecteurs §5.2 (en-têtes → mapper, mirroring, taille CHR-RAM, région).
- Notes : le cas `0x23`/`0x64` reproduit une correction de compatibilité de la référence pour un dump mal étiqueté. La référence ne saute pas le trainer : la gestion correcte du trainer est une divergence acceptée.

**REQ-CORE-013** (P2) — Le chargement DOIT renvoyer la **région détectée** : NTSC, PAL, MULTI (compatible les deux, minutage NTSC) ou DENDY. Règle :
- en-tête NES 2.0 : bits 0 et 1 de l'octet 12 → 0 = NTSC, 1 = PAL, 2 = MULTI, 3 = DENDY ;
- en-tête iNES 1.0 : bit 0 de l'octet 9 = 1 → PAL ; sinon NTSC ;
- en-tête iNES « sale » (REQ-CORE-012) : NTSC, sans lire les octets 7 à 15.

Le cœur NE DOIT PAS appliquer lui-même cette région : c'est l'hôte qui appelle « choisir la région », selon son réglage « région automatique » (SPEC-07, SPEC-09).
- Critère d'acceptation : en-tête NES 2.0 dont l'octet 12 vaut 3 → DENDY ; iNES 1.0 dont l'octet 9 vaut 1 → PAL ; tous ces octets nuls → NTSC.
- Notes : la référence lit les 2 bits bas de l'octet 10 avec le codage 0/1/2/3 = NTSC/PAL/MULTI/DENDY, ce qui donne des résultats erronés pour les en-têtes NES 2.0. La règle ci-dessus suit la documentation publique (divergence acceptée, voir §6 Q2).

**REQ-CORE-014** (P1) — Le nom de ROM fourni au chargement DOIT être conservé par l'instance, inclus dans le savestate et renvoyé à la restauration (REQ-CORE-091).
- Critère d'acceptation : charger avec le nom « Test.nes », sauvegarder, charger une autre ROM, restaurer → le nom renvoyé est « Test.nes ».

### 3.3 Régions et minutage

**REQ-CORE-015** (P1) — Le cœur DOIT émuler les quatre régions avec le minutage public de nesdev (« Cycle reference chart », « Clock rate ») :

| Région | Horloge CPU | Points PPU par cycle CPU | Lignes par frame | Ligne de début du vblank (drapeau et NMI) | Lignes visibles | Cadence nominale de l'hôte |
|---|---|---|---|---|---|---|
| NTSC | 1 789 773 Hz | 3 | 262 | 241 | 240 | 60 fps |
| PAL | 1 662 607 Hz | 3,2 | 312 | 241 | 240 | 50 fps |
| DENDY | 1 773 448 Hz | 3 | 312 | 291 | 240 | 50 fps |
| MULTI | comme NTSC | | | | | 60 fps |

En NTSC, une ligne fait 341 points, et le point sauté des frames impaires (rendu actif) s'applique ; il ne s'applique ni en PAL ni en DENDY. Les tables de périodes du bruit et du DMC sont celles du PAL en région PAL, celles du NTSC en NTSC, MULTI et DENDY.
- Critère d'acceptation : en NTSC, le nombre moyen de cycles CPU par frame émulée vaut 29 780,5 ± 0,5 sur 1 000 frames ; en PAL, 33 247,5 ± 0,5 ; en DENDY, 35 464 ± 1. Les ROMs blargg de minutage NMI/VBL réussissent en NTSC (§3.12).
- Notes : la « cadence nominale » est la fréquence à laquelle l'hôte de référence enchaîne les frames (SPEC-07) et sur laquelle est calé le rééchantillonnage audio (REQ-CORE-070). La référence affiche 239 lignes en PAL et en DENDY. On exige ici les 240 lignes du matériel (divergence acceptée).

**REQ-CORE-016** (P2) — Changer de région pendant le jeu DOIT être possible entre deux frames. Le changement s'applique dès la frame suivante, sans réinitialiser la machine.
- Critère d'acceptation : en jeu NTSC, passer en PAL → les frames suivantes durent 33 247,5 cycles CPU en moyenne ; la RAM est intacte.

### 3.4 Exécution d'une frame et entrées

**REQ-CORE-020** (P1) — **Bornes d'une frame émulée.** « Exécuter une frame » DOIT faire avancer l'émulation depuis le point où la frame précédente s'est arrêtée jusqu'au **début du vblank suivant** (passage à 1 du drapeau VBlank : ligne 241 point 1 en NTSC et PAL, ligne 291 en DENDY). Une frame émulée contient donc la fin du vblank précédent, la ligne de pré-rendu, puis les 240 lignes visibles. Le rappel vidéo est appelé **une seule fois**, de façon synchrone, à la fin de l'exécution, avec les tiles des lignes visibles qui viennent d'être rendues. L'état observable à cet instant (mémoire lue par les scripts Lua, REQ-CORE-080) est celui du début du vblank : le gestionnaire NMI du jeu n'a pas encore exécuté plus de quelques instructions.
- Critère d'acceptation : avec la ROM de test minimale, NMI activé et un gestionnaire NMI qui incrémente `$0010`, après N frames `$0010` vaut N − 1 ou N, et jamais plus. Le rappel vidéo est appelé exactement une fois par frame.
- Notes : juste après le chargement ou la réinitialisation, la première frame commence au début du vblank. La réinitialisation tombe donc en début de vblank.

**REQ-CORE-021** (P1) — Le rappel vidéo DOIT être appelé même quand le lot est vide (rendu désactivé pendant toute la frame, nombre d'enregistrements = 0).
- Critère d'acceptation : ROM minimale avec PPUMASK = `$00` → un rappel par frame, 0 enregistrement.

**REQ-CORE-022** (P1) — **Mot d'entrées** (format §4.4). L'octet du port 1 occupe les bits 0 à 7, celui du port 2 les bits 8 à 15. Pour un port de type manette, les bits suivent l'ordre de lecture série de la manette standard : A, B, Select, Start, Haut, Bas, Gauche, Droite, du bit 0 au bit 7. Le mot reste valable pendant toute la frame ; le jeu le lit par le protocole standard (`$4016` écrit pour le strobe, lectures série sur `$4016` et `$4017`), comme décrit sur nesdev.
- Critère d'acceptation : mot `0x0081` → le jeu lit A = 1, puis B, Select, Start, Haut, Bas, Gauche = 0, puis Droite = 1 sur `$4016` ; mot `0x0800` → Start enfoncé sur `$4017`.
- Notes : les bits d'actions frontend au-delà de `0xFF` (caméra, menu, rollback… table SPEC-08 §4.1) ne sont **jamais** transmis au cœur. L'hôte compose `(J1 & 0xFF) | ((J2 & 0xFF) << 8)` (SPEC-08). Le 9e bit lu et les suivants valent 1, comme sur une manette officielle (nesdev).

**REQ-CORE-023** (P3) — Le cœur PEUT aussi offrir un rappel d'interrogation des entrées. Seul le mot passé à « exécuter une frame » fait foi.
- Critère d'acceptation : sans objet si le rappel n'est pas offert.

**REQ-CORE-024** (P1) — Chaque port DOIT pouvoir être configuré, à tout moment entre deux frames, en **manette** (valeur 0) ou en **Zapper** (valeur 1). Le type par défaut est manette sur les deux ports.
- Critère d'acceptation : port 2 en Zapper → une lecture de `$4017` renvoie l'octet Zapper (REQ-CORE-061) et non une donnée série ; retour en manette → lecture série.

### 3.5 Contrat vidéo par tiles

#### 3.5.1 Lot de frame

**REQ-CORE-030** (P1) — Le rappel vidéo DOIT recevoir, pour chaque frame émulée :
1. la liste ordonnée des enregistrements de tiles (§4.1) et leur nombre ;
2. la **palette** : un tableau de 32 octets (REQ-CORE-045) ;
3. **maskX** et **maskY** (REQ-CORE-046).

Les données ne sont valides que pendant l'appel : l'hôte doit les copier s'il les conserve.
- Critère d'acceptation : vecteurs de test §5.3.

**REQ-CORE-031** (P1) — Les **infos vidéo** DOIVENT valoir : largeur = 33, hauteur = 31, nombre de sprites = 128. Elles dimensionnent la grille du moteur 3D : 33 colonnes de tiles de fond (32 plus une, à cause du scroll fin), 31 rangées (30 plus une), 128 emplacements de sprites (64 sprites, deux enregistrements par sprite en mode 8×16).
- Critère d'acceptation : appel → (33, 31, 128).

**REQ-CORE-032** (P1) — Capacité : un lot DOIT contenir au plus **1 023 enregistrements de fond** (33 × 31) et au plus **128 enregistrements de sprites**, donc 1 151 au total. Les enregistrements qui dépasseraient l'une de ces limites dans une frame DOIVENT être abandonnés, sans erreur, et l'émulation continue.
- Critère d'acceptation : une ROM qui force fy = 1 sur toutes les lignes (effet de raster) produit au plus 1 023 enregistrements de fond par frame, sans plantage ; ses sprites restent présents.
- Notes : dans la référence, un dépassement arrête l'émulation. Aucun jeu des profils embarqués n'en produit.

**REQ-CORE-033** (P1) [ROM] — **Lecture sans effet de bord.** Les lectures de données de motifs faites pour construire les enregistrements NE DOIVENT avoir aucun effet sur l'émulation : pas de comptage d'IRQ par A12 (MMC3 et compatibles), pas de bascule de verrou (MMC2/MMC4), pas de changement de tampon de lecture. L'émulation (images, sons, état) DOIT être identique, que le rappel vidéo soit enregistré ou non.
- Critère d'acceptation : la suite mmc3_test donne les mêmes résultats avec et sans rappel vidéo ; la ROM de test MMC2 de REQ-CORE-039 produit la même RAM et les mêmes lots après 600 frames avec et sans rappel. [ROM] Punch-Out!! produit la même RAM après 3 000 frames avec et sans rappel.

#### 3.5.2 Contenu d'un enregistrement

**REQ-CORE-034** (P1) — Les **plans** d'un enregistrement DOIVENT contenir les 16 octets du motif tel que stocké en CHR, **sans retournement appliqué** :
- plan 0 (« low ») : les 8 octets du premier plan ;
- plan 1 (« high ») : les 8 octets du second plan.

Chaque plan est un entier de 64 bits non signé. L'octet de la rangée r (r = 0 en haut) occupe les bits 8r à 8r+7. Dans cet octet, le bit 7 est le pixel le plus à gauche (colonne 0) et le bit 0 la colonne 7. La valeur de couleur d'un pixel source (colonne c, rangée r) vaut bit(plan0, 8r + 7 − c) + 2 × bit(plan1, 8r + 7 − c), entre 0 et 3, où 0 est transparent.

Le pixel affiché à la position (px, py) de la tile (0 à 7) est le pixel source de colonne `miroirH ? 7 − px : px` et de rangée `miroirV ? 7 − py : py`.
- Critère d'acceptation : un motif CHR dont le plan 0 vaut `80 00 00 00 00 00 00 01` et le plan 1 vaut `00`×8 → plan0 = `0x0100000000000080`, plan1 = 0. Son pixel source (0,0) vaut 1, (7,7) vaut 1, et les autres valent 0.
- Notes : ce codage est aussi celui des plans conservés dans les profils (SPEC-03), qui servent de clé de reconnaissance des tiles (SPEC-04). Il DOIT être respecté au bit près.

**REQ-CORE-035** (P1) — Champs d'un enregistrement (types et bornes §4.1) :
- **x, y** : position à l'écran, en pixels, du coin haut-gauche de la tile (entiers signés) ;
- **indice** : REQ-CORE-040 et REQ-CORE-043 ;
- **miroirH, miroirV** : retournements ;
- **caché** (« hidden ») : pour un sprite, bit de priorité « derrière le fond » ; toujours faux pour le fond ;
- **palette** : numéro de sous-palette, 0 à 3 (sous-palette de fond pour le fond, de sprite pour un sprite) ;
- **fond** : vrai pour une tile de fond, faux pour un sprite.
- Critère d'acceptation : vecteurs §5.3.

**REQ-CORE-036** (P1) — Le cœur DOIT émettre les enregistrements **même si les plans sont entièrement nuls** (tile vide, sprite transparent). Le filtrage des tiles vides relève du consommateur (SPEC-04).
- Critère d'acceptation : vecteur V1 (§5.3) : 960 enregistrements de fond aux plans nuls.

#### 3.5.3 Tiles de fond

**REQ-CORE-037** (P1) — **Moment de capture d'une rangée de fond.** Pour chaque ligne visible L, si le rendu du fond est activé (bit 3 de PPUMASK) et que le **scroll Y fin utilisé pour afficher la ligne L vaut 1**, le cœur DOIT émettre une **rangée de fond** construite à partir de l'état du PPU à cet instant : scroll interne (cx, cy, nametable), scroll X fin fx, table de motifs du fond (bit 4 de PPUCTRL), contenu des nametables, mirroring courant et banques CHR en vigueur. Exception : si L = 0 et que le mirroring courant est vertical, aucune rangée n'est émise pour cette ligne.
- Critère d'acceptation : vecteurs V1, V5 et V6 (§5.3).
- Justification de l'exception : c'est une règle de l'original, sans motif fonctionnel connu ; elle supprime, dans les jeux à mirroring vertical (défilement horizontal), la rangée capturée à la ligne 0, placée en y = −1 et donc presque entièrement hors de l'image. Elle est reproduite parce qu'elle change le contenu des lots (V6) et donc la segmentation des profils ; ce n'est pas une erreur de transcription.
- Notes :
  - Sans scroll vertical, les rangées sont capturées aux lignes 1, 9, 17… 233, soit 30 rangées. La capture se fait une fois par rangée de 8 lignes ; avec un scroll vertical fixe, elle a lieu sur la deuxième ligne de la rangée.
  - Conséquence : quand le scroll Y fin en haut de l'écran vaut 2 à 7, la rangée partiellement visible en haut n'est **pas** émise.
  - Un changement de scroll en milieu de frame (barre de statut, split-screen par sprite 0 ou IRQ de mapper) est pris en compte naturellement, puisque chaque rangée reflète l'état du moment où elle est capturée.
  - Tolérance : quand un changement de scroll tombe sur la ligne même de capture, la référence peut appliquer le nouvel état une ligne plus tôt que le matériel. Un écart d'une ligne sur le moment d'application d'un split est toléré.

**REQ-CORE-038** (P1) — **Contenu d'une rangée de fond** capturée à la ligne L. Pour k = 0, 1, …, 32 :
- colonne de nametable c = cx + k. Si c ≤ 31, la tile vient de la nametable courante, colonne c. Sinon, elle vient de la nametable **horizontalement voisine** (nametable logique dont le bit de sélection horizontale est inversé, puis mirroring appliqué), colonne c − 32 ;
- rangée de nametable : cy ;
- x = 8k − fx ; y = L − 1 (c'est-à-dire L − fy, la ligne écran de la rangée de pixels 0 de la tile) ;
- motif : le numéro de tile lu dans la nametable, dans la table de motifs du fond ;
- palette : les 2 bits de la table d'attributs de la même nametable correspondant à (c, cy) ;
- miroirH = miroirV = faux ; caché = faux ; fond = vrai.

Un enregistrement dont x ≥ 256 NE DOIT PAS être émis : c'est le cas de k = 32 quand fx = 0. Une rangée compte donc 32 enregistrements si fx = 0, et 33 sinon.
- Critère d'acceptation : vecteurs V1 et V4 (§5.3).
- Notes : avec cy = 30 ou 31 (scroll « négatif » de certains jeux), les octets d'attributs sont lus comme des numéros de tiles, ce qui correspond au comportement matériel.

**REQ-CORE-039** (P1) [ROM] — Le motif de chaque tile de fond DOIT être celui que le matériel afficherait pour cette tile sur la ligne de capture. En particulier, pour MMC2 et MMC4 (mappers 9 et 10), il tient compte des bascules de verrou déclenchées par les tiles `$FD` et `$FE` situées plus à gauche sur la même ligne. Les lectures restent toutefois sans effet de bord (REQ-CORE-033) : seul l'affichage réel fait basculer les verrous.
- Critère d'acceptation : ROM de test construite comme au §5.1 avec le mapper 9 (MMC2) : banques CHR distinctes sélectionnées par les verrous `$FD` et `$FE` ; une rangée de nametable contient la tile `$FE` en colonne 2, la tile `$FD` en colonne 8 et la tile `$41` en colonnes 5 et 12 → dans le lot, l'enregistrement de la colonne 5 porte les plans de `$41` de la banque « `$FE` » et celui de la colonne 12 les plans de `$41` de la banque « `$FD` » ; la RAM et les lots sont identiques avec et sans rappel vidéo (REQ-CORE-033). [ROM] Punch-Out!! : dans les lots, les tiles de l'adversaire et du ring correspondent à l'image 2D d'un émulateur de référence public à la même frame.

**REQ-CORE-040** (P3) — **Indice des tiles de fond** : indice = ⌊L / 8⌋ × 33 + k, où L est la ligne de capture et k le rang dans la rangée (0 à 32). L'unicité n'est pas garantie lors de changements de scroll en milieu de frame.
- Critère d'acceptation : vecteur V1 : la tile de la rangée capturée en L = 9, k = 4 a pour indice 37.
- Notes : le moteur 3D de référence place les tiles de fond d'après x et y, pas d'après l'indice.

#### 3.5.4 Tiles de sprites

**REQ-CORE-041** (P1) — **Moment de capture des sprites.** À chaque ligne visible L telle que L mod 8 = 7, si le rendu des sprites est activé (bit 4 de PPUMASK), le cœur DOIT émettre un enregistrement pour **chacun des 64 sprites** de l'OAM dont la ligne écran du haut, Ytop = (octet Y de l'OAM) + 1, vérifie à la fois ⌊Ytop / 8⌋ = ⌊L / 8⌋ et Ytop < 240. L'état utilisé est celui de l'OAM, de PPUCTRL et des banques CHR au moment de la ligne L.
- Critère d'acceptation : vecteurs V2, V3 et V8 (§5.3). Un sprite d'OAM Y = `$EF` (Ytop = 240) ou Y = `$FF` n'est jamais émis. Un sprite d'OAM Y = 0 est émis avec y = 1 lors de la capture de la ligne 7.
- Notes :
  - La limite matérielle de 8 sprites par ligne **ne s'applique pas** aux enregistrements : les 64 sprites sont émis, même quand le matériel n'en afficherait que 8 sur une ligne. C'est un choix de 3dSen, qui évite le clignotement en 3D. Le comportement émulé (drapeau d'overflow, affichage 2D éventuel) suit, lui, le matériel.
  - La masquage de la colonne de gauche (bits 1 et 2 de PPUMASK) ne retire **aucun** enregistrement. Il est seulement signalé par maskX (REQ-CORE-046).

**REQ-CORE-042** (P1) — **Contenu d'un enregistrement de sprite** (sprite n, octets OAM Y, T, A, X) :
- x = X (0 à 255) ; y = Y + 1 ;
- miroirH = bit 6 de A ; miroirV = bit 7 de A ; caché = bit 5 de A ; palette = bits 0 et 1 de A ; fond = faux ;
- **sprites 8×8** (bit 5 de PPUCTRL = 0) : motif T dans la table de motifs des sprites (bit 3 de PPUCTRL) ;
- **sprites 8×16** : table = bit 0 de T (`$0000` ou `$1000`), paire de tiles (T & `$FE`, T | 1). Le cœur émet deux enregistrements :
  - **haut** : indice n, y = Y + 1, motif T & `$FE` (ou T | 1 si miroirV) ;
  - **bas** : indice 64 + n, y = Y + 9, motif T | 1 (ou T & `$FE` si miroirV) ; il n'est émis que si Y + 9 < 240.

  Les deux portent les mêmes x, palette, miroirs et caché. Chaque moitié est retournée individuellement par miroirV.
- Critère d'acceptation : vecteurs V2 et V3 (§5.3).

**REQ-CORE-043** (P1) — **Indice des sprites** : n (0 à 63) pour un sprite 8×8 ou la moitié haute d'un 8×16, 64 + n pour la moitié basse. Le moteur 3D l'utilise comme emplacement de sprite et comme ordre de priorité (SPEC-04).
- Critère d'acceptation : V3 : indices 2 et 66.

#### 3.5.5 Ordre, palette, marges, effets non transmis

**REQ-CORE-044** (P2) — **Ordre des enregistrements dans le lot** : ordre chronologique des lignes de capture. Pour une même ligne, les sprites de la ligne sont émis **avant** la rangée de fond. Entre eux, les sprites sont émis par numéro OAM croissant, la moitié haute d'un 8×16 suivie immédiatement de sa moitié basse. Exception : pour MMC2 et MMC4 (mappers 9 et 10), les sprites d'une même capture sont émis par X croissant, et par numéro OAM croissant à X égal (départage déterministe exigé par REQ-ARCH-012). La référence emploie un tri non stable qui peut, à X égal, s'écarter de cet ordre ; l'écart est accepté **(D)**, sans effet connu sur les profils embarqués.
- Critère d'acceptation : V8 (§5.3) : rangée de fond (L = 1), puis sprite (L = 7), puis rangée de fond (L = 9).

**REQ-CORE-045** (P1) — **Palette du lot** : copie des 32 octets de la RAM de palette du PPU (`$3F00` à `$3F1F`) au moment du rappel. Chaque valeur est un indice de couleur NES de 0 à 63 (6 bits). L'entrée 4p + c (p = 0 à 3, c = 1 à 3) est la couleur c de la sous-palette de fond p ; l'entrée 16 + 4p + c est la couleur c de la sous-palette de sprite p ; l'entrée 0 est la couleur d'arrière-plan universelle.
- Les entrées `$10`, `$14`, `$18` et `$1C` sont des miroirs de `$00`, `$04`, `$08` et `$0C` (nesdev). Le cœur DEVRAIT y placer la valeur miroir.
- Le consommateur DOIT utiliser l'entrée 0 pour la couleur 0 de toutes les sous-palettes, et ignorer les entrées 4, 8, 12, 16, 20, 24 et 28.
- La palette transmise reflète l'état **à la fin de la frame**. Les changements de palette en milieu de frame ne sont pas représentés.
- Critère d'acceptation : V7 (§5.3).

**REQ-CORE-046** (P1) — **maskX, maskY** : largeur, en pixels NES, des bandes de bord de l'image que le consommateur doit considérer comme peu fiables (artefacts de scroll, colonne masquée). maskX s'applique aux bords gauche **et** droit, maskY aux bords haut **et** bas (usage : SPEC-04). Valeurs, déterminées à partir de l'état au début de la ligne visible 0 de la frame :

| Mirroring courant | maskX | maskY |
|---|---|---|
| horizontal | 8 | 0 |
| vertical | 0 | 8 |
| quatre écrans ou écran unique | 0 | 0 |

Ensuite, si le bit 1 **ou** le bit 2 de PPUMASK vaut 0 (masquage de la colonne de gauche actif pour le fond ou pour les sprites), maskX vaut au moins 8.
- Critère d'acceptation : V9 (§5.3).

**REQ-CORE-047** (P2) — Les bits de **niveaux de gris** (bit 0 de PPUMASK) et d'**accentuation de couleur** (bits 5 à 7) NE DOIVENT modifier ni les enregistrements ni la palette transmise.
- Critère d'acceptation : V10 (§5.3) : lots identiques avec PPUMASK `$1E` et `$FF`.

**REQ-CORE-048** (P3) — Le cœur PEUT produire en plus une image 2D de 256 × 240 pixels (indices de couleur NES), par exemple pour un mode de débogage. Cette image n'est pas exigée.
- Critère d'acceptation : sans objet.

### 3.6 Zapper (pistolet optique)

**REQ-CORE-060** (P1) — Pour un port de type Zapper, l'hôte fournit au cœur, **une fois par frame, au début de l'exécution de la frame**, la position visée en pixels NES (x, y entiers, éventuellement hors de [0, 256[ × [0, 240[ ; SPEC-08 REQ-INP-063/064). La position reste fixe pendant la frame.
- Critère d'acceptation : test unitaire : la position lue pendant la frame N est celle fournie avant l'appel N.

**REQ-CORE-061** (P1) — **Octet lu par le jeu** sur un port Zapper (`$4016` pour le port 1, `$4017` pour le port 2) :
- bit 4 = bit 4 de l'octet du port dans le mot d'entrées, recopié tel quel. Convention de l'hôte de référence, inscrite dans SPEC-08 §4.1 (DEC-02-Q1) : 1 quand la gâchette est relâchée, 0 quand elle est pressée ;
- bit 3 = 0 si de la lumière est détectée (REQ-CORE-062), 1 sinon ;
- les autres bits sont nuls.

Une lecture sur un port Zapper ne fait pas avancer de registre série.
- Critère d'acceptation : mot dont l'octet du port 2 vaut `0x10`, pointeur hors écran → lecture de `$4017` = `0x18` ; octet `0x00`, pointeur sur un pixel blanc déjà balayé → `0x00`.

**REQ-CORE-062** (P1) [ROM] — **Détection de lumière.** À une lecture du port faite pendant que le PPU est à la ligne Lc, la lumière est détectée si et seulement si :
1. la position visée (x, y) est dans l'image : 0 ≤ x < 256, 0 ≤ y < 240 ;
2. le balayage a dépassé ce pixel dans la frame courante : Lc > y, la période de vblank comptant comme « après » toutes les lignes visibles ;
3. la couleur affichée en (x, y) est « claire ».

La couleur affichée est déterminée à partir des enregistrements déjà émis dans la frame courante, selon la composition NES : un pixel non transparent du premier sprite (indice le plus faible) non « caché » ; sinon un pixel non transparent du fond ; sinon un pixel non transparent d'un sprite « caché » ; sinon la couleur universelle (entrée 0 de la palette).

Est « claire » toute couleur NES **sauf** `$0D`, `$0E`, `$0F`, `$1E`, `$1F`, `$2E`, `$2F`, `$3E` et `$3F` (les noirs).

Le résultat PEUT être recalculé une seule fois par groupe de 8 lignes et réutilisé dans le groupe.
- Critère d'acceptation : [ROM] Duck Hunt (profil 15), Hogan's Alley (20) et Wild Gunman (44) : un tir pointé sur une cible la touche, un tir à côté la manque (10 essais chacun, au moins 9 conformes). Test unitaire (sans ROM commerciale) : fond noir (`$0F`), un sprite à palette blanche (`$30`) en (100, 100) ; pointeur (103, 103) → lumière à partir de la ligne 104, et pas avant.
- Notes : la classification « clair / sombre » correspond à un seuil de luminance très bas : seuls les noirs sont sombres. Comme la lumière est mémorisée jusqu'à la fin de la frame, un jeu qui lit le Zapper pendant le vblank voit encore la lumière de la frame qui vient d'être rendue.

### 3.7 Audio

**REQ-CORE-070** (P1) — Le cœur DOIT produire un signal audio **mono**, rééchantillonné à la **fréquence de sortie** choisie par l'hôte (par défaut 48 000 Hz, configurable de 22 050 à 96 000 Hz). Le rééchantillonnage est à bande limitée : sur un balayage de fréquence du canal pulse de 100 Hz à 10 kHz, les produits de repliement (composantes spectrales qui ne sont pas des harmoniques du signal émulé) restent à −60 dBFS ou moins. Il est calé de sorte qu'une frame émulée à vitesse 1 produise en moyenne `fréquence de sortie / cadence nominale` échantillons (REQ-CORE-015), soit 800 échantillons à 48 kHz en NTSC et 960 en PAL.
- Critère d'acceptation : 600 frames NTSC à 48 kHz → 480 000 ± 800 échantillons produits ; analyse spectrale du balayage ci-dessus : aucun produit de repliement au-dessus de −60 dBFS.
- Notes : le cœur DOIT en outre accepter un ajustement fin du rapport de rééchantillonnage demandé par le frontend (± 0,5 % au plus, SPEC-11 REQ-AUD-005), qui s'ajoute au calage ci-dessus sans modifier l'émulation. Caler la production sur la cadence nominale (60 ou 50 fps) plutôt que sur la fréquence exacte du matériel (60,0988 Hz) garde l'audio synchrone avec l'hôte, qui enchaîne les frames à la cadence nominale. Le son est alors plus aigu d'environ 0,16 %, comme dans la référence.

**REQ-CORE-071** (P1) — **Lecture de l'audio** : l'hôte demande à remplir un tampon de N valeurs flottantes, en mono ou en stéréo.
- En stéréo, le tampon est entrelacé (G, D, G, D…) avec G = D, et N/2 échantillons mono sont consommés.
- La lecture est **tout ou rien** : si le cœur dispose de moins d'échantillons qu'il n'en faut pour remplir le tampon entier, il renvoie 0 sans rien consommer, et l'hôte complète par du silence. Sinon, il remplit entièrement le tampon et renvoie le nombre de valeurs écrites (N).
- Les valeurs sont dans [−1, 1].
- La lecture DOIT pouvoir être appelée depuis un autre thread que celui qui exécute les frames (un producteur, un consommateur) sans corruption.
- Le tampon interne contient au moins 10 000 échantillons. En cas de débordement, les échantillons les plus anciens DEVRAIENT être abandonnés.
- Critère d'acceptation : après une frame à 48 kHz (environ 800 échantillons disponibles), demander 1 024 valeurs mono → 0 ; demander 512 → 512, puis il reste environ 288 échantillons ; demander 512 valeurs stéréo → 512, dont 256 échantillons consommés, avec G = D.

**REQ-CORE-072** (P2) — **Vitesse** : régler un facteur de vitesse s > 0 (s = 1,5 pendant l'avance rapide, SPEC-07 REQ-LOOP-016) modifie uniquement le calage audio : chaque frame émulée produit `fréquence / (cadence × s)` échantillons en moyenne, et le son est donc accéléré (tempo et hauteur multipliés par s, l'hôte enchaînant s fois plus de frames). L'émulation elle-même n'est pas modifiée.
- Critère d'acceptation : s = 2, 600 frames NTSC à 48 kHz → 240 000 ± 800 échantillons ; s = 0,5 → 960 000 ± 800.

**REQ-CORE-073** (P2) — **Mixage** : les cinq canaux (2 pulses, triangle, bruit, DMC) sont mixés selon l'**approximation linéaire** de nesdev (« APU Mixer », linear approximation) :
- pulse : 0,00752 par pas et par canal ;
- triangle : 0,00851 ;
- bruit : 0,00494 ;
- DMC : 0,00335.

La sortie vaut 1,526 × mix, bornée à [−1, 1], suivie d'un filtre passe-haut coupant la composante continue (fréquence de coupure de 10 à 40 Hz).
- Critère d'acceptation : un pulse seul au volume 15 (rapport cyclique 50 %) produit un signal crête à crête de 0,172 ± 0,02 après stabilisation ; un silence prolongé (tous canaux à niveau constant) tend vers 0 en moins de 0,2 s.
- Notes : 1,526 ≈ 50 000 / 32 768 est le gain de la référence. Le mixage non linéaire de nesdev est toléré si les niveaux relatifs restent à ±10 % (divergence acceptée).

**REQ-CORE-074** (P2) — La restauration d'un savestate et la réinitialisation DOIVENT vider les échantillons audio en attente. L'audio repart du silence.
- Critère d'acceptation : 10 frames produites et non lues, restaurer un état → la lecture suivante d'un tampon de 512 renvoie 0 tant qu'une nouvelle frame n'a pas été exécutée.

**REQ-CORE-075** (P3) — L'audio d'extension des cartouches (VRC6, VRC7, MMC5, Namco 163, Sunsoft 5B, FDS) PEUT être émulé. Il n'est pas exigé : la référence ne l'émule pas.
- Critère d'acceptation : sans objet.

### 3.8 Accès mémoire (lecture et écriture d'octets)

**REQ-CORE-080** (P1) — **Lecture d'un octet** à une adresse CPU de 16 bits, appelable entre deux frames et pendant le rappel vidéo (usage : scripts Lua, SPEC-05) :
- `$0000` à `$1FFF` : RAM CPU (2 Kio en miroir) ;
- `$6000` à `$7FFF` : PRG-RAM ;
- `$8000` à `$FFFF` : PRG-ROM telle que vue par le CPU avec les banques courantes.

Ces lectures sont **sans effet de bord**.
- Critère d'acceptation : écrire `0x42` en `$0012` par une écriture mémoire, puis lire `$0012` et `$0812` → `0x42` ; lire `$FFFC` → l'octet bas du vecteur de reset de la banque courante.
- Notes : les 101 profils embarqués ne lisent que la RAM CPU (adresses `$0001` à `$075F` relevées). La lecture s'appelle `ReadMem` côté Lua (SPEC-05).

**REQ-CORE-081** (P2) — Les lectures des registres (`$2000` à `$401F`) et de la zone `$4020` à `$5FFF` DEVRAIENT être sans effet de bord et renvoyer 0 (ou la valeur d'un registre de mapper lisible, sans effet de bord).
- Critère d'acceptation : lire `$2002` pendant le vblank, puis laisser le jeu lire `$2002` → le jeu voit toujours le drapeau VBlank à 1.
- Notes : la référence effectue ici une vraie lecture de bus, avec effets de bord. Aucun profil embarqué n'en dépend.

**REQ-CORE-082** (P2) — **Écriture d'un octet** : effet identique à une écriture du CPU sur le bus à cette adresse (RAM, PRG-RAM si activée, registres PPU et APU, registres de mapper), sans faire avancer le temps émulé.
- Critère d'acceptation : écrire `0x3F` en `$2006`, `0x00` en `$2006` puis `0x30` en `$2007` (hors période d'échauffement du PPU) → l'entrée 0 de la palette du lot suivant vaut `0x30`.
- Notes : aucun profil embarqué n'écrit en mémoire (fonction Lua `WriteMem`, SPEC-05). L'écriture sert aux vecteurs de test §5.

### 3.9 Savestates

**REQ-CORE-090** (P1) — « Sauvegarder l'état » DOIT renvoyer un bloc d'octets **autonome** contenant tout ce qu'il faut pour reprendre l'émulation à l'identique :
- état complet du CPU, du PPU, de l'APU et du mapper (banques, compteurs d'IRQ, verrous) ;
- RAM CPU, PRG-RAM, CHR-RAM, VRAM de nametables (4 Kio pour couvrir le mode quatre écrans), OAM, palette ;
- état des ports d'entrée ;
- compteur de frames ;
- **nom de la ROM** ;
- **image complète de la ROM** (en-tête, PRG et CHR).

Le bloc commence par un numéro de version du format.
- Critère d'acceptation : sauvegarder un état, fermer la ROM, puis restaurer **sans recharger de ROM** → l'émulation reprend : lot de frame et RAM identiques à ceux de l'exécution non interrompue.
- Notes : l'hôte de référence restaure un emplacement de sauvegarde de la bibliothèque sans recharger la ROM : l'autonomie du bloc est donc indispensable. La compatibilité avec les fichiers de la référence n'est **pas** exigée (SPEC-09).

**REQ-CORE-091** (P1) — « Restaurer l'état » prend un tableau d'octets et une **position de départ** dans ce tableau. Elle renvoie le **nombre d'octets consommés** et le **nom de ROM** stocké, pour que l'hôte puisse lire d'autres données placées à la suite (état Lua, état du moteur 3D, SPEC-09). En cas de bloc invalide ou tronqué, la restauration DOIT échouer et le signaler (erreur « savestate corrompu »), laisser le cœur arrêté (REQ-CORE-005) et renvoyer un nom vide. Le nom renvoyé permet à l'hôte de mettre à jour le jeu courant ou de vérifier qu'il correspond (SPEC-07, SPEC-09).
- Critère d'acceptation : concaténer 7 octets quelconques, un bloc d'état puis 100 octets ; restaurer à la position 7 → valeur renvoyée = taille du bloc ; un bloc tronqué de moitié → échec, cœur arrêté.

**REQ-CORE-092** (P1) — **Déterminisme** : à partir du même état et avec la même suite de mots d'entrées (et de positions Zapper), le cœur DOIT produire exactement les mêmes lots de frame, la même mémoire et les mêmes échantillons audio (à fréquence de sortie égale), quel que soit l'environnement. Sauvegarder puis restaurer immédiatement ne modifie rien.
- Critère d'acceptation : exécuter 1 000 frames d'entrées pseudo-aléatoires fixées → empreinte E1 des lots et de la RAM. Recommencer en sauvegardant et restaurant à la frame 500 → empreinte identique.

**REQ-CORE-093** (P2) — Performance : sauvegarder ou restaurer un état d'une ROM de 512 Kio DEVRAIT prendre moins de 2 ms sur la machine de test (SPEC-12). Le rollback de SPEC-07 sauvegarde fréquemment. Pour son historique en mémoire, le cœur PEUT offrir une variante allégée sans image de ROM, à condition qu'elle ne soit jamais écrite sur disque.
- Critère d'acceptation : banc de mesure sur 1 000 sauvegardes et restaurations.

### 3.10 PRG-RAM sauvegardée par pile

**REQ-CORE-095** (P3) — La référence **ne persiste pas** la PRG-RAM à pile (pas de fichier .sav de cartouche) ; le joueur passe par les savestates. Le cœur DEVRAIT néanmoins exposer, pour une ROM dont le bit 1 de l'octet 6 est à 1, un accès en lecture et en écriture à la PRG-RAM (8 Kio), pour qu'un hôte PUISSE la persister (notamment sous libretro : zone mémoire « SAVE_RAM »). Si l'hôte la recharge, la réinitialisation (REQ-CORE-004) NE DOIT alors PAS l'effacer.
- Critère d'acceptation : écrire la PRG-RAM, l'exporter, recharger la ROM, l'importer → `$6000` à `$7FFF` identiques.

### 3.11 Mappers exigés

**REQ-CORE-100** (P1) — Le cœur DOIT supporter les cartes suivantes, avec le comportement documenté sur nesdev (banques PRG et CHR, mirroring piloté, IRQ, PRG-RAM) :

| iNES | Carte(s) | Remarques |
|---|---|---|
| 0 | NROM | |
| 1 | MMC1 (SxROM), y compris SUROM et SOROM (PRG 512 Kio) | |
| 2 | UxROM | |
| 3 | CNROM | |
| 4 | MMC3 (TxROM), MMC6 | IRQ de ligne, comportement « rev B » |
| 5 | MMC5 (ExROM) | au minimum : modes de banques PRG et CHR, PRG-RAM, IRQ de ligne. ExRAM, attributs étendus, split vertical : P2 |
| 7 | AxROM | écran unique |
| 9 | MMC2 (PxROM) | verrous `$FD`/`$FE` |
| 10 | MMC4 (FxROM) | verrous |
| 11 | Color Dreams | |
| 13 | CPROM | CHR-RAM 16 Kio |
| 15 | 100-in-1 Contra Function 16 | |
| 16 | Bandai FCG | IRQ ; EEPROM : P3 |
| 18 | Jaleco SS88006 | |
| 21, 22, 23, 25 | Konami VRC2 / VRC4 (toutes variantes de câblage) | IRQ VRC4 |
| 27 | VRC4 pirate (CHR étendue) | |
| 28 | Action 53 | |
| 30 | UNROM 512 | CHR-RAM 32 Kio |
| 32 | Irem G-101 | |
| 34 | BNROM et NINA-001 | |
| 41 | Caltron 6-in-1 | |
| 64 | Tengen RAMBO-1 | |
| 66 | GxROM | |
| 71 | Camerica / Codemasters | |
| 87 | Jaleco J87 | |
| 101 | Jaleco (variante de 87) | |
| 225 | multicart 52/64-in-1 | |
| 228 | Action 52 | |

- Critère d'acceptation : chaque mapper de la liste démarre au moins une ROM publique de test ou un homebrew libre (par exemple la suite « Holy Mapperel » pour les mappers qu'elle couvre). [ROM] Les 101 titres des profils embarqués démarrent et sont jouables jusqu'au premier niveau (§3.12, REQ-CORE-115).

**REQ-CORE-101** (P3) — Le mapper 116 PEUT être supporté. La référence l'accepte mais l'émule seulement comme une carte VRC2 simple, de façon incomplète.
- Critère d'acceptation : sans objet si non supporté ; un chargement refusé produit le message de REQ-CORE-011.

**REQ-CORE-102** (P2) — Tout autre numéro de mapper DOIT être refusé au chargement (REQ-CORE-011). Ajouter des mappers au-delà de la liste est permis.
- Critère d'acceptation : en-tête annonçant le mapper 200 → échec.

### 3.12 Exactitude

**REQ-CORE-110** (P1) — CPU : `nestest.nes` en mode automatique (départ en `$C000`) DOIT produire une trace identique au journal public `nestest.log` (registres, cycles, opcodes non officiels compris) jusqu'à la fin du test. La suite blargg `instr_test-v5` (all_instrs) DOIT passer.
- Critère d'acceptation : comparaison automatique de la trace ; `instr_test-v5` affiche « Passed ».

**REQ-CORE-111** (P2) — Minutage CPU et interruptions : `instr_timing`, `cpu_timing_test6`, `cpu_interrupts_v2` et `branch_timing_tests` DOIVENT passer.
- Critère d'acceptation : chaque ROM affiche « Passed » (ou son code de succès).

**REQ-CORE-112** (P1) — PPU, tests bloquants : `sprite_hit_tests_2005.10.05` (11 tests), `ppu_vbl_nmi` tests 1 à 3, `blargg_ppu_tests_2005.09.15b` (palette_ram, sprite_ram, vram_access, vbl_clear_time) DOIVENT passer. Le test power_up_palette est exclu : la palette démarre à 0.
- Critère d'acceptation : chaque ROM affiche « Passed ».
- Notes : la référence n'émule qu'à la ligne près et échoue à plusieurs de ces tests. L'exactitude exigée ici ne change pas le contrat des tiles, dont les moments de capture sont définis aux §3.5.3 et §3.5.4.

**REQ-CORE-116** (P2) — PPU, tests importants : `ppu_vbl_nmi` tests 4 à 10, `vbl_nmi_timing`, `oam_read` et `ppu_open_bus` DEVRAIENT passer. `sprite_overflow_tests` PEUT passer (P3).
- Critère d'acceptation : chaque ROM affiche « Passed ».

**REQ-CORE-113** (P2) — APU : `apu_test` (tests 1 à 8) et `blargg_apu_2005.07.30` DOIVENT passer. `dmc_dma_during_read4` : P3.
- Critère d'acceptation : chaque ROM affiche « Passed ».

**REQ-CORE-114** (P1) [ROM] — Mappers : `mmc3_test_2` tests 1 à 5 DOIVENT passer (le test 6, « MMC3 alt », peut échouer). Le comportement des IRQ MMC3 DOIT être exact à la ligne près pour les barres de statut (Super Mario Bros. 3, Mega Man 3, Ninja Gaiden III).
- Critère d'acceptation : chaque ROM affiche « Passed » ; [ROM] la barre de statut de SMB3 ne tremble pas en jeu (inspection sur 60 s).

**REQ-CORE-115** (P1) [ROM] — **Jeux des profils embarqués** : chacun des 101 titres de `assets-neutral/profiles.json` (ROM commerciale ou homebrew fournie par l'utilisateur) DOIT démarrer, afficher son écran titre et être jouable jusqu'au premier niveau, sans plantage ni blocage, en NTSC (en PAL ou DENDY pour les versions de ces régions). Points sensibles :
- jeux au Zapper : Duck Hunt, Hogan's Alley, Wild Gunman ;
- verrous MMC2 : Punch-Out!! ;
- IRQ MMC3 et changements de banques CHR en milieu de frame : Super Mario Bros. 3, Mega Man 3 ;
- split par sprite 0 : Super Mario Bros. ;
- scroll vertical en milieu de frame par `$2006` : The Legend of Zelda, Metroid ;
- grandes ROMs MMC1 : Dragon Warrior III, Final Fantasy.

Le profil « 2D Mode » est générique.
- Critère d'acceptation : grille de recette du lot L1, un jeu par ligne (écran titre, début du niveau 1, 2 minutes de jeu), avec capture et verdict.

### 3.13 Choix du cœur

**REQ-CORE-120** (P3) — La référence ne contient qu'**un seul** cœur utilisable. Un réglage « Core » existe : la liste ne contient qu'une entrée, et le nom du cœur est enregistré en tête des fichiers de savestate (SPEC-09). L'implémentation DOIT fournir un seul cœur. Si le sélecteur de cœur est affiché (SPEC-10), il ne propose que ce cœur. Restaurer un savestate créé par un autre cœur DOIT échouer avec le message « Save state used a different core. »
- Critère d'acceptation : la liste des cœurs a une seule entrée ; un fichier dont l'en-tête porte un autre nom de cœur est refusé avec ce message.

## 4. Formats et interfaces

### 4.1 Enregistrement de tile

| Champ | Type | Bornes et valeurs |
|---|---|---|
| plan0 (« low ») | entier 64 bits | REQ-CORE-034 |
| plan1 (« high ») | entier 64 bits | REQ-CORE-034 |
| x | entier signé 16 bits | fond : −7 à 255 ; sprite : 0 à 255 |
| y | entier signé 16 bits | fond : −1 à 238 en général (REQ-CORE-038) ; sprite : 1 à 239 (moitié basse jusqu'à 239) |
| indice | entier 32 bits | fond : REQ-CORE-040 ; sprite : 0 à 127 |
| miroirH | booléen | sprite : bit 6 de l'attribut ; fond : faux |
| miroirV | booléen | sprite : bit 7 de l'attribut ; fond : faux |
| caché | booléen | sprite : bit 5 de l'attribut (derrière le fond) ; fond : faux |
| palette | octet | 0 à 3 |
| fond | booléen | vrai = tile de fond |

Disposition binaire recommandée, utile seulement pour une interface native (par exemple libretro ou une bibliothèque C) : structure compactée sans alignement de 29 octets, dans l'ordre du tableau, entiers en petit-boutiste, booléens sur 1 octet (0 ou 1). L'ordre de cette disposition n'est pas exigé pour une interface interne .NET.

### 4.2 Palette du lot
32 octets, indices de couleur NES de `0x00` à `0x3F` ; voir REQ-CORE-045.

### 4.3 Infos vidéo
(largeur, hauteur, nombre de sprites) = (33, 31, 128).

### 4.4 Mot d'entrées

| Bits | Contenu |
|---|---|
| 0 à 7 | port 1 : manette = A, B, Select, Start, Haut, Bas, Gauche, Droite ; Zapper = bit 4 « gâchette » (REQ-CORE-061) |
| 8 à 15 | port 2 : même codage |
| 16 et au-delà | ignorés par le cœur |

Les actions frontend codées au-delà de `0xFF` dans le masque d'un joueur (SPEC-08 §4.1 : ZoomIn `0x100`, ZoomOut `0x200`, RotLeft `0x400`, RotRight `0x800`, RotUp `0x1000`, RotDown `0x2000`, Menu `0x4000`, RollBack `0x8000`, FastForward `0x10000`, QuickSave `0x20000`, QuickLoad `0x40000`) sont retirées par l'hôte avant la composition du mot d'entrées.

### 4.5 Régions
Énumération : NTSC = 0, PAL = 1, MULTI = 2, DENDY = 3 (valeurs visibles des réglages persistés, SPEC-09). Minutages : REQ-CORE-015.

## 5. Vecteurs de test

### 5.1 ROM de test minimale (à construire par le vérificateur)
Construction :
- en-tête iNES : `4E 45 53 1A 02 01 01 00 00…` (2 × 16 Kio de PRG, 1 × 8 Kio de CHR, mirroring vertical, mapper 0). Pour les vecteurs en mirroring horizontal, l'octet 6 vaut `00` ;
- PRG : `$8000` contient `4C 00 80` (boucle infinie) ; vecteurs NMI, RESET et IRQ = `$8000` ;
- CHR : la tile n (0 à 255) de chaque table a pour plan 0 huit octets égaux à n, et pour plan 1 huit octets égaux à (255 − n). Exception : la tile 0 de la table `$0000` est entièrement nulle.

Mise en place :
1. exécuter 3 frames, pour dépasser l'échauffement du PPU (environ 29 658 cycles CPU) ;
2. programmer l'état voulu par des **écritures mémoire** (REQ-CORE-082) dans `$2000` à `$2007` : nametables et palette par `$2006`/`$2007`, OAM par `$2003`/`$2004`, scroll par deux écritures de `$2005` (en nombre pair depuis la dernière écriture de `$2006`), puis PPUCTRL et PPUMASK ;
3. exécuter une frame, puis examiner le lot de la frame **suivante**.

Avec t = scroll écrit, le registre interne est rechargé à la ligne de pré-rendu ; la frame suivante reflète donc entièrement l'état programmé.

Notation des motifs : M(table, n) désigne les 16 octets du motif n de la table ; par exemple plan0(M($0000, 3)) = `0x0303030303030303`.

### 5.2 En-têtes

| Octets 4 à 15 de l'en-tête | Attendu |
|---|---|
| `02 01 01 00 00 00 00 00 00 00 00 00` | mapper 0, vertical, CHR-ROM 8 Kio, NTSC |
| `08 00 12 00 00 00 00 00 00 00 00 00` | mapper 1, horizontal, PRG-RAM à pile, CHR-RAM 32 Kio |
| `02 01 41 44 69 73 6B 44 75 64 65 21` (« DiskDude! ») | mapper 4 (quartet haut de l'octet 7 ignoré), vertical, NTSC |
| `02 00 23 64 00 00 00 00 00 00 00 00` | mapper 2 (cas particulier) |
| `02 01 08 00 00 00 00 00 00 00 00 00` | quatre écrans |
| `02 01 00 08 00 00 00 00 03 00 00 00` (NES 2.0, octet 12 = 3) | DENDY |
| `02 01 00 00 00 01 00 00 00 00 00 00` | PAL (octet 9, bit 0) |

### 5.3 Contrat des tiles
Sauf mention contraire : PPUCTRL = `$00` (tables `$0000`, sprites 8×8), toutes les nametables à 0, OAM à `$FF`, mirroring vertical.

- **V1 — fond nu.** PPUMASK = `$0A` (fond seul, colonne gauche du fond affichée), scroll (0, 0).
  - Attendu : 960 enregistrements, tous de fond, en 30 rangées capturées aux lignes L = 8r + 1 (r = 0 à 29) ;
  - pour la rangée r, rang k (0 à 31) : x = 8k, y = 8r, indice = 33r + k, palette 0, plans nuls, miroirs et caché faux. Exemple : r = 1, k = 4 → x = 32, y = 8, indice 37 ;
  - aucun sprite ; maskX = 8 (bit 2 de PPUMASK à 0), maskY = 8.
- **V2 — sprite 8×8.** PPUMASK = `$1E`, table des sprites `$1000` (PPUCTRL = `$08`). OAM sprite 5 = (Y `$50`, T `$21`, A `$63`, X `$80`).
  - Attendu, parmi les sprites : un seul enregistrement : fond = faux, x = 128, y = 81, indice 5, palette 3, caché = vrai, miroirH = vrai, miroirV = faux ; plans = M($1000, `$21`) : plan0 = `0x2121212121212121`, plan1 = `0xDEDEDEDEDEDEDEDE` ;
  - il est capturé à la ligne 87 ; dans le lot, il suit la rangée de fond capturée en L = 81 et précède celle de L = 89.
- **V3 — sprite 8×16.** PPUCTRL = `$20`, PPUMASK = `$1E`. OAM sprite 2 = (Y `$20`, T `$15`, A `$80`, X `$10`).
  - Attendu : deux enregistrements consécutifs :
    - (indice 2, x 16, y 33, miroirV vrai, motif M($1000, `$15`)) ;
    - (indice 66, x 16, y 41, miroirV vrai, motif M($1000, `$14`)) ;
  - sans miroirV (A = `$00`) : indice 2 → M($1000, `$14`), indice 66 → M($1000, `$15`) ;
  - avec Y = `$E7` (Ytop = 232, moitié basse à 240) : seul l'indice 2 est émis.
- **V4 — scroll X = 13.** PPUMASK = `$0A` ; nametable `$2000` colonne c = tile c ; nametable `$2400` colonne c = tile 32 + c (pour chaque rangée).
  - Attendu par rangée : 33 enregistrements, x = −5, 3, 11, …, 251 ;
  - k = 0 → tile 1 (motif M($0000, 1)) ; k = 30 → tile 31 ; k = 31 → tile 32 (colonne 0 de `$2400`) ; k = 32 → tile 33.
- **V5 — scroll Y = 3.** PPUMASK = `$0A`, rangée de nametable r de `$2000` remplie avec la tile r.
  - Attendu : 30 rangées capturées en L = 8j + 6 (j = 0 à 29) avec y = 8j + 5 ;
  - la première rangée contient la tile 1 (rangée 1 de la nametable), indice de ligne ⌊6/8⌋ = 0 ; la rangée 0 de la nametable (qui serait en y = −3) n'est pas émise ;
  - la dernière (L = 238, y = 237) vient de la rangée 0 de la nametable verticalement voisine, `$2800`, miroir de `$2000` en mirroring vertical : tile 0.
- **V6 — scroll Y = 1.**
  - En mirroring vertical : pas de rangée en L = 0 ; la première rangée est capturée en L = 8, y = 7.
  - En mirroring horizontal (octet 6 = `00`) : une rangée est capturée en L = 0, y = −1, indices 0 à 31.
- **V7 — palette.** Écrire `$3F00` ← `$0F`, `$3F01` ← `$30`, `$3F11` ← `$16`.
  - Attendu : palette[0] = `0x0F`, palette[1] = `0x30`, palette[17] = `0x16` ;
  - puis écrire `$3F10` ← `$21` → palette[0] = `0x21` (et palette[16] = `0x21` si le cœur remplit les miroirs).
- **V8 — ordre.** PPUMASK = `$1E`, sprite 0 en OAM (Y `$00`, T `$01`, A `$00`, X `$08`), les autres à Y = `$FF`.
  - Attendu : 32 enregistrements de fond (L = 1, y = 0), puis le sprite (indice 0, y = 1), puis 32 enregistrements de fond (L = 9, y = 8), etc.
- **V9 — marges.**
  - Mirroring horizontal, PPUMASK `$1E` → (8, 0) ;
  - vertical, `$1E` → (0, 8) ;
  - vertical, `$18` → (8, 8) ;
  - quatre écrans, `$1E` → (0, 0) ;
  - quatre écrans, `$1A` (sprites masqués à gauche) → (8, 0).
- **V10 — effets ignorés.** V2 avec PPUMASK `$FF` au lieu de `$1E` → lot identique (enregistrements, palette, marges).

### 5.4 Audio et Zapper
- 600 frames NTSC à 48 kHz → 480 000 ± 800 échantillons ; avec vitesse 2 → 240 000 ± 800 (REQ-CORE-070, REQ-CORE-072).
- Zapper : la classification claire/sombre des 64 couleurs (REQ-CORE-062) est vérifiée exhaustivement : exactement 9 couleurs sombres.

## 6. Questions ouvertes

- **Q1** — *Close par DEC-02-Q1* (convention conservée, SPEC-08 §4.1). **Polarité de la gâchette.** L'hôte de référence envoie bit 4 = 1 gâchette relâchée, 0 pressée, et le cœur le recopie. nesdev décrit bit 4 = 1 pour une gâchette « à mi-course ». On garde la convention de référence, puisque les profils Zapper fonctionnent ainsi ; à confirmer en test boîte noire sur Duck Hunt.
- **Q2** — *Close par DEC-02-Q2* (règle publique, D). **Détection de région.** La règle publique (REQ-CORE-013) diffère de la référence, qui lit l'octet 10. Faut-il reproduire la règle de référence pour les ROMs iNES 1.0 dont l'octet 10 est non nul ? Proposition : non (divergence D).
- **Q3** — *Close par DEC-02-Q3*. **Reset.** Le reset est équivalent à une mise sous tension (référence). Faut-il plutôt un reset « doux » fidèle au matériel ? Proposition : garder la référence, sauf pour la PRG-RAM à pile si REQ-CORE-095 est implémentée.
- **Q4** — *Close par DEC-02-Q4* (tolérance d'une ligne, différentiel). **Moment de capture par rapport aux écritures en milieu de ligne.** La tolérance d'une ligne (REQ-CORE-037) suffit-elle pour les profils qui reconnaissent les barres de statut ? À vérifier au round 2 par un différentiel des lots sur SMB, SMB3 et Zelda.
- **Q5** — *Close par DEC-02-Q5* (non requis, P3). **Audio d'extension.** Aucun titre embarqué ne semble en dépendre (titres US ou Famicom sans VRC6/VRC7/N163). À confirmer par l'inventaire des ROMs des 101 titres.

## Historique

- 1.1-draft (AMD-001, relecture R2) : phrase LEAK de REQ-CORE-120 supprimée ; seuil de repliement chiffré et ajustement fin du rapport (REQ-CORE-070) ; vecteur MMC2 synthétique (REQ-CORE-039) ; critère de REQ-CORE-002 ; justification de l'exception de REQ-CORE-037 ; départage OAM à X égal (REQ-CORE-044, D) ; polarité Zapper renvoyée à SPEC-08 §4.1 ; vitesse d'avance rapide 1,5 (REQ-CORE-072) ; étiquettes [ROM] ; questions closes.
