# AMD-010 — Interface d'émulateur classique (parité fonctionnelle)

Version : 1.0 | 2026-10-21 | **Normatif** (remplace la présentation de SPEC-10 ; complète la spec v1.1 gelée)
Origine : décision de l'utilisateur (R0), 2026-10-21.

## 1. Principe
L'interface de la reconstruction est une **interface d'émulateur de bureau classique** : barre de menus, boîtes de dialogue et fenêtre d'options, avec raccourcis clavier préconfigurés (modèle de référence : les émulateurs de bureau courants). Elle **ne reproduit pas** la présentation de l'original : écran à onglets GAME / LOAD / SAVE / SETTING / QUIT, grilles de tuiles paginées, navigation par onglets et pages, dimensions du canevas.

- **Les fonctions** de SPEC-10 restent **obligatoires** (§3), et leur comportement observable (textes des messages, validations, effets) reste celui de SPEC-10, SPEC-07, SPEC-08 et SPEC-09.
- **La présentation et la navigation** de SPEC-10 (disposition, onglets, pagination, nombre de tuiles par page, navigation Haut/Bas/Gauche/Droite dans les menus, style visuel) deviennent **informatives**. Les exigences de SPEC-10 qui ne portent que sur la présentation passent en **P3** ; tout écart de présentation est classé **D** d'avance au round 2.
- La navigation des menus **à la manette n'est pas requise**. Les actions de jeu et les raccourcis peuvent être liés à la manette (SPEC-08).

## 2. Structure exigée (nouvelles exigences)

**REQ-UI-200** (P1) — Barre de menus en haut de la fenêtre, avec au minimum les menus et entrées ci-dessous. Chaque entrée affiche son raccourci clavier par défaut.

| Menu | Entrées |
|---|---|
| **Fichier** | Ouvrir une ROM… ; ROMs récentes ▸ (10 dernières) ; Choisir le profil 3D… ; Importer un profil… ; Charger un état ▸ (emplacements 1–10, état rapide) ; Sauvegarder l'état ▸ (emplacements 1–10, état rapide) ; Quitter |
| **Émulation** | Pause / Reprise ; Reset ; Arrêter ; Avance rapide (bascule) ; Rollback ; Vitesse ▸ ; Région ▸ (Auto, NTSC, PAL, Dendy) ; Ports ▸ (Manette / Zapper pour chaque port) |
| **Vidéo** | Mode 3D / Mode 2D ; Mode caméra ▸ ; Réinitialiser la vue ; Skybox ▸ ; Boîtier TV ▸ ; Ombres ; Plein écran ; Afficher les FPS |
| **Audio** | Volume ▸ ; Muet ; Musique de menu (on/off) |
| **Outils** | Options… ; Configuration des touches… ; Dossier des données ; À propos |

- Critère d'acceptation : chaque entrée existe et déclenche l'action décrite par la spec concernée ; un test d'interface (ou un test sur le modèle de commandes, cf. REQ-UI-205) déclenche chaque commande et vérifie son effet.

**REQ-UI-201** (P1) — **Ouvrir une ROM** : boîte de dialogue de fichiers (filtres `.nes`, `.zip` si pris en charge par SPEC-07, « Tous les fichiers »), mémorisation du dernier dossier. Après le choix, le profil 3D est déterminé ainsi : (1) le profil déjà associé à ce fichier dans la bibliothèque (`library.json`, SPEC-03/SPEC-09) ; sinon (2) une boîte de sélection du profil liste le catalogue embarqué et les profils utilisateur (avec une recherche par titre), plus « 2D Mode ». Le choix est mémorisé pour ce fichier. « Choisir le profil 3D… » rouvre cette sélection pour le jeu en cours.
- Critère d'acceptation : première ouverture d'une ROM → sélection du profil ; seconde ouverture du même fichier → lancement direct avec le profil mémorisé.

**REQ-UI-202** (P1) — **Fenêtre d'options** (Outils ▸ Options…) : tous les réglages de portée U de SPEC-09 que SPEC-10 rendait visibles, regroupés par thème (Vidéo, Caméra, Lumière et ombres, Audio, Émulation, Interface), avec leurs bornes, valeurs par défaut et effets ; un bouton « Valeurs par défaut » par onglet. Les réglages hors UI dans l'original peuvent y figurer (P3).
- Critère d'acceptation : chaque réglage U listé par SPEC-10 se modifie dans la fenêtre, prend effet selon SPEC-09 et est persisté.

**REQ-UI-203** (P1) — **Configuration des touches** (Outils ▸ Configuration des touches…) : les actions de SPEC-08 (boutons NES par joueur, actions caméra, hotkeys) **et** les commandes de menu, liables au clavier et à la manette selon les règles de capture de SPEC-08 (DEC-08-2, DEC-08-5).
- Critère d'acceptation : relier une commande de menu à un bouton de manette, puis l'appuyer → la commande s'exécute.

**REQ-UI-204** (P1) — **Raccourcis clavier par défaut** : ceux de SPEC-08 pour le jeu et la caméra, plus les raccourcis de menu suivants, sans conflit avec les touches de jeu par défaut :

| Commande | Raccourci |
|---|---|
| Ouvrir une ROM | Ctrl+O |
| Charger l'état n | Ctrl+Fn (n = 1–10) ; Fn seul reste aux touches caméra de SPEC-08 (Q-030) |
| Sauvegarder l'état n | Maj+Fn |
| État rapide : sauvegarder / charger | hotkeys QuickSave / QuickLoad de SPEC-08 |
| Pause / Reprise | P (en plus du comportement de la touche Menu de SPEC-08) |
| Reset | Ctrl+R |
| Avance rapide | hotkey FastForward de SPEC-08 |
| Rollback | hotkey RollBack de SPEC-08 |
| Mode 3D / 2D | F11 |
| Plein écran | Alt+Entrée |
| Réinitialiser la vue | appui long sur Menu (SPEC-08) |
| Options | Ctrl+, |
| Quitter | Ctrl+Q |

- Critère d'acceptation : chaque raccourci déclenche sa commande avec la configuration par défaut.

**REQ-UI-205** (P1) — Les commandes sont implémentées comme un **modèle de commandes** indépendant de l'UI (nom stable, action, état activé/coché), que la barre de menus, les raccourcis et la manette invoquent. C'est ce modèle que les tests du round 1 pilotent.
- Critère d'acceptation : le modèle énumère toutes les commandes de REQ-UI-200 ; chacune est invocable par programme.

**REQ-UI-206** (P1) — **Notifications** : affichées en surimpression (OSD) dans la vue du jeu, avec les textes, les sévérités et les durées de SPEC-10.
- Critère d'acceptation : ceux de SPEC-10 pour les notifications.

**REQ-UI-207** (P2) — Quand aucune ROM n'est chargée, la fenêtre affiche un écran d'accueil simple : « Fichier ▸ Ouvrir une ROM » et la liste des ROMs récentes. La musique de menu de SPEC-11 joue si elle est activée.

**REQ-UI-208** (P2) — **Emplacements de sauvegarde** : 10 emplacements numérotés plus l'état rapide sont exposés dans les menus. Le modèle de persistance de SPEC-09 reste valable ; les emplacements au-delà de 10 ne sont pas exposés **(D)**. Les sous-menus indiquent pour chaque emplacement la date et l'heure de l'état, ou « vide ».

**REQ-UI-209** (P1) — **Plein écran** : en mode plein écran, la barre de menus (et tout autre élément d'interface hors OSD) est **masquée** ; la vue du jeu occupe tout l'écran. Tous les raccourcis clavier et manette restent actifs. Quitter le plein écran (Alt+Entrée ou la commande liée) fait réapparaître la barre de menus. Les fenêtres d'options et de configuration, si elles sont ouvertes par raccourci, s'affichent par-dessus la vue. Le curseur suit la règle de masquage de SPEC-08.
- Critère d'acceptation : passer en plein écran → plus aucune barre de menus visible, les raccourcis (pause, sauvegarde rapide, Alt+Entrée) fonctionnent ; revenir en fenêtré → la barre de menus est de nouveau visible.
- Notes : afficher temporairement la barre quand le pointeur touche le bord haut de l'écran est PERMIS (P3), pas exigé.

## 3. Fonctions de SPEC-10 qui restent obligatoires
Choix d'une ROM et de son profil (bibliothèque) ; import et suppression de profils utilisateur ; sauvegarde et chargement d'états ; reset, reprise, arrêt et sortie ; tous les réglages visibles ; configuration des touches ; notifications et messages d'erreur (textes exacts) ; bloc « À propos » (DEC-10-Q3).

## 4. Ce qui est retiré ou neutralisé
- La présentation à onglets, les grilles paginées et les jaquettes en tuiles de SPEC-10 : informatives, P3. Une jaquette PEUT être affichée dans la sélection de profil.
- La navigation des menus à la manette (SPEC-10) : non requise.
- L'onglet initial (DEC-10-Q6) et le bouton « Site » (DEC-10-Q5) : sans objet.

## 5. Vérification
- **Round 1** : le testeur vérifie REQ-UI-200 à 208 et les fonctions du §3, principalement via le modèle de commandes (REQ-UI-205) et la persistance ; la présentation n'est pas jugée.
- **Round 2** : le juge ne compare pas l'interface visuellement à l'original ; il vérifie seulement que chaque fonction de l'original couverte par le périmètre existe et se comporte comme spécifié.
