# AMD-048 — Section pleine enveloppée : intervalle de profondeur vide

Version : 1.1 | 2026-10-31 | **Normatif** (R1, à la suite de la comparaison R5 avec 3dSen 2.0 et de Q-082). **Amende SPEC-04B REQ-E3D-450** et **remplace la réponse à Q-015 d'AMD-008** ; ajoute TV-Y9 et TV-Y10 à SPEC-04B §5.3 ; clôt Q-082.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-048 : 48-1 à 48-10).

## 1. Besoin

AMD-008 a répondu à Q-015 que, pour une section pleine enveloppée (`solid = vrai`,
`fullWarp = vrai`), `z` couvre `[min(z⁻, z⁺), max(z⁻, z⁺)]`. Le programme d'origine (1.x et 2.0,
comportement identique) **n'écrit aucun voxel** pour un point de contour dont `z⁻ > z⁺`.

Les points concernés sont ceux de la région 2 (REQ-E3D-446) dont les bornes sont inversées :
`q = 0` si `D` est impair, `q = 0` et `q = 1` si `D` est pair (la région 1 n'a jamais
`z⁻ > z⁺`). Leurs colonnes ne sont remplies par un autre point que si la région 2 retient
`(A, 2)` (`2A ≤ B²`) ou si la région 1 atteint `u⁺ = f` ; sinon la section perd ses colonnes
extrêmes. Deux cas l'illustrent, tous deux constatés sur le programme d'origine :

- les cordes du ring de « Punch-Out!! » (`HCYLINDER`, `D = 3`, lignes de 2 pixels) : une seule
  tranche de profondeur au lieu de trois, donc des cordes trois fois moins épaisses que sous la
  lecture d'AMD-008 (TV-Y9) ;
- une rangée pleine de 8 pixels à `D = 4` : 20 voxels, sans les colonnes 0 et 7 (TV-Y10).

La lecture alternative de Q-082 §4.2 (région 2 sans son point `q = 0`) est écartée : elle viderait
la région 2 pour une rangée pleine à `D = 2`, alors que Q-014, confirmé au round 2 (AMD-021 §3),
y relève les texels inversés des colonnes extrêmes, qui ne viennent que de ce point.

## 2. Exigence

**REQ-E3D-450 amendée** (P1) — **Section pleine enveloppée** (`solid = vrai`, `fullWarp = vrai`).
Pour chaque point du contour et pour chaque entier `z` tel que `z⁻ ≤ z ≤ z⁺` (l'ensemble est
**vide** si `z⁻ > z⁺`), le moteur DOIT écrire sur tous les axes : si `Q0` et `Q1` sont actifs,
`(u⁺, z)` avec le texel de `Q2` ; si `Q2` et `Q3` sont actifs, `(u⁻, z)` avec le texel de `Q0`.
Aucun test « texel non nul » n'est appliqué (seule la règle de transparence REQ-E3D-423 vaut). La
règle vaut pour tous les types (pleins et demi, `antiShadow` compris). Le reste de REQ-E3D-450 est
inchangé.
- Critère d'acceptation : TV-Y4 inchangé, TV-Y9, TV-Y10 (§3).
- Critère d'acceptation [ROM] : « Punch-Out!! », vue de dessus du ring : les cordes ont une
  épaisseur d'une unité de scène.
- Notes :
  - TV-Y4 reste exact parce que, dans ses deux cas, les points de région 2 à bornes inversées sont
    suivis d'un point de même `u` dont l'intervalle n'est pas vide (`(3, 2)` pour `D = 4`,
    `(7, 2)` et `(7, 3)` pour `D = 8`) ; l'argument contraire de Q-015 et de Q-082 §4.1 omettait
    ce point.
  - REQ-E3D-448 (contour seul) et REQ-E3D-449 (section pleine non enveloppée) sont inchangés : le
    point de région 2 à bornes inversées y pose ses voxels comme avant ; le même segment de
    2 pixels à `D = 3` y occupe toujours les trois tranches. L'asymétrie est celle du programme
    d'origine.
  - `D′ = B + 1` (REQ-E3D-452) se déduit de la spec ; le maillage, centré sur `D`
    (REQ-E3D-504), n'en dépend pas.

## 3. Changements documentaires

1. SPEC-04B REQ-E3D-450 : texte remplacé par celui du §2.
2. AMD-008, ligne Q-015 : réponse remplacée par « Intervalle `{z : z⁻ ≤ z ≤ z⁺}`, vide si
   `z⁻ > z⁺` (AMD-048) ».
3. SPEC-04B §5.3, vecteurs ajoutés (convention de §5.3 : types H en `(j, k)`, `j = 7 − y`,
   REQ-E3D-421) :
   - **TV-Y9** — Colonne d'un `HCYLINDER`, `D = 3`, `solid = fullWarp = vrai`, segment aux rangées
     d'image `y = r` et `y = r + 1` (`L = 1`), texels `t_r` et `t_{r+1}` → exactement deux voxels,
     `(7 − r, 1)` avec le texel `t_r` et `(6 − r, 1)` avec le texel `t_{r+1}` ; `D′ = 2`. Une tuile
     de 8 colonnes identiques donne un maillage de 8 × 2 × 1 unités : boîte locale
     `x ∈ [−4, 4]`, `y ∈ [2 − r, 4 − r]`, `z ∈ [−0,5 ; 0,5]` (centrage sur `D = 3`).
   - **TV-Y10** — Rangée `11111111`, `VCYLINDER`, `D = 4`, `solid = fullWarp = vrai` → 20 voxels :
     `i = 2…5`, `k ∈ [0, 3]` ; `i = 1` et `i = 6`, `k ∈ {1, 2}` ; aucun voxel en `i = 0` ni en
     `i = 7` ; chaque voxel `(i, k)` porte le texel `i` ; `D′ = 4` ; boîte locale `x ∈ [−3, 3]`,
     `z ∈ [−2, 2]`.
4. Q-082 : clos par AMD-048 (questions 1 et 2 : règle ci-dessus ; question 3, `D′` : déduit de
   REQ-E3D-452, sans effet sur le maillage).

## 4. Questions

- Aucune.
