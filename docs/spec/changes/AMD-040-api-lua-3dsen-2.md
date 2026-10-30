# AMD-040 — API Lua de 3dSen 2.0

Version : 1.2 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30). Complète SPEC-05 §4.1 et §4.2, et SPEC-09.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-039-045 : L-1, 40-2 à 40-11 ; REQ-LUA-114 réduite) ; 1.2 (seconde relecture R2-review-AMD-039-045-v1.1 : N-40-1 à N-40-3, T-2, T-3).

## 1. Constat

Parmi les quatre profils intégrés modifiés en 3dSen 2.0 (AMD-039 §5), **un seul**, `punchout`,
lit des membres absents de SPEC-05 : la version du programme, son titre et une nouvelle distance
de caméra, dans une fonction appelée au démarrage de la partie. Sans eux, le script s'arrête sur
une erreur (REQ-LUA-060 à 064) et le profil perd son cadrage. Les trois autres ne changent que
par des membres déjà spécifiés : `Final_Fantasy_USA` lit en plus `shape.Shape2D.Count`
(SPEC-05 §4.2, type `Shape`, alias de `InsCount`) ; `DQ3_DW3` et `Super Mario Bros` ne lisent
aucun membre nouveau.

Le corpus de référence de REQ-ACC-011 et le contrôle de couverture de REQ-LUA-080 portent sur les
profils de la 1.1.2 : ils **ne valident aucun** des membres de cet amendement, qui ont leurs
propres critères ci-dessous.

## 2. Exigences

**REQ-LUA-112** (P1) [R2] — **Table `CS`** : l'environnement Lua des profils DOIT contenir une globale
`CS`, table dont le champ `Nes3D` est une table, dont le champ `Info` est une table avec :

| Membre | Type | Accès | Règle | Valeur recommandée |
|---|---|---|---|---|
| `Version` | chaîne | L | commence par un entier décimal **≥ 2** | `"2.0"` |
| `Title` | chaîne | L | différente de `"3dSenMaker"` | `"Zaxnes"` |

- Les scripts de la 2.0 lisent l'entier initial de `Version` : s'il vaut au moins 2, ils
  prennent le chemin de la 2.0, qui lit `Camera.ResetDistance` (REQ-LUA-113). Sinon, ils
  comparent `Title` à `"3dSenMaker"` (le nom de l'éditeur de profils) pour choisir un FOV.
  Exemple illustratif, tiré d'un script de profil : `tonumber(CS.Nes3D.Info.Version:match("^%d+"))`.
- Tout autre champ de `CS`, `CS.Nes3D` ou `CS.Nes3D.Info` vaut nil, sans erreur.
- Une écriture dans ces tables n'a aucun effet sur le programme.
- Critère d'acceptation (auto) : `tonumber(CS.Nes3D.Info.Version:match("^%d+")) >= 2` → vrai ;
  `CS.Nes3D.Info.Title ~= "3dSenMaker"` → vrai ; `CS.Nes3D.Info.AppId` → nil ;
  `CS.Nes3D.Autre` → nil.
- Critère d'acceptation [R2] : le profil `punchout` de la 2.0 démarre sans erreur Lua et cadre la
  caméra comme dans 3dSen 2.0.

**REQ-LUA-113** (P1) — **`Camera.ResetDistance`** (L, nombre) : la distance cadrée d₀ que donnerait
une réinitialisation de vue au FOV courant (AMD-045, REQ-SCN-069), **avant** le bornage de
REQ-SCN-011. Elle se lit à tout moment, même sans réinitialisation, et suit les changements de
FOV.
- Critère d'acceptation (auto) : FOV 70° → 189,65 (±0,05) ; après `Camera.Distance = 50`,
  `Camera.ResetDistance` est inchangé ; FOV 45° → 320,60 (±0,05).

**REQ-LUA-114** (P2) — **Nouveaux membres de `Setting`** (SPEC-05 §4.2, `SettingAPI`) :

| Membre | Genre | Type | Accès | Effet |
|---|---|---|---|---|
| `RunAheadFrames` | prop | entier | LE | réglage Run-ahead d'AMD-041 (REQ-LOOP-064) ; écriture ramenée dans [0, 3] |
| `VisualStyle` | prop | entier | LE | 0 = Classic, 1 = Enhanced ; défaut 0 ; toute autre valeur écrite donne 0. Persisté (portée U, ligne §3), sans effet sur le rendu de Zaxnes (qui n'a qu'un rendu) |
| `GetVisualStyleParameter` | méth | nombre | — | `GetVisualStyleParameter(style, id)` : valeur du paramètre (§3), sa valeur par défaut s'il n'a jamais été écrit ; 0 si le style ou l'id est inconnu |
| `SetVisualStyleParameter` | méth | booléen | — | `SetVisualStyleParameter(style, id, v)` : enregistre `v` normalisé (§3) et renvoie vrai ; renvoie faux, sans rien enregistrer, si le style est inconnu ou si l'id n'existe pas pour ce style |
| `ResetVisualStyleParameters` | méth | nil | — | `ResetVisualStyleParameters(style)` : tous les paramètres du style reviennent à leur défaut ; sans effet si le style est inconnu |

- Les valeurs écrites par `SetVisualStyleParameter` sont les réglages de courbure d'AMD-043
  (REQ-SCN-066) : écrire par Lua ou par l'interface revient au même.
- Le défaut des quatre paramètres est 0 : un script ne distingue donc pas « inconnu » de « valeur
  nulle » en lisant ; c'est voulu (`SetVisualStyleParameter`, qui renvoie faux, permet de tester
  l'existence).
- Critère d'acceptation (auto) : `Setting.RunAheadFrames = 7` → 3 ;
  `Setting.VisualStyle = 5` → 0 ;
  `Setting:SetVisualStyleParameter(1, "spl.bend_zx", 500)` → vrai, puis
  `Setting:GetVisualStyleParameter(1, "spl.bend_zx")` → 180 ; une écriture de 40 donne 30, de 45
  donne 60, de 15 donne 0 ; `Setting:SetVisualStyleParameter(0, "spl.bend_zx", 30)` → faux ;
  `Setting:SetVisualStyleParameter(7, "spl.bend_zx", 30)` → faux ;
  `Setting:GetVisualStyleParameter(1, "inconnu")` → 0.

**REQ-LUA-115** (P3) — **Export des maillages d'une instance** : le type `Instance` (SPEC-05 §4.2)
DOIT offrir les méthodes `BuildExportMeshes()` et `ReleaseExportMeshes()`. L'export de la scène
est hors périmètre (PLAN §1) : elles DOIVENT exister, ne rien faire et ne pas lever d'erreur.
- Critère d'acceptation (auto) : `pattern:BuildExportMeshes(); pattern:ReleaseExportMeshes()` →
  aucune erreur, aucun changement visible.
- Notes : aucun profil intégré ne les appelle ; elles sont gardées parce qu'un script 2.0 qui les
  appellerait perdrait sinon tous ses hooks (REQ-LUA-062). Elles ne portent aucun paramètre ni
  aucune sémantique.

## 3. Paramètres du style 1 (pour REQ-LUA-114)

Le style 0 (Classic) n'a aucun paramètre. Le style 1 (Enhanced) a les quatre paramètres de
courbure d'AMD-043 :

| Id | Bornes | Pas | Défaut | Réglage (AMD-043, REQ-SCN-066) |
|---|---|---|---|---|
| `spl.bend_zx` | −180 à 180 | 30 | 0 | Bend gauche-droite |
| `spl.bend_zy` | −180 à 180 | 30 | 0 | Bend haut-bas |
| `spl.arc_yx` | −180 à 180 | 30 | 0 | Arc horizontal |
| `spl.arc_center` | 0 à 1 | 1 | 0 | Centre de l'arc (0 milieu, 1 bord) |

**Normalisation** d'une valeur v écrite : la ramener dans les bornes [a, b] ; puis la remplacer
par a + n × pas, où n est l'entier le plus proche de (v − a) / pas, **à égalité l'entier pair**
(par exemple 7,5 → 8, 6,5 → 6) ; puis la ramener de nouveau dans [a, b].

Les autres identifiants de paramètres de 3dSen 2.0 ne sont pas spécifiés : pour Zaxnes, ils sont
« inconnus » au sens de REQ-LUA-114.

**Lignes de SPEC-09.** §4.1 :

| Clé | Lua | N° | Type | Défaut | Bornes | Portée | Effet | UI | Libretro |
|---|---|---|---|---|---|---|---|---|---|
| `render.visual_style` | `VisualStyle` | — | entier | 0 | 0 ou 1 | U | imm. (mémorisé, sans effet de rendu) | aucune | non exposé |

Les quatre paramètres de courbure ont leurs lignes dans AMD-043 §6, `RunAheadFrames` dans AMD-041 §3.

§4.2 (membres de `Setting`) :

| Membre | Accès | Retour |
|---|---|---|
| `VisualStyle` | L, É | entier |
| `RunAheadFrames` | L, É | entier |
| `GetVisualStyleParameter(style, id)` | appel | nombre |
| `SetVisualStyleParameter(style, id, v)` | appel | booléen |
| `ResetVisualStyleParameters(style)` | appel | nil |

## 4. Questions

- Aucune.
