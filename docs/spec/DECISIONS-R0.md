# Décisions R0 sur les questions ouvertes (spec v1.0)

Version : 1.1 | 2026-10-20 | gelé avec la spec v1.1 | **Normatif** : en cas de conflit avec le texte d'une SPEC, la décision ci-dessous prime. Chaque décision est référencée `DEC-<doc>-<question>`.

Principe : **fidélité par défaut en v1**. Un écart volontaire est noté **(D)**, divergence acceptée. « Différentiel » signifie : à vérifier au round 2 par comparaison en boîte noire, la règle écrite s'appliquant d'ici là.

## Arbitrages après la relecture R2 (v1.1, R2-review-v1)
- **DEC-R2-T1** `Clipping` : indice 0 = marge **gauche**, 1 = marge **basse**, 2 = marge **droite**, 3 = marge **haute** (bords de l'image NES : « haute » est le haut de l'écran). SPEC-04, SPEC-05 et SPEC-06 font foi ; SPEC-09 est corrigée.
- **DEC-R2-T2** Repère de scène : un amendement unique fixe la relation exacte entre pixel NES, grille de conversion (hauteur 248) et plan de scène, ainsi que le centre de caméra par défaut, et réécrit les passages concernés de SPEC-04B, SPEC-06 et SPEC-08. La règle de substitution « 240 → 248 » de la version 1.0 est **retirée**.
  - *Constat R1 (AMD-001, vérifié dans l'original)* : l'image NES est placée par le retournement `y = 248 − py` (grille de 31 rangées), donc en y ∈ [8, 248] ; le rectangle de découpe du rendu, le visé corrigé de la caméra, le tunnel TV et le backdrop utilisent le cadre [0, 256] × [0, 240] décalé des marges, soit R = [c0, 256 − c2] × [c1, 240 − c3] ; la cible par défaut de la caméra et le centre du HUD valent (128, 120, 0), 8 unités sous le centre de l'image ; la visée du Zapper convertit un point de scène en pixel par `py = tronc(240 − y)`, donc 8 lignes au-dessus du pixel affiché. Ces valeurs fidèles sont reprises par SPEC-04B REQ-E3D-410 ; les vecteurs caméra et TV de SPEC-06 v1.0 étaient déjà calculés dans ce repère (inchangés). Constat annexe : l'avance rapide de l'original applique le réglage de vitesse s = 2, soit k = 1,5 et non 2× (libellé de DEC-07-Q2 à corriger par R0).
- **DEC-R2-T3** La cadence nominale de l'hôte est de **60 images/s** (NTSC) et de **50 images/s** (PAL, Dendy), avec 800 et 960 échantillons par frame à 48 kHz. SPEC-07 et SPEC-11 s'alignent sur SPEC-02.
- **DEC-R2-T4** Confirmation de DEC-06-Q4 : le décalage de ±0,1 des sprites est appliqué **par le rendu**. SPEC-04B est corrigée (REQ-E3D-405, TV-W2).
- **DEC-R2-T5** Le savestate de la reconstruction **inclut** l'état appris de la conversion (registre d'images, patterns appris et temporaires, tracker), pour garantir REQ-ARCH-012 et REQ-ARCH-013 et la sérialisation libretro **(D)**. REQ-LOOP-040 et REQ-PERS-056 sont amendées en conséquence.
- **DEC-R2-T6** L'ordre de restauration d'un savestate est celui de SPEC-05 (REQ-LUA-072) : profil rechargé avant l'affectation de `PData`. REQ-LOOP-044 est corrigée.
- **DEC-R2-T7** Tout critère d'acceptation doit être exécutable par le côté propre (sur `assets-neutral/`, sur des ROMs de test publiques ou avec le harnais). Les outils du côté sale ne sont jamais des critères ; les sections qui les décrivent sont informatives.
- **DEC-R2-T8** Deux étiquettes de vérifiabilité s'ajoutent : **[ROM]**, vérifiable au round 1 avec la ROM fournie par l'utilisateur, et **[R2]**, vérifiable seulement au round 2 par comparaison avec l'original. Les exigences ainsi étiquetées comptent comme justifiées pour REQ-ACC-050.
- **DEC-R2-D4** Carte d'ombre : **1024 texels par défaut** (remplace DEC-06-Q3).
- **DEC-R2-D5** DEC-07-Q4 prime : `retro_serialize` et `retro_unserialize` appellent les hooks `Save` et `Load`. REQ-LOOP-063 et REQ-LUA-075 sont alignées.
- **DEC-R2-D7** DEC-08-6 prime : le réglage « bouton Menu émulé » n'a pas d'effet. REQ-PERS-017 est alignée.
- **DEC-R2-12-2** Budget de conversion : moyenne ≤ 4 ms **et** 99ᵉ centile ≤ 8 ms (REQ-ACC-031 et REQ-E3D-360 alignées).
- **DEC-R2-521** La numérotation des coins de REQ-E3D-521 devient **normative**, puisqu'elle correspond aux tables publiques de Lorensen–Cline et que REQ-E3D-522 en dépend.
- **DEC-R2-T9** Les questions ouvertes déjà tranchées par ce document sont marquées « close par DEC-… » dans chaque SPEC.

## SPEC-02 — Cœur NES
- **DEC-02-Q1** Polarité de la gâchette Zapper : on garde la convention décrite (bit 4 = 1 gâchette relâchée). Différentiel sur Duck Hunt.
- **DEC-02-Q2** Région : règle publique de REQ-CORE-013 **(D)**.
- **DEC-02-Q3** Reset = remise sous tension (comportement décrit). La RAM sauvegardée par pile est conservée.
- **DEC-02-Q4** Tolérance d'une ligne maintenue. Différentiel SMB, SMB3, Zelda.
- **DEC-02-Q5** Audio d'extension non requis (P3 optionnel).

## SPEC-03 — Format de profil
- **DEC-03-Q01** Ordre : détection automatique de la région depuis la ROM, **puis** application du `TVMode` du profil au chargement d'un jeu différent (fidèle). Voir aussi DEC-09-Q1.
- **DEC-03-Q02** ~~Tri ordinal~~ **Révisé après SPEC-05** : les scripts sont triés par nom selon une **comparaison culturelle invariante** (règles linguistiques indépendantes de la langue du système : la casse et la ponctuation pèsent moins que les lettres), et non ordinale, parce que l'ordre change le résultat d'au moins un profil embarqué (Excitebike). SPEC-05 fait foi pour les exemples.
- **DEC-03-Q03** Profil sans listes 14/15 : reconstituer 5 emplacements par défaut.
- **DEC-03-Q04** Seule l'édition `full` du catalogue est utilisée. L'édition `demo` est ignorée.

## SPEC-04 — Conversion
- **DEC-04-Q01** `DetectShape` n'est jamais appelé (fidèle).
- **DEC-04-Q02** Égalité de candidats : ordre d'inscription. Différentiel sur les profils à patterns concurrents.
- **DEC-04-Q03** Les valeurs du masque relèvent de SPEC-02.
- **DEC-04-Q04 / Q05** Les deux bizarreries (`TopRight`, chevauchement de la frame épinglée) sont conservées (fidèle).
- **DEC-04-Q06** Le budget de 4 ms par frame pour la conversion est une **cible** (P2), mesurée sur la machine de référence de SPEC-12.
- **DEC-04-Q07** Vocabulaire : voir le glossaire de SPEC-00.

## SPEC-04B — Géométrie
- **DEC-04B-Q1** La table des 64 couleurs NES est portée par **SPEC-04** (gestion de palette). SPEC-04B et SPEC-06 y renvoient.
- **DEC-04B-Q2** Déformation appliquée dans le repère local de l'objet, **avant** échelle et rotation. Différentiel.
- **DEC-04B-Q3** Texture répétée hors de ses bornes (modulo dans son emplacement).
- **DEC-04B-Q4** Découpage par marges dans le **plan XY de la scène**, avant la caméra.
- **DEC-04B-Q5** On reproduit seulement l'état stable des ombres par couche **(D)**.
- **DEC-04B-Q6** Les deux cas hérités de couleur 0 ne sont pas obligatoires (P3, D).

## SPEC-05 — Lua
- **DEC-05-Q1** Le plan est corrigé : l'original exécute un dialecte compatible Lua 5.1 (un seul type numérique en double précision, `goto`, bibliothèque `bit`). Le runtime retenu reste Lua 5.4 (KeraLua), avec les contre-mesures de SPEC-05 (nombres de l'hôte poussés en flottants, affichage sans `.0` des flottants entiers, alias `unpack`).
- **DEC-05-Q2** Le compteur `frame` part de 0 au lancement du jeu (cohérent avec SPEC-04) **(D)**.
- **DEC-05-Q3** État Lua recréé à chaque changement de profil **(D)**.
- **DEC-05-Q4** Lecture de palette décalée d'une frame (fidèle).
- **DEC-05-Q5** Le reset conserve les drapeaux d'erreur des scripts (fidèle).
- **DEC-05-Q6** Seuls les membres des tables du §4 de SPEC-05 sont requis. Les autres membres exposés par l'original sont P3.

## SPEC-06 — Scène et rendu
- **DEC-06-Q1** Résolu : les cubemaps sont réextraits avec leurs 6 faces (`assets-neutral/cubemaps/<nom>/{px,nx,py,ny,pz,nz}.png`, conventions dans `cubemaps.json`). Certains sont des masques encodés par canaux.
- **DEC-06-Q2** Intensité lumineuse : même conversion sRGB → linéaire que les couleurs. Différentiel.
- **DEC-06-Q3** Carte d'ombre de 512 texels par défaut. La tolérance de REQ-SCN-030 s'applique.
- **DEC-06-Q4** Les positions livrées par SPEC-04B **n'incluent pas** le décalage de ±0,1 des sprites : c'est le rendu (SPEC-06) qui l'applique. Déformation : voir DEC-04B-Q2.
- **DEC-06-Q5** Les écritures Lua sur des réglages passent par une **couche de session** non persistée **(D)**. Voir DEC-09-Q3.
- **DEC-06-Q6** Glisser à la souris : SPEC-08 fixe le bouton. Quand la Zapper est active sur un port, le clic de la Zapper prime sur la caméra pour le même bouton.
- ~~Correction transverse 240 → 248~~ : retirée, remplacée par DEC-R2-T2.

## SPEC-07 — Boucle
- **DEC-07-Q1** Vitesse −8..+8. Différentiel.
- **DEC-07-Q2** Avance rapide fixe (fidèle) : elle applique le réglage de vitesse s = 2, soit un facteur **×1,5** (corrigé après le constat R1 d'AMD-001 ; la valeur « 2× » de la v1.0 était erronée).
- **DEC-07-Q3** Le rollback restaure le cœur seul (fidèle). L'extension au savestate complet reste une option P3.
- **DEC-07-Q4** En libretro, `retro_serialize` et `retro_unserialize` produisent et restaurent un savestate complet, hooks Lua `Save`/`Load` compris. Le run-ahead n'est pas pris en charge en v1.
- **DEC-07-Q5** La quick save peut capturer une vignette **(D)**.

## SPEC-08 — Entrées
- **DEC-08-1** Une manette attribuée à un joueur est retirée de l'autre **(D)**.
- **DEC-08-2** On empêche le double effet d'Échap pendant la capture. Échap n'est pas réservée.
- **DEC-08-3** Molette : 5 unités par cran.
- **DEC-08-4** La visée de la Zapper tient compte de l'échelle de scène **(D)** et vise **le pixel NES affiché sous le pointeur** : la conversion scène → pixel utilise le même retournement que le placement de l'image (py = tronc(248 − y) dans le repère de REQ-E3D-410, après retrait du décalage et de l'échelle de scène). Le décalage de 8 lignes de l'original n'est pas reproduit **(D)**. Cette décision prime sur la formule fidèle de REQ-INP-063 et clôt SPEC-08 Q-9.
- **DEC-08-5** Codes physiques des touches ; SPEC-10 affiche le libellé de la disposition active.
- **DEC-08-6** Le réglage « bouton Menu émulé » est conservé dans le modèle mais n'a pas d'effet.
- **DEC-08-7** Hotkeys en pause : comportement décrit. Différentiel pour QuickLoad.
- **DEC-08-8** Zones mortes : valeurs de la spec retenues.

## SPEC-09 — Persistance
- **DEC-09-Q1** `GraphicQuality` et `TVMode` restent portés par le profil (fidèle).
- **DEC-09-Q2** Repère du Clipping : celui des tiles de SPEC-02 et SPEC-04 (y croît vers le bas de l'écran). `clip.top` désigne le haut de l'écran.
- **DEC-09-Q3** Couche de session pour les écritures Lua acceptée **(D)**.
- **DEC-09-Q4** Pas de contrôle d'empreinte ROM au chargement d'un savestate. L'information est journalisée.
- **DEC-09-Q5** Préréglages libretro de couleur de fond : noir, blanc, gris, bleu ciel NES 0x21, bleu foncé, vert, rouge, violet.
- **DEC-09-Q6** Les types de port reviennent à Manette/Manette à chaque chargement d'un jeu **différent** **(D)**.
- **DEC-09-Q7** Nom produit **« Sen »**. Racine utilisateur `%LOCALAPPDATA%\Sen` ; mode portable si un fichier `portable.txt` est présent à côté de l'exécutable. Sous-dossier system libretro : `sen`.

## SPEC-10 — UI
- **DEC-10-Q1** Export OBJ exclu.
- **DEC-10-Q2** Windows uniquement en v1. Une seule méthode de rendu, pas d'option `Tex Scale`.
- **DEC-10-Q3** Bloc « à propos » : identité de la reconstruction (« Sen », licence GPL v3), crédits au jeu original 3dSen et à 3DNes_Engine (Geod Studio).
- **DEC-10-Q4** Navigation tuile par tuile : option P3, sans changer les raccourcis existants.
- **DEC-10-Q5** Bouton « Site » retiré **(D)**.
- **DEC-10-Q6** Onglet initial : GAME **(D)**.
- **DEC-10-Q7** Compteur FPS conservé comme option, désactivé par défaut.

## SPEC-11 — Audio
- **DEC-11-Q1** Reprise de la musique de menu : à la position de suspension. Différentiel.
- **DEC-11-Q2** Fondu de durée fixe : 200/60 s ≈ 3,33 s, soit la valeur d'origine à 60 images/s **(D)**.
- **DEC-11-Q3** Choix de REQ-AUD-005 accepté.
- **DEC-11-Q4** Adaptation à la fréquence du périphérique acceptée.
- **DEC-11-Q5** Différentiel.
