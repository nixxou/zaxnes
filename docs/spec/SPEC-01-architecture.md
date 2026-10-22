# SPEC-01 — Architecture et contraintes transverses
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-00, PLAN.md (« Contraintes d'architecture pour la portabilité (libretro) »)

## 1. Objet et périmètre
Ce document fixe les exigences d'architecture **observables ou vérifiables** que toutes les autres specs supposent. Le découpage interne de chaque module reste libre.

## 2. Vocabulaire
Hôte, frame, savestate : voir SPEC-00.

## 3. Exigences

### 3.1 Modules et dépendances
**REQ-ARCH-001** (P1) — La solution DOIT comporter des bibliothèques séparées pour : le cœur NES, le format de profil, la conversion 2D→3D et la géométrie, le runtime Lua, le rendu, la plateforme (fenêtre, entrées, audio) et l'application.
- Critère d'acceptation : la solution contient des projets distincts pour ces responsabilités ; le graphe de références est celui de REQ-ARCH-002.

**REQ-ARCH-002** (P1) — Le cœur NES, le format de profil, la conversion 2D→3D et le runtime Lua NE DOIVENT dépendre d'aucune API de fenêtrage, de rendu, d'audio ou d'entrée.
- Critère d'acceptation : ces bibliothèques compilent et s'exécutent dans un processus sans fenêtre (le harnais headless) ; leurs références de paquets n'incluent aucune bibliothèque graphique, audio ou d'entrée.

**REQ-ARCH-003** (P1) — Le rendu DOIT dessiner dans une cible fournie par l'hôte (identifiant de framebuffer et dimensions) et NE DOIT PAS supposer qu'il possède la fenêtre ni le framebuffer par défaut.
- Critère d'acceptation : le harnais peut faire un rendu hors écran dans un FBO qu'il a créé et en relire les pixels.

### 3.2 Pilotage par l'hôte et déterminisme
**REQ-ARCH-010** (P1) — L'émulation DOIT avancer uniquement quand l'hôte demande « exécuter une frame » avec l'état des entrées de cette frame. Aucune horloge interne ne fait avancer le cœur ou la conversion.
- Critère d'acceptation : sans appel de l'hôte, l'état du cœur ne change pas ; N appels produisent exactement N frames émulées.

**REQ-ARCH-011** (P1) — Il NE DOIT y avoir aucun état global mutable. Deux instances d'émulation dans le même processus sont indépendantes.
- Critère d'acceptation : deux instances chargées avec deux ROMs différentes et exécutées en alternance produisent les mêmes frames que si chacune tournait seule.

**REQ-ARCH-012** (P1) — À ROM, profil, savestate initial et séquence d'entrées identiques, la suite des frames émulées, des tiles produites, des résultats de conversion et de l'état Lua DOIT être identique d'une exécution à l'autre.
- Critère d'acceptation : deux exécutions du harnais avec les mêmes entrées produisent des exports identiques octet par octet (tiles, instances, meshes).
- Notes : tout aléa visible par les scripts (s'il existe) doit être graine-able et sauvegardé dans les savestates.

**REQ-ARCH-013** (P1) — Un savestate DOIT contenir tout ce qui est nécessaire pour reprendre à l'identique : cœur, état appris du moteur de conversion (registre d'images, patterns appris et temporaires, tracker ; DEC-R2-T5), caméra, données persistantes des scripts écrites par les hooks, réglages de session pertinents (SPEC-07 REQ-LOOP-040, SPEC-09 REQ-PERS-056). Une borne supérieure de sa taille DOIT pouvoir être annoncée avant la sérialisation, pour la ROM et le profil **chargés**, et rester valable pendant toute la session (le bloc du cœur contient l'image de la ROM, de taille variable d'une ROM à l'autre) ; l'implémentation PEUT, pour cela, réserver une capacité fixe à l'état appris de la conversion.
- Critère d'acceptation : sauvegarder à la frame N, exécuter M frames, recharger, puis rejouer les mêmes entrées reproduit les mêmes M frames ; la taille réelle est toujours inférieure ou égale à la taille annoncée au chargement de la ROM et du profil.

**REQ-ARCH-014** (P2) — Le travail en arrière-plan (chargement de ROM ou de profil, construction de meshes) PEUT être parallélisé, à condition que le résultat visible reste celui de REQ-ARCH-012.
- Critère d'acceptation : le test de REQ-ARCH-012 passe avec le parallélisme activé.

### 3.3 Entrées et réglages abstraits
**REQ-ARCH-020** (P1) — Les entrées DOIVENT passer par une couche abstraite d'actions logiques (boutons NES par joueur, actions caméra, hotkeys, Zapper) définie dans SPEC-08. Les bibliothèques indépendantes de la plateforme ne voient que ces actions.
- Critère d'acceptation : le harnais pilote une partie complète en fournissant uniquement des actions logiques.

**REQ-ARCH-021** (P1) — Le modèle de réglages (SPEC-09) DOIT être indépendant de l'UI : chaque réglage a une clé stable et un type énumérable, ce qui permet de l'exposer en core option libretro.
- Critère d'acceptation : la liste des réglages peut être énumérée par programme, avec clé, type, valeurs ou bornes et défaut.

### 3.4 Plateforme, outils et licence
**REQ-ARCH-030** (P1) — L'application cible Windows x64, .NET 10 et OpenGL 3.3 core. Les shaders DEVRAIENT rester compatibles GLES 3.0 quand c'est possible.
- Critère d'acceptation : l'application démarre et rend sur une machine Windows x64 qui dispose d'OpenGL 3.3.

**REQ-ARCH-031** (P1) — Le harnais headless DOIT pouvoir charger une ROM et un profil, rejouer une séquence d'entrées et exporter, pour des frames choisies, les tiles, les instances reconnues, les meshes (sous forme textuelle ou binaire documentée) et une capture hors écran.
- Critère d'acceptation : une commande documentée produit ces exports pour une ROM de test et une liste de frames.

**REQ-ARCH-032** (P1) — La reconstruction est distribuée sous GPL v3. Les portions adaptées de `third_party/3DNes_Engine` portent l'en-tête d'attribution défini dans `third_party/README.md`.
- Critère d'acceptation : `impl/LICENSE` contient la GPL v3 ; chaque fichier adapté porte l'en-tête.

**REQ-ARCH-033** (P2) — Les données utilisateur sont placées sous `%LOCALAPPDATA%\Sen`, sauf en mode portable (fichier `portable.txt` à côté de l'exécutable), où elles sont placées dans le dossier `userdata\` situé à côté de l'exécutable (DEC-09-Q7, SPEC-09 REQ-PERS-071, qui fait foi).
- Critère d'acceptation : selon la présence de `portable.txt`, les réglages sont écrits sous `%LOCALAPPDATA%\Sen` ou sous `userdata\` à côté de l'exécutable.

## 4. Flux d'une frame (informatif)
L'hôte lit les entrées et les convertit en actions logiques. Il demande au cœur d'exécuter une frame ; le cœur produit les tiles, la palette et l'audio (SPEC-02). Les hooks Lua sont appelés aux moments définis par SPEC-05. La conversion produit les instances (SPEC-04), puis les meshes (SPEC-04B). Le rendu dessine la scène dans la cible fournie par l'hôte (SPEC-06), et l'hôte présente l'image et envoie l'audio (SPEC-07, SPEC-11).

## 5. Questions ouvertes
Aucune.

## Historique

- 1.1-draft (AMD-001, relecture R2) : périmètre du savestate aligné sur DEC-R2-T5 et borne de taille précisée (REQ-ARCH-013) ; racine portable `userdata\` (REQ-ARCH-033).
