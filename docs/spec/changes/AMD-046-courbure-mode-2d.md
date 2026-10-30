# AMD-046 — Courbure en mode 2D

Version : 1.1 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30). Complète AMD-043 (REQ-SCN-068), SPEC-06 REQ-SCN-055 lu avec AMD-009 Q-022 (dont il reprend l'image en pixels : la parenthèse « tiles converties » de REQ-SCN-055 ne s'applique plus), et SPEC-09 ; **amende AMD-043 REQ-INP-124** (exception du mode 2D plat) ; **clôt Q-079**.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-046 : 46-1 à 46-13).
Priorités : le comportement par défaut (le rectangle du mode 2D se courbe comme la géométrie de la scène de jeu, subdivisé en au moins 32 × 32 cellules, lecture 1 de Q-079) est P2 et acquis indépendamment du réglage ; si le réglage « Courber le mode 2D » n'est pas retenu, seule la possibilité de le désactiver (P3) disparaît.

## 1. Besoin

En mode 2D (REQ-SCN-055, Q-022), la scène ne contient qu'une surface plane portant l'image en
pixels du cœur, et la caméra peut toujours tourner autour. AMD-043 REQ-SCN-068 ne dit pas si
cette surface se courbe (Q-079).

Dans le programme d'origine, le mode 2D se courbe comme la scène 3D. Le propriétaire veut garder
ce comportement par défaut et pouvoir le désactiver : un écran plat vu en 3D est un usage
raisonnable du mode 2D. L'option est propre à Zaxnes (divergence acceptée, catégorie D du plan).

## 2. Exigence

Le **rectangle du mode 2D** est la surface plane qui porte l'image en pixels du cœur en mode 2D
(AMD-009 Q-022) : le rectangle z = 0, x ∈ [0, 256], y ∈ [8, 248] du repère de REQ-E3D-410,
texturé par l'image NES 256 × 240. Il ne projette pas d'ombre et n'en reçoit pas.

**REQ-SCN-074** (P2) — **Courbure du mode 2D** (réglage propre à Zaxnes, divergence acceptée **(D)**) :
- en mode 2D, le rectangle du mode 2D DOIT être subdivisé en au moins 32 × 32 cellules ;
- quand le réglage « Courber le mode 2D » est actif (défaut), la courbure de REQ-SCN-067 DOIT
  s'appliquer au rectangle comme à la géométrie de la scène de jeu (REQ-SCN-068), y compris la
  clause d'ordre de REQ-SCN-068 : la découpe par le rectangle visible (REQ-SCN-036) est évaluée sur
  les positions avant courbure, de sorte que l'ensemble des lignes et colonnes NES visibles ne
  dépend pas du réglage ;
- quand le réglage est inactif, la position de chaque sommet du rectangle DOIT être sa position
  plate (B = identité), quels que soient les réglages de REQ-SCN-066 ;
- si un backdrop est présent (REQ-SCN-048, piloté par Lua), il reste soumis à REQ-SCN-068, que le
  réglage soit actif ou non ;
- en mode 3D, le réglage n'a aucun effet ;
- le réglage s'applique à chaud, dès la frame rendue suivante ; il est persisté ;
- **REQ-INP-124 amendée** : en mode 2D, quand le réglage est inactif, REQ-INP-124 NE S'APPLIQUE
  PAS et la visée est celle de REQ-INP-063, comme à courbure nulle ; quand le réglage est actif,
  REQ-INP-124 s'applique sans changement. Le profil « 2D Mode » n'ayant pas de script de ports
  (REQ-INP-060, REQ-PROF-061), un port de type Zapper n'y est obtenu que par l'hôte ou le harnais.
- Critère d'acceptation (auto) : harnais, mode 2D, Courbure gauche-droite à 90°. Réglage actif :
  chaque sommet exporté du rectangle, de position plate P, est à moins de 0,05 unité de B(P) ; le
  rectangle compte au moins 1 089 sommets distincts (grille d'au moins 33 × 33). Réglage inactif :
  tous les sommets exportés sont dans le plan z = 0, à leur position plate à 10⁻³ unité près. Les
  lignes NES visibles sont les mêmes dans les deux cas, pour les marges par défaut. En mode 3D,
  basculer le réglage ne change aucun sommet exporté.
- Critère d'acceptation (auto) : harnais, mode 2D, port 2 forcé en Zapper, ZapperOffset nul,
  Courbure gauche-droite à 90°. Réglage inactif : un pointeur sur le point de scène (128, 120, 0)
  donne la position transmise (128, 128) ±1 (vecteur de SPEC-08 §5.3) ; réglage actif : le résultat
  est celui de REQ-INP-124 à 0,5 unité près.
- Critère d'acceptation (auto) : la valeur survit à un redémarrage ; elle vaut « actif » sur une
  installation neuve.
- Critère d'acceptation [ROM] (facultatif) : « 2D Mode » sur une ROM quelconque, Courbure
  gauche-droite à 90°, réglage actif : l'image est courbée d'un seul tenant, sans facette visible
  ni fente.
- Notes : cela ne crée pas de chemin de rendu séparé au sens de REQ-SCN-055 : seule la
  subdivision d'une surface change, le pipeline reste unique. Le réglage n'a pas d'accès Lua : le
  programme d'origine n'en a pas.

## 3. Ligne de SPEC-09 et libellés

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | Libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `scene.bend_2d_mode` | — | — | booléen | vrai | — | U | imm. | — dans l'original (réglage propre à Zaxnes) ; dans Zaxnes : case « Bend 2D Mode » / « Courber le mode 2D », groupe « Géométrie » (AMD-043 §6), après « Centre de l'arc » | on/off | courbe le rectangle du mode 2D (REQ-SCN-074) ; sans effet en mode 3D |

Table §4.6 de SPEC-09 (libretro), ligne ajoutée : `3dsen_scene_bend_2d_mode`, valeurs
`enabled, disabled`, défaut `enabled`, catégorie `Scène`.

Libellés et aides (AMD-035 ; anglais par défaut / français) :

| Élément | Anglais | Français |
|---|---|---|
| Courber le mode 2D | `Bend 2D Mode` — « Also applies the bend settings to the flat picture of 2D mode. Turn it off to keep the picture flat. » | `Courber le mode 2D` — « Applique aussi la courbure à l'image plate du mode 2D. Désactivez-le pour garder l'image plate. » |

## 4. Questions

- Aucune. Q-079 est close : lecture 1 par défaut (le rectangle se courbe, subdivision 32 × 32),
  lecture 2 sur option.
