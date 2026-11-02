# AMD-052 — Cadence des frames, file de frames GPU et antialiasing par défaut

Version : 1.1 | 2026-10-31 | **Normatif** (R1, choix du propriétaire du 2026-10-31 : reprise des pistes du rapport de comparaison avec 3dSen 2.0, §4.2, §3.5 et §2). **Amende SPEC-07 REQ-LOOP-012** (énoncé **et** critère), **REQ-LOOP-013** (événement ajouté) **et le vecteur T5** ; **amende la ligne `video.anti_alias` de SPEC-09 §4.1.1** (défaut `0` → `1`) et la table §4.6 ; ajoute REQ-LOOP-077 et REQ-PERS-089 ; **retire REQ-SCN-077** (déplacée dans SPEC-07).
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-050-053 : 52-1 à 52-13, tous retenus ; pour 52-4, REQ-SCN-077 devient REQ-LOOP-077, son numéro restant attribué avec la mention « retirée » ; pour 52-3, la variante stricte est retenue — seul `emu.game_speed` recale l'horloge, le cycle de REQ-LOOP-016 jamais ; pour 52-7, la clause du défaut d'anticrénelage reçoit le numéro REQ-PERS-089 (P3) et vise la ligne de SPEC-09 §4.1.1 ; pour 52-10, `video.max_queued_frames` n'est pas exposé en libretro). ; 1.2 (R2-review-AMD-050-053-v1.1 : 52-N1, critère d'avance rapide sur l'énoncé de REQ-LOOP-011 à f = 90, constante de la Note corrigée).
Priorités : REQ-LOOP-074 (P1) est acquise seule. Si REQ-LOOP-075 (P2) n'est pas retenue, la liste de REQ-LOOP-013 reste celle de SPEC-07. Si REQ-LOOP-077 (P2) n'est pas retenue, aucune borne n'est imposée à la file de frames et le réglage `video.max_queued_frames` n'existe pas. Si REQ-PERS-089 (P3) n'est pas retenue, le défaut de `video.anti_alias` reste `0` et rien d'autre ne change (AMD-032 §2 inchangé par ailleurs).

## 1. Besoin

3dSen 2.0 rattrape au plus 2 frames après un retard (5 auparavant) : au-delà, le retard est
abandonné et l'horloge repart, ce qui évite les accélérations visibles après une saccade. Il recale
aussi l'horloge quand la vitesse de jeu change. Il ajoute un réglage du nombre de frames que le GPU
peut garder en attente d'affichage (compromis latence / fluidité) et livre l'antialiasing activé.

Les trois sujets sont indépendants et sont traités séparément ci-dessous.

## 2. Exigences

### 2.1 Cadence (SPEC-07 §3.2)

**REQ-LOOP-074** (P1) — **Rattrapage limité à 2 frames** (remplace la limite de 5 de REQ-LOOP-012) :
si l'émulation prend du retard (frame plus longue que `1/f`, système chargé), la boucle DOIT
exécuter les frames en retard à la suite, sans attente, **dans la limite de 2 frames de retard**,
comptées en frames émulées : **le nombre de frames dues et non encore exécutées NE DOIT jamais
dépasser 2**, chaque frame due correspondant à une période `1/f` calculée avec le facteur `k`
courant (avance rapide comprise). Le retard au-delà de 2 frames DOIT être abandonné et l'horloge de
cadence réinitialisée (REQ-LOOP-013) à l'instant présent.
- Critère d'acceptation (auto, vecteur T5 amendé) : blocage de 1 s de la boucle, NTSC : au plus
  2 frames rattrapées ; ≈ 58 frames perdues.
- Critère d'acceptation (auto, critère amendé de REQ-LOOP-012) : bloquer la boucle 500 ms à 60 Hz →
  à la reprise, au plus 2 frames sont exécutées en rafale, puis la cadence normale reprend ; le
  compteur de frames accuse ≈ 28 frames de moins qu'une exécution sans blocage.
- Notes : la grandeur bornée est exactement celle de REQ-LOOP-012 (le **retard accumulé**), reprise
  mot pour mot, et non un nombre de frames « de rattrapage d'affilée » : les deux lectures diffèrent
  d'une frame. Arithmétique du vecteur T5 : la cadence nominale est de 60 images/s en NTSC
  (DEC-R2-T3, SPEC-07 §2), donc 60 frames sont dues sur 1 s de blocage, 2 sont exécutées au
  rattrapage et 58 sont abandonnées. Pour 500 ms : 30 dues, 2 exécutées, 28 perdues (contre 5 et 25
  avec la limite précédente).

**REQ-LOOP-075** (P2) — **Changement de vitesse** : une modification du réglage `emu.game_speed`
(REQ-LOOP-015, y compris par les raccourcis ±1) DOIT réinitialiser l'horloge de cadence à l'instant
du changement, sans rattrapage. En revanche, l'**armement, l'expiration et le réarmement** d'une
avance rapide (REQ-LOOP-016) NE DOIVENT PAS la réinitialiser.
- Critère d'acceptation (auto) : passer de la vitesse normale à ×2 puis revenir ne produit aucune
  rafale de frames de rattrapage.
- Critère d'acceptation (auto) : NTSC, `s = 0`, maintenir la hotkey d'avance rapide 5 s → le
  compteur de réinitialisations d'horloge reste à 0 pendant le maintien ; le critère de
  REQ-LOOP-016 (≈ 450 frames émulées ±5 %) reste vérifié, et l'écart entre deux frames émulées
  consécutives reste dans `1/f ± 2 ms` avec `f = 90` (énoncé de REQ-LOOP-011) pendant le maintien,
  puis avec `f = 60` après le relâchement.
- Notes : l'exclusion du cycle de REQ-LOOP-016 est nécessaire. L'avance rapide s'arme par salves de
  30 frames émulées, réarmées dès qu'elles s'achèvent tant que la hotkey est maintenue (une demande
  à chaque image affichée, REQ-INP-103) : un maintien produit donc deux changements d'état
  (expiration puis réarmement) par salve, soit ≈ 3 salves et ≈ 6 changements d'état par seconde. Réinitialiser l'horloge à chacun
  abandonnerait le reliquat de retard plusieurs fois par seconde et rendrait invérifiables les
  critères de REQ-LOOP-016 et de REQ-LOOP-011.

**REQ-LOOP-077** (P2) — **File de frames GPU** (remplace REQ-SCN-077 de la v1.0, retirée) : le
frontend desktop DOIT offrir un réglage `video.max_queued_frames` (portée U, persisté, valeurs 1 ou
2, défaut 2) qui borne le nombre de frames rendues que le GPU peut avoir en attente d'affichage : à
1, le programme attend, avant de commencer le rendu d'une frame, que la frame précédente soit
terminée par le GPU (latence minimale) ; à 2, il peut avoir une frame d'avance. L'attente NE DOIT
PAS décaler la cadence des frames émulées (REQ-LOOP-010) et ne constitue pas un retard au sens de
REQ-LOOP-074 ; à 1, le saut de rendu de REQ-LOOP-014 reste applicable. Le réglage est **sans objet
en libretro**, où la présentation et la chaîne d'échange appartiennent à l'hôte (REQ-PERS-086) et où
le moteur dessine dans une cible fournie par l'hôte (REQ-SCN-057).
- Critère d'acceptation (auto) : le réglage est persisté et survit à un redémarrage.
- Critère d'acceptation (auto, GL 3.3 core, REQ-SCN-056) : un objet de synchronisation créé après la
  dernière commande de la frame *n* (`glFenceSync`, interrogé par `glClientWaitSync` à délai nul)
  DOIT être signalé **avant** la première commande de la frame *n + 1* quand
  `video.max_queued_frames` = 1 ; à 2, ce n'est pas exigé.
- Critère d'acceptation (auto) : sur le banc de REQ-ACC-030, la latence mesurée entre la fin de
  l'émulation d'une frame et sa présentation est plus courte à 1 qu'à 2.
- Critère d'acceptation (auto) : à 1, NTSC, `k = 1`, 10 s → 600 ± 1 frames émulées (critère de
  REQ-LOOP-010 inchangé), y compris quand le rendu est volontairement ralenti par le harnais.
- Notes : borner le nombre de frames **présentées** en attente est une affaire de présentation,
  donc du frontend desktop ; c'est pourquoi l'exigence a changé de document (REQ-SCN- → REQ-LOOP-).
  Sa rédaction en REQ-SCN- de la v1.0 contredisait REQ-SCN-057 (P1) et la contrainte transverse de
  portabilité du SPEC-GUIDE, qui placent le moteur dans un périmètre où il ne contrôle ni la chaîne
  d'échange ni la présentation.

### 2.2 Anticrénelage par défaut (SPEC-09)

**REQ-PERS-089** (P3) — **Anticrénelage activé à l'installation** (divergence assumée **(D)**) : sur
une installation neuve (racine utilisateur vierge, REQ-PERS-075), la valeur par défaut de
`video.anti_alias` DOIT être **1**, soit MSAA 4× par AMD-032 §2. Un fichier de réglages existant
garde sa valeur (REQ-PERS-020, REQ-PERS-023) ; REQ-PERS-017 (« sans effet dans l'original ») est
inchangée, de même que le domaine, les libellés, le champ de profil 32 et l'API Lua.
- Critère d'acceptation (auto) : racine utilisateur vierge → `video.anti_alias` = 1 et
  `Zax.Harness render` rapporte 4 échantillons accordés (critère d'AMD-032 §2) ; un `settings.json`
  portant `"video.anti_alias": 0` → la valeur reste 0 après le lancement.
- Notes : c'est une **divergence (D)** supplémentaire, et la seule clause de cet amendement qui
  change l'image produite à la première ouverture : le programme d'origine force son anticrénelage à
  « désactivé » (REQ-PERS-017) et son défaut est 0. Justification, déjà écrite dans AMD-032 §2 : le
  multi-échantillonnage ne touche que les arêtes géométriques et ne touche pas au pixel art, qu'un
  filtre de post-traitement flouterait. La valeur par défaut d'un réglage de portée U ne circule pas
  par les profils (REQ-PERS-005 point 2) : l'aller-retour du champ 32 est inchangé.
- Notes (rédaction) : la v1.0 énonçait cette clause sous le titre « AMD-032 §2 amendé », sans
  identifiant REQ et sans viser le texte qui porte réellement la valeur. Elle était inopérante :
  la valeur par défaut `0` est fixée par la colonne « Défaut » de la ligne `video.anti_alias` de
  SPEC-09 §4.1.1, et AMD-032 §2 dit au contraire que « les libellés **et le domaine** de SPEC-09
  §4.1.1 sont **inchangés** ». C'est donc la ligne de SPEC-09 qui est amendée (§3 point 5).

## 3. Changements documentaires

1. SPEC-07 §3.2 REQ-LOOP-012, énoncé amendé : « **dans la limite de 2 frames de retard** » au lieu
   de 5, « le nombre de frames dues et non encore exécutées ne dépasse jamais **2** », « Le retard
   au-delà de **2** frames DOIT être abandonné » ; le reste de l'énoncé est inchangé.
2. SPEC-07 §3.2 REQ-LOOP-012, critère d'acceptation amendé : « bloquer la boucle 500 ms à 60 Hz → à
   la reprise, **au plus 2 frames** sont exécutées en rafale, puis la cadence normale reprend ; le
   compteur de frames accuse **≈ 28 frames** de moins qu'une exécution sans blocage ».
3. SPEC-07 §3.2 REQ-LOOP-013 : la liste des réinitialisations de l'horloge de cadence reçoit « à
   chaque modification du réglage `emu.game_speed` (REQ-LOOP-075) ». L'avance rapide
   (REQ-LOOP-016) n'y figure pas.
4. SPEC-07 §3.2 : REQ-LOOP-074 et REQ-LOOP-075 ajoutées ; REQ-LOOP-077 ajoutée après
   REQ-LOOP-014 (cadence de rendu). SPEC-06 §3.17 : REQ-SCN-077 inscrite comme **retirée**
   (déplacée dans SPEC-07 par cet amendement).
5. SPEC-07 §5, vecteur T5, ligne réécrite :
   `| T5 | blocage de 1 s de la boucle, NTSC | au plus 2 frames rattrapées ; ≈ 58 frames perdues |`
6. SPEC-09 §4.1.1, ligne `video.anti_alias` réécrite (seule la colonne « Défaut » change, plus un
   renvoi en « Comportement ») :

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `video.anti_alias` | `AntiAlias` | 32 | entier | **1** | 0 None, 1 FFAA, 2 SMAA | U | imm. | « Anti Alias » : None/FFAA/SMAA | None/FFAA/SMAA | sans effet dans l'original (REQ-PERS-017), SPEC-06 ; multi-échantillonnage dans Zaxnes (AMD-032 §2) ; défaut 1 **(D)**, REQ-PERS-089 |

7. SPEC-09 §4.1.1, ligne ajoutée pour le réglage de REQ-LOOP-077 :

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `video.max_queued_frames` | — | — | entier | 2 | 1 ou 2 | U | imm. | liste « Max Queued Frames » / « Frames en file GPU », groupe **Vidéo** (AMD-010 REQ-UI-202), après « Render Rate » | — | borne les frames rendues en attente d'affichage (REQ-LOOP-077) ; sans objet en libretro |

8. SPEC-09 §4.6 : la ligne `zaxnes_video_anti_alias | None, FFAA, SMAA | **FFAA** | Rendu` est
   ajoutée à la table (préfixe `zaxnes_` d'AMD-029 §3, qui remplace le `3dsen_` encore écrit dans
   SPEC-09 §4.6) ; `video.max_queued_frames` est ajouté à la liste « Non exposés en libretro »,
   à côté de `video.vsync` et `video.frame_rate`.
9. SPEC-09 §3.2 : REQ-PERS-089 ajoutée après REQ-PERS-024.
10. AMD-035, table des textes (anglais par défaut / français) :

| Élément | Anglais | Français |
|---|---|---|
| Frames en file GPU | `Max Queued Frames` — « Limits how many rendered frames the GPU may keep waiting for display. 1 gives the lowest latency; 2 is smoother. » | `Frames en file GPU` — « Limite le nombre de frames rendues que le GPU peut garder en attente d'affichage. 1 donne la latence la plus basse ; 2 est plus fluide. » |

11. SPEC-v1.2-INDEX : REQ-LOOP-074, REQ-LOOP-075 et REQ-LOOP-077 ajoutées à la liste de SPEC-07,
    REQ-PERS-089 à celle de SPEC-09, REQ-SCN-077 à celle de SPEC-06 avec la mention « retirée » ;
    priorité et critère de REQ-LOOP-012 mis à jour.

## 4. Étanchéité (pour mémoire)

Le §1 décrit des comportements observables de 3dSen 2.0 (limite de rattrapage, recalage de l'horloge
à un changement de vitesse, réglage de file de frames, anticrénelage activé) et un réglage
utilisateur-visible. La limite précédente de 5 était déjà écrite et justifiée dans REQ-LOOP-012.
Aucun nom interne, aucun code, aucune architecture interne.

## 5. Questions

- Aucune.
