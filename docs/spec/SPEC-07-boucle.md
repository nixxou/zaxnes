# SPEC-07 — Boucle d'émulation, états, cadence, rollback et savestates à l'exécution
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-00 (glossaire), SPEC-02 (cœur, régions, savestate du cœur), SPEC-03 (profils), SPEC-04 (conversion 3D, chargement des patterns), SPEC-05 (hooks Lua), SPEC-06 (caméra), SPEC-08 (hotkeys), SPEC-09 (persistance), SPEC-10 (UI, notifications), SPEC-11 (audio)

## 1. Objet et périmètre

Ce document décrit le cycle de vie d'une session de jeu dans le frontend desktop (non VR) : les états de l'émulation et leurs transitions, la cadence des frames émulées (régions, vitesse, avance rapide, ralenti), le chargement d'un jeu, l'historique de rollback, les savestates à l'exécution (quick save, slots), le reset, l'arrêt, la sortie, la perte de focus et les garanties d'ordonnancement nécessaires au déterminisme (frontend libretro).

Hors périmètre :
- format et emplacement des fichiers de savestate, de réglages et de slots : SPEC-09 ;
- contenu exact de l'état sérialisé du cœur : SPEC-02 ;
- enregistrement et relecture de « movies » (fichiers d'entrées fm2/bk2) et enregistrement vidéo de l'écran : exclus de la réimplémentation ;
- mode « ROM » de l'outil de création de profils (ouverture directe d'un fichier .nes/.sav sans bibliothèque, export de données 3D) : hors périmètre ; seul le mode « bibliothèque de profils » est spécifié ;
- estimation de vitesse et d'accélération des contrôleurs VR : exclue (VR hors périmètre) ;
- liaisons physiques des touches : SPEC-08 ; présentation des écrans : SPEC-10 ; comportement audio détaillé : SPEC-11.

## 2. Concepts et vocabulaire

- **Frame émulée** : exécution d'une trame vidéo complète de la console par le cœur (voir SPEC-02), suivie de la conversion 3D de cette trame et du hook Lua de mise à jour (SPEC-04, SPEC-05).
- **Frame de rendu** : image affichée par le frontend. Sa cadence est indépendante de celle des frames émulées.
- **Cadence nominale** (appelée « fréquence nominale » en v1.0) : nombre de frames émulées par seconde demandé par l'hôte pour la région active : **60 images/s en NTSC, 50 images/s en PAL et en Dendy** (DEC-R2-T3, colonne « cadence nominale de l'hôte » de SPEC-02 REQ-CORE-015). Ce n'est pas la fréquence exacte du matériel (≈ 60,0988 Hz en NTSC) : l'original enchaîne les frames à 60 ou 50 par seconde, et l'audio est calé sur cette même cadence (SPEC-02 REQ-CORE-070, SPEC-11).
- **Facteur de vitesse** `k` : multiplicateur appliqué à la fréquence nominale.
- **Jeu courant** : profil de jeu (entrée de la bibliothèque) actuellement chargé dans l'émulateur, ou aucun.
- **Snapshot** : état sérialisé du cœur seul, conservé en mémoire pour le rollback.
- **Savestate** : sauvegarde complète d'une session (cœur, données persistantes des scripts, état de la caméra), écrite dans un slot.
- **Slot** : emplacement de savestate nommé. Il existe 36 slots numérotés (1 à 36) et un slot rapide (« quick »). La persistance des slots relève de SPEC-09.
- **Commande** : demande adressée à la boucle d'émulation (jouer, pause, charger un jeu, sauvegarder…). Les commandes sont appliquées **entre deux frames émulées**, jamais pendant.

## 3. Exigences

### 3.1 États de l'émulation

**REQ-LOOP-001** (P1) — L'émulation DOIT se trouver à tout instant dans exactement un des trois états suivants : **Arrêté** (aucun jeu en cours d'exécution), **En cours** (les frames émulées s'enchaînent à la cadence cible), **En pause** (un jeu est chargé, son état est conservé, aucune frame émulée ne s'exécute). Un indicateur **Chargement**, orthogonal à l'état, DOIT signaler qu'une opération longue (chargement d'un jeu, d'un savestate, régénération des meshes) est en cours.
- Critère d'acceptation : à chaque instant, l'UI ou une API de test renvoie un et un seul des trois états, plus la valeur booléenne de l'indicateur Chargement.
- Notes : il n'existe pas d'état « inactif » distinct d'Arrêté. Au démarrage de l'application, l'état est Arrêté.

**REQ-LOOP-002** (P1) — Au démarrage de l'application, le frontend DOIT charger les réglages (SPEC-09), afficher la bibliothèque (menu principal) dans l'état Arrêté, sans jeu courant, et lancer la musique de menu en fondu (SPEC-11).
- Critère d'acceptation : lancer l'application → menu visible, aucun jeu courant, musique de menu audible après le fondu d'entrée, aucun son d'émulation.

**REQ-LOOP-003** (P1) — Le tableau suivant DOIT être respecté pour la présentation visible dans chaque état :

| État | Vue de jeu 3D | Menu / bibliothèque | Son d'émulation | Musique de menu |
|---|---|---|---|---|
| Arrêté | masquée | affiché, sans vignette « jeu courant » | silence | fondu d'entrée, puis lecture |
| En pause | masquée | affiché, avec la vignette « jeu courant » | silence (fondu de sortie) | fondu d'entrée, puis lecture |
| En cours | affichée | masqué | audible (fondu d'entrée) | fondu de sortie, puis suspendue |
| Chargement (quel que soit l'état) | selon l'état | affiché avec une barre de progression | silence | selon l'état |

- Critère d'acceptation : passer d'En cours à En pause → la vue 3D disparaît et le menu apparaît dans la même frame de rendu (± 1 frame) ; la barre de progression n'est visible que pendant Chargement et reflète l'avancement (0 à 100 %) de la génération des meshes (SPEC-04).
- Notes : le menu étant opaque, la scène 3D n'est pas visible en pause. Les détails de fondu sont dans SPEC-11.

**REQ-LOOP-004** (P1) — Les transitions d'état DOIVENT être exactement celles-ci :

| Depuis | Déclencheur | Vers | Effets visibles |
|---|---|---|---|
| Arrêté | lancement d'un jeu (3.3) réussi | En cours | voir 3.3 |
| Arrêté | chargement d'un savestate réussi (3.5) | En cours | voir 3.5 |
| tout état | lancement d'un jeu ou chargement d'un savestate en échec | Arrêté | notification d'erreur, jeu courant effacé |
| En cours | commande Pause (bouton Menu, appui court) | En pause | capture d'écran de la vue (vignette de slot), menu affiché |
| En pause | commande Jouer (bouton Menu appui court, clic sur la vignette du jeu courant, « Resume ») | En cours | horloge de cadence réinitialisée (pas de rattrapage) |
| En cours ou En pause | commande Rollback (3.4) | En cours | état du cœur restauré |
| En cours ou En pause | relancement du même jeu (« Reset ») ou lancement d'un autre jeu | En cours (après chargement) | voir 3.3 et 3.6 |
| En cours ou En pause | suppression du jeu courant de la bibliothèque | Arrêté | jeu courant effacé, hook Lua de fin exécuté |
| En cours | exception levée par le cœur pendant une frame | Arrêté | retour au menu |
| tout état | demande de sortie | (fin de l'application) | voir 3.6 |

Une commande Pause reçue hors de l'état En cours, ou une commande Jouer reçue hors de l'état En pause, NE DOIT avoir aucun effet.
- Critère d'acceptation : envoyer Jouer dans l'état Arrêté → l'état reste Arrêté ; envoyer Pause dans l'état En pause → aucun changement, aucune nouvelle capture.
- Notes : en cas d'exception du cœur, l'original ne montre aucun message ; la réimplémentation DEVRAIT afficher une notification d'erreur (amélioration tolérée).

**REQ-LOOP-005** (P1) — Le bouton Menu (SPEC-08) DOIT basculer entre En cours et En pause sur un **appui court** (relâché moins de 0,3 s après l'appui). Un appui long (≥ 0,3 s) NE DOIT PAS basculer l'état (il réinitialise la vue caméra, SPEC-06). Si le navigateur de fichiers est ouvert, l'appui court DOIT fermer ce navigateur (équivalent « Annuler ») au lieu de basculer. Le basculement DOIT être ignoré pendant Chargement et lorsqu'une boîte de dialogue est ouverte.
- Critère d'acceptation : appui de 0,1 s en cours de jeu → pause ; appui de 0,5 s → pas de pause ; appui court pendant la barre de progression → rien.

**REQ-LOOP-006** (P2) — Au passage En cours → En pause, une capture de la vue de jeu DOIT être prise **avant** l'affichage du menu. Cette capture sert de vignette au prochain savestate écrit dans un slot depuis le menu, et, si le jeu courant est l'entrée « 2D Mode », remplace sa jaquette dans la bibliothèque.
- Critère d'acceptation : jouer, mettre en pause, sauvegarder dans le slot 3 → la vignette du slot 3 montre la scène au moment de la pause, sans le menu.

**REQ-LOOP-007** (P1) — Les commandes DOIVENT être appliquées entre deux frames émulées, dans l'ordre de leur émission. Pendant Chargement, les commandes reçues DOIVENT être différées (ni perdues silencieusement pendant l'opération, ni exécutées en parallèle) et appliquées une fois Chargement terminé. Les commandes DOIVENT aussi être traitées dans les états En pause et Arrêté.
- Critère d'acceptation : demander une quick save pendant qu'un chargement est en cours → la sauvegarde s'effectue après la fin du chargement et contient l'état post-chargement.
- Notes : l'original ne conserve qu'une commande en attente (la dernière remplace la précédente). Une file FIFO est exigée ici ; fusionner deux commandes identiques consécutives est permis. L'UI empêche déjà la sélection d'un jeu ou d'un slot pendant Chargement (SPEC-10).

### 3.2 Cadence des frames émulées

**REQ-LOOP-010** (P1) — En état En cours, le frontend desktop DOIT exécuter les frames émulées à la cadence `f = cadence nominale de la région × k`, mesurée sur une horloge monotone, indépendamment de la cadence de rendu et de la synchronisation verticale.
- Critère d'acceptation : NTSC, `k = 1`, 10 s mesurées → 600 ± 1 frames émulées ; PAL et Dendy → 500 ± 1. Écart de la cadence moyenne inférieur à 0,1 % sur 60 s.
- Notes : l'émulation n'est **pas** asservie à l'audio (SPEC-11) ni au rafraîchissement de l'écran.

**REQ-LOOP-011** (P1) — L'ordonnancement DOIT viser une granularité d'environ 1 ms : chaque frame est due à l'instant `t0 + n / f` (où `t0` est l'instant de la dernière réinitialisation de l'horloge) et PEUT démarrer jusqu'à 1 ms avant cet instant. L'écart entre deux frames émulées consécutives (hors rattrapage) DEVRAIT rester dans `1/f ± 2 ms`.
- Critère d'acceptation : histogramme des intervalles entre frames à `k = 1` NTSC → 95 % des intervalles dans [14,6 ms ; 18,6 ms].

**REQ-LOOP-012** (P1) — Rattrapage : si l'émulation prend du retard (frame plus longue que `1/f`, système chargé), le frontend DOIT exécuter les frames en retard à la suite, sans attente, **dans la limite de 5 frames de retard**, comptées en frames émulées : le nombre de frames dues et non encore exécutées ne dépasse jamais 5, chaque frame due correspondant à une période `1/f` calculée avec le facteur `k` courant (avance rapide comprise). Le retard au-delà de 5 frames DOIT être abandonné (le jeu ralentit au lieu d'accélérer brutalement ensuite).
- Critère d'acceptation : bloquer la boucle 500 ms à 60 Hz → à la reprise, au plus 5 frames sont exécutées en rafale, puis la cadence normale reprend ; le compteur de frames accuse ≈ 25 frames de moins qu'une exécution sans blocage.

**REQ-LOOP-013** (P1) — L'horloge de cadence DOIT être réinitialisée (aucun rattrapage du temps écoulé) à chaque passage En pause → En cours, à chaque reset, à la fin de chaque chargement (jeu, savestate, régénération des meshes) et à chaque restauration de rollback.
- Critère d'acceptation : pause de 10 s puis reprise → la première seconde après reprise contient 60 ± 1 frames (pas de rafale).

**REQ-LOOP-014** (P2) — Rendu : chaque frame de rendu DOIT afficher le résultat de la dernière frame émulée complètement convertie. Lorsque le réglage « saut de rendu » est actif (défaut : actif) et que l'état est En cours, une frame de rendu pour laquelle aucune nouvelle frame émulée n'est disponible PEUT être omise (l'image précédente reste affichée). La cadence de rendu cible est celle des réglages (synchronisation verticale ou plafond 60/90/120 images/s, SPEC-09/SPEC-10).
- Critère d'acceptation : rendu plafonné à 120 i/s, jeu NTSC → chaque frame émulée apparaît sur une ou deux frames de rendu consécutives ; aucune frame de rendu ne montre un mélange de deux frames émulées.

**REQ-LOOP-015** (P1) — Réglage de vitesse : un réglage entier `s ∈ [−8 ; +8]`, défaut 0, DOIT définir le facteur `k` ainsi : `k = 1 + s/4` si `s ≥ 0` ; `k = 1 / (1 − s/4)` si `s < 0`. Un changement de réglage DOIT prendre effet à la frame émulée suivante, sans réinitialiser l'état du jeu.
- Critère d'acceptation : table de vérité : s = −8 → k = 1/3 ; −4 → 0,5 ; −1 → 0,8 ; 0 → 1 ; +1 → 1,25 ; +4 → 2 ; +8 → 3. NTSC, s = +4, 10 s → 1200 ± 2 frames.
- Notes : il n'y a pas de mode ralenti distinct : les valeurs négatives du réglage en tiennent lieu. L'audio suit le facteur (hauteur modifiée, SPEC-11).

**REQ-LOOP-016** (P2) — Avance rapide : chaque **demande** d'avance rapide (émise à chaque image affichée tant que la hotkey est maintenue, SPEC-08 REQ-INP-103) DOIT, si aucune avance rapide n'est en cours, armer une avance rapide de **30 frames émulées** pendant lesquelles le facteur de vitesse vaut celui du réglage `s = 2`, soit **`k = 1,5`**, quel que soit le réglage de vitesse ; une demande reçue pendant une avance rapide en cours est sans effet. Tant que la touche est maintenue, l'avance rapide est donc réarmée dès qu'elle s'achève (vitesse ×1,5 quasi continue) ; au relâchement, le reliquat est de 0 à 30 frames émulées, puis `k` revient à la valeur du réglage. Pendant le maintien, une notification « Fast Forward » DOIT être affichée (durée 1 s, rafraîchie tant que la touche est tenue).
- Critère d'acceptation : NTSC, s = 0, maintenir 5 s → ≈ 450 frames émulées (±5 %) pendant les 5 s ; au relâchement, au plus 30 frames supplémentaires à ×1,5, puis 60 i/s ; un appui bref isolé → exactement 30 frames à ×1,5.
- Notes : la vitesse ×1,5 est celle de l'original (l'avance rapide y applique la valeur 2 du **réglage** de vitesse, convertie par la formule de REQ-LOOP-015) ; la v1.0 indiquait ×2 par erreur. Si le réglage de vitesse est supérieur à 2 (s > 2), l'avance rapide ralentit donc le jeu ; comportement fidèle (DEC-07-Q2 : avance rapide de vitesse fixe). Toutes les frames émulées pendant l'avance rapide sont converties en 3D (pas de saut de conversion).

**REQ-LOOP-017** (P1) — Région : la région active (NTSC, PAL, Dendy) est un réglage (SPEC-09, défaut NTSC). Si le réglage « région automatique » est actif (défaut : inactif), le chargement d'une ROM DOIT remplacer la région par celle que le cœur détecte dans l'en-tête de la ROM. Un changement de région DOIT être appliqué immédiatement au cœur et à la cadence, y compris en cours de jeu.
- Critère d'acceptation : région automatique active, charger une ROM marquée PAL → réglage région = PAL et cadence 50 Hz ; changer la région en NTSC en cours de jeu → cadence 60 Hz dès la seconde suivante.

### 3.3 Lancement d'un jeu

**REQ-LOOP-020** (P1) — Sélection dans la bibliothèque : sur sélection d'une entrée de jeu (validation), le frontend DOIT :
1. si l'entrée n'a pas de ROM associée, **ou** si la sélection est faite avec le bouton secondaire (clic droit, ou bouton Select maintenu), ouvrir le navigateur de fichiers (titre « Select ROM », extensions nes, zip, 7z). En cas d'annulation, rien d'autre ne se passe. Sinon, le chemin choisi est associé à l'entrée et persisté (SPEC-09) puis le jeu est lancé ;
2. sinon, si l'entrée est le jeu courant, afficher une boîte de dialogue titrée du nom du jeu, message « Do you want to reset the progress? », boutons « Reset » (relance complète, 3.6) et « Resume » (commande Jouer) ;
3. sinon, lancer le jeu.
La sélection DOIT être ignorée pendant Chargement.
- Critère d'acceptation : sélectionner un jeu sans ROM → navigateur ouvert ; choisir un fichier valide → jeu lancé et chemin mémorisé au redémarrage de l'application.

**REQ-LOOP-021** (P1) — Le lancement d'un jeu DOIT enchaîner, de façon observable, les étapes suivantes :
1. la date de dernier lancement de l'entrée est mise à jour (tri « récents » de la bibliothèque, SPEC-10) ;
2. l'indicateur Chargement passe à vrai (barre de progression visible) ;
3. si un jeu tournait, il est arrêté : hook Lua de fin (SPEC-05) ; l'état de la scène 3D est vidé ; les données du profil (patterns, palette, atlas, scripts) sont déchargées **seulement si** le nouveau jeu diffère du précédent ou si le jeu est l'entrée « 2D Mode » ;
4. la ROM est lue (3.3.1) et transmise au cœur, qui démarre à froid ; la région est éventuellement ajustée (REQ-LOOP-017) ;
5. si les données du profil ont été déchargées, le profil du jeu est chargé (SPEC-03) ; puis le hook Lua de démarrage est exécuté ;
6. l'état passe à En pause, et le frontend attend que la génération des meshes atteigne 100 % (SPEC-04), en vérifiant au moins toutes les 100 ms ;
7. le cœur est réinitialisé, l'horloge de cadence est réinitialisée, l'historique de rollback est vidé, l'état passe à En cours et l'indicateur Chargement à faux.
- Critère d'acceptation : lancer un jeu → barre de progression visible jusqu'à 100 %, puis vue de jeu affichée dès que l'état passe à En cours ; aucune frame émulée n'a tourné avant la fin de la barre (compteur de frames du cœur = 0 au passage à En cours).
- Notes : relancer le même jeu ne recharge pas le profil (étape 5 limitée au hook de démarrage), ce qui est observable par un chargement nettement plus court.

**REQ-LOOP-022** (P1) — Lecture de la ROM : un fichier `.zip` ou `.7z` DOIT être décompressé et sa **première entrée** utilisée comme image de ROM ; le nom de ROM retenu est alors le nom de l'archive avec l'extension `.nes`. Tout autre fichier est lu tel quel, son nom de fichier servant de nom de ROM.
- Critère d'acceptation : `Foo.zip` contenant `bar.nes` → ROM chargée, nom de ROM `Foo.nes`.

**REQ-LOOP-023** (P1) — Erreurs de lancement : en cas d'échec à n'importe quelle étape, le frontend DOIT afficher une notification d'erreur (SPEC-10) avec le message correspondant, effacer le jeu courant, remettre l'indicateur Chargement à faux et passer à l'état Arrêté. Messages exacts :

| Cas | Message |
|---|---|
| archive illisible (zip ou 7z corrompu) | `Corrupted zip file` |
| échec de décompression de l'entrée | `Uncompression is not successful` |
| contenu refusé par le cœur (en-tête invalide, mapper non supporté…) | `Invalid rom file.` |
| fichier introuvable ou illisible | message d'erreur du système de fichiers (texte libre) |

- Critère d'acceptation : associer à un jeu un fichier texte renommé `.nes` → notification `Invalid rom file.`, état Arrêté, aucune vignette « jeu courant ».

**REQ-LOOP-024** (P2) — Changement d'algorithme de rendu des meshes (réglage, SPEC-04/SPEC-09) en cours de session : si des meshes 3D sont déjà générés, le frontend DOIT passer en Chargement, régénérer les meshes, attendre 100 %, réinitialiser l'horloge de cadence puis quitter Chargement, sans modifier l'état (En cours ou En pause) ni l'état du cœur.
- Critère d'acceptation : changer l'algorithme en pause → barre de progression, puis retour à l'état En pause, partie inchangée.

### 3.4 Rollback (retour arrière)

**REQ-LOOP-030** (P2) — Historique : pendant l'exécution, le frontend DOIT capturer un snapshot de l'état du cœur au début de chaque frame émulée dont le compteur de frames du cœur (SPEC-02) est un multiple de 60, c'est-à-dire avant l'exécution de cette frame. L'historique DOIT conserver au plus **30** snapshots ; au-delà, le plus ancien est remplacé.
- Critère d'acceptation : NTSC, 45 s de jeu → 30 snapshots, le plus ancien correspondant au compteur de frames le plus grand multiple de 60 situé ≈ 29–30 s en arrière.
- Notes : la profondeur est donc ≈ 30 s en NTSC et ≈ 36 s en PAL/Dendy. Mémoire : 30 × la taille d'un état du cœur (bornée, SPEC-02). Le snapshot ne contient que le cœur : ni les données persistantes des scripts, ni la caméra, ni l'état de la scène 3D.

**REQ-LOOP-031** (P2) — L'historique DOIT être vidé au lancement d'un jeu, au chargement d'un savestate et à chaque reset. Il NE DOIT PAS être vidé par la pause, ni par une quick save, ni par un rollback.
- Critère d'acceptation : charger un savestate puis déclencher immédiatement un rollback → aucun état restauré (l'historique est vide), l'émulation reprend normalement.

**REQ-LOOP-032** (P2) — Commande Rollback (hotkey, SPEC-08) : à chaque appui, le frontend DOIT :
1. afficher la notification « Roll Back » (durée 1 s) ;
2. restaurer dans le cœur l'**avant-dernier** snapshot de l'historique et retirer de l'historique le dernier snapshot ainsi que le snapshot restauré ;
3. passer à l'état En cours (y compris si l'état était En pause) avec horloge de cadence réinitialisée.
Le snapshot restauré étant pris à un compteur multiple de 60, il est recapturé dès la frame suivante.
- Critère d'acceptation : NTSC, jouer 10,5 s puis rollback → compteur de frames du cœur = 540 (snapshot avant-dernier, le dernier étant 600) ; un second appui moins d'une seconde plus tard → 480. Chaque appui recule donc de 1 à 2 s, puis d'environ 1 s par appui supplémentaire rapproché.
- Notes : avec 1 seul snapshot, la réimplémentation DOIT restaurer ce snapshot ; avec 0 snapshot, rien n'est restauré mais l'étape 3 s'applique. (Le comportement de l'original dans ces cas limites est défectueux ; il n'est pas reproduit.)

**REQ-LOOP-033** (P3) — Le frontend PEUT proposer un panneau d'historique (curseur sur les snapshots disponibles, du plus ancien au plus récent, et bouton de restauration) ; s'il existe, restaurer l'entrée d'indice `i` (0 = plus ancien) DOIT ne conserver dans l'historique que les entrées antérieures à `i`, restaurer l'entrée `i` et passer à En cours ; le panneau est inactif si l'historique est vide.
- Critère d'acceptation : historique de 10 entrées, restaurer l'indice 4 → compteur de frames du cœur égal à celui de l'entrée 4.
- Notes : l'interface livrée ne comporte pas de panneau d'historique ; l'implémentation PEUT en ajouter un.

**REQ-LOOP-034** (P2) — Dans le frontend libretro, le rollback interne PEUT être désactivé au profit du rembobinage de l'hôte, qui repose sur la sérialisation (3.7).
- Critère d'acceptation : sans objet côté desktop ; côté libretro, le rembobinage de RetroArch fonctionne via la sérialisation.

### 3.5 Savestates à l'exécution

**REQ-LOOP-040** (P1) — Contenu : un savestate DOIT capturer, dans cet ordre logique : l'état complet du cœur (autonome : il permet de reprendre la partie sans relire le fichier ROM, SPEC-02), les données persistantes des scripts Lua du profil (SPEC-05), l'état de la caméra (mode, position, orientation ; SPEC-06) et l'**état appris de la conversion** (registre d'images, patterns appris et patterns temporaires, état du tracker ; SPEC-04 REQ-E3D-007, 008, 160, 300), conformément à DEC-R2-T5 **(D)**. Il NE DOIT PAS contenir les frames converties ni les maillages, qui sont reconstruits à partir du profil, de l'état appris et des frames suivantes.
- Critère d'acceptation : sauvegarder, déplacer la caméra, modifier une donnée persistante par script, charger → caméra et données persistantes reviennent à leurs valeurs au moment de la sauvegarde ; l'image de jeu est identique à celle de la sauvegarde dès la première frame émulée après chargement.
- Notes : format binaire et emplacement : SPEC-09 (pas de compatibilité avec les sauvegardes d'origine). L'original ne sauvegarde pas l'état appris de la conversion (il le conserve d'une partie à l'autre du même jeu, SPEC-04 REQ-E3D-008) ; l'inclure garantit le rejeu à l'identique (REQ-ARCH-012, REQ-ARCH-013) et la sérialisation libretro (REQ-LOOP-062).

**REQ-LOOP-041** (P1) — Sauvegarde : une sauvegarde NE DOIT avoir lieu que si l'état n'est pas Arrêté et qu'il y a un jeu courant. Elle DOIT exécuter le hook Lua de sauvegarde (SPEC-05) **avant** de capturer l'état, puis écrire le savestate entre deux frames émulées. L'état de l'émulation (En cours / En pause) NE DOIT PAS changer.
- Critère d'acceptation : quick save en cours de jeu → l'émulation continue sans pause visible (au plus une frame de rendu dupliquée) ; un script dont le hook de sauvegarde écrit une donnée persistante voit cette donnée incluse dans le savestate.

**REQ-LOOP-042** (P2) — Sauvegarde dans un slot numéroté (onglet Save du menu, SPEC-10) : le frontend DOIT jouer le son de validation (SPEC-11), associer au slot l'identifiant du jeu courant, la capture prise à la pause (REQ-LOOP-006) et l'horodatage courant, persister immédiatement les métadonnées des slots (SPEC-09), puis écrire le savestate. Un slot occupé est écrasé sans confirmation.
- Critère d'acceptation : sauvegarder dans le slot 5 → le slot 5 affiche le jeu, la vignette et la date ; écraser le slot 5 → ancienne sauvegarde remplacée, aucune boîte de dialogue.

**REQ-LOOP-043** (P1) — Quick save (hotkey, SPEC-08 ; aussi Select + Rot Down sur manette) : si un jeu courant existe, le frontend DOIT effectuer la sauvegarde dans le slot rapide comme en REQ-LOOP-042 mais **sans** capture (vignette vide/noire), et afficher la notification « Quick Save » (durée 4 s). Sans jeu courant, rien ne se passe.
- Critère d'acceptation : en jeu, quick save → notification « Quick Save », son de validation, slot rapide daté de l'instant.

**REQ-LOOP-044** (P1) — Chargement d'un slot (onglet Load du menu, ou quick load) : le frontend DOIT :
1. jouer le son de validation et passer l'indicateur Chargement à vrai ;
2. retrouver le jeu associé au slot ; s'il n'existe plus dans la bibliothèque, échouer avec le message `The game profile doesn't exist anymore.` ;
3. arrêter le jeu en cours comme en REQ-LOOP-021 étape 3 (hook de fin ; déchargement du profil seulement si le jeu change ou s'il s'agit de « 2D Mode ») ;
4. si nécessaire, charger le profil du jeu (ce qui crée un nouvel état Lua, SPEC-05 REQ-LUA-014) ;
5. restaurer le cœur, l'état appris de la conversion et la caméra depuis le savestate, puis affecter les données persistantes des scripts (`PData`) ; exécuter le hook Lua de démarrage puis le hook Lua de chargement, dans l'ordre de SPEC-05 REQ-LUA-072 (DEC-R2-T6) ; vider l'historique de rollback ;
6. attendre que la génération des meshes atteigne 100 % ;
7. passer à En cours (horloge réinitialisée) et remettre Chargement à faux.
Le chargement de slot DOIT être ignoré si Chargement est déjà vrai.
- Critère d'acceptation : depuis l'état Arrêté, charger le slot 2 d'un jeu A → jeu A en cours à l'endroit sauvegardé, sans avoir demandé la ROM ; les hooks démarrage puis chargement sont appelés une fois chacun, dans cet ordre ; un savestate d'un jeu B chargé pendant que le jeu A tourne restaure les `PData` enregistrées (elles ne sont pas remises à zéro par le chargement du profil de B).

**REQ-LOOP-045** (P1) — Quick load (hotkey, SPEC-08 ; aussi Select + Rot Up sur manette) : si le slot rapide n'est pas vide, le frontend DOIT charger ce slot (REQ-LOOP-044) et afficher la notification « Quick Load » (durée 4 s). Si le slot rapide est vide, rien ne se passe (ni notification, ni son).
- Critère d'acceptation : slot rapide vide → hotkey sans effet ; après une quick save → quick load restaure l'état et affiche « Quick Load ».

**REQ-LOOP-046** (P1) — Validation au chargement : si les données du savestate sont illisibles ou incohérentes, le frontend DOIT afficher la notification d'erreur `Corrupted save state.`, remettre Chargement à faux et passer à l'état Arrêté (jeu courant effacé). Un savestate est toujours chargé avec le jeu auquel son slot est associé : il n'y a pas de contrôle « mauvaise ROM » distinct, puisque l'état du cœur est autonome.
- Critère d'acceptation : tronquer le fichier d'un slot puis le charger → notification `Corrupted save state.`, état Arrêté.
- Notes : le format SPEC-09 DEVRAIT inclure une signature et une version permettant de rejeter proprement un fichier étranger avec ce même message.

### 3.6 Reset, arrêt, sortie, focus

**REQ-LOOP-050** (P1) — Reset : dans le frontend desktop, le reset DOIT être une relance complète du jeu courant (REQ-LOOP-020 cas 2, bouton « Reset ») : relecture de la ROM, démarrage à froid du cœur (mémoire réinitialisée), hook de fin puis hook de démarrage, historique vidé, sans rechargement du profil 3D. Il n'existe pas de reset « doux » (équivalent du bouton RESET de la console) dans l'interface livrée.
- Critère d'acceptation : en cours de partie, sélectionner le jeu courant puis « Reset » → le jeu redémarre à son écran titre, état En cours, compteur de frames du cœur reparti de 0.
- Notes : le frontend libretro DEVRAIT associer `retro_reset` au même démarrage à froid du cœur, sans recharger le profil ni passer par l'état Arrêté ; un reset doux PEUT être ajouté comme option.

**REQ-LOOP-051** (P1) — Arrêt : l'arrêt d'un jeu (suppression du jeu courant de la bibliothèque, ou étape interne d'un lancement/chargement) DOIT exécuter le hook Lua de fin, fermer la ROM dans le cœur, vider la scène 3D et passer à Arrêté. Seule la suppression du jeu courant efface le jeu courant ; l'arrêt interne d'un lancement ne l'efface pas avant l'issue du lancement.
- Critère d'acceptation : supprimer le jeu courant → menu sans vignette « jeu courant », état Arrêté, musique de menu.

**REQ-LOOP-052** (P1) — Sortie : une demande de sortie (fermeture de la fenêtre, bouton Exit du menu) DOIT être exécutée **sans confirmation** et **sans sauvegarde automatique de la partie**. Elle DOIT arrêter le jeu courant (hook de fin), arrêter la boucle d'émulation, enregistrer les réglages et les métadonnées des slots (SPEC-09), puis fermer l'application.
- Critère d'acceptation : fermer la fenêtre en cours de jeu → l'application se ferme en moins de 2 s, aucun nouveau savestate n'est créé, les réglages modifiés pendant la session sont présents au relancement.

**REQ-LOOP-053** (P2) — Perte de focus : la perte de focus de la fenêtre NE DOIT PAS mettre l'émulation en pause ; l'émulation et l'audio continuent en arrière-plan. Si la plateforme ou l'hôte **suspend** l'application (événement de suspension explicite), l'émulation DOIT passer en pause, et à la reprise revenir à En cours si et seulement si elle était En cours au moment de la suspension.
- Critère d'acceptation : cliquer sur une autre fenêtre pendant le jeu → le jeu continue et reste audible.

**REQ-LOOP-054** (P3) — Une seule instance de l'application DEVRAIT pouvoir s'exécuter à la fois (le second lancement active la première instance ou se ferme).
- Critère d'acceptation : lancer deux fois l'exécutable → une seule fenêtre.

### 3.7 Travaux asynchrones et déterminisme

**REQ-LOOP-060** (P1) — Pour une frame émulée donnée, les opérations suivantes DOIVENT s'exécuter de façon synchrone et dans cet ordre, sans entrelacement avec une commande : (a) lecture de l'état des entrées des deux manettes (SPEC-08) ; (b) exécution de la frame par le cœur ; (c) conversion 3D de la trame produite (SPEC-04) ; (d) hook Lua de mise à jour (SPEC-05). La capture de snapshot de rollback a lieu avant (a).
- Critère d'acceptation : deux exécutions à partir du même savestate avec la même séquence d'entrées par frame produisent, frame par frame, des états du cœur, des données persistantes de scripts et des listes de tiles 3D identiques (comparaison d'empreintes).

**REQ-LOOP-061** (P1) — Seules les tâches sans effet sur l'état du jeu PEUVENT être asynchrones : génération des meshes 3D des patterns et des textures d'atlas (répartie sur les frames de rendu dans un budget de temps égal à la durée d'une frame de rendu cible), lecture audio (SPEC-11), capture d'écran, écritures de fichiers. Leur avancement ne DOIT influencer ni l'état du cœur ni l'état Lua. Lors d'un lancement ou d'un chargement, le passage à En cours DOIT attendre la fin de la génération initiale des meshes (100 %).
- Critère d'acceptation : rejouer une séquence d'entrées sur une machine lente et sur une machine rapide → états du cœur identiques frame par frame ; seule l'apparition visuelle de meshes générés en cours de jeu peut différer de quelques frames de rendu.

**REQ-LOOP-062** (P1) — La logique de cadence (horloge, rattrapage, vitesse, avance rapide) DOIT être propre au frontend desktop. Le noyau (cœur + moteur 3D + scripts) DOIT exposer une opération « exécuter exactement une frame émulée avec ces entrées » et des opérations de sérialisation/désérialisation de session déterministes, sans horloge interne, afin que le frontend libretro appelle une frame par `retro_run` et délègue à l'hôte la cadence, l'avance rapide, le ralenti et le rembobinage.
- Critère d'acceptation : sous libretro, `retro_run` appelé N fois → exactement N frames émulées ; `retro_serialize_size` renvoie une borne valable pour toute la session du jeu chargé (REQ-ARCH-013) ; sérialiser puis désérialiser et rejouer donne les mêmes empreintes (REQ-LOOP-060).

**REQ-LOOP-063** (P2) — La sérialisation utilisée par libretro DOIT contenir le même périmètre qu'un savestate (REQ-LOOP-040). Conformément à DEC-07-Q4 et DEC-R2-D5, `retro_serialize` DOIT appeler le hook `Save` de chaque script actif avant la capture, et `retro_unserialize` DOIT, après la restauration, affecter `PData` puis appeler le hook `Load` de chaque script actif (sans `End` ni `Start`, le profil restant chargé), quel que soit le motif de l'appel (savestate, rembobinage, netplay). Le run-ahead n'est pas pris en charge en v1.
- Critère d'acceptation : sous RetroArch, un `retro_serialize` produit exactement un appel `Save` par script actif, un `retro_unserialize` exactement un appel `Load` par script actif, dans l'ordre des scripts.
- Notes : question Q4 close par DEC-07-Q4 et DEC-R2-D5.

## 4. Formats et interfaces

Messages utilisateur de ce module (texte exact, langue anglaise comme l'original) :

| Événement | Texte | Type | Durée |
|---|---|---|---|
| rollback | `Roll Back` | normal | 1 s |
| avance rapide | `Fast Forward` | normal | 1 s |
| quick save | `Quick Save` | normal | 4 s |
| quick load | `Quick Load` | normal | 4 s |
| erreurs de lancement / chargement | voir REQ-LOOP-023, 044, 046 | erreur | durée par défaut de SPEC-10 |
| dialogue relance | titre = nom du jeu ; `Do you want to reset the progress?` ; boutons `Reset`, `Resume` | dialogue | — |

Réglages utilisés (clés et persistance : SPEC-09) : vitesse (entier −8..+8, défaut 0), région (NTSC/PAL/Dendy, défaut NTSC), région automatique (booléen, défaut faux), saut de rendu (booléen, défaut vrai), plafond de rendu (60/90/120), synchronisation verticale, algorithme de rendu des meshes.

## 5. Vecteurs de test

| # | Entrée | Résultat attendu |
|---|---|---|
| T1 | s = 0, NTSC, 10 s | 600 ± 1 frames |
| T2 | s = −4, PAL, 10 s | 250 ± 1 frames |
| T3 | s = +8, NTSC, 1 s | 180 ± 1 frames |
| T4 | avance rapide maintenue 1 s, s = 0, NTSC, puis relâchée | ≈ 90 frames pendant la seconde (k = 1,5), puis 0 à 30 frames à ×1,5 après relâchement |
| T5 | blocage de 1 s de la boucle, NTSC | au plus 5 frames rattrapées ; ≈ 55 frames perdues |
| T6 | jeu lancé, 100 frames émulées, rollback | historique {0, 60} avant ; compteur restauré = 0 ; historique après = {0} recapturé à la frame suivante |
| T7 | pause 30 s puis reprise | aucune rafale ; cadence nominale immédiate |
| T8 | quick load avec slot rapide vide | aucun effet, aucune notification |
| T9 | charger un slot dont le jeu a été supprimé | notification `The game profile doesn't exist anymore.`, état Arrêté |
| T10 | archive zip vide | notification d'erreur (`Corrupted zip file` ou `Uncompression is not successful`), état Arrêté |

## 6. Questions ouvertes

- **Q1** — *Close par DEC-07-Q1* (−8..+8, différentiel). La plage exacte du curseur de vitesse de l'UI livrée n'a pas pu être confirmée ; −8..+8 entier est déduit des bornes appliquées par les raccourcis clavier de l'outil. À confirmer en différentiel.
- **Q2** — *Close par DEC-07-Q2* (vitesse fixe, fidèle). Correction v1.1 : la vitesse fidèle est ×1,5 (réglage s = 2), et non ×2 (REQ-LOOP-016) ; R0 est invité à corriger le libellé « 2× » de DEC-07-Q2.
- **Q3** — *Close par DEC-07-Q3* (cœur seul, fidèle). Le rollback ne restaure ni la caméra ni les données persistantes Lua (fidèle). Pour les profils dont les scripts tiennent un état dépendant du jeu, cela peut produire un affichage incohérent pendant quelques frames. Faut-il étendre le snapshot au périmètre complet d'un savestate ? Proposition : fidèle en v1, extension en option.
- **Q4** — *Close par DEC-07-Q4 et DEC-R2-D5* (hooks `Save`/`Load` appelés, REQ-LOOP-063). Comportement des hooks Lua de sauvegarde/chargement lors des sérialisations libretro (rembobinage, run-ahead, netplay) : à arbitrer avec SPEC-05.
- **Q5** — *Close par DEC-07-Q5* (capture autorisée, D). La vignette d'une quick save est vide (pas de capture) dans l'original ; faut-il capturer la vue courante ? Proposition : capture autorisée (amélioration visible mineure), à valider par R0.

## Historique

- 1.1-draft (AMD-001, relecture R2) : cadence nominale 60/50 images/s (DEC-R2-T3) et vecteurs T1, T4 recalculés ; avance rapide : modèle de demandes réarmées et vitesse fidèle ×1,5 vérifiée dans l'original (REQ-LOOP-016) ; limite de rattrapage précisée (REQ-LOOP-012) ; savestate avec état appris de la conversion (REQ-LOOP-040, DEC-R2-T5) ; ordre de restauration (REQ-LOOP-044, DEC-R2-T6) ; hooks Save/Load en libretro (REQ-LOOP-063, DEC-R2-D5) ; note LEAK de REQ-LOOP-033 reformulée ; questions closes.
