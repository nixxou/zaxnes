# SPEC-12 — Critères d'acceptation globaux
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : toutes

## 1. Objet et périmètre
Ce document définit les critères qui conditionnent le passage des gates P3, P4 et P5, ainsi que la sortie du projet. Il complète les critères propres à chaque exigence.

## 2. Vocabulaire
**Machine de référence** : Windows 10 x64, GPU NVIDIA RTX 3060, résolution 1920×1080.
**ROM de l'utilisateur** : dump légalement possédé par l'utilisateur. Aucune ROM commerciale n'est livrée avec le projet ; seules les ROMs de test publiques le sont. Les jeux de référence de REQ-ACC-020 (dont Quarth pour l'algorithme MARCHING) supposent que l'utilisateur fournit les ROMs correspondantes.

**Catégories de vérification** (DEC-R2-T7, DEC-R2-T8). Chaque critère d'acceptation relève d'une des catégories suivantes, indiquée dans la matrice de traçabilité :
- **auto** : exécutable par le côté propre sans ROM commerciale (sur `assets-neutral/`, sur des ROMs de test publiques ou des ROMs construites, avec le harnais) ;
- **[ROM]** : exige la ROM fournie par l'utilisateur ; vérifiable au round 1 avec elle ;
- **[R2]** : exige une comparaison avec le programme d'origine ; vérifiable seulement au round 2 ;
- **revue** : verdict par revue visuelle documentée (capture et justification archivées dans `verify/`).
Une exigence porte l'étiquette [ROM] ou [R2] dans son titre quand au moins une partie de son critère en relève ; la partie « auto » éventuelle reste exigible au round 1. Les outils du côté sale ne sont jamais des critères.

## 3. Exigences

### 3.1 Cœur
**REQ-ACC-001** (P1) — Le cœur DOIT passer les ROMs de test publiques listées dans SPEC-02 (nestest avec son journal de référence, les suites blargg cpu, ppu et apu, les tests de sprite 0 et de débordement, mmc3_test 1 à 5).
- Critère d'acceptation : un test automatisé exécute chaque ROM et vérifie son verdict (code de résultat en mémoire ou comparaison au journal).

### 3.2 Profils
**REQ-ACC-010** (P1) — Les 101 profils du catalogue DOIVENT se charger sans erreur, et le décodage puis le réencodage de chacun DOIVENT reproduire le même contenu logique (SPEC-03).
- Critère d'acceptation : test automatisé sur `assets-neutral/profiles/`.

**REQ-ACC-011** (P1) — Pour chacun des 89 profils qui contiennent des scripts, l'exécution des hooks `Start`, puis de 600 frames d'`Update`, `UpdateS` et `LateUpdate`, puis de `End`, sur une frame de test synthétique, NE DOIT produire aucune erreur Lua due à un membre d'API manquant.
- Critère d'acceptation : test automatisé ; les erreurs dues aux données de la frame synthétique sont tolérées si elles sont documentées.

### 3.3 Jeux de référence
**REQ-ACC-020** (P1) [ROM] — Avec la ROM de l'utilisateur, les jeux de référence suivants DOIVENT démarrer, se jouer 5 minutes sans plantage et afficher un rendu 3D cohérent avec la description de SPEC-06 : Super Mario Bros, Super Mario Bros 3, The Legend of Zelda, Mega Man 2, Contra, Metroid, Duck Hunt (Zapper), Excitebike, Tetris, Final Fantasy, Blaster Master, Quarth (algorithme MARCHING).
- Critère d'acceptation : session manuelle ou séquence d'entrées rejouée par le harnais ; captures archivées dans `verify/`.

**REQ-ACC-021** (P2) [ROM] [R2] — Au round 2, les captures différentielles de la reconstruction et de l'original, faites aux mêmes frames pour les jeux de REQ-ACC-020, DEVRAIENT atteindre un SSIM ≥ 0,90 en vue par défaut. Les écarts classés D sont exclus de ce calcul ainsi : une capture dont la différence est entièrement due à un écart D est retirée de l'échantillon ; sinon, les régions d'image affectées par un écart D (rectangles listés dans le rapport, par exemple la zone d'un objet de profil concerné) sont remplacées par la même couleur neutre dans les deux captures avant le calcul ; la liste des captures retirées et des régions masquées accompagne le score.
- Critère d'acceptation : rapport de round 2 avec les scores.

### 3.4 Performances
**REQ-ACC-030** (P1) — Sur la machine de référence, l'application DOIT maintenir 60 images/s en 1080p sur les jeux de référence, dans la vue par défaut.
- Critère d'acceptation : moins de 1 % de frames au-delà de 16,7 ms sur une séquence de 5 minutes.

**REQ-ACC-031** (P2) [ROM] — La conversion d'une frame (SPEC-04 et SPEC-04B, hors construction initiale des meshes) DEVRAIT tenir en 4 ms en moyenne et en 8 ms au 99ᵉ centile sur la machine de référence (DEC-R2-12-2, même cible que SPEC-04 REQ-E3D-360).
- Critère d'acceptation : banc du harnais de REQ-E3D-360 (trame synthétique pleine, profil « Final_Fantasy_USA ») ; [ROM] mesure du harnais sur 1 000 frames de Super Mario Bros 3.

### 3.5 Robustesse et déterminisme
**REQ-ACC-040** (P1) — Le test de déterminisme de REQ-ARCH-012 et le test d'aller-retour de savestate de REQ-ARCH-013 DOIVENT passer sur au moins trois jeux de référence.
- Critère d'acceptation : tests automatisés avec le harnais.

**REQ-ACC-041** (P1) — Un fichier corrompu (ROM, profil, réglages, savestate) NE DOIT jamais faire planter l'application : il produit le message défini par la spec concernée.
- Critère d'acceptation : test avec des fichiers tronqués ou aléatoires.

### 3.6 Gates
**REQ-ACC-050** — Gate P4 : 100 % des exigences P1 et au moins 95 % des exigences P2 conformes au round 1, sans exigence « non testable » non justifiée. Pour ce décompte : une exigence étiquetée **[ROM]** est jugée sur sa partie « auto » et, si la ROM de l'utilisateur est disponible, sur sa partie [ROM] ; à défaut de ROM, elle compte comme **justifiée** (verdict « en attente de ROM ») ; une exigence étiquetée **[R2]** est jugée au round 1 sur sa partie « auto » et compte comme justifiée pour la partie [R2] (verdict « vérifiable au round 2 ») ; une exigence de catégorie **revue** est conforme si la revue est archivée. Les verdicts « en attente de ROM » et « vérifiable au round 2 » sont reportés dans la matrice de traçabilité et repris au round 2.
**REQ-ACC-051** — Sortie du round 2 : aucun écart bloquant ; au plus 5 écarts visibles, tous documentés ; REQ-ACC-010, 011, 020 et 030 conformes.

## 4. Questions ouvertes
Aucune. (Question R2 12-3 close : la ROM de Quarth est attendue de l'utilisateur, comme toutes les ROMs commerciales.)

## Historique

- 1.1-draft (AMD-001, relecture R2) : catégories de vérification auto / [ROM] / [R2] / revue (DEC-R2-T7, T8) et REQ-ACC-050 rendue compatible ; REQ-ACC-031 alignée sur DEC-R2-12-2 ; méthode d'exclusion des écarts D pour le SSIM (REQ-ACC-021) ; ROMs de référence fournies par l'utilisateur (Quarth).
