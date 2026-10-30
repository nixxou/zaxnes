# AMD-043 — Courbure de l'espace (Bend et Arc)

Version : 1.2 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30). Complète SPEC-06 (rendu), SPEC-08 (visée) et SPEC-09 (réglages) ; **amende la parenthèse de REQ-SCN-028** sur la normale.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-039-045 : 43-1 à 43-16) ; 1.2 (seconde relecture R2-review-AMD-039-045-v1.1 : N-43-1 à N-43-4).
Priorités : Si une exigence P2 de cet amendement qui remplace une clause P1 n'est pas retenue, la clause d'origine reste valable (T-1).

## 1. Besoin

3dSen 2.0 peut courber toute la scène : comme le verre bombé d'une vieille télévision, en
enveloppant le joueur, ou en enroulant un niveau à défilement horizontal autour d'une « petite
planète ». Cette **courbure** est un effet purement géométrique, appliqué au rendu. Elle ne touche
ni les profils ni la conversion 2D → 3D. Elle est distincte de la **déformation animée** des motifs
(REQ-SCN-043).

## 2. Réglages

**REQ-SCN-066** (P2) — **Réglages de courbure** (portée U, persistés, SPEC-09 ; lignes du §6) :

| Réglage | Id Lua (AMD-040 §3, style 1) | Valeurs | Défaut |
|---|---|---|---|
| Courbure gauche-droite | `spl.bend_zx` | −180° à 180°, pas de 30° | 0 |
| Courbure haut-bas | `spl.bend_zy` | idem | 0 |
| Arc horizontal | `spl.arc_yx` | idem | 0 |
| Centre de l'arc | `spl.arc_center` | 0 = centre de l'écran, 1 = bord bas du cadre de référence (y = 0, 8 unités sous le bas de l'image NES) | 0 |

Ces réglages s'appliquent à chaud, dès la frame rendue suivante. Dans Zaxnes, ils agissent quel
que soit `Setting.VisualStyle` (Zaxnes n'a qu'un rendu) et se lisent et s'écrivent aussi par
`Setting:Get/SetVisualStyleParameter(1, id, …)` (REQ-LUA-114), avec la normalisation d'AMD-040
§3. Ils DEVRAIENT être présentés dans un groupe « Géométrie », marqué avancé, avec les libellés et
aides du §6.
- Critère d'acceptation (auto) : écrire 45 dans Courbure gauche-droite donne 60 (AMD-040 §3) ; la
  valeur survit à un redémarrage.

## 3. Courbure

**REQ-SCN-067** (P2) — **Transformation des points** : avec s l'échelle de scène (1 sur desktop,
REQ-SCN-002) et C = (128, 120, 0) le centre du cadre de référence (REQ-E3D-410), qui est aussi la
cible de caméra par défaut, chaque point P = (x, y, z) de la scène courbée (REQ-SCN-068) DOIT être
rendu à la position B(P) obtenue ainsi :

1. Pour chacun des trois réglages d'angle θ (en radians) et sa longueur de référence L
   (256 pour Bend gauche-droite et pour Arc horizontal, 240 pour Bend haut-bas) :
   - courbure k = θ / (L · s) ;
   - avec φ = θ / 4 : facteur de recadrage f = φ / sin φ et décalage h = 2 sin²(φ / 2) / k.
2. L'**enroulement** E(k, f, h) d'un couple (u, w) — u le long de l'axe courbé, w la
   profondeur — sur un cylindre de rayon 1/k est :
   - w′ = 0,95 / k si w · k > 0,95, sinon w′ = w (on ne dépasse pas 95 % du rayon, dans les deux
     sens de courbure) ;
   - a = u · k ;
   - u ← f · (1 − w′ k) · sin(a) / k ;
   - w ← f · ((1 − cos a) / k + w′ cos a − h).
3. Partant de (u, v, w) = P − C, appliquer dans cet ordre :
   - Bend gauche-droite : E sur (u, w) ;
   - Bend haut-bas : E sur (v, w) ;
   - Arc horizontal : avec le pivot p = 0 (centre de l'écran) ou p = −120 · s (bord bas du cadre de référence), E sur
     (u, p − v), puis v ← p − (la seconde composante obtenue).
4. B(P) = (u, v, w) + C.

B est continue en θ et vaut l'identité en θ = 0 (limites k → 0, f → 1, h → 0) : un axe dont
l'angle est nul ne transforme rien. Une implémentation PEUT traiter comme nul tout |θ| inférieur à
un seuil de son choix ne dépassant pas 10⁻⁴ rad.

L'axe z est la profondeur de la scène (la caméra par défaut regarde vers les z croissants) : un
angle positif éloigne les bords (bombé), un angle négatif les rapproche (enveloppant).
- Critère d'acceptation (auto, vecteurs, ±0,05 unité) :

| Réglages (bend_zx, bend_zy, arc_yx, centre) | P | B(P) |
|---|---|---|
| (60, 0, 0, milieu) | (128, 120, 0) | (128 ; 120 ; −8,43) |
| (60, 0, 0, milieu) | (256, 120, 0) | (251,64 ; 120 ; 24,70) |
| (60, 0, 0, milieu) | (0, 120, 0) | (4,36 ; 120 ; 24,70) |
| (60, 0, 0, milieu) | (256, 120, 10) | (246,58 ; 120 ; 33,46) |
| (−60, 0, 0, milieu) | (256, 120, 0) | (251,64 ; 120 ; −24,70) |
| (−180, 0, 0, milieu) | (256, 120, −200) | (132,53 ; 120 ; −64,00) (bornage actif) |
| (0, 60, 0, milieu) | (128, 240, 0) | (128 ; 235,91 ; 23,16) |
| (0, 60, 0, milieu) | (128, 0, 0) | (128 ; 4,09 ; 23,16) |
| (0, 0, 60, milieu) | (0, 120, 0) | (4,36 ; 95,30 ; 0) |
| (0, 0, 60, milieu) | (128, 240, 0) | (128 ; 249,81 ; 0) |
| (0, 0, 60, bord) | (0, 0, 0) | (4,36 ; −24,70 ; 0) |
| (0, 0, 60, bord) | (128, 240, 0) | (128 ; 251,19 ; 0) |
| (0, 0, −180, milieu) | (128, 240, 0) | (128 ; 179,47 ; 0) (bornage actif) |

- Notes : les constantes (φ = θ/4, le seuil de 95 %, les longueurs 256 et 240, le pivot −120) sont
  mesurées sur 3dSen 2.0. φ = θ/4 rend l'image courbée **approximativement** de la même taille
  apparente que l'image plate (à 60° gauche-droite, la demi-largeur passe de 128 à 123,6) ; les
  vecteurs font foi.

**REQ-SCN-068** (P2) [ROM] — **Portée** :
- la courbure DOIT s'appliquer à la géométrie de la scène de jeu — tiles et sprites convertis
  (opaques et transparents) et fond (backdrop) — **après** la déformation animée (REQ-SCN-043) et
  après la position, l'échelle et la rotation propres à chaque objet, donc sur la position finale
  dans la scène ;
- elle DOIT s'appliquer aussi à la passe d'ombres, pour que les ombres suivent la surface courbée ;
- elle NE DOIT PAS s'appliquer au HUD (plan attaché à la caméra, REQ-SCN-045), à la skybox ni au
  boîtier TV, qui restent plats ;
- les surfaces planes assez grandes pour se courber visiblement DEVRAIENT être subdivisées ; le
  fond DEVRAIT l'être en au moins 32 × 32 cellules par face ;
- **ordre avec les calculs en coordonnées de scène** : la découpe par le rectangle visible
  (REQ-SCN-036), le point visé P* de la caméra (REQ-SCN-008) et le segment d'occultation du FPV
  (REQ-SCN-022) DOIVENT être évalués sur les positions **avant** courbure ; la courbure n'intervient
  qu'au placement final des sommets. Une tile gardée par la découpe reste donc visible même si son
  image courbée sort du rectangle R ;
- **REQ-SCN-028 amendée** : la normale N utilisée pour l'ombrage DEVRAIT être celle du maillage
  transportée par la courbure (la parenthèse « inchangée par la déformation de REQ-SCN-043 » reste
  vraie pour la déformation animée).
- Critère d'acceptation (auto) : harnais, Courbure gauche-droite à 90° ; les sommets exportés d'une
  tile située en (256, 120, 0) sont à B(P) près ; le HUD exporté est inchangé par rapport à une
  courbure nulle ; avec Arc horizontal = 60° (bord) et les marges par défaut, la tile du coin
  P = (0, 8, 0) est rendue (B(P) = (0,32 ; −17,70 ; 0), hors de R, mais P est dans R).
- Critère d'acceptation [ROM] : Courbure gauche-droite à 90°, « Super Mario Bros. » : le sol, les blocs,
  le fond et leurs ombres forment un arc continu, sans fente entre les tiles ni ombre décollée ; le
  score reste plat et lisible (REQ-SCN-044).

## 4. Visée

**REQ-INP-124** (P2) [ROM] — **Zapper à la souris sur une scène courbée** (complète REQ-INP-063 et
REQ-INP-064) : quand une courbure est active, avec R le rayon de visée de REQ-INP-063 et z₀ le plan
`z = ZapperOffset.z` :
- les **candidats** sont les points P du plan z = z₀ tels que B(P) appartienne à R, du côté
  avant de la caméra ;
- parmi les candidats qui tombent dans la **fenêtre de visée** |x − 128| ≤ 144 · s et
  |y − 128| ≤ 136 · s (l'image NES de REQ-SCN-001, élargie de 16 unités de scène sur ses quatre
  bords), le point visé est celui dont l'image B(P) est la plus proche de l'origine du rayon ; ses
  coordonnées NES s'obtiennent ensuite comme en REQ-INP-063 ;
- s'il n'y a de candidat que hors de la fenêtre, le plus proche est transmis tel quel, sans
  bornage, comme un tir hors écran (REQ-INP-064) ;
- s'il n'y a aucun candidat, la dernière position valide est conservée (REQ-INP-064) ;
- la position retenue DOIT être à moins de 0,5 unité de scène du point défini ci-dessus.

B est inversible là où le bornage de profondeur de REQ-SCN-067 ne s'applique pas ; au-delà, la
visée n'est pas spécifiée. Sans courbure, la visée est celle de REQ-INP-063.
- Critère d'acceptation (auto) : pour chacun des vecteurs de REQ-SCN-067 sans bornage, un rayon
  qui passe par B(P) donne un point visé à moins de 0,5 unité de P (avec z₀ = z de P).
- Critère d'acceptation [ROM] : Courbure gauche-droite à 90°, « Duck Hunt » à la souris : viser le
  centre d'un canard situé à moins de 32 unités du bord droit de l'image le touche.

## 5. Questions

- Aucune. (La courbure du HUD, de la skybox et du boîtier TV est tranchée en REQ-SCN-068.)

## 6. Lignes de SPEC-09 et libellés

| Clé | Lua | N° | Type | Défaut | Bornes | Portée | Effet | UI | Libretro |
|---|---|---|---|---|---|---|---|---|---|
| `scene.bend_lr` | `GetVisualStyleParameter(1, "spl.bend_zx")` | — | entier (degrés) | 0 | −180 à 180, pas 30 | U | imm. | liste « Bend Left-Right » / « Courbure gauche-droite » | option, 13 valeurs |
| `scene.bend_tb` | `… "spl.bend_zy"` | — | entier (degrés) | 0 | idem | U | imm. | liste « Bend Top-Bottom » / « Courbure haut-bas » | option, 13 valeurs |
| `scene.arc_h` | `… "spl.arc_yx"` | — | entier (degrés) | 0 | idem | U | imm. | liste « Arc Horizontal » | option, 13 valeurs |
| `scene.arc_center` | `… "spl.arc_center"` | — | entier | 0 | 0 ou 1 | U | imm. | liste « Arc Center » | option, 2 valeurs |

Libellés et aides (AMD-035 ; anglais par défaut / français) :

| Élément | Anglais | Français |
|---|---|---|
| groupe | `Geometry` | `Géométrie` |
| Bend gauche-droite | `Bend Left-Right` — « Bends the scene horizontally. Positive values bulge it outwards like a CRT screen; negative values curve it around the player. » | `Courbure gauche-droite` — « Courbe la scène horizontalement. Une valeur positive la bombe comme un écran cathodique ; une valeur négative l'enroule autour du joueur. » |
| Bend haut-bas | `Bend Top-Bottom` — « Bends the scene vertically. Positive values bulge it outwards; negative values curve it around the player. » | `Courbure haut-bas` — « Courbe la scène verticalement. Une valeur positive la bombe ; une valeur négative l'enroule autour du joueur. » |
| Arc horizontal | `Arc Horizontal` — « Rolls the level onto an arc, as if it stood on a tiny planet. Works best with side-scrollers. » | `Arc horizontal` — « Pose le niveau sur un arc, comme sur une toute petite planète. Idéal pour les jeux à défilement horizontal. » |
| Centre de l'arc | `Arc Center` — `Screen Center` / `Screen Edge` | `Centre de l'arc` — `Centre de l'écran` / `Bord de l'écran` |
