# AMD-047 — Marges de clipping négatives

Version : 1.1 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30, à la suite de la comparaison R5 avec 3dSen 2.0). **Amende SPEC-09 §4.1.5** (colonne « Domaine » des quatre marges et dernière phrase du paragraphe qui suit le tableau) ; précise SPEC-06 REQ-SCN-003, REQ-SCN-007, REQ-SCN-008 et REQ-SCN-036, et SPEC-04 REQ-E3D-002, pour des marges négatives.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-047 : 47-1 à 47-12) ; 1.2 (Q-080 : bornes de la zone émise par le cœur et troisième critère corrigés).

## 1. Besoin

SPEC-09 §4.1.5 borne les quatre marges de clipping à [0 ; 256] (gauche, droite) et [0 ; 248]
(bas, haut), et REQ-PERS-003 ramène toute valeur hors domaine dans ces bornes, y compris quand
elle vient d'un script. Le programme d'origine ne borne pas ces marges : une marge négative est
conservée telle quelle.

Des profils s'en servent. Le profil de « Punch-Out!! » (versions 1.x et 2.0), en vue de dessus du
ring, écrit −128 dans les marges gauche et droite et −80 dans la marge basse : la cible effective
et le point visé de la caméra descendent vers l'avant du ring, et la découpe cesse de rogner les
bordures. Ni le champ de vision ni la distance ne changent. Avec les marges bornées à 0, la cible
est 40 unités trop haut et l'avant du ring, avec le personnage du joueur, sort du champ.

## 2. Exigence

**REQ-PERS-087** (P1) — **Domaine des marges de clipping** :
- les quatre marges (`clip.left`, `clip.bottom`, `clip.right`, `clip.top` ; `Setting.Clipping[0]`
  à `[3]`, `x`, `y`, `z`, `w`, `target`) acceptent **tout nombre réel fini**, négatif compris,
  qu'il vienne du profil (SPEC-03 §4.6, champ 4, appliqué par REQ-PROF-041) ou d'un script ; la
  valeur est conservée telle quelle et relue à l'identique. Une valeur non finie (NaN, ±∞) est
  hors domaine et remplacée par la valeur par défaut de la marge, par la règle générale de
  REQ-PERS-003 ;
- les formules qui utilisent les marges s'appliquent sans changement à des valeurs négatives :
  - le rectangle visible R = [c0, 256 − c2] × [c1, 240 − c3] (SPEC-06 §2) peut donc déborder de
    l'image NES et du cadre de référence ;
  - la conservation des tiles (SPEC-09 §4.1.5, SPEC-04 REQ-E3D-002) et la découpe des fragments
    (REQ-SCN-036) gardent ce qui tombe dans R. Au-delà de la zone capturée par le cœur (SPEC-04 §2 :
    fond : x écran ∈ [−7, 255], y écran ∈ [−1, 238] (SPEC-02 REQ-CORE-037) ; sprites x ∈ [0, 255], y < 240)
    il n'y a pas de tile à garder : une marge négative ne découvre que les huit unités de bordure
    partiellement défilées de chaque côté ;
  - la cible effective T′ (REQ-SCN-007 étape 2) et le point visé P* (REQ-SCN-008) suivent leurs
    formules avec les marges négatives ;
- si R est vide (c0 + c2 ≥ 256 ou c1 + c3 ≥ 240), l'image de la scène de jeu PEUT être vide ;
  l'implémentation NE DOIT PAS lever d'erreur ni cesser de rendre les autres éléments (HUD, fond,
  skybox).
- Critère d'acceptation (auto) : un script écrit `Setting.Clipping[1] = -80` puis lit
  `Setting.Clipping[1]` → −80 ; écrit `Clipping.x = -128` → `Setting.Clipping[0]` vaut −128. Un
  profil dont le champ 4 du message de réglages (SPEC-03 §4.6, `Clipping`) vaut
  (−128 ; −80 ; −128 ; 0) donne, après chargement (REQ-PROF-041), `Setting.Clipping[0]` = −128 et
  `Setting.Clipping[1]` = −80. Une écriture de NaN dans `Setting.Clipping[0]` donne la valeur par
  défaut de la marge.
- Critère d'acceptation (auto) : caméra TPV, lissage stabilisé, consignes rx = −60°, ry = 0, rz = 0,
  distance de consigne d = 265,51 (valeur libre, dans les bornes de REQ-SCN-011), cible par défaut
  C = (128, 120, 0), marges (−128 ; −80 ; −128 ; 0) : T′ = (128 ; 80 ; 0),
  E ≈ (128,0 ; −149,9 ; −132,8) ±0,5, P* ≈ (128 ; 41,5) ±0,6 (REQ-SCN-008) et
  R = [−128, 384] × [−80, 240].
- Critère d'acceptation (auto) : une tile de fond émise en (x, y) = (−7, 100) (défilement fin de 7,
  SPEC-02 REQ-CORE-037) occupe la scène x ∈ [−7, 1] : avec les marges (0,1 ; 8,1 ; 8,1 ; 0,1),
  ses fragments de x < 0,1 sont éliminés par REQ-SCN-036 ; avec (−16 ; 8,1 ; 8,1 ; 0,1)
  (R = [−16 ; 247,9] × …), aucun ne l'est. Dans les deux cas la tile est conservée par
  REQ-E3D-002.
- Critère d'acceptation [ROM] : « Punch-Out!! », vue de dessus du ring : le ring entier, cordes
  avant et personnage du joueur compris, est dans le champ.
- Notes : le boîtier TV et son tunnel (REQ-SCN-053, REQ-SCN-054) ont été retirés par AMD-031 ;
  les marges n'ont plus d'effet décoratif.

## 3. Changements de SPEC-09 §4.1.5

Colonne « Domaine » :

| Clé | Domaine (était) | Domaine (devient) |
|---|---|---|
| `clip.left` | [0 ; 256] | réel fini ; non fini (NaN, ±∞) → valeur par défaut (REQ-PERS-087) |
| `clip.bottom` | [0 ; 248] | idem |
| `clip.right` | [0 ; 256] | idem |
| `clip.top` | [0 ; 248] | idem |

Dernière phrase du paragraphe qui suit le tableau, remplacée par : « Les quatre marges acceptent
tout réel fini, négatif compris (REQ-PERS-087) ; [0 ; 256] et [0 ; 248] ne sont plus des bornes
mais les dimensions de référence de l'image et de la grille de conversion. »

Les autres colonnes sont inchangées (pas d'UI, pas de libretro : la discrétisation de
REQ-PERS-081 ne les concerne pas).

SPEC-06 REQ-SCN-003, liste des valeurs typiques complétée : « …, 24,1, et des valeurs négatives
écrites par script (−128, −80), qui élargissent le rectangle visible (REQ-PERS-087). »

## 4. Questions

- Aucune.
