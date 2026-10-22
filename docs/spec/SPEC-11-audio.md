# SPEC-11 — Audio : sortie de l'émulation, musique et sons de l'interface
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-02 (APU et production d'échantillons), SPEC-07 (états, vitesse, rollback), SPEC-09 (réglages), SPEC-10 (UI)

## 1. Objet et périmètre

Ce document décrit ce que l'utilisateur entend : la sortie audio de la console émulée (fréquence, canaux, latence, mise en tampon, synchronisation, volume) et l'habillage sonore de l'interface (musiques de menu, sons de navigation et de validation), ainsi que leur comportement en pause, en avance rapide, au rollback et à la perte de focus.

Hors périmètre : la synthèse des voies de l'APU, le mixage non linéaire et le rééchantillonnage à bande limitée (SPEC-02) ; la spatialisation 3D (aucune : tout le son est non spatialisé) ; l'audio VR.

## 2. Concepts et vocabulaire

- **Volume général** `V` : réglage dans [0 ; 1], affiché en pourcentage entier 0..100 dans l'UI, défaut 1 (100 %).
- **Musique de menu activée** `M` : réglage booléen « BG Music », défaut vrai.
- **Gain** : facteur multiplicatif linéaire appliqué à l'amplitude des échantillons (1 = niveau d'origine du signal ou du fichier).
- **Bloc de sortie** : quantité d'échantillons demandée à chaque appel par le périphérique audio.
- **Fondu** : variation linéaire du gain dans le temps.
- **Fichiers audio** : ceux de `assets-neutral/audio/` ; tous en PCM 44,1 kHz.

| Fichier | Rôle | Canaux | Durée |
|---|---|---|---|
| `happynes.wav` | musique de menu 1 | stéréo | ≈ 112,1 s |
| `Komiku_-_02_-_Home.wav` | musique de menu 2 | stéréo | ≈ 66,1 s |
| `dire-space-emergency-chiptune.wav` | musique de menu 3 | stéréo | ≈ 164,1 s |
| `sfx_menu_move3.wav` | changement d'onglet | mono | ≈ 0,076 s |
| `sfx_menu_move2.wav` | changement de page | mono | ≈ 0,060 s |
| `sfx_menu_move1.wav` | survol / déplacement de sélection | mono | ≈ 0,039 s |
| `sfx_coin_double1.wav` | validation | mono | ≈ 0,209 s |

## 3. Exigences

### 3.1 Sortie audio de l'émulation

**REQ-AUD-001** (P1) — La sortie DOIT être ouverte à **48 000 Hz** si le périphérique l'accepte, sinon à la fréquence native du périphérique ; le cœur DOIT produire ses échantillons directement à la fréquence de sortie effective (rééchantillonnage dans le cœur, SPEC-02).
- Critère d'acceptation : sur un périphérique 48 kHz, enregistrement en boucle locale d'un carré de l'APU à 440 Hz → fondamentale à 440 Hz ± 0,5 % ; sur un périphérique forcé à 44,1 kHz → même fondamentale.

**REQ-AUD-002** (P1) — Le signal de la console est **monophonique**. Sur une sortie stéréo, le même échantillon DOIT être envoyé sur les deux canaux (aucune panoramique, aucun effet).
- Critère d'acceptation : canaux gauche et droit identiques échantillon par échantillon.

**REQ-AUD-003** (P1) — Modèle de lecture : le périphérique réclame des blocs d'environ **512 trames** (≈ 10,7 ms à 48 kHz). Si le tampon du cœur contient au moins un bloc complet, le bloc DOIT être rempli avec les échantillons les plus anciens disponibles. S'il en contient moins, le bloc entier DOIT être rempli de silence (zéros) et les échantillons disponibles DOIVENT être conservés pour l'appel suivant (sous-alimentation sans craquement partiel).
- Critère d'acceptation : mettre en pause → la sortie devient du silence exact (zéros) au plus tard au bloc suivant l'épuisement du tampon ; aucun bloc ne contient un mélange de signal et de zéros de remplissage.
- Notes : la latence totale visée (tampon du cœur + bloc + pilote) est inférieure à 60 ms ; DEVRAIT être inférieure à 40 ms.

**REQ-AUD-004** (P1) — Synchronisation : l'émulation est cadencée par son horloge propre (SPEC-07), **pas** par l'audio. Le calage du rééchantillonneur est celui de SPEC-02 (REQ-CORE-070 et REQ-CORE-072), qui fait foi : chaque frame émulée produit en moyenne `fréquence de sortie / (cadence nominale × k)` échantillons, la cadence nominale valant 60 images/s en NTSC et 50 en PAL et Dendy (DEC-R2-T3) et `k` étant le facteur de vitesse courant (SPEC-07). Comme le frontend enchaîne `cadence nominale × k` frames par seconde, le nombre d'échantillons produits par seconde réelle reste égal à la fréquence de sortie, à toute vitesse.
- Critère d'acceptation : 60 s de jeu à k = 1 → écart entre échantillons produits et échantillons consommés inférieur à 0,2 % ; pas de sous-alimentation récurrente en régime établi (moins d'un bloc de silence par minute hors pause).

**REQ-AUD-005** (P2) — Sur-alimentation : le tampon du cœur DOIT être borné (capacité recommandée : 100 à 200 ms). S'il est plein, les échantillons **les plus anciens** DOIVENT être abandonnés afin que la latence ne croisse pas indéfiniment. Le frontend DEVRAIT en outre corriger la dérive entre l'horloge d'émulation et l'horloge du périphérique par un ajustement du rapport de rééchantillonnage d'au plus ± 0,5 % (variation de hauteur ≤ 0,5 %, soit moins de 9 cents) ; le cœur expose à cet effet un réglage fin de ce rapport (SPEC-02 REQ-CORE-070, notes).
- Critère d'acceptation : 30 min de jeu à k = 1 → latence mesurée stable (variation < 20 ms), aucun abandon d'échantillons en régime établi.
- Notes : le comportement de l'original en sur-alimentation n'est pas observable directement ; cette exigence fixe un comportement sûr.

**REQ-AUD-006** (P1) — Volume de l'émulation : le gain appliqué au signal de la console est `V` (gain 1 au volume 100 %), sous réserve des fondus de REQ-AUD-020. Un changement de `V` DOIT prendre effet en moins de 100 ms. Il n'existe pas de réglage « muet » distinct : `V = 0` rend tout silencieux (émulation, musique et sons d'interface).
- Critère d'acceptation : V = 50 % → amplitude crête réduite de moitié (−6 dB ± 0,5 dB) par rapport à V = 100 %.

**REQ-AUD-007** (P2) — Dans le frontend libretro, les échantillons d'une frame émulée DOIVENT être livrés en un lot à l'hôte après chaque `retro_run`, en stéréo 16 bits (canaux identiques), à la fréquence déclarée dans les informations AV ; la synchronisation, le volume et la latence relèvent alors de l'hôte. Le gain `V` et les fondus PEUVENT être désactivés dans ce frontend.
- Critère d'acceptation : NTSC, fréquence déclarée 48 000 Hz → 800 trames par `retro_run` en moyenne (±1 sur 600 appels) ; PAL → 960 (cadence nominale de DEC-R2-T3, SPEC-02 REQ-CORE-070).

### 3.2 Comportement selon l'état et les actions

**REQ-AUD-010** (P1) — En pause et à l'arrêt, aucune frame émulée ne produit d'échantillons : la sortie de l'émulation DOIT être le silence (après épuisement du tampon, au plus ≈ 1 bloc + tampon). Au chargement d'un jeu ou d'un savestate et au reset, le tampon du cœur DOIT être vidé, afin qu'aucun son de la session précédente ne soit rejoué.
- Critère d'acceptation : mettre en pause pendant une note tenue → silence en moins de 100 ms ; lancer un autre jeu → aucun fragment du jeu précédent n'est audible.

**REQ-AUD-011** (P2) — Vitesse et avance rapide : l'audio de l'émulation DOIT suivre le facteur `k` par compression ou dilatation temporelle **avec changement de hauteur proportionnel** (k = 2 → une octave plus haut ; k = 0,5 → une octave plus bas). Il n'y a ni coupure du son ni étirement à hauteur constante.
- Critère d'acceptation : carré à 440 Hz joué à k = 2 (réglage s = +4) → 880 Hz ± 1 % ; pendant l'avance rapide (k = 1,5, SPEC-07 REQ-LOOP-016) → 660 Hz ± 1 % ; à k = 0,8 → 352 Hz ± 1 %.

**REQ-AUD-012** (P2) — Rollback : la restauration d'un snapshot (SPEC-07) NE DOIT PAS couper la sortie ; le son reprend à partir de l'état APU restauré. Une discontinuité audible (clic) à l'instant de la restauration est tolérée.
- Critère d'acceptation : rollback pendant une musique de jeu → la musique reprend dans la seconde, sans silence de plus de 50 ms.

**REQ-AUD-013** (P2) — Perte de focus : la perte de focus de la fenêtre NE DOIT PAS couper ni atténuer le son (l'émulation continue, SPEC-07). Une suspension explicite de l'application provoque la pause, donc le silence (REQ-AUD-010).
- Critère d'acceptation : jeu en cours, cliquer sur une autre fenêtre → son inchangé.

### 3.3 Fondus entre jeu et menu

**REQ-AUD-020** (P2) — Fondu de l'émulation : à chaque bascule entre la vue de jeu et le menu (SPEC-07, REQ-LOOP-003), le gain de l'émulation DOIT varier linéairement : vers `V` (fondu d'entrée) lorsque l'état devient En cours, vers 0 (fondu de sortie) sinon. Un fondu complet (de 0 à `V` ou de `V` à 0) DOIT durer **200/60 s ≈ 3,33 s** de temps réel, quelle que soit la cadence de rendu (DEC-11-Q2, divergence acceptée **(D)** : l'original applique une pente de `V / 200` par frame de rendu, d'où 3,33 s à 60 i/s mais 2,2 s à 90 i/s). Un fondu en cours est remplacé par le nouveau, à partir du gain courant, avec la même pente `V / (200/60 s)`.
- Critère d'acceptation : rendu à 60 i/s puis à 120 i/s, reprendre après une pause → le gain de l'émulation passe de 0 à V en 3,33 s ± 0,1 s dans les deux cas ; le premier lancement d'un jeu après le démarrage de l'application démarre directement au gain V (aucun fondu préalable n'a abaissé le gain).

**REQ-AUD-021** (P2) — Musique de menu, niveau : le gain cible de la musique est `0,02 × V` si `M` est vrai, sinon 0. Le gain des sons d'interface est `0,07 × V`. Ces gains s'appliquent aux échantillons des fichiers, sans normalisation.
- Critère d'acceptation : V = 100 %, M vrai, menu affiché depuis plus de 5 s → amplitude crête de la musique égale à 0,02 × crête du fichier (± 0,5 dB).
- Notes : ces faibles valeurs reflètent le mixage d'origine ; le jeu émulé est nettement plus fort que l'habillage sonore.

**REQ-AUD-022** (P2) — Musique de menu, fondus : tant que l'état n'est pas En cours (Arrêté, En pause, Chargement), la musique DOIT être en lecture et monter linéairement jusqu'au gain cible ; dès que l'état devient En cours, elle DOIT descendre linéairement jusqu'à 0 puis être suspendue. Un fondu complet de la musique DOIT durer 200/60 s ≈ 3,33 s de temps réel, comme celui de REQ-AUD-020 (DEC-11-Q2, D ; l'original applique `min(gain cible / 200 ; 0,001)` par frame de rendu, soit la même durée à 60 i/s avec les gains de REQ-AUD-021). Au démarrage de l'application, la musique part du gain 0 et monte avec cette même pente.
- Critère d'acceptation : lancer un jeu depuis le menu → la musique décroît en 3,33 s ± 0,1 s puis s'arrête, quelle que soit la cadence de rendu ; mettre en pause → elle revient progressivement.
- Notes : à la reprise du fondu d'entrée, la piste courante PEUT reprendre là où elle avait été suspendue ou repartir de son début (choix libre ; voir Q1).

**REQ-AUD-023** (P2) — Changement des réglages : un changement de `V` DOIT mettre à jour immédiatement (sans fondu) le gain des sons d'interface et, si la musique est dans sa phase active (menu affiché), le gain de la musique. Activer ou désactiver `M` DOIT porter immédiatement le gain de la musique à sa nouvelle cible (0,02 × V ou 0).
- Critère d'acceptation : menu affiché, décocher « BG Music » → musique coupée en moins de 100 ms ; recocher → musique de retour immédiatement, sans fondu.

### 3.4 Sélection des musiques

**REQ-AUD-030** (P2) — Les trois musiques DOIVENT être jouées **sans boucle** individuelle. Au démarrage, une musique est tirée au hasard ; chaque fois que la musique courante se termine alors que la musique est en phase active, une nouvelle musique DOIT être tirée au hasard **parmi les deux autres** (jamais deux fois de suite la même).
- Critère d'acceptation : laisser le menu ouvert 10 min → enchaînement de musiques sans répétition immédiate, sans silence de plus de 0,5 s entre deux pistes.
- Notes : le tirage n'a pas besoin d'être reproductible ; il DEVRAIT varier d'un lancement à l'autre.

### 3.5 Sons de l'interface

**REQ-AUD-040** (P2) — Les sons d'interface DOIVENT être joués sur une **voix unique** : déclencher un son interrompt immédiatement celui en cours. Le démarrage d'un son PEUT être retardé d'au plus 10 ms. Ils sont joués quel que soit l'état de l'émulation, y compris en jeu (quick save/load).
- Critère d'acceptation : deux validations à 50 ms d'intervalle → le premier son est coupé à 50 ms, le second est joué en entier.

**REQ-AUD-041** (P2) — Correspondance événement → son :

| Événement d'interface (SPEC-10) | Son |
|---|---|
| changement d'onglet du menu principal | `sfx_menu_move3.wav` |
| changement de page d'une grille paginée | `sfx_menu_move2.wav` |
| changement de l'élément survolé/sélectionné dans les grilles Jeux, Load, Save | `sfx_menu_move1.wav` |
| validation : sélection d'un jeu, ouverture d'un élément, sauvegarde ou chargement d'un slot, quick save, quick load, modification d'un réglage (curseur de volume, lumière, échelle, listes déroulantes, cases à cocher), import de jaquette, ajout ou suppression d'un profil, ouverture du lien communauté, export de scène | `sfx_coin_double1.wav` |

- Critère d'acceptation : déplacer la sélection dans la bibliothèque → son « move1 » à chaque déplacement ; déplacer le curseur de volume → son de validation à chaque changement de valeur, joué au nouveau volume.
- Notes : les curseurs de la page Réglages (dont Vitesse et FOV) déclenchent aussi un son d'interface à chaque changement ; le clip exact associé au curseur de vitesse n'a pas pu être déterminé (Q5), le son de validation est retenu par défaut.

## 4. Formats et interfaces

Constantes :

| Grandeur | Valeur |
|---|---|
| fréquence de sortie demandée | 48 000 Hz |
| taille de bloc demandée | 512 trames |
| canaux du signal de console | 1 (dupliqué en stéréo) |
| gain émulation | V |
| gain musique | 0,02 × V × (M ? 1 : 0) |
| gain sons d'interface | 0,07 × V |
| durée des fondus | 200 frames de rendu |

Réglages (persistance SPEC-09) : volume (0..1, UI 0..100 %, défaut 1), musique de menu (booléen, défaut vrai).

## 5. Vecteurs de test

| # | Entrée | Résultat attendu |
|---|---|---|
| A1 | V = 1, jeu en cours depuis 5 s, signal APU crête x | crête de sortie x (gain 1) |
| A2 | V = 0,25 | crête x/4 (−12 dB) |
| A3 | V = 1, M vrai, menu depuis 10 s | musique à 0,02 × crête du fichier ; aucun son d'émulation |
| A4 | V = 1, M faux, menu | musique silencieuse ; sons d'interface audibles |
| A5 | k = 2 (réglage s = +4) ; avance rapide (k = 1,5) | hauteur ×2 ; hauteur ×1,5 ; débit d'échantillons constant |
| A6 | pause | silence numérique (zéros) en moins de 100 ms |
| A7 | fin de piste de menu | nouvelle piste différente de la précédente démarre en moins de 0,5 s |

## 6. Questions ouvertes

- **Q1** — *Close par DEC-11-Q1* (reprise à la position de suspension, différentiel). À la reprise de la musique de menu après un jeu, l'original relance la lecture de la piste suspendue ; il n'est pas certain qu'elle reprenne à la position de suspension plutôt qu'au début. Vérification en différentiel souhaitée.
- **Q2** — *Close par DEC-11-Q2* : durée fixe de 200/60 s ≈ 3,33 s (REQ-AUD-020, REQ-AUD-022).
- **Q3** — *Close par DEC-11-Q3* (choix de REQ-AUD-005). Le comportement exact de l'original en sur-alimentation du tampon (abandon ou blocage) n'est pas observable ; REQ-AUD-005 fixe un choix sûr à valider par R0.
- **Q4** — *Close par DEC-11-Q4*. La fréquence de sortie réelle de l'original dépend du périphérique ; si le périphérique est à 44,1 kHz, l'original s'y adapte. REQ-AUD-001 retient ce comportement.
- **Q5** — *Close par DEC-11-Q5* (différentiel) : le clip associé aux curseurs Vitesse et FOV n'a pas pu être déterminé en boîte noire.

## Historique

- 1.1-draft (AMD-001, relecture R2) : calage audio renvoyé à SPEC-02 et cadence 60/50 (DEC-R2-T3) : 800 échantillons par retro_run (REQ-AUD-007) ; fondus de durée fixe 200/60 s (DEC-11-Q2) ; ajustement fin du rapport (REQ-AUD-005) ; vitesse d'avance rapide 1,5 ; note LEAK de Q5 reformulée ; questions closes.
