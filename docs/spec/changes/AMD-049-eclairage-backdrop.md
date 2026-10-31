# AMD-049 — Éclairage du backdrop

Version : 1.1 | 2026-10-31 | **Normatif** (R1, choix du propriétaire du 2026-10-31, à la suite de la comparaison R5 avec 3dSen 2.0). **Amende SPEC-06 REQ-SCN-048** (phrase « éclairé de façon diffuse (REQ-SCN-028) » et critère d'acceptation).
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-049 : 49-1 à 49-12).
Priorités : si REQ-SCN-075 (P2) n'est pas retenue, l'éclairage diffus d'origine de REQ-SCN-048 (P1) reste valable (T-1).

## 1. Besoin

REQ-SCN-048 éclaire le backdrop « de façon diffuse (REQ-SCN-028) », comme une tile. Le programme
d'origine l'éclaire avec le modèle de réflexion **Standard** de Unity (matériau non métallique,
presque mat), dès la version 1.x. Par rapport à un éclairage diffus, la part diffuse est réduite
de 4 % et un reflet spéculaire faible, de la couleur de la lumière, s'y ajoute ; un albédo nul
n'est donc plus rendu noir. L'écart est visible quand le backdrop occupe une grande surface : dans
« Punch-Out!! », en vue de dessus, la grande surface bleue qui tient le sol du ring est un peu plus
sombre et légèrement teintée par la lumière rosée du profil.

Le modèle Standard de Unity est publié avec les sources publiques des shaders intégrés de Unity ;
il est rappelé ici en entier pour que l'exigence se suffise à elle-même.

## 2. Exigence

**REQ-SCN-075** (P2) — **Éclairage du backdrop** (remplace, dans REQ-SCN-048, « éclairé de façon
diffuse (REQ-SCN-028, albédo = `Backdrop.Color`) ») : le backdrop DOIT être éclairé ainsi, en
espace linéaire, pour chaque point de normale unitaire N (normale de la face du pavé, transportée
par la courbure d'AMD-043 REQ-SCN-068 quand elle est active), avec :

- V le vecteur unitaire du point vers l'œil ;
- L = −`Light.Direction` normalisé (vers la lumière) ;
- H = (L + V) normalisé, et H = N si L + V = 0 ;
- c l'albédo (`Backdrop.Color` converti en linéaire) ;
- E = lumière_lin de REQ-SCN-028 (couleur de la lumière multipliée par l'intensité, convertie en
  linéaire) ;
- s le facteur d'ombre de REQ-SCN-031 (1 hors ombre, 0,2 dans l'ombre, si `RecvShadow` et les
  ombres sont actifs ; 1 sinon) ;
- Amb l'ambiante de REQ-SCN-027 (0,1812 en linéaire).

Calcul :

- produits : nl = max(N·L, 0), nv = |N·V|, nh = max(N·H, 0), lh = max(L·H, 0) ;
- constantes : rugosité perceptuelle ρ = 0,99, α = ρ² = 0,9801 ;
- terme diffus : F90 = 0,5 + 2 · lh² · ρ ;
  Fd = (1 + (F90 − 1)(1 − nl)⁵) · (1 + (F90 − 1)(1 − nv)⁵) ;
- terme spéculaire : Vis = 0,5 / (nl · (nv · (1 − α) + α) + nv · (nl · (1 − α) + α) + 10⁻⁵) ;
  D = α² / (π · (nh² · (α² − 1) + 1)²) ; Spec = Vis · D · π · nl ;
  F = 0,04 + 0,96 · (1 − lh)⁵ ;
- couleur linéaire : `c · 0,96 · (Amb + s · E · Fd · nl) + s · E · Spec · F`, puis encodage
  identique à REQ-SCN-028 (sRGB à l'affichage, saturé à 1, sans tone mapping) ; le backdrop est
  opaque.

Il n'y a pas de reflet de l'environnement (skybox) : dans l'ombre, la part spéculaire est réduite
dans le même rapport s que la part diffuse directe. La grille de voxels du style « Enhanced » de
3dSen 2.0 n'est pas reprise. Le reste de REQ-SCN-048 (pavé, ombres reçues, pas d'ombre projetée,
pas de découpe, propriétés Lua) est inchangé.
- Critère d'acceptation (auto, valeurs linéaires avant encodage, ±0,001) : N = (0, 0, −1), œil sur
  l'axe (V = (0, 0, −1)), `Light.Direction` = (0, 0, 1), lumière blanche d'intensité 1, hors
  ombre : nl = nv = nh = lh = 1 ; Fd = 1 ; Vis = 0,25 ; D = 0,3314 ; Spec = 0,2603 ; F = 0,04 ; pour
  un albédo linéaire c, la couleur vaut c · 0,96 · 1,1812 + 0,01041 par canal : avec c = 0,2,
  0,2372 ; dans l'ombre (s = 0,2) : c · 0,96 · (0,1812 + 0,2) + 0,2 · 0,01041 : avec c = 0,2,
  0,0753.
- Critère d'acceptation (auto, sRGB 8 bits ±2) : même configuration, albédo nul : 0,01041 en linéaire,
  soit sRGB (26, 26, 26), hors ombre ; 0,00208, soit sRGB (7, 7, 7), dans l'ombre.
- Critère d'acceptation [ROM] : « Punch-Out!! », scène de combat (le script du profil y met
  `Backdrop.Enable = true`, `Backdrop.Color = Palette.FrameColor(0)` = NES `0x0078FC`,
  `Light.Color = Color(1, 0.8, 0.8)`, `Setting.LocalCam = true`), lumière du profil
  `Light.Direction = (1, 1, 2)`, intensité 1, ombres actives, hors ombre : la **face avant** du pavé
  (N = (0, 0, −1)), mesurée en sRGB, vaut (26 ; 100 ; 209) ±3 pour toute plongée de caméra de 0° à
  45°.

## 3. Changements documentaires

1. SPEC-06 REQ-SCN-048 : « Il DOIT être éclairé de façon diffuse (REQ-SCN-028, albédo =
   `Backdrop.Color`) » devient « Il DOIT être éclairé selon REQ-SCN-075 (albédo =
   `Backdrop.Color`) ».
2. SPEC-06 REQ-SCN-048, critère d'acceptation : « un panneau noir » devient « un panneau presque
   noir (sRGB ≈ 21 hors ombre, ≈ 5 dans l'ombre, avec la lumière par défaut et l'intensité 1,
   REQ-SCN-075) », le reste inchangé.
3. SPEC-v1.2-INDEX : REQ-SCN-075 ajoutée à la liste de SPEC-06 (§3.13, backdrop).

## 4. Questions

- Aucune.
