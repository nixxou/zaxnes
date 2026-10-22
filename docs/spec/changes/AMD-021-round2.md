# AMD-021 — Amendements issus du round 2

Version : 1.0 | 2026-10-21 | **Normatif** (complète la spec v1.1 gelée ; remplace AMD-020 pour Q-048)
Rédigé par R1 à partir du jugement fonctionnel du round 2. Les identifiants `R2-NNN` servent uniquement à la traçabilité.

## 1. Exigences modifiées ou ajoutées (à implémenter)

### R2-001 — Mapper 28 (REQ-CORE-100, P2)
Une écriture en `$5000-$5FFF` choisit le registre cible par ses bits 7 et 0 : `$00` banque CHR (2 bits), `$01` banque PRG interne (4 bits), `$80` mode (6 bits), `$81` banque PRG externe (6 bits). Les écritures en `$8000-$FFFF` vont au registre choisi. Le registre externe désigne une banque de 32 Kio. On note `m = 2^(t+1) − 1` (en banques de 16 Kio), avec `t` = bits 4-5 du mode, `E = 2 × externe` et `I` = interne.
- Mode PRG (bits 2-3 du mode) 0 ou 1 : `$8000` montre la banque 16 Kio `(E & ~m) | (2I & m)`, et `$C000` la suivante.
- Mode 2 : `$8000` montre `E`, et `$C000` montre `(E & ~m) | (I & m)`.
- Mode 3 : `$8000` montre `(E & ~m) | (I & m)`, et `$C000` montre `E + 1`.
- Mirroring selon les bits 0-1 du mode (écran A, écran B, vertical, horizontal). Tant que le bit 1 du mode vaut 0, toute écriture dans le registre CHR ou PRG interne remplace le bit 0 du mode par le bit 4 de la valeur écrite.
- À la mise sous tension : externe = 63, interne = 15, mode = 0.
- Critère : ROM synthétique de 1 Mio dont chaque banque de 16 Kio porte son numéro. (ext = 5, mode = 0) → 10/11 ; (ext = 2, mode = `$18`, int = 3) → 4/7 ; (ext = 2, mode = `$1C`, int = 3) → 7/5 ; écriture CHR de `$10` avec mode = 0 → écran B.

### R2-002 — Mapper 15 (REQ-CORE-100, P2)
Toute écriture en `$8000-$FFFF` charge `B` = bits 0-5 de la valeur (banque de 16 Kio), le mirroring = bit 6 (0 vertical, 1 horizontal) et `h` = bit 7 ; le mode est donné par les bits 0-1 de l'adresse.
- Mode 0 : `$8000` = B, `$C000` = B + 1.
- Mode 1 : `$8000` = B, `$C000` = dernière banque de 16 Kio de la ROM.
- Mode 2 : la banque de 8 Kio `2B + h` dans les quatre fenêtres.
- Mode 3 : B dans `$8000` et dans `$C000`.
- À la mise sous tension, les 32 premiers Kio sont visibles.
- Critère : ROM synthétique de 1 Mio numérotée. `$8001 ← $05` → 5 / 63 ; `$8002 ← $85` → banque 8 Kio n° 11 partout ; `$8000 ← $40` → mirroring horizontal.

### R2-003 — Mapper 101 (REQ-CORE-100, P3)
Une écriture à une adresse dont les bits 13 et 14 valent 1 (`$6000-$7FFF` et `$E000-$FFFF`) choisit la banque CHR de 8 Kio `valeur & 3`.

### R2-005 — Instant de capture d'une rangée de fond (REQ-CORE-037, P1 ; remplace la réponse à Q-004 sur ce point)
La rangée capturée à la ligne L DOIT refléter l'état du PPU **après** toutes les écritures du CPU effectuées pendant la ligne L, à l'exception de la recopie horizontale du registre temporaire et de l'incrément vertical de fin de ligne, qui ne s'appliquent qu'à la ligne suivante. La condition « scroll Y fin = 1 » s'évalue sur cet état ; les banques CHR et les nametables sont lues dans ce même état.
- Critère 1 : une ROM de test qui écrit `$2006` pendant la ligne 100, avec une adresse de Y fin 1, produit une rangée avec `y = 99`. Si Y fin 2 est écrit pendant une ligne de Y fin 1, aucune rangée n'est émise pour cette ligne.
- Critère 2 [ROM] : Super Mario Bros. 3 (écran titre et démo) n'émet aucune rangée de fond à la ligne où le split écrit `$2006` ; la première rangée du bandeau est émise à la ligne suivante. Sur 5 000 frames, le nombre de rangées par frame ne dépasse pas celui d'une image sans split.

### R2-006 — Détection de lumière du Zapper (REQ-CORE-062, complément, P3)
Pendant le vblank et la ligne de pré-rendu, la couleur considérée est l'arrière-plan universel. Un pixel d'abscisse ≥ 239 est toujours vu avec la couleur d'arrière-plan universel (rend normatif le point 3 de la réponse à Q-005).

### R2-008 — Ordre des candidats après retrait (REQ-E3D-091, note, P3)
Quand un pattern est retiré du registre puis qu'un autre pattern contenant la même image est inscrit, le nouveau prend, dans l'ordre des candidats de cette image, **la place du dernier pattern retiré** de cette image ; sinon, il se place en dernier.

### R2-009 — Mélange des transparences en espace linéaire (REQ-SCN-040, ajout, P1)
Le mélange des objets transparents DOIT être effectué sur les couleurs **linéaires** : couleur affichée = sRGB(α × couleur linéaire de l'objet + (1 − α) × couleur linéaire déjà présente). Il en va de même pour le HUD transparent et pour l'occultation en vue FPV.
- Critère : un motif d'opacité 0,5 de couleur NES `F4C0E0`, face frontale éclairée (N·L = 0,577, lumière blanche d'intensité 1, sans ombre), donne sRGB (158, 124, 145) ±2 devant un fond noir, et G = 147 ±2 devant un fond sRGB (0, 120, 131).

### R2-010 — Pas d'ombre sur les transparents (REQ-SCN-032, P1 ; le « DEVRAIT » devient « NE DOIT PAS »)
Une tile rendue en transparence (REQ-SCN-040, y compris l'occultation de REQ-SCN-022) et une tile du HUD NE DOIVENT recevoir aucune ombre, quels que soient les drapeaux de son motif et de sa couche.
- Critère : dans la scène synthétique d'ombre de REQ-SCN-030, une tile réceptrice d'alpha 0,5 a la couleur de la même tile non ombrée.

### R2-011 — Seuil numérique des commandes analogiques (REQ-INP-030/031, P2 ; **remplace AMD-020 pour Q-048**)
Le seuil numérique de 0,34 s'applique à la valeur **mise en forme** de REQ-INP-031 : zone morte radiale 0,2 et saturation 0,9 pour un stick, zone morte 0,2 et saturation 0,9 pour une gâchette, avec ré-étalement linéaire entre les deux.
- Critère : stick poussé droit vers le haut à 0,40 de course → pas d'appui ; à 0,45 → appui ; stick à 45° avec une course de 0,6 → Haut et Droite pressés (composantes mises en forme 0,404).
- Q-056 est sans objet (la mise en forme redevient observable).

### R2-013 — Sons des réglages (REQ-AUD-041, note, P3 ; clôt DEC-11-Q5)
Les réglages Vitesse et FOV ne déclenchent aucun son. Tous les autres réglages de la fenêtre d'options déclenchent le son de validation à chaque changement de valeur.

### R2-014 — Matériaux des modèles TV (REQ-SCN-054, compléments, P2 ; précise Q-023)
- Les modèles TV 1 et TV 3 sont **opaques** : le canal alpha de leurs textures n'a aucun effet.
- Seul le premier sous-maillage de chaque modèle est dessiné ; l'écran de TV 3 ne l'est pas.
- TV 3 : albédo = texture × 0,208, plus une émission égale à la carte d'émission.
- Les touches de TV 3 projettent et reçoivent les ombres ; les corps des téléviseurs ne font ni l'un ni l'autre.
- L'aspect spéculaire (lissé 0,5 pour TV 1, carte de brillance pour TV 2 et TV 3) est **informatif** : il relève de la revue visuelle, pas d'un critère mesuré.

## 2. Divergences acceptées (D), documentées, sans changement d'implémentation
| Réf. | Exigence | Comportement retenu pour la reconstruction |
|---|---|---|
| R2-004 | REQ-CORE-100 (mapper 27, VRC2/VRC4) | Comportement nesdev pour le mapper 27 ; différences de largeur CHR (≥ 256 Kio) et de mirroring VRC2 sans effet sur les cartouches commerciales. |
| R2-006 (autres points) | REQ-CORE-062 | Pas de cache par bande de 8 lignes ; priorité sprite/fond selon la composition NES. |
| R2-007 | REQ-CORE-022 | Au-delà de la 8ᵉ lecture d'un port manette, la valeur renvoyée reste celle de la reconstruction ; aucun profil embarqué n'en dépend. |
| R2-012 | REQ-SCN-040 | À travers un objet transparent, les autres objets transparents situés derrière restent visibles (tri de l'arrière vers l'avant). |
| R2-015 | REQ-SCN-028 | Linéariser séparément la couleur et l'intensité de la lumière est accepté (écart d'au plus 1 niveau). |

## 3. Confirmations du round 2 (aucun changement)
DEC-02-Q1 (polarité Zapper), DEC-04B-Q2 (déformation dans le repère de l'objet), Q-014 (cas de contrôle des cylindres), Q-016 (orientation MARCHING), DEC-06-Q2, Q-019, Q-024 (lecture A), Q-025, Q-032, Q-020 révisé, DEC-07-Q1, DEC-08-7 (QuickLoad en pause relance le jeu), DEC-11-Q1, Q-057 (`<id>`), mappers 225 et 228. (R2-015 n'est pas une confirmation : c'est une divergence D, voir §2.)

## 4. Corrections après le filtre R2 (R2-review-AMD-021, 2026-10-21) — normatives

### 4.1 Remplacements déclarés
| Élément remplacé | Remplacé par |
|---|---|
| Réponse à Q-004, point 3, et DEC-02-Q4 (tolérance d'une ligne sur l'instant de capture) | R2-005 : la capture reflète l'état de fin de ligne ; la tolérance d'une ligne ne s'applique plus à l'instant de capture. |
| Réponse à Q-005, point 1 (« les enregistrements restent lisibles pendant le vblank ») et note de REQ-CORE-062 | R2-006 : pendant le vblank et la ligne de pré-rendu, la détection de lumière ne voit que la couleur d'arrière-plan universel. Les enregistrements de la frame dessinée ne sont plus consultés pour la détection. |
| Réponse à Q-005, point 3 (« jamais de lumière pour x ≥ 239 », optionnel) | R2-006 : un pixel d'abscisse ≥ 239 est vu avec la couleur d'arrière-plan universel (donc lumineux si cette couleur est claire), **exigé**. |
| REQ-E3D-091 et DEC-04-Q02 (ordre d'inscription pur) | R2-008, avec la priorité de REQ-E3D-091 (et non P3). |
| REQ-INP-031 : zone morte de gâchette 0,1 | R2-011 : zone morte de gâchette 0,2. |
| AMD-020 (Q-048, course brute) | R2-011 (valeur mise en forme). |
| Notes et tableau de REQ-AUD-041 (sons des curseurs) | R2-013. |
| REQ-SCN-054 : « les modèles TV ne reçoivent pas d'ombre » et albédo TV 3 « 0,21 » | R2-014 : les touches de TV 3 projettent et reçoivent les ombres ; albédo TV 3 = 0,208. |

### 4.2 Critères d'acceptation ajoutés
- **R2-003** : ROM synthétique (mapper 101) dont chaque banque CHR de 8 Kio porte son numéro. `$6000 ← $06` → banque CHR 2 ; `$E000 ← $03` → banque 3 ; `$8000 ← $01` → aucun changement.
- **R2-006** : (a) pointeur en (100, 100) sur un sprite blanc, lecture pendant le vblank → pas de lumière si l'arrière-plan universel est sombre, lumière s'il est clair (`$30`) ; (b) pointeur en (240, 100) en plein balayage → lumière si et seulement si l'arrière-plan universel est clair, quel que soit le contenu affiché.
- **R2-008** : registre avec les patterns P1 puis P2 contenant l'image X ; retrait de P1 puis inscription de P3 contenant X → ordre des candidats de X : P3, P2. Sans retrait préalable, P3 se place après P2.
- **R2-013** : dans la fenêtre d'options, changer la valeur de Vitesse ou de FOV ne joue aucun son ; changer une valeur de chacun des autres réglages joue exactement une fois le son de validation (modèle audio observé par test).
- **R2-014** : capture hors écran de chaque modèle TV sur fond uni : TV 1 et TV 3 sans pixel transparent à travers le modèle ; écran de TV 3 absent ; une source de lumière rasante produit l'ombre des touches de TV 3 sur le corps, et aucune ombre portée par le corps.
