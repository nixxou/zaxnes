# AMD-045 — Caméra de 3dSen 2.0 : distance cadrée, zoom minimal, angles, verticale, chargement

Version : 1.2 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30). Amende SPEC-06 **REQ-SCN-007 (étape 4)**, REQ-SCN-011, REQ-SCN-012 (formule et sa Note), REQ-SCN-015, REQ-SCN-016, REQ-SCN-019 (clause « sans repliement »), REQ-SCN-024 et **REQ-SCN-045** (distance du HUD).
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-039-045 : 45-1 à 45-11) ; 1.2 (seconde relecture R2-review-AMD-039-045-v1.1 : N-45-1 à N-45-3, T-1).
Priorités : Si une exigence P2 de cet amendement qui remplace une clause P1 n'est pas retenue, la clause d'origine reste valable (T-1).

## 1. Constat

3dSen 2.0 change cinq comportements de la caméra. La distance par défaut, et celle du HUD, suivent
une nouvelle formule, identique à l'ancienne à FOV 70° seulement. L'utilisateur ne peut plus coller
la caméra contre l'image. Les angles restent dans un tour. La verticale de la caméra suit son
orientation. Une caméra rechargée apparaît directement à sa place au lieu de glisser depuis
l'ancienne vue.

## 2. Exigences

**REQ-SCN-069** (P1) — **Distance cadrée** (remplace la formule de REQ-SCN-012 et la Note « la
formule d'origine est conservée telle quelle ») : la distance de réinitialisation DOIT valoir

d₀ = 132,7964 / tan(FOV / 2) × s

avec FOV en degrés (champ vertical) et s l'échelle de scène (1 sur desktop). 132,7964 est la
demi-hauteur à cadrer, en unités de scène : à la distance d₀, cette demi-hauteur occupe
exactement la moitié du champ vertical (constante mesurée sur 3dSen 2.0). La réinitialisation de
vue écrit d₀ dans la distance de consigne, **soumise à la borne [0, 512] de REQ-SCN-011** (d₀
dépasse 512 sous un FOV de 29,1°, atteignable par script). La valeur non bornée est lisible par
Lua en `Camera.ResetDistance` (AMD-040, REQ-LUA-113), recalculée à chaque changement de FOV, même
sans réinitialisation de vue.
- Critère d'acceptation (auto) : FOV 70° ⇒ 189,65 ; 45° ⇒ 320,60 ; 95° ⇒ 121,69 (±0,05). À 70°,
  d₀ est égal, à 10⁻³ près, à la valeur de l'ancienne formule de REQ-SCN-012
  (104 / atan(FOV_rad / 2), FOV_rad en radians) : 189,65291 contre 189,65285.
- Notes : l'écart avec l'ancienne formule est de 6,9 · 10⁻⁵ en absolu à 70° (3,6 · 10⁻⁷ en
  relatif), la valeur par défaut ; il grandit aux autres FOV (277,93 → 320,60 à 45°, 150,25 →
  121,69 à 95°). La constante qui reproduirait exactement l'ancienne valeur à 70° est 132,796352 :
  132,7964 en est l'arrondi mesuré.

**REQ-SCN-045** (amendée) — La distance du plan HUD devient d_HUD = 76,6133 / tan(FOV / 2) (FOV en
degrés), au lieu de 60 / atan(FOV_rad / 2) ; elle vaut toujours **109,42 à 70°**, le facteur
d'échelle 0,5 et le reste de REQ-SCN-045 sont inchangés (à 70°, la valeur, 109,415, est inchangée). Le HUD garde ainsi la même proportion de
l'image que la scène quel que soit le FOV.
- Critère d'acceptation (auto) : FOV 70° ⇒ 109,42 ; 45° ⇒ 184,96 ; 95° ⇒ 70,20 (±0,05).

**REQ-SCN-070** (P2) — **Zoom minimal** (complète REQ-SCN-015 et 016) : les commandes de zoom de
l'utilisateur (actions ZoomIn et ZoomOut, molette, pincement tactile) NE DOIVENT PAS amener la
distance de consigne sous 16 × s (constante mesurée sur 3dSen 2.0, soit deux tiles NES). Un zoom
avant depuis une distance supérieure s'arrête à 16 × s ; depuis une distance déjà inférieure ou
égale (posée par un profil, un script ou la réinitialisation de vue, par exemple la distance FPV
de 0 au lancement), il n'a aucun effet. Le zoom arrière n'est pas concerné (borne haute de
REQ-SCN-011 inchangée). L'écriture Lua de `Camera.Distance` garde ses bornes [0, 512 × s]
(REQ-LUA-088).
- Critère d'acceptation (auto) : maintenir ZoomIn 5 s depuis la vue par défaut ⇒ distance de
  consigne 16 ; `Camera.Distance = 5` puis ZoomIn ⇒ 5 ; puis ZoomOut 1 s ⇒ 75 (±2 %).

**REQ-SCN-071** (P2) — **Angles dans un tour** (remplace la seconde phrase de REQ-SCN-011, et la
parenthèse « sans repliement modulo 360° » de REQ-SCN-019) : avant chaque mise à jour de la caméra,
chaque angle de consigne (rx, ry, rz de **chaque** mode) DOIT être ramené dans l'intervalle
]−180°, 180°] par ajout ou retrait de multiples de 360°. L'angle affiché (lissé, REQ-SCN-019)
n'existe que pour le mode courant : il DOIT être décalé du même multiple que la consigne de ce
mode, pour qu'aucun saut ne soit visible. Ce décalage ne change pas la vitesse interne v du
lissage, qui reste continue. L'utilisateur peut toujours orbiter sans limite.
- Le critère d'acceptation de REQ-SCN-011 (« rx au-delà de 360° » après 13 s) est remplacé par
  celui-ci ; celui de REQ-SCN-014 (3 s → 90°) reste vrai.
- Critère d'acceptation (auto) : maintenir RotDown 13 s (390°) ⇒ `Camera.Rot.x` lu par Lua reste
  dans ]−180, 180] (ici ≈ 30) et l'orientation de la caméra varie continûment ;
  `Camera.Rot.y = 270` puis lecture à la frame suivante ⇒ −90.

**REQ-SCN-072** (P2) — **Verticale de la caméra** (remplace l'étape 4 de REQ-SCN-007 et sa
parenthèse) : en TPV, la caméra est orientée vers le point P* de REQ-SCN-008 avec, comme verticale,
**celle de la caméra elle-même**, q · (0, 1, 0) (REQ-SCN-007 étape 1), et non l'axe Y du monde.
Conséquence : rz produit désormais un **roulis visible en TPV** (`Camera.Rot.z`, REQ-SCN-023 et
REQ-LUA-088), comme en FPV. TPV et caméra locale (REQ-SCN-009) utilisent ainsi la même verticale ;
la seule différence restante est l'orientation vers P*.
- Critère d'acceptation (auto) : en TPV, rz = 0 : entre rx = 89,5° et rx = 90,5°, l'orientation de
  la caméra varie d'au plus 2° (contre environ 180° avec la verticale du monde) ; rx = ry = 0 et
  rz = 30° : l'image est tournée de 30° autour de l'axe de visée.

**REQ-SCN-073** (P2) — **Caméra rechargée sans glissement** (remplace « la caméra affichée les
rejoint par le lissage » de REQ-SCN-024) : après le chargement d'un savestate, d'une quick save ou
d'un snapshot de rollback (AMD-042), la caméra affichée DOIT prendre **immédiatement** les valeurs
de consigne rechargées (angles, distance, cible), avec des vitesses de lissage remises à zéro. Le
lissage reprend normalement à partir de la frame suivante. La fenêtre de transition de mode
(REQ-SCN-006, 020) est restaurée avec son **temps restant**, borné à [0, 3] s (AMD-042 amende en ce
sens REQ-SCN-024, qui parlait d'échéance).
- Critère d'acceptation (auto) : sauvegarder en vue par défaut, tourner de 90° et zoomer,
  recharger ⇒ la première image après le chargement montre déjà la vue sauvegardée. Sauvegarder
  1 s après un passage en FPV, recharger, puis déplacer la cible de 100 unités ⇒ pendant les 2 s
  qui suivent le chargement, la caméra affichée rejoint la cible par le lissage (écart restant
  ≈ 40 après Tₛ = 0,4 s, REQ-SCN-019) ; après ces 2 s, elle suit la cible dans la même frame
  (REQ-SCN-020).

## 3. Non repris

- En 3dSen 2.0, la rotation par glissement de la souris passe au **bouton droit** en mode profil
  (0,1° par pixel), le glissement gauche ne restant actif qu'en mode ROM. Ce changement n'est
  pas retenu : REQ-SCN-017 reste la référence, inchangée.
- Les réglages VR de la caméra restent hors périmètre.

## 4. Questions

- Aucune.
