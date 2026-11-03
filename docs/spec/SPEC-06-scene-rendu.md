# SPEC-06 — Scène et rendu
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-00 (glossaire), SPEC-03 (profil : réglages de scène embarqués, attributs de motifs), SPEC-04 / SPEC-04B (meshes, atlas, palettes, couches Z, tracker), SPEC-05 (API Lua), SPEC-07 (boucle, savestates), SPEC-08 (actions d'entrée), SPEC-09 (persistance des réglages), SPEC-10 (UI), SPEC-12 (performances)

## 1. Objet et périmètre

Ce document décrit **ce que l'utilisateur voit** une fois les meshes produits par le moteur 2D→3D : la caméra et ses contrôles, la lumière et les ombres, l'ombrage des tiles converties, le HUD, la couleur de fond, le *backdrop*, les skyboxes, le boîtier TV décoratif, le « mode 2D », et les attentes sur le pipeline OpenGL de la réimplémentation. Il se termine par des vues de référence comparables au programme d'origine lancé en boîte noire.

Hors périmètre : VR (casques, échelle de scène VR, miroir, suivi de tête), éditeur de profils (sélection de tiles, grisage de sélection, boîte de sélection), enregistrement vidéo et export de modèles, méthode de rendu « texture intermédiaire » propre à macOS et Linux (voir REQ-SCN-063). La génération des meshes, l'atlas et le contenu des tables de palette relèvent de SPEC-04/04B. La **persistance** des réglages cités ici relève de SPEC-09 : ce document en fixe le comportement, les valeurs par défaut et les bornes. Les **noms** de l'API Lua relèvent de SPEC-05 : ce document en fixe l'effet visuel.

## 2. Concepts et vocabulaire

- **Repère scène** : repère cartésien **main gauche**. X vers la droite, Y vers le haut, Z vers la profondeur (en s'éloignant de la caméra par défaut). L'unité est le **pixel NES**. La définition complète et unique du repère est celle de **SPEC-04B REQ-E3D-410** ; ce document en reprend les termes :
- **Image NES** : l'image 256 × 240 « à plat » dans le plan z = 0 occupe x ∈ [0, 256], **y ∈ [8, 248]** (le pixel (px, py) occupe [px, px + 1] × [247 − py, 248 − py]) ; son centre géométrique est (128, 128, 0).
- **Cadre de référence** : rectangle z = 0, x ∈ [0, 256], **y ∈ [0, 240]**, de centre **C = (128, 120, 0)**, décalé de 8 unités vers le bas par rapport à l'image NES. Le rectangle visible, la cible par défaut de la caméra, le HUD, le backdrop et le boîtier TV sont exprimés dans ce cadre (valeurs de l'original).
- **Rectangle visible R** : partie de la scène conservée à l'écran, définie par quatre marges de **clipping** (c0 gauche, c1 bas, c2 droite, c3 haut, DEC-R2-T1), en unités scène : R = [c0, 256 − c2] × [c1, 240 − c3]. En lignes NES, R montre les lignes py ∈ [8 + c3, 248 − c1] (REQ-E3D-410).
- **Couches Z** : UNIDENTIFIED (indice 0), UI (1), L1 (2), L2 (3), L3 (4), L4 (5). Les indices sont ceux qu'utilisent les scripts Lua (SPEC-05). Leur effet géométrique (décalages en profondeur) est défini par SPEC-04.
- **Motif / shape** : groupe de tiles converti en objet 3D (SPEC-04). Les attributs de rendu d'un motif cités ici (alpha, style de rendu, projection et réception d'ombre, palette forcée, drapeau HUD, déformation animée) sont stockés dans le profil (SPEC-03) et modifiables par Lua (SPEC-05).
- **HUD** : motifs marqués « UI » (score, vies…), attachés à la caméra (REQ-SCN-045).
- **Réglage utilisateur** : valeur choisie dans l'UI et conservée d'une partie à l'autre (SPEC-09). **Réglage de profil** : valeur chargée depuis le bloc de réglages embarqué dans le profil au démarrage d'une partie (SPEC-03), puis éventuellement modifiée par Lua.
- **Jeu en cours** : état « en marche » de la boucle d'émulation (ni arrêté, ni en pause, ni menu ouvert), défini par SPEC-01/07.
- **Lissage critique** : filtre de suivi défini en REQ-SCN-019.
- **sRGB / linéaire** : les couleurs NES et les couleurs de réglage sont données en sRGB (0–1 ou 0–255). `lin(c)` est la conversion sRGB → linéaire standard (IEC 61966-2-1), `srgb(l)` l'inverse.

## 3. Exigences

### 3.1 Repère, échelle, rectangle visible

**REQ-SCN-001** (P1) — Le rendu DOIT utiliser le repère scène de SPEC-04B REQ-E3D-410, rappelé en §2. Le pixel NES de colonne px (0 à gauche) et de ligne py (0 en haut) occupe dans le plan z = 0 la zone [px, px + 1] × [247 − py, 248 − py].
- Critère d'acceptation : une tile de fond plate posée en NES (x = 0, y = 8) couvre le carré [0, 8] × [232, 240] ; caméra par défaut et marges (0 ; 7,9 ; 0 ; 0,1), elle apparaît en haut à gauche du rectangle visible (seule sa bande supérieure de 0,1 unité est découpée), alors qu'une tile posée en (0, 0) (carré [0, 8] × [240, 248]) n'est pas visible.
- Notes : OpenGL étant main droite, une implémentation peut convertir (x, y, z)scène → (x, y, −z)GL, ce qui inverse l'ordre des sommets des faces. Les fichiers OBJ de `assets-neutral/meshes/` sont eux exprimés avec l'axe X inversé par rapport au repère d'origine des modèles (REQ-SCN-054).

**REQ-SCN-002** (P1) — Sur desktop, l'échelle de scène DOIT valoir 1 en permanence : toutes les distances de ce document sont en unités scène non mises à l'échelle. La propriété Lua `Video.SceneScale` DOIT renvoyer 1 en lecture. Une écriture DOIT être acceptée sans effet géométrique ; si la valeur écrite diffère de 1 et de la dernière valeur écrite, elle DOIT toutefois déclencher une réinitialisation de vue (REQ-SCN-012), comme un changement de FOV.
- Critère d'acceptation : un script qui écrit `Video.SceneScale = 0.02` puis relit la valeur obtient 1 ; la caméra revient à sa vue par défaut à l'instant de l'écriture ; la taille des objets à l'écran est inchangée.
- Notes : deux profils (variantes « flappy ») écrivent 0,02 au démarrage, une valeur destinée à la VR.

**REQ-SCN-003** (P1) — Les quatre marges de clipping DOIVENT provenir du réglage de profil (SPEC-03) au chargement de la partie. Elles DOIVENT être modifiables pendant la partie par Lua (`Setting.Clipping[i]` et l'alias global `Clipping`, i = 0 gauche, 1 bas, 2 droite, 3 haut), avec un effet dès la frame rendue suivante sur la découpe (REQ-SCN-039), le cadrage de la caméra (REQ-SCN-007, -008) et le boîtier TV (REQ-SCN-053). Valeurs typiques des profils : 0,1, 7,9, 8,1, 16,1, 24,1, et des valeurs négatives écrites par script (−128, −80), qui élargissent le rectangle visible (REQ-PERS-087). Valeurs par défaut en l'absence de profil : (0,1 ; 8,1 ; 8,1 ; 0,1).
- Critère d'acceptation : avec un profil dont le clipping vaut (0 ; 7,9 ; 0 ; 0,1), R = [0, 256] × [7,9 ; 239,9] : les 8 premières lignes de l'image NES (py < 8, y > 240) ne sont jamais visibles, la dernière ligne (py = 239, y ∈ [8, 9]) l'est entièrement. Avec (0,1 ; 8,1 ; 8,1 ; 0,1), les colonnes px ≥ 247,9 sont en outre masquées.

### 3.2 Projection

**REQ-SCN-004** (P1) — La caméra DOIT utiliser une projection perspective dont le champ de vision **vertical** vaut le réglage utilisateur FOV, avec un plan proche à 0,3 unité et un plan lointain à 5000 unités. Le rapport d'aspect DOIT être celui de la cible de rendu, sans bandes noires : une fenêtre plus large montre davantage de scène horizontalement.
- Critère d'acceptation : en 16:9 avec FOV = 70°, à la distance 189,65 du plan NES, la hauteur visible dans ce plan vaut 265,6 unités (±1 %), la largeur visible 472,2 unités.

**REQ-SCN-005** (P1) — Le réglage FOV DOIT valoir 70° par défaut et prendre les valeurs 45° à 95° par pas de 5° (curseur UI de −5 à +5, FOV = 70 + 5 × valeur). Tout changement DOIT s'appliquer immédiatement et déclencher une réinitialisation de vue (REQ-SCN-012). Le FOV est un réglage utilisateur : le chargement d'un profil ne le modifie pas.
- Critère d'acceptation : passer de 70° à 90° replace la caméra à la distance 156,21 du centre, sans transition brutale de l'angle de vue (seule la distance est lissée).

### 3.3 Modes de caméra et placement

**REQ-SCN-006** (P1) — La caméra DOIT avoir deux modes, de valeurs numériques visibles par Lua : **TPV = 0** (vue orbitale autour de l'écran NES, mode par défaut) et **FPV = 1** (vue qui suit un personnage). Chaque mode DOIT posséder son propre jeu de **valeurs de consigne** : distance d, angles r = (rx, ry, rz) en degrés et point cible T (en coordonnées scène). Les commandes et l'API Lua agissent sur le jeu du mode courant. Changer de mode DOIT ouvrir une **fenêtre de transition de 3 secondes** (REQ-SCN-020).
- Critère d'acceptation : en TPV, régler d = 150 puis passer en FPV, régler d = 40, revenir en TPV : la caméra retourne vers d = 150.

**REQ-SCN-007** (P1) — En TPV (caméra locale désactivée), la position de l'œil E et l'orientation DOIVENT être calculées ainsi, à partir des valeurs **lissées** (REQ-SCN-019) :
1. rotation q = R_Y(ry) · R_X(rx) · R_Z(rz), où, dans le repère main gauche, R_X(a) envoie (0, 0, 1) sur (0, −sin a, cos a), R_Y(b) envoie (0, 0, 1) sur (sin b, 0, cos b) et R_Z(θ) envoie (1, 0, 0) sur (cos θ, sin θ, 0) ;
2. cible effective T′ = T + ((c0 − c2) / 2, (c1 − c3) / 2, 0) : la cible est recentrée sur le rectangle visible ;
3. E = T′ − d · f, avec f = q · (0, 0, 1) = (cos rx · sin ry, −sin rx, cos rx · cos ry) quand rz = 0 ;
4. la caméra est ensuite orientée vers le point P* de REQ-SCN-008, avec l'axe Y du monde comme verticale (pas de roulis ; rz n'a donc pas d'effet visible en TPV).
- Critère d'acceptation : rx = 30°, ry = −20°, d = 189,65, clipping (0 ; 7,9 ; 0 ; 0,1) ⇒ E ≈ (184,2 ; 218,7 ; −154,3) ±0,5. rx > 0 place la caméra au-dessus de l'écran ; ry > 0 la place à gauche.

**REQ-SCN-008** (P1) — En TPV (caméra locale désactivée), le point visé P* DOIT être, dans le plan z = 0, l'intersection des droites M1M2 et M3M4, où, avec A = (c0, c1), B = (256 − c2, c1), Cc = (c0, 240 − c3), D = (256 − c2, 240 − c3) et l'œil E :
- M1 est le point de [A, B], M2 celui de [Cc, D], M3 celui de la diagonale [A, D] et M4 celui de la diagonale [B, Cc] où aboutit la **bissectrice issue de E** de l'angle sous lequel E voit le segment, soit, pour un segment [P, Q] : P + (Q − P) · |P − E| / (|P − E| + |Q − E|) ;
- l'intersection se calcule sur les seules composantes x et y.
L'effet recherché : l'écran NES reste centré dans l'image, corrigé de la perspective, quel que soit l'angle d'orbite.
- Critère d'acceptation : vue par défaut, clipping (0,1 ; 8,1 ; 8,1 ; 0,1) : P* ≈ (123,4 ; 124,6) à ±0,6 près. Pour les cas dégénérés (E confondu avec un sommet), le point du segment est son milieu.
- Notes : l'original ajoute un biais d'arrondi de 0,5 unité à chaque composante du résultat, dans le sens allant de M1 vers l'intersection (donc +0,5 en y en pratique, ±0,5 en x). Le reproduire est facultatif ; la tolérance de ±0,6 le couvre.

**REQ-SCN-009** (P2) — Le réglage de profil **caméra locale** (Lua : `Setting.LocalCam`, alias `Setting.TopDownCam`, faux par défaut) DOIT remplacer, dans les deux modes, la rotation par q = R_Z(ry) · R_X(rx) (rz ignoré) et supprimer l'orientation vers P* : la caméra regarde exactement selon q · (0, 0, 1), avec la verticale q · (0, 1, 0). En TPV, la cible effective T′ reste recentrée (REQ-SCN-007, étape 2). Avec ce réglage, ry fait tourner l'image autour de l'axe de visée et rx incline la vue.
- Critère d'acceptation : caméra locale active, rx = 0, ry = 90° : l'image NES apparaît tournée d'un quart de tour, le haut de l'image NES vers la droite de l'écran ; elle est centrée sur T′.
- Notes : utilisé par 3 profils (jeux à vue de dessus), parfois basculé par Lua selon la scène du jeu.

**REQ-SCN-010** (P1) — En FPV, l'œil DOIT être E = T − d · f (sans recentrage sur le rectangle visible) et l'orientation exactement q (REQ-SCN-007 étape 1, roulis rz inclus, ou REQ-SCN-009 si la caméra locale est active), sans orientation vers P*.
- Critère d'acceptation : en FPV avec r = (0, 0, 0), la cible est exactement au centre de l'image ; avec rz = 90°, l'image est tournée d'un quart de tour.

**REQ-SCN-011** (P1) — La distance de consigne DOIT être bornée à [0, 512] (toute écriture hors bornes, par l'utilisateur ou par Lua, est ramenée dans l'intervalle). Les angles NE DOIVENT PAS être bornés : l'utilisateur peut orbiter sans limite.
- Critère d'acceptation : `Camera.Distance = 1000` puis lecture ⇒ 512 ; `Camera.Distance = -5` ⇒ 0. Maintenir la rotation vers le bas pendant 13 s fait passer rx au-delà de 360°.

### 3.4 Réinitialisation et valeurs par défaut

**REQ-SCN-012** (P1) — La **réinitialisation de vue** DOIT : appliquer le FOV courant ; mettre les angles de consigne des **deux** modes à (0,81° ; 0,81° ; 0) ; mettre la cible des deux modes à C = (128, 120, 0) ; mettre la distance du **mode courant seulement** à d₀ = 104 / atan(FOV_rad / 2), où atan est bien l'arc tangente de la moitié du FOV exprimé en radians. La distance de l'autre mode est inchangée. Au lancement de l'application, la distance FPV vaut 0. Les valeurs affichées rejoignent ces consignes par le lissage (REQ-SCN-019).
- Critère d'acceptation : FOV 70° ⇒ d₀ = 189,65 ; 45° ⇒ 277,93 ; 95° ⇒ 150,25 (±0,05).
- Notes : la formule d'origine est conservée telle quelle, car elle détermine le cadrage par défaut observable (l'écran NES occupe environ 87 % de la hauteur de l'image à 70°). La cible C = (128, 120, 0) est le centre du cadre de référence, 8 unités **sous** le centre de l'image NES (128, 128) (REQ-E3D-410) : la vue par défaut est donc centrée légèrement plus bas que l'image, comme dans l'original ; avec les marges par défaut, la cible effective T′ = (124, 124, 0) (REQ-SCN-007) est le centre de R.

**REQ-SCN-013** (P1) — La réinitialisation de vue DOIT être déclenchée par : le démarrage de l'application ; la **fin d'une partie**, qui DOIT en outre repasser en TPV, remettre le temps de lissage à 0,4 s et remettre la vitesse de jeu à sa valeur normale (SPEC-07) ; tout changement du réglage FOV ; un **appui long** (relâché après plus de 0,3 s) sur l'action Menu (SPEC-08) ; l'appel Lua `Camera:ResetView()` ; le cas de REQ-SCN-002.
- Critère d'acceptation : après avoir tourné et zoomé, un appui long sur Menu ramène la vue par défaut en environ 1 s, sans ouvrir le menu.

### 3.5 Contrôles utilisateur

**REQ-SCN-014** (P1) — Les actions étendues de rotation (SPEC-08) DOIVENT modifier les angles de consigne à **30° par seconde** de temps réel : RotLeft diminue ry, RotRight augmente ry, RotDown augmente rx, RotUp diminue rx. Si RotLeft et RotRight sont pressées ensemble, seule RotLeft agit ; de même RotDown l'emporte sur RotUp.
- Critère d'acceptation : maintenir RotRight 3 s fait augmenter ry de 90° (±2 %), quel que soit le nombre d'images par seconde.

**REQ-SCN-015** (P1) — Les actions ZoomIn et ZoomOut DOIVENT diminuer ou augmenter la distance de consigne de **70 unités par seconde** de temps réel. ZoomIn l'emporte si les deux sont pressées.
- Critère d'acceptation : maintenir ZoomIn 1 s depuis la vue par défaut (FOV 70°) amène la distance de consigne à 119,65 (±2 %).

**REQ-SCN-016** (P2) — Quand aucune action de zoom n'est pressée, la molette de la souris DOIT modifier la distance de consigne : un cran vers l'avant la diminue de **5 unités**, un cran vers l'arrière l'augmente d'autant.
- Critère d'acceptation : 10 crans vers l'avant depuis la vue par défaut ⇒ distance de consigne 139,65 (±1).
- Notes : dans l'original, l'effet d'un cran est proportionnel à la durée de la frame (5 unités à 60 images/s) ; la réimplémentation DEVRAIT le rendre indépendant de la cadence.

**REQ-SCN-017** (P2) — Un glissement avec le bouton gauche de la souris (ou un doigt sur écran tactile), hors interaction avec un élément d'UI et sans touche Maj enfoncée, DOIT faire tourner la caméra : rx diminue de 0,1° par pixel de déplacement vertical vers le haut (axe y de l'écran orienté vers le haut) et ry augmente de 0,1° par pixel de déplacement vers la droite.
- Critère d'acceptation : glisser de 300 pixels vers la droite augmente ry de 30°.

**REQ-SCN-018** (P1) — Les rotations et zooms des REQ-SCN-014 à 016 NE DOIVENT agir que pendant que le jeu est en cours (pas en pause ni dans les menus).
- Critère d'acceptation : menu de pause ouvert, la molette et les touches de rotation n'ont aucun effet sur la vue retrouvée à la reprise.

### 3.6 Lissage

**REQ-SCN-019** (P1) — La caméra affichée DOIT rejoindre ses valeurs de consigne par un **lissage critique** (ressort amorti critique, sans dépassement) de temps caractéristique Tₛ, appliqué séparément à chacune des composantes rx, ry, rz (sans repliement modulo 360°), à la distance et au point cible. Pour un pas de temps Δt et une vitesse interne v (initialement nulle), avec ω = 2 / Tₛ, x = ω·Δt et k = 1 / (1 + x + 0,48 x² + 0,235 x³) : en posant δ = valeur − consigne et w = (v + ω·δ)·Δt, la nouvelle vitesse est (v − ω·w)·k et la nouvelle valeur consigne + (δ + w)·k ; si la consigne est franchie, la valeur est la consigne et la vitesse devient nulle. Tₛ vaut 0,4 s par défaut, est modifiable par Lua (`Setting.SmoothTime`, n'est pas persisté) et revient à 0,4 s à la fin de chaque partie.
- Critère d'acceptation : saut de consigne de distance de 100 unités avec Tₛ = 0,4 s : après 0,4 s l'écart restant est 40,6 (±3) ; après 1,2 s il est inférieur à 2.
- Notes : ce filtre est l'amortisseur classique « smooth damp » de la littérature du jeu vidéo. Lisser les angles composante par composante produit, pour de grands changements, une trajectoire différente d'une interpolation de quaternions : c'est le comportement attendu.

**REQ-SCN-020** (P1) — En TPV, la cible effective T′ DOIT toujours être lissée. En FPV, la cible DOIT être lissée pendant la fenêtre de transition de 3 s qui suit un changement de mode ; ensuite, la caméra DOIT suivre la cible instantanément.
- Critère d'acceptation : en FPV, 5 s après l'entrée dans le mode, un déplacement brusque du personnage suivi est reproduit par la caméra dans la même frame.

### 3.7 Mode FPV : suivi et transparence d'occultation

**REQ-SCN-021** (P1) — L'entrée en FPV DOIT être possible : par Lua, avec `Camera:SetFPV(shape)` (la cible prend la position de la première tile de la shape, et le suivi de cette shape démarre via le tracker de SPEC-04), `Camera:SetFPV()` (changement de mode seul, cible inchangée), `Camera.Mode = 1` et `Camera:ToggleMode()`. À chaque frame NES en FPV, si une shape est suivie, la cible DOIT devenir la position de la première tile de la shape retrouvée par le tracker ; si le tracker perd la shape, le suivi cesse et la cible reste en place.
- Critère d'acceptation : un profil qui appelle `Camera:SetFPV(s)` sur le personnage voit la caméra se centrer sur lui puis l'accompagner pendant ses déplacements.

**REQ-SCN-022** (P2) — En FPV avec une shape suivie, toute tile d'une **autre** shape dont la géométrie coupe le segment allant de l'œil à la cible, et qui serait rendue opaque, DOIT être rendue cette frame-là en transparence avec une opacité de 0,5 (REQ-SCN-040).
- Critère d'acceptation : un mur placé entre la caméra et le personnage suivi devient semi-transparent tant qu'il masque le personnage, puis redevient opaque.

### 3.8 Pilotage par les profils et état sauvegardé

**REQ-SCN-023** (P1) — L'API caméra exposée à Lua (noms exacts : SPEC-05) DOIT avoir les effets suivants, toujours sur les **consignes** du mode courant (le lissage s'applique ensuite) :

| Membre | Effet |
|---|---|
| `Camera.Distance` (lecture/écriture) | distance de consigne, bornée à [0, 512] |
| `Camera.Rot` (objet x, y, z modifiable en place) | angles de consigne rx, ry, rz en degrés |
| `Camera.Mode` (lecture/écriture), `Camera.IsFPV`, `Camera.IsTPV` | mode courant (TPV = 0, FPV = 1) ; une écriture qui change le mode ouvre la fenêtre de 3 s |
| `Camera.LookAt` (lecture/écriture), `Camera:SetTarget(x, y, z)`, `Camera:GetTarget()` | cible T en coordonnées scène |
| `Camera.FpvShape` (lecture/écriture) | shape suivie ; l'écriture d'une shape non nulle place la cible sur sa première tile |
| `Camera:SetFPV([shape])`, `Camera:SetTPV()`, `Camera:ToggleMode()` | changements de mode (REQ-SCN-021) |
| `Camera:ResetView()` | REQ-SCN-012 |

- Critère d'acceptation : `Camera:SetTarget(128, 180, 0)` en TPV déplace progressivement la vue vers le haut de l'écran NES (cible effective (128 + (c0 − c2)/2 ; 180 + (c1 − c3)/2 ; 0)).

**REQ-SCN-024** (P1) — L'état de la caméra DOIT faire partie des sauvegardes d'état (format : SPEC-07) : mode courant, et pour chaque mode distance, angles et cible de consigne, ainsi que l'échéance de la fenêtre de transition. Au chargement, ces consignes sont restaurées et la caméra affichée les rejoint par le lissage.
- Critère d'acceptation : sauvegarder en FPV, passer en TPV et zoomer, recharger : la caméra revient en FPV avec la distance d'alors.

### 3.9 Lumière

**REQ-SCN-025** (P1) — La scène DOIT être éclairée par une seule **lumière directionnelle**. Le vecteur `Light.Direction` donne le **sens de propagation** des rayons (il est normalisé) ; la valeur par défaut (1, −1, 1) correspond à une lumière venant d'en haut, de la gauche et du côté de la caméra. La couleur `Light.Color` vaut blanc par défaut, l'intensité `Light.Intensity` 1,0. Tout changement DOIT prendre effet à la frame rendue suivante, sans transition.
- Critère d'acceptation : avec la direction (1, −1, 1), les faces d'un cube tournées vers −Z, +Y et −X reçoivent le même éclairement (N·L = 0,577) ; les faces +X, −Y et +Z ne reçoivent que la lumière ambiante.

**REQ-SCN-026** (P1) — La direction et la couleur de la lumière DOIVENT être des réglages de **profil** (chargés au début de la partie, modifiables par Lua pendant la partie). L'intensité DOIT être un réglage **utilisateur** : conservée d'un profil à l'autre, ignorée dans le profil, réglable de 0,50 à 1,50 par pas de 0,01 (curseur UI 0–100, intensité = 0,5 + valeur / 100, défaut 1,0).
- Critère d'acceptation : 45 des 101 profils embarqués utilisent (1, −1, 1), 14 utilisent (1, −1, 2) ; régler l'intensité à 1,2 puis changer de jeu conserve 1,2.

**REQ-SCN-027** (P1) — La lumière ambiante DOIT être uniforme (même valeur dans toutes les directions), de couleur sRGB (118, 118, 118), soit 0,1812 en linéaire. Elle ne dépend ni de la lumière directionnelle, ni de la skybox, ni du profil.
- Critère d'acceptation : une face blanche (NES 0x30) ne recevant que l'ambiante s'affiche en sRGB (117, 115, 117) ±2.

**REQ-SCN-028** (P1) — L'ombrage des tiles DOIT être **diffus lambertien**, calculé en espace linéaire, sans composante spéculaire ni émissive : couleur_lin = albedo_lin × (ambiante_lin + max(0, N·L) × lumière_lin × S), où L est l'opposé de la direction de propagation, N la normale du mesh (inchangée par la déformation de REQ-SCN-043), S le facteur d'ombre (REQ-SCN-031) et lumière_lin la couleur de la lumière multipliée par l'intensité, convertie en linéaire. Le résultat est encodé en sRGB à l'affichage, saturé à 1, sans opérateur de tone mapping.
- Critère d'acceptation : voir les vecteurs de test §5.2 (tolérance ±3 par canal sur 255).
- Notes : pour une intensité différente de 1, l'original applique au facteur d'intensité la même courbe sRGB → linéaire qu'aux couleurs (≈ intensité^2,2), comportement retenu par DEC-06-Q2 (différentiel au round 2).

**REQ-SCN-029** (P2) — Le HUD (REQ-SCN-045) DOIT être éclairé avec la même couleur, la même intensité et le même modèle que la scène, mais avec une direction exprimée dans le **repère de la caméra** : le vecteur `Light.Direction` y est interprété comme si la caméra était dans son orientation par défaut (regard vers +Z, verticale +Y). L'éclairage du HUD ne change donc pas quand la caméra tourne. Le HUD ne projette ni ne reçoit d'ombre.
- Critère d'acceptation : orbiter de 90° autour de l'écran ne change pas la teinte des chiffres du score en 3D.

### 3.10 Ombres

**REQ-SCN-030** (P1) [ROM] — La lumière directionnelle DOIT projeter des **ombres dures** (bords nets, sans filtrage doux) au moyen d'une carte d'ombre unique (sans cascades), couvrant la partie de la scène située à moins de 512 unités de la caméra. Le réglage utilisateur **Ombres** (défaut : activé) DOIT pouvoir les désactiver complètement : plus aucune tile ne projette ni ne reçoit d'ombre.
- Critère d'acceptation : en harnais, scène synthétique (une tile pleine en L1 au-dessus d'une tile plate en L2 qui reçoit les ombres, lumière (1, −1, 1)) : l'ombre portée est présente avec un bord net (transition sur au plus 2 pixels de la capture 640 × 480) ; ombres désactivées, elle disparaît. [ROM] En vue par défaut sur Super Mario Bros, les blocs flottants projettent une ombre nette sur les éléments des couches plus profondes.
- Notes : la carte d'ombre DOIT avoir **1024 texels** de côté par défaut (DEC-R2-D4) ; l'original produit des bords plus crénelés, et cet écart de crénelage est toléré.

**REQ-SCN-031** (P1) — Dans une zone ombrée, le terme de lumière directe DOIT être multiplié par **0,2** (force d'ombre 0,8) ; l'ambiante n'est pas affectée. Un biais de profondeur d'environ 0,02 unité et un décalage selon la normale d'environ 0,02 unité DEVRAIENT être appliqués pour éviter l'acné d'ombre.
- Critère d'acceptation : une face blanche frontale (N·L = 0,577) passe de sRGB 223 en plein éclairage à 146 ±3 dans l'ombre.

**REQ-SCN-032** (P1) — Une tile DOIT **projeter** une ombre si et seulement si : les ombres sont activées, elle n'appartient pas au HUD, elle est rendue opaque (REQ-SCN-040), le drapeau de projection de son motif (SPEC-03/04) est vrai et le drapeau de projection de sa couche Z est vrai. Exception : une tile de style « ombre seule » (REQ-SCN-041) projette toujours son ombre. Une tile DOIT **recevoir** les ombres si et seulement si : les ombres sont activées, elle n'appartient pas au HUD, le drapeau de réception de son motif et celui de sa couche sont vrais. Les tiles rendues en transparence DEVRAIENT ne pas recevoir d'ombre.
- Critère d'acceptation : désactiver par Lua la projection de la couche L2 fait disparaître les ombres de ses tiles dès la frame suivante.

**REQ-SCN-033** (P1) — Au chargement de chaque profil, les drapeaux d'ombre des couches DOIVENT être réinitialisés à : L1 projette et reçoit ; L2, L3 et L4 reçoivent sans projeter ; UI et UNIDENTIFIED ni l'un ni l'autre. Les valeurs éventuellement stockées dans le profil sont ignorées. Lua PEUT les modifier pendant la partie (`Setting.Layer[i].CastShadow`, `Setting.Layer[i].RecvShadow`, i de 0 à 5 selon §2).
- Critère d'acceptation : 13 scripts embarqués activent `Setting.Layer[3].CastShadow` (L2) ; sans ce script, L2 ne projette rien.

### 3.11 Matériaux et ombrage des tiles

**REQ-SCN-034** (P1) — La couleur d'albédo d'un fragment de tile DOIT être obtenue ainsi (atlas, coordonnées et répétition : SPEC-04B) : en mode **indexé**, l'indice de couleur lu dans l'atlas sélectionne une couleur dans une ligne de la table des palettes de la frame : ligne de la palette forcée du motif si elle est définie, sinon palette de la tile (0–3) pour une shape de fond et palette de la tile + 4 pour une shape de sprites ; en mode **couleur directe**, la couleur RVB vient de l'atlas. L'échantillonnage DOIT être au plus proche voisin, sans mipmap. La table des palettes est celle de SPEC-04 (REQ-E3D-250) et la palette RVB des 64 couleurs NES celle du §4.1 de SPEC-04 (DEC-04B-Q1) ; la résolution indexée suit SPEC-04B REQ-E3D-560.
- Critère d'acceptation : une tile de fond de palette 2 et d'indice 1 prend la couleur NES de l'entrée 2×4 + 1 de la table de palettes de la frame.

**REQ-SCN-035** (P1) — Les couleurs de palette et de textures couleur DOIVENT être traitées comme du sRGB : converties en linéaire avant l'éclairage, et le résultat ré-encodé en sRGB (REQ-SCN-028). Les canaux non colorimétriques (indice, alpha) ne sont pas convertis.
- Critère d'acceptation : vecteurs §5.2.

**REQ-SCN-036** (P1) — Tout fragment de tile (opaque ou transparente, hors HUD) dont la position scène (x, y) sort du rectangle visible R DOIT être éliminé, quelle que soit sa profondeur. Les objets 3D sont donc découpés nettement aux bords de R, comme par un emporte-pièce prolongé en Z.
- Critère d'acceptation : en vue orbitale à 45°, un objet à cheval sur le bord gauche de R apparaît tranché net dans le plan x = c0 ; les ombres qu'il projette ne sont pas découpées.

**REQ-SCN-037** (P2, amendée par AMD-057) — Les tiles de sprite (shapes non-fond) DOIVENT être décalées en profondeur, après placement, de `n = 0,001 × (i mod 64) − 0,1` unité (vers la caméra) pour un sprite devant le fond et de `n = 0,001 × (i mod 64) + 0,1` unité (vers le fond) pour un sprite à priorité « derrière le fond » (priorité NES), où `i` est l'**indice du sprite** de SPEC-02 REQ-CORE-043 (0 à 63, ou 64 + n pour la moitié basse d'un 8×16, qui prend donc le même décalage que sa moitié haute). Les tiles de fond ne sont pas décalées. Le rendu est **seul propriétaire** de ce décalage (DEC-06-Q4, DEC-R2-T4) : les transformations livrées par SPEC-04B ne l'incluent pas (REQ-E3D-405, 641) ; il est appliqué **par tuile**, dans le repère local de la shape, avant son échelle et sa rotation, une seule fois.
- Critère d'acceptation : un sprite plat posé sur une tile de fond plate de même profondeur reste visible devant elle, sans scintillement ; TV-W2 de SPEC-04B : face avant d'un sprite d'indice 0 en L1 FRONT affichée à z = −0,1, d'indice 5 à z = −0,095, moitié basse d'indice 69 à z = −0,095.
- Critère d'acceptation (AMD-057) : deux sprites plats qui se recouvrent, d'indices 3 et 10 → le sprite 3 est dessiné devant, sans scintillement.
- Note (AMD-057 §1) : la conséquence est le but de l'amendement — à recouvrement égal, le sprite d'indice le plus faible, le plus prioritaire sur la NES, est devant. Le terme d'indice ne vaut au plus que 0,063 unité, donc un sprite devant le fond reste devant lui pour tout indice.

**REQ-SCN-038** (P2) — Le rendu DOIT appliquer à chaque shape la position, l'échelle et la rotation livrées par le moteur (SPEC-04B), et ne rendre que les shapes actives dont le mesh est prêt.
- Critère d'acceptation : couverture par les vecteurs de SPEC-04B.

**REQ-SCN-039** (P1) — Le rectangle de découpe R DOIT être recalculé à chaque frame rendue depuis le clipping courant (REQ-SCN-003).
- Critère d'acceptation : un script qui modifie `Setting.Clipping[0]` de 0,1 à 16,1 masque dès la frame suivante la bande x < 16,1.

**REQ-SCN-040** (P1) — Un motif d'opacité alpha < 1 (et de style différent de « ombre seule ») DOIT être rendu en **transparence** : d'abord un passage qui n'écrit que la profondeur, puis un passage de couleur mélangé (source × α + destination × (1 − α)), avec α = alpha du texel (canal alpha de la couleur de palette ou de la texture) × alpha du motif. Seule la surface la plus proche de chaque objet transparent est ainsi visible (pas de faces internes). Les objets transparents DOIVENT être dessinés après tous les objets opaques, triés du plus lointain au plus proche. Ils sont éclairés (REQ-SCN-028) mais ne projettent pas d'ombre.
- Critère d'acceptation : un motif d'alpha 0,5 devant un fond rouge laisse voir le fond à environ 50 %, et on ne distingue pas ses faces arrière.

**REQ-SCN-065** (P2, créée par AMD-012, complétée par AMD-058) — **Jointures** : la jointure de deux tuiles voisines d'une surface plane NE DOIT montrer ni grille sombre, ni fissure claire, ni discontinuité d'ombre, que ces tuiles appartiennent au même objet ou à deux objets différents de même profondeur et de même couche. L'exigence DOIT tenir pour **chaque** valeur de `video.anti_alias` (0, 1 et 2). Les moyens sont **libres** — traitement des échantillons, de la profondeur des parois internes, de la résolution des échantillons — à l'exclusion :
- du retrait des parois latérales du maillage dessiné (AMD-057, SPEC-04B REQ-E3D-501) ;
- de la désactivation de l'anticrénelage ou de la baisse du nombre d'échantillons : le réglage choisi DOIT garder son effet sur les bords des objets (AMD-032 §2).

Résidu accepté (AMD-014 Q-032, P3) : 1 à 2 px sur les translucides obliques.
- Critère d'acceptation (auto) : la surface de 4 × 4 tuiles identiques, éclairée et recevant l'ombre, passe avec `video.anti_alias` = 0, 1 et 2.
- Critère d'acceptation (auto) : la même surface faite de tuiles appartenant à **deux objets** voisins (deux colonnes de 2 × 4 tuiles) passe aussi, pour les trois valeurs.
- Critère d'acceptation (revue) : façade de la tour de Mega Man 2 (frame 599 de l'intro, sans entrée) et mur bleu d'Excitebike (frame 599, Start aux frames 200 et 300), rendus en 2560 × 1440 avec `video.anti_alias` = 1 : aucune ligne claire ; la capture avec anticrénelage diffère de celle sans anticrénelage seulement au bord des objets (contours adoucis).
- Notes (AMD-058 §1 et §3) : le défaut venait du seul chemin anticrénelé — à `video.anti_alias` = 0, même binaire et même scène, aucune ligne. La cause est l'**extrapolation** de la coordonnée de texel (REQ-E3D-506) à un fragment partiellement couvert dont le centre de pixel tombe hors de la primitive : le `floor` de la recherche d'atlas (REQ-E3D-563) atteint alors un texel que la face ne porte pas. L'écart avec l'original reste une divergence D assumée (la référence rend sans anticrénelage, AMD-032 §1) ; cette exigence demande seulement qu'elle n'ajoute pas de défaut.

**REQ-SCN-041** (P1) — Le style de rendu d'un motif (SPEC-03/04) DOIT être respecté : 0 = invisible (ni dessiné, ni ombre) ; 1 = normal ; 2 = **ombre seule** (non dessiné, mais projette son ombre, toujours en opaque).
- Critère d'acceptation : un motif de style 2 n'apparaît pas à l'écran mais son ombre est visible sur le décor qui reçoit les ombres.

**REQ-SCN-042** (P3) — Le rendu des tiles NE DOIT comporter ni contour, ni effet CRT, ni lignes de balayage, ni émission propre, ni brouillard.
- Critère d'acceptation : inspection visuelle des vues de référence §5.4.

**REQ-SCN-043** (P2) — Un motif dont la déformation animée est active DOIT voir ses sommets déplacés à chaque frame rendue selon **SPEC-04B REQ-E3D-568, qui est normative** (pivot `Pd` avec le décalage `h₀` dépendant de la version du profil, temps de frame = frames émulées / 60, application avant l'échelle et la rotation de l'objet, DEC-04B-Q2). Le rendu PEUT appliquer ce déplacement lui-même à partir des paramètres livrés (REQ-E3D-641). Les normales ne sont pas modifiées. L'animation suit le temps **émulé** : elle se fige en pause et avance en accéléré.
- Critère d'acceptation : vecteur TV-D1 de SPEC-04B ; avec `deltaSpeed[1][3] = 2π` en plus, l'onde fait un cycle par seconde de jeu et reste figée pendant une pause.

### 3.11bis Style visuel Enhanced (AMD-055)

Section nouvelle (AMD-055 §3 point 8). Le **style visuel** choisit entre le rendu de ce
chapitre, dit `Classic`, et un rendu `Enhanced` qui lui ajoute quatre effets optionnels, tous
désactivés par défaut : à `render.visual_style` = 0, et en Enhanced avec les quatre effets à
0, le rendu est celui des sections précédentes, sans changement d'image ni de coût
(REQ-SCN-080). Les exigences ci-dessous sont reprises d'AMD-055 §2 ; leurs paramètres sont
les treize réglages du §4.1 de ce chapitre et de SPEC-09 §4.1.1. Les renvois internes « §2.1 » à
« §2.5 » sont ceux de l'amendement, et désignent ici les sous-sections 3.11bis.1 à 3.11bis.5 ; les
renvois « §3 » et « §4 » sont ceux de l'amendement également (changements documentaires, lignes de
SPEC-09 et libellés).


#### 3.11bis.1 Réglage du style

**REQ-SCN-078** (P2) — **Styles.** `render.visual_style` (AMD-040 §3) DOIT désormais choisir le
rendu : **Classic** (rendu de SPEC-06, inchangé) ou **Enhanced** (§2.2 à 2.5). L'identifiant du
style Enhanced est **4** dans le programme d'origine ; Zaxnes DOIT accepter 4 et 1 comme
identifiants d'Enhanced, en écriture (`Setting.VisualStyle`, réglage) comme en argument des
méthodes `Get/Set/ResetVisualStyleParameter(s)` (REQ-LUA-114), et DOIT renvoyer **4** en lecture de
`Setting.VisualStyle` quand Enhanced est actif ; 0 reste Classic ; toute autre valeur écrite donne 0,
y compris quand elle vient du fichier de réglages (SPEC-09 REQ-PERS-003, règle de la colonne
« Domaine » du §4). Le changement de style ou d'un de ses réglages s'applique à la frame rendue
suivante, sans recharger le jeu. Les réglages du style ne viennent jamais du profil.

La **courbure** d'AMD-043 (REQ-SCN-066 à REQ-SCN-068) n'est **pas** un effet du style : elle agit
en Classic comme en Enhanced **(D)**, de sorte qu'un changement de style ne fasse pas perdre un
réglage de courbure (§3 point 4).
- Critère d'acceptation (auto) : `Setting.VisualStyle = 4` → relu 4 ; `= 1` → relu 4 ;
  `Setting:SetVisualStyleParameter(4, "spl.bend_zx", 30)` → vrai ; `Setting:SetVisualStyleParameter(1,
  "ao.strength", 5)` → vrai ; `= 2` → relu 0 ; fichier de réglages contenant `render.visual_style = 1`
  → relu 4, `= 7` → relu 0.
- Critère d'acceptation (auto) : `Setting.VisualStyle = 0` avec Courbure gauche-droite à 60° : la
  scène reste courbée (sommets exportés à B(P) près, AMD-043 REQ-SCN-067).

**REQ-SCN-079** (P2) — **Paramètres d'Enhanced** (portée U, persistés sous les clés du §4,
normalisation d'AMD-040 §3 : valeur bornée puis arrondie au pas, pas 1) ; ils s'ajoutent aux quatre
paramètres de courbure d'AMD-043 :

| Id | Libellé (EN / FR) | Valeurs | Défaut |
|---|---|---|---|
| `spl.ao` | Ambient Occlusion / Occlusion ambiante | 0 Off, 1 MSVO (occlusion multi-échelle de REQ-SCN-083 ; rendue comme Depth tant que son portage n'est pas disponible, **D**), 2 Depth | 0 |
| `ao.strength` | Strength / Force | 1 à 5 | 3 |
| `ao.radius` | Radius / Rayon | 1 à 5 | 3 |
| `ao.thickness` | Thickness / Épaisseur | 1 à 5 (sans effet observable hors du mode MSVO, voir ci-dessous) | 3 |
| `ao.quality` | AO Quality / Qualité de l'occlusion | 0 Low, 1 Medium, 2 High | 1 |
| `spl.pixel_shadow` | Pixel Shadows / Ombres pixel | 0 Off, 1 On | 0 |
| `spl.voxel_grid` | Voxel Grid / Grille de voxels | 0 Off, 1, 2, 3 — côté de cellule `N = 2^spl.voxel_grid` voxels (1 → 2, 2 → 4, 3 → 8) | 0 |
| `spl.reflections` | Reflections / Reflets | 0 Off, 1 On | 0 |
| `ssr.shine` | Floor Shine / Brillance du sol | 1 à 5 | 3 |
| `ssr.clarity` | Clarity / Netteté | 1 à 5 | 3 |
| `ssr.length` | Length / Longueur | 1 à 5 | 3 |
| `ssr.quality` | Reflection Quality / Qualité des reflets | 0 Low, 1 Medium, 2 High, 3 Ultra | 2 |

Ces douze identifiants sont normatifs parce qu'ils sont des **arguments de l'API Lua**
(REQ-LUA-114) et, pour cette raison, des faits d'interopérabilité ; les autres identifiants de
paramètres de 3dSen 2.0 restent « inconnus » au sens d'AMD-040 §3.

`ao.thickness` est conservé, persisté et exposé. Il **n'a aucun effet observable** tant que le mode
MSVO est rendu comme Depth (repli **(D)** de REQ-SCN-083), parce que l'occlusion Depth ne l'utilise
pas ; il est alors présenté **grisé (D)**, comme les réglages de SPEC-09 REQ-PERS-017 — par analogie
seulement, puisque REQ-PERS-017 énumère des réglages sans effet **dans l'original**, ce qui n'est pas
le cas ici. Quand le portage MSVO est disponible, `ao.thickness` **agit** en mode MSVO
(REQ-SCN-083) et n'est plus grisé pour `spl.ao` = 1.

Présentation (voir §3 point 6 et §4) : les paramètres sont présentés dans la grille de paramètres
desktop (SPEC-10 REQ-UI-061 amendée), à la suite de la ligne `Style`, dans un groupe repliable
« Style » ; les réglages d'occlusion ne sont visibles que si `spl.ao` ≠ 0, ceux des reflets que si
`spl.reflections` = 1. Aides (EN / FR) : occlusion « Soft shade in corners and hollows. » /
« Ombre douce dans les coins et les creux. » ; ombres pixel « Shadows that follow the game's pixel
grid. » / « Des ombres qui suivent la grille de pixels du jeu. » ; grille « Grooves every 2, 4 or 8
voxels, like toy bricks. » / « Des rainures tous les 2, 4 ou 8 voxels, comme des briques de
jouet. » ; reflets « The floor reflects the scene like a polished surface. » / « Le sol reflète la
scène comme une surface polie. ».
- Critère d'acceptation (auto) : chaque paramètre, **écrit depuis l'UI ou depuis le modèle de
  réglages**, survit à un redémarrage ; `ao.strength` écrit à 9 → 5, à 1,4 → 1, à 0 → 1 ;
  `spl.voxel_grid` écrit à 4 → 3 ; `spl.ao` écrit à 1,4 → 1.
- Critère d'acceptation (auto) : une écriture **par Lua** suit SPEC-09 REQ-PERS-010 (couche de
  session) : `Setting:SetVisualStyleParameter(4, "ao.strength", 5)` pendant une partie, puis fin de
  partie → `ao.strength` revient à la préférence de l'utilisateur.
- Critère d'acceptation (auto) : changer `ao.thickness` de 1 à 5, `spl.ao` = **2**, tous les autres
  réglages fixés : la capture est identique au pixel près. (En mode MSVO effectif, c'est le critère
  inverse qui s'applique, REQ-SCN-083.)

#### 3.11bis.2 Éclairage de base et ordre des effets

**REQ-SCN-080** (P2) — En Enhanced, l'éclairage des tiles, du backdrop et du HUD est celui de
Classic (tiles : REQ-SCN-028 ; backdrop : REQ-SCN-075 ; HUD : REQ-SCN-029 pour l'éclairage et
REQ-SCN-044 pour le rendu), auquel s'appliquent les effets de §2.3 à 2.5 quand ils sont activés.
Tous les effets désactivés, une capture Enhanced est identique à une capture Classic.

**Ordre des calculs**, par analogie avec la clause d'ordre d'AMD-043 REQ-SCN-068 :
- la grille de voxels (REQ-SCN-081) et les ombres pixel (REQ-SCN-082) sont évaluées **avant la
  courbure**, dans le repère local du maillage (REQ-E3D-504, REQ-E3D-522) pour les tiles et dans le
  repère de scène pour le backdrop : rainures et carrés d'ombre **suivent** donc la surface courbée,
  comme ils suivent l'échelle et la rotation de l'objet ;
- l'occlusion ambiante (REQ-SCN-083) et les reflets (REQ-SCN-084) travaillent sur la profondeur et
  la couleur **rendues**, donc **après** la courbure ; le test de sol et l'étendue de REQ-SCN-084
  sont en revanche évalués sur la normale et la boîte englobante **avant** courbure, de sorte
  qu'une courbure forte n'interrompe pas un reflet en cours de surface ;
- l'ordre de frame de REQ-SCN-058 devient : carte d'ombre ; effacement ou skybox ; objets opaques ;
  **reflets du sol (REQ-SCN-084)** ; **occlusion ambiante (REQ-SCN-083)** ; objets transparents
  triés ; HUD ; puis l'UI (§3 point 3) ;
- **ce qui occulte et ce qui est reflété** découle de cet ordre, et de lui seul. Les **objets
  transparents** (REQ-SCN-040) reçoivent la grille de voxels mais **pas** les ombres pixel (ils ne
  reçoivent déjà pas d'ombre, REQ-SCN-032) ; comme ils sont dessinés **après** les deux passes
  d'écran — leur passage de profondeur compris, la passe transparente étant indivisible par objet
  (REQ-SCN-040 lu avec AMD-012) —, ils **n'occultent pas** pour l'occlusion ambiante, ne sont **pas
  reflétés**, et ne sont ni assombris par l'occlusion ni porteurs de reflets. Le **HUD**, dessiné
  encore après, non plus. La **skybox**, dessinée avant les objets opaques, **peut être reflétée**
  mais **n'occulte jamais** : elle est à distance infinie, derrière toute la scène (REQ-SCN-049),
  donc jamais à moins de `2 r` d'un point de la scène. Le **tunnel**, les **modèles de téléviseur**
  (REQ-SCN-053, REQ-SCN-054) et le **rectangle du mode 2D** (AMD-046 §2), dessinés avec les objets
  opaques, **occultent** et **peuvent être reflétés**, sans être eux-mêmes modifiés par les deux
  effets. Les tiles de style « **ombre seule** » (REQ-SCN-041), qui ne sont pas dessinées, n'écrivent
  aucune profondeur : elles n'occultent pas et ne sont pas reflétées, mais elles projettent leur
  ombre comme en Classic ;
- en Enhanced, la scène PEUT être rendue dans une **cible intermédiaire** puis composée dans la
  cible fournie par l'hôte ; celle-ci n'est jamais relue (REQ-SCN-057). Les deux effets d'écran
  travaillent sur la couleur et la profondeur **résolues** (après résolution du MSAA éventuel de
  `video.anti_alias`, AMD-032 §2) et sont appliqués à la résolution de la cible de rendu courante,
  elle-même éventuellement réduite par `video.res_scale` (REQ-SCN-063).
- Critère d'acceptation (auto) : même scène, Classic et Enhanced tous effets à Off : captures
  identiques à ±2 niveaux sRGB par canal.
- Critère d'acceptation (auto), **clause d'ordre vis-à-vis de la courbure** : **cadrage large B** de
  REQ-SCN-081, Courbure gauche-droite (`spl.bend_zx`) à **90°**. La position d'une ligne de rainure
  est relevée, colonne de pixels par colonne de pixels, au **minimum local de luminance** ; sa corde
  joint ses deux extrémités dans la capture. Chaque ligne de rainure **horizontale** de la face avant
  du backdrop située à au moins 8 et au plus 56 unités du centre C — soit `y` = 64, 72, …, 112 et
  `y` = 128, 136, …, 176 — est **courbée** : son écart maximal à sa corde dépasse **15 pixels**, et
  vaut `2,661 × |y − 120|` pixels, soit 21, 43, 64, 85, 106, 128 et 149 px pour `|y − 120|` = 8, 16,
  24, 32, 40, 48 et 56 (tolérance : 3 pixels ou 3 %, la plus grande des deux). Le coefficient se
  recalcule ainsi, avec REQ-SCN-067 (`L` = 256, `s` = 1, θ = 90°, donc `k` = 0,0061359,
  `f` = 1,026175 et `h` = 12,4041) et une focale de `(1080/2)/tan 35° = 771,2` pixels : la face
  avant (`w` = −0,5) passe de `z` = −13,242 en son centre à `z` = +35,892 à ses bords (`u` = ±128,
  portés à `u′` = ±118,62), soit des profondeurs de 97,258 et 146,392 depuis la caméra, donc 7,929
  et 5,268 pixels par unité de `|y − 120|` ; l'écart à la corde est leur différence, 2,661. À
  courbure nulle, la même ligne est **droite** à moins de 1 pixel. La rainure horizontale qui passe
  par le centre (`y` = 120) et les rainures **verticales** restent droites dans les deux captures et
  sont exclues de la mesure : l'enroulement gauche-droite de REQ-SCN-067 laisse la coordonnée `v`
  inchangée, une ligne à `u` constant reste donc un segment droit dans la scène. Voir la Note
  « portée du critère de courbure » de REQ-SCN-081 pour ce que ce critère discrimine.

#### 3.11bis.3 Grille de voxels et ombres pixel

**REQ-SCN-081** (P2) — **Grille de voxels** (`spl.voxel_grid` ≠ 0 ; côté de cellule
`N = 2^spl.voxel_grid` voxels, soit 2, 4 ou 8). La grille module **l'albédo** des faces des
maillages de tiles et des faces du pavé du backdrop, avant tout calcul d'éclairage (REQ-SCN-028
pour les tiles ; terme `c` de REQ-SCN-075 pour le backdrop, de sorte que la rainure n'assombrisse
pas le terme spéculaire).

**Repère.** Pour une tile, la position `p` utilisée est celle du point dans le **repère local du
maillage de la tile** (SPEC-04B REQ-E3D-504 pour GREEDY et GREEDY_MULTICOLOR, REQ-E3D-522 pour
MARCHING — les deux repères coïncident : `x`, `y` ∈ [−4, 4], `z` ∈ [−D/2, D/2], `D` étant la
profondeur de construction), prise **avant** la transformation d'objet (déformation animée, échelle,
rotation, position, REQ-E3D-406) et **avant** la courbure : la grille suit donc l'objet. Les **arêtes
de cellule** sont les coordonnées multiples de `N` dans ce repère : pour `N = 8` et `N = 4`, une
arête passe par le centre de la tile ; pour `N = 2`, les arêtes sont aux coordonnées paires. Pour le
backdrop, `p` est la position du point sur le pavé en **unités de scène** (repère de scène de
REQ-E3D-410 multiplié par l'échelle de scène, avant courbure) et les arêtes sont aux coordonnées
multiples de `N × échelle de scène`.

**Modulation.** Soit `n` la normale unitaire du maillage au point considéré (normale de la face en
GREEDY et GREEDY_MULTICOLOR, normale plate du triangle en MARCHING (REQ-E3D-522)), exprimée dans le même
repère que `p`. On pose, pour chacun des trois axes `a` :

- poids `w_a = clamp(4 × (1 − |n_a|), 0, 1)` : un axe perpendiculaire à la face a un poids nul et
  **ne crée aucune rainure** ; les axes du plan de la face ont un poids 1 ;
- distance à l'arête la plus proche, en **fraction de cellule** :
  `d_a = min_{j entier} |p_a − j·N| / N`, donc `d_a ∈ [0 ; 0,5]` ;
- **alignement sur un axe** : avec `m = max_a |n_a|` et `t = clamp((m − 0,8) / 0,1, 0, 1)`, on pose
  un facteur de rainure `k = 0,38 + 0,24 · t` et un éclaircissement maximal `e = 0,20 − 0,10 · t`.
  Une face alignée sur un axe (`m ≥ 0,9`, donc `t` = 1 : toutes les faces GREEDY, REQ-E3D-505)
  donne `k = 0,62` et `e = 0,10` ; une face à 45° ou plus oblique (`m ≤ 0,8`, donc `t` = 0) donne
  `k = 0,38` et `e = 0,20` ; entre les deux, `k` et `e` sont interpolés linéairement en `t` ;
- facteur de l'axe `a` :
  - **rainure**, pour `d_a ≤ 0,035` (bande de 3,5 % de la cellule de part et d'autre de l'arête) :
    `f_a = 1 − w_a · (1 − k)` ;
  - **arête claire**, pour `0,035 ≤ d_a ≤ 0,16` :
    `f_a = 1 + w_a · e · (0,16 − d_a) / 0,125`, maximale au bord de la rainure et décroissant vers
    l'intérieur de la cellule, où elle atteint exactement 1 en `d_a = 0,16` ;
  - `f_a = 1` ailleurs ;
  - la transition en `d_a = 0,035` (de la rainure à l'arête claire) DOIT être lissée sur environ un
    pixel d'écran ; en `d_a = 0,16`, le facteur vaut déjà 1 et la transition est donc continue ;
- l'albédo est multiplié par `f_x · f_y · f_z`.

La grille NE DOIT PAS s'appliquer au HUD (REQ-SCN-044, -045), à la skybox, au tunnel ni aux modèles
de téléviseur (REQ-SCN-053, REQ-SCN-054), ni au rectangle du mode 2D (AMD-046 §2, transparence
AMD-050 REQ-SCN-076). Elle s'applique aux tiles opaques **et** transparentes (REQ-SCN-080).
- **Gros plan G** (configuration commune aux critères de §2.3, exécutable par un tiers) : harnais,
  mode 3D, capture **1920 × 1080**, `video.anti_alias` = 0, `video.res_scale` = 0 (Native), échelle
  de scène 1, courbure nulle, TV Box = None, skybox `Color` noire, valeurs lues en sRGB 8 bits puis
  converties en linéaire. Surface mesurée : une shape de **4 × 4 tiles pleines** (32 × 32 unités),
  de profondeur de construction `D` = 8, **centrée sur C = (128, 120, 0)** (REQ-E3D-410), **sans
  rotation ni mise à l'échelle** (sauf mention contraire d'un critère) : elle occupe donc
  `x` ∈ [112, 144], `y` ∈ [104, 136], `z` ∈ [−4, 4], et la face mesurée est sa **face avant**, de
  normale (0, 0, −1), dans le plan `z` = −4. Texture personnalisée au **format direct** de couleur
  RVB (188, 188, 188), soit un albédo linéaire de **0,503 ± 0,002**. Lumière blanche
  `Light.Direction` = (1, −1, 1), intensité 1. Caméra en **(128 ; 120 ; −34)**, soit à **30 unités**
  du centre de la face mesurée, sur la normale de cette face, visant ce centre, FOV 70° : la hauteur
  visible vaut `2 × 30 × tan 35° = 42,0` unités, soit **25,7 pixels par unité de scène** ; à `N = 4`
  une rainure couvre `2 × 0,035 × 4 = 0,28` unité, soit **7,2 pixels**.
- **Cadrage large B** (configuration du critère de courbure de REQ-SCN-080, exécutable par un
  tiers) : harnais, mode 3D, capture **1920 × 1080**, `video.anti_alias` = 0, `video.res_scale` = 0
  (Native), échelle de scène 1, TV Box = None, skybox `Color` noire, `render.shadow` = **faux**,
  `spl.pixel_shadow` = 0, `spl.ao` = 0, `spl.reflections` = 0, `spl.voxel_grid` = **3** (donc
  `N` = 8), **aucune shape dans la scène**. Surface mesurée : le **backdrop** (REQ-SCN-048), avec
  `Backdrop.Enable` = vrai, `Backdrop.Color` = RVB (188, 188, 188) (albédo linéaire 0,503), tailles
  et décalages **par défaut** (256 × 240 × 1, décalages nuls) : sa face avant est le rectangle
  `z` = −0,5, `x` ∈ [0, 256], `y` ∈ [0, 240], de normale (0, 0, −1), et les arêtes de cellule y sont
  aux coordonnées de scène **multiples de 8** en `x` et en `y`. Lumière blanche
  `Light.Direction` = (1, −1, 1), intensité 1 (la face avant est éclairée partout, `N·L` allant de
  0,816 au bord gauche à 0 au bord droit à 90° de courbure : chaque rainure reste repérable, d'au
  moins 15 niveaux sRGB sous ses voisines). Caméra en **(128 ; 120 ; −110,5)**, soit **110 unités**
  devant la face avant, visant C = (128, 120, 0), FOV 70° : la hauteur visible sur la face vaut
  `2 × 110 × tan 35° = 154,0` unités (**7,01 pixels par unité**) et la largeur visible 273,9 unités,
  donc toute la largeur de la face (256 unités) est dans le cadre, avec 8,9 unités de marge de
  chaque côté ; à 90° de courbure, les bords de la face sont à 118,6 unités de l'axe pour une
  demi-largeur visible de 182,2 unités, donc toujours dans le cadre.
- Critère d'acceptation (auto) : gros plan G, face de normale (0, 0, −1), `render.shadow` = faux,
  `spl.pixel_shadow` = 0, `spl.voxel_grid` = 2 (donc `N = 4`). Points donnés dans le repère local de
  **la tile adjacente au centre de la face du côté `+x`, `+y`** (REQ-E3D-504 ; son centre est en
  (132, 132) dans le repère de scène). Rapport des **valeurs linéaires** (grille On / grille Off) :

  | Point local | Situation | Rapport attendu | sRGB attendu (On) | sRGB attendu (Off) |
  |---|---|---|---|---|
  | (0 ; 2) | centre d'une rainure, un seul axe | 0,62 ± 0,03 | 133 ± 3 | 166 ± 3 |
  | (2 ; 2) | centre de cellule | 1,00 ± 0,01 | 166 ± 3 | 166 ± 3 |
  | (0,4 ; 2) | arête claire, `d = 10 %` | 1,048 ± 0,02 | 169 ± 3 | 166 ± 3 |
  | (0 ; 0) | croisement de deux rainures | 0,384 ± 0,03 | 107 ± 3 | 166 ± 3 |

  En Classic, et en Enhanced avec `spl.voxel_grid` = 0, les quatre points valent 166 ± 3 : le
  critère est donc discriminant.
- Critère d'acceptation (auto) : gros plan G, `spl.voxel_grid` = 1, 2 puis 3 : l'écartement des
  lignes de rainure mesuré à l'écran vaut respectivement **2, 4 et 8 unités de scène** (±0,1), et
  dans les trois cas une ligne passe par le centre de chaque tile ; à `spl.voxel_grid` = 1 et 2,
  une ligne passe aussi par la jonction de deux tiles voisines.
- Critère d'acceptation (auto) : même scène avec `scale = (2, 1, 1)` sur l'objet : l'écartement des
  rainures mesuré à l'écran **double** sur l'axe x (la grille suit l'objet) ; avec
  `rot = (0, 0, 30)`, les rainures tournent de 30° avec la face.
- Critère de revue (SPEC-12) : « Super Mario Bros. » 1-1, `spl.voxel_grid` = 2, en vue rapprochée :
  les blocs et le sol montrent des rainures régulières qui donnent l'aspect de briques de jouet ;
  le ciel, le score et le boîtier TV éventuel sont inchangés.
- Notes : les constantes (3,5 % ; 16 % ; 0,62 ; 0,38 ; 10 % ; 20 % ; le facteur 4 du poids `w` ; les
  seuils 0,8 et 0,9 de l'interpolation d'alignement) sont mesurées sur 3dSen 2.0 ; **les critères
  font foi**. Au croisement de deux arêtes les facteurs se multiplient (0,62² = 0,384) : le coin de
  cellule est plus sombre que la rainure, c'est voulu. La grille n'est **lisible qu'en vue
  rapprochée** : au cadrage par défaut (FOV 70°, distance de réinitialisation de REQ-LUA-113, soit
  189,65 unités), une rainure de `N = 4` ne couvre que 1,1 pixel à 1080p et la grille se réduit à une
  texture fine ; c'est normal pour un effet de « briques de jouet ». Comme les arêtes sont aux
  multiples de `N` du repère local de chaque tile et que les centres de tiles voisines sont distants
  de 8 unités, les rainures se raccordent d'une tile à la suivante pour `N` = 2, 4 et 8. En MARCHING,
  la surface est faite de triangles plans à normale plate (REQ-E3D-522) : la formule s'applique par
  triangle, et `k` et `e` peuvent changer d'un triangle à l'autre.
- Note, **portée du critère de courbure** (REQ-SCN-080) : ce critère établit que la grille est
  **peinte sur la géométrie** — une grille évaluée sur la position **projetée**, ou en espace écran,
  donnerait des lignes droites et uniformément espacées dans les deux captures, et échouerait donc.
  Il ne distingue pas, en revanche, une évaluation **avant** d'une évaluation **après** l'enroulement
  de REQ-SCN-067 : cet enroulement est presque une isométrie le long de la surface (sur toute la
  largeur de la face, à 90°, l'écart cumulé entre la position matérielle et la position courbée
  n'atteint que 9,4 unités, soit un peu plus d'une cellule à `N` = 8, et il n'est observable que par
  le nombre de lignes de rainure, 33 contre 29, dont les plus extérieures sont sous le pixel). Cette
  partie de la clause d'ordre est donc **déclarée** plutôt que mesurée ; la partie qui porte sur la
  transformation d'objet, elle, est mesurée par les deux critères `scale` et `rot` ci-dessus.

**REQ-SCN-082** (P2) — **Ombres pixel** (`spl.pixel_shadow` = 1) : sur les faces des maillages de
**tiles opaques** seulement, l'ombre (REQ-SCN-030, REQ-SCN-031) DOIT être évaluée non pas au
fragment, mais au **centre du voxel** du fragment, obtenu ainsi :

1. prendre la position `p` du point dans le repère local du maillage de la tile (REQ-E3D-504,
   REQ-E3D-522), avant la transformation d'objet et avant la courbure, comme en REQ-SCN-081 ;
2. pour les **deux axes du plan de la face**, c'est-à-dire les deux axes de plus petit `|n_a|`
   (`n` = normale du maillage dans ce repère ; en cas d'égalité, l'axe de plus petit indice dans
   l'ordre `x`, `y`, `z`), ramener la coordonnée au centre du voxel qui la contient : la grille des
   voxels est celle de REQ-E3D-504, donc `p_x ← ⌊p_x⌋ + 0,5`, `p_y ← ⌊p_y⌋ + 0,5` et
   `p_z ← ⌊p_z + D/2⌋ + 0,5 − D/2` (sur `z`, les bords de voxel sont décalés de `D/2` ; pour `D`
   pair c'est aussi « partie entière + 0,5 ») ; le troisième axe est laissé tel quel, de sorte que
   le point reste sur la face. Sur une face GREEDY (normale `±X`, `±Y` ou `±Z`, REQ-E3D-505), cette
   règle sélectionne exactement les deux axes dont `|n_a|` = 0 ; sur une surface MARCHING, dont la
   normale est quelconque, elle sélectionne toujours deux axes, sans plaque où l'effet disparaîtrait ;
3. transformer ce point **comme un sommet** (déformation animée, puis échelle, rotation et position
   de l'objet, dans l'ordre de REQ-E3D-406, puis courbure), appliquer le biais de profondeur et le
   décalage selon la normale de REQ-SCN-031, et y lire la carte d'ombre.

L'ombre change donc par carrés d'un voxel, comme la grille de pixels du jeu. L'effet est sans
objet sur le backdrop, le HUD, le tunnel et les modèles de téléviseur (REQ-SCN-053, REQ-SCN-054),
le rectangle du mode 2D (AMD-046 §2) et les objets transparents (REQ-SCN-040, qui ne reçoivent pas
d'ombre).
- Critère d'acceptation (auto) : harnais, capture 1920 × 1080, `video.anti_alias` = 0,
  `spl.voxel_grid` = 0, `render.shadow` = vrai, albédo et couleurs du gros plan G. Sol : une shape
  de 4 × 4 tiles pleines (32 × 32 unités) avec `rot = (90, 0, 0)`, donc de normale de scène
  (0, 1, 0). Cube : une tile pleine de `sizeZ` = 1 (8 × 8 × 1 unités), empreinte verticale centrée
  sur le centre du sol, face basse à **29 unités** au-dessus du sol. Lumière blanche
  `Light.Direction` = (1, −3, 1), intensité 1 : l'ombre de la plaque est un carré d'environ 8 × 8 unités dont le
  centre est à (+29/3 ; +29/3) = (+9,667 ; +9,667) du centre du sol dans le plan du sol, et ses
  bords tombent donc à 5,667 et 13,667 unités du centre du sol, **au milieu d'un voxel**. Caméra à
  30 unités du centre de l'ombre, élévation 45°, azimut 0 (depuis −z), FOV 70°.
  - avec `spl.pixel_shadow` = 1 : pour chaque cellule de 1 voxel du sol entièrement visible, tous
    les pixels de la capture qui s'y projettent ont la **même** valeur à ±2 niveaux sRGB (sRGB 195
    ± 3 éclairé, 118 ± 3 ombré) ; la frontière entre cellules ombrées et cellules éclairées suit
    **exactement** des bords de cellule, aux coordonnées entières 6 et 14 unités du centre du sol ;
  - avec `spl.pixel_shadow` = 0 (et en Classic) : au moins une cellule de chaque bord de l'ombre
    contient à la fois des pixels ombrés et des pixels éclairés. **C'est cette dernière clause qui
    rend le critère discriminant** : les ombres de REQ-SCN-030 sont déjà dures, la netteté du bord
    ne distingue pas les deux styles.
- Critère de revue (SPEC-12) : « Super Mario Bros. » 1-1, `spl.pixel_shadow` = 1 : les ombres des
  blocs sur le sol et sur le décor sont découpées en carrés d'un pixel NES, alignés sur les pixels
  du jeu ; le score et le ciel sont inchangés.
- Notes : le point d'évaluation se déplace de 0,7 unité au plus le long de la face ; le biais de
  REQ-SCN-031 est appliqué **à ce point déplacé**, et non au fragment, afin que l'acné d'ombre
  reste évitée. L'alignement des cellules est exactement celui de REQ-SCN-081 (repère de
  REQ-E3D-504), avec `N` = 1 voxel.

#### 3.11bis.4 Occlusion ambiante

**REQ-SCN-083** (P3) — **Occlusion ambiante** (`spl.ao` ≠ 0 ; 1 MSVO : voir *Mode MSVO* ci-dessous).

*Intention.* Les coins, les creux et le pied des objets reçoivent moins de lumière ambiante.
L'effet est **doux**, sans contour net, ne dépend que de la géométrie visible, et ne noircit jamais
complètement une surface.

*Mise en œuvre.* L'implémentation DEVRAIT utiliser une technique publique et classique d'occlusion
ambiante d'écran par échantillonnage d'un **hémisphère orienté par la normale**, reconstruite à
partir de la **profondeur** de la scène, suivie d'un lissage qui respecte les discontinuités de
profondeur. Elle DEVRAIT suivre la technique décrite publiquement par le **tutoriel SSAO de John
Chapman (2013)** : noyau d'échantillons tiré dans l'hémisphère de la normale et **resserré près du
point** considéré, **rotation du noyau par pixel**, **test de portée** qui écarte les échantillons
trop éloignés en profondeur, puis **flou**. Cette référence **oriente sans être normative** : la loi
de pondération et d'atténuation de l'occultation, la répartition et la variation par pixel des
échantillons, le rayon et le seuil du lissage restent des **choix d'implémentation libres**, et ce
sont les paramètres et les invariants ci-dessous qui font foi. Le motif d'échantillonnage PEUT
varier par pixel, à condition qu'aucun motif fixe ne reste visible après lissage.

*Paramètres normatifs.*
- La passe s'applique **après le rendu des objets opaques et des reflets, et avant les objets
  transparents** (REQ-SCN-058 amendée, REQ-SCN-080).
- Elle multiplie la couleur de chaque pixel de la scène par un facteur `1 − O`, `O ∈ [0, 1]`,
  **en espace linéaire, avant l'encodage sRGB de REQ-SCN-028**.
- **Rayon** d'influence : `r = (8 + 6 × (ao.radius − 1)) × échelle de scène` unités de scène, soit
  8, 14, 20, 26 et 32 pour les réglages 1 à 5 (20 par défaut).
- **Force** : `O` est proportionnel à `ao.strength` sous un **plafond d'assombrissement**
  `O ≤ min(0,95 ; 0,3 × ao.strength)` en tout pixel, soit 0,3 / 0,6 / 0,9 / 0,95 / 0,95 pour les
  réglages 1 à 5 (0,9 par défaut). (v1.4 : mesuré sur 3dSen 2.0, l'occlusion par défaut assombrit
  au contact jusqu'à plus de 85 % en linéaire ; voir Q-097.) `O` DOIT être nul là où aucune géométrie occultante ne se
  trouve à moins de `2 r`.
- `ao.thickness` n'a aucun effet hors du mode MSVO (REQ-SCN-079).
- L'implémentation DEVRAIT prendre de l'ordre de **6, 10 et 16 échantillons** par pixel pour
  `ao.quality` Low, Medium et High, et calculer l'occlusion à la **moitié de chaque dimension de la
  cible de rendu courante** sauf en High. Ces nombres ne sont pas observables : ils ne sont donnés
  qu'en ordre de grandeur, et c'est l'aspect (ci-dessous) qui fait foi.
- **Portée** : l'effet NE DOIT PAS modifier la skybox, le HUD (REQ-SCN-029, REQ-SCN-044), le
  tunnel ni les modèles de téléviseur (REQ-SCN-053, qui les veut « non éclairés, sans ombre », et
  REQ-SCN-054), le rectangle du mode 2D (AMD-046 §2) ni les objets transparents (REQ-SCN-040).
  **Occultent** en revanche toutes les surfaces déjà dessinées, donc présentes dans la profondeur,
  au moment de la passe : tiles opaques, backdrop, tunnel, modèles de téléviseur et rectangle du
  mode 2D. N'occultent pas : les objets transparents et le HUD, dessinés après la passe, les tiles
  « ombre seule » (REQ-SCN-041), qui n'écrivent pas de profondeur, et la skybox, à distance infinie
  (REQ-SCN-080, 4ᵉ tiret).

*Mode MSVO* (`spl.ao` = 1). Le libellé est une valeur visible du réglage, qui doit se relire (§1).
Ce mode DEVRAIT (P3) être rendu par une occlusion ambiante d'écran **multi-échelle** : l'occultation
est estimée à plusieurs résolutions de la profondeur puis combinée, ce qui donne à la fois un
contact fin et un assombrissement large à coût modéré. L'implémentation DEVRAIT pour cela **porter
l'implémentation publique sous licence MIT** de l'effet d'occlusion ambiante multi-échelle du
**MiniEngine de Microsoft** (dépôt public `DirectX-Graphics-Samples`), **adaptée à OpenGL 3.3 en
passes de fragments** (OpenGL 3.3 n'a pas de nuanceurs de calcul). La licence MIT est compatible
avec la GPL-3.0 sous laquelle la réimplémentation est distribuée.
- **Conditions.** Le portage DOIT être placé sous `third_party/`, porter l'en-tête de licence MIT
  de son origine, et faire l'objet d'une entrée dans `third_party/README.md` (origine, licence,
  statut dans la salle blanche, règles d'usage), sur le modèle de l'entrée `3DNes_Engine` (§3
  point 11).
- **Repli (D).** Tant que ce portage n'est pas disponible, `spl.ao` = 1 DOIT être rendu exactement
  comme `spl.ao` = 2 (Depth) : repli déclaré **(D)**, le libellé `MSVO` restant exposé et relu.
- **Paramètres.** `ao.radius` et `ao.strength` gardent la signification et les valeurs normatives
  ci-dessus (rayon 8 / 14 / 20 / 26 / 32 ; plafond `O ≤ min(0,95 ; 0,3 × ao.strength)`). `ao.thickness` **agit**
  dans ce mode, et dans celui-là seulement : il règle l'**épaisseur supposée** des occulteurs
  connus seulement par leur profondeur. Qualitativement, à `ao.thickness` = 1 un échantillon plus
  proche que le point considéré est traité comme un **écran mince**, qui n'occulte que son voisinage
  immédiat en profondeur ; à 5, il est traité comme un **volume plein** s'étendant vers le fond, qui
  occulte donc davantage derrière lui. À géométrie fixe, l'assombrissement NE DOIT PAS décroître
  quand `ao.thickness` croît. `ao.quality` DEVRAIT faire varier le nombre d'échelles et
  d'échantillons ; ces nombres ne sont pas observables, comme ci-dessus.
- **Conformité.** Les invariants et les critères de revue de cette section valent **aussi** pour ce
  mode, à la seule exception de l'invariant d'identité entre `spl.ao` = 1 et `spl.ao` = 2, qui ne
  vaut que sous le repli.

*Gros plan A* (configuration commune aux invariants de §2.4, exécutable par un tiers) : harnais,
mode 3D, capture **1920 × 1080**, `video.anti_alias` = 0, `video.res_scale` = 0 (Native), échelle de
scène 1, courbure nulle, TV Box = None, skybox `Color` noire, `render.shadow` = **faux**, **tous les
autres effets de §2.3 à §2.5 à Off** (`spl.voxel_grid` = 0, `spl.pixel_shadow` = 0,
`spl.reflections` = 0), `spl.ao` = 2 et les quatre réglages `ao.*` à leur défaut (`ao.strength` = 3,
`ao.radius` = 3 donc `r` = 20, `ao.thickness` = 3, `ao.quality` = 1). Lumière blanche
`Light.Direction` = (1, −1, 1), intensité 1. Valeurs lues en sRGB 8 bits puis converties en linéaire.
- **Sol** : une shape de **12 × 12 tiles pleines** (96 × 96 unités), `D` = 8, avec
  `rot = (90, 0, 0)`, donc de normale de scène (0, 1, 0), centrée sur C ; texture personnalisée au
  format direct de couleur RVB (188, 188, 188), soit un albédo linéaire de 0,503 ± 0,002. Le sol
  s'étend ainsi à 48 unités du centre, soit plus de `2 r` = 40 unités.
- **Cube** : une tile pleine de `sizeZ` = 8 (8 × 8 × 8 unités), même texture, posée sur le sol, son
  empreinte verticale centrée sur le centre du sol.
- **Caméra** : à **80 unités** du centre du sol, dans la direction (−0,5 ; +0,5 ; −0,707) depuis ce
  centre (à gauche, au-dessus et devant : élévation 30°), visant ce centre, FOV 70° — vue de trois
  quarts du cube.
- **Variante A-h** : même configuration avec le réglage `Skybox` = 1 (Gradient) et un motif HUD
  visible au centre de l'image (REQ-SCN-044), pour l'invariant de portée.

*Invariants (auto).* Tous sont des **rapports de valeurs linéaires** entre deux captures du gros
plan A qui ne diffèrent que par le réglage indiqué.
- Gros plan A **sans le cube** (un plan seul, sans autre géométrie) : le rapport
  (occlusion On / Off) vaut 1,00 ± 0,02 en tout pixel du sol.
- Gros plan A : sur les pixels du sol au contact de la base du cube, le rapport est **strictement
  inférieur à 1** et **supérieur ou égal à 0,10**, à un niveau sRGB 8 bits près (plafond
  `O ≤ min(0,95 ; 0,3 × ao.strength)` = 0,9) ; en un
  pixel du sol situé à au moins `2 r` = 40 unités du cube et visible dans la capture (par exemple à
  40 unités du cube, du côté opposé à la caméra), il vaut 1,00 ± 0,02.
- `ao.strength` = 5 : le rapport est partout ≥ 0,05 ; `ao.strength` = 1 : partout ≥ 0,70, soit
  `1 − min(0,95 ; 0,3 × ao.strength)`, à un niveau sRGB 8 bits près (v1.5, Q-098). Le rapport est
  non croissant en `ao.strength` au pixel de contact, et strictement décroissant de 1 à 4.
- Sous le repli **(D)**, `spl.ao` = 1 et `spl.ao` = 2 donnent des captures identiques à ±2 niveaux
  sRGB. Avec le portage MSVO disponible : `spl.ao` = 1, changer `ao.thickness` de 1 à 5 modifie au
  moins un pixel de plus de 2 niveaux sRGB, et le rapport au pixel de contact ne **croît** pas.
- `spl.ao` = 0 : capture identique à une capture Classic de la même scène, à ±2 niveaux sRGB
  (REQ-SCN-080 ; les autres effets sont à Off dans le gros plan A).
- Variante A-h : les pixels de la skybox et du HUD sont identiques à ±1 niveau sRGB entre
  `spl.ao` = 0 et `spl.ao` = 2.

*Critères de revue* (catégorie **revue** de SPEC-12, captures archivées dans `verify/`).
- Gros plan A : l'assombrissement est **progressif**, sans contour net, sans bande ni motif de bruit
  fixe, et ne varie pas quand la caméra est immobile.
- « Super Mario Bros. » 1-1, occlusion Depth, réglages par défaut : le pied des buissons, des
  collines et des blocs est visiblement assombri sans devenir noir ; le ciel et le HUD sont
  inchangés. Même critère en mode MSVO quand le portage est disponible.
- Les trois valeurs de `ao.quality` donnent le même aspect général, Low pouvant être plus bruité ;
  aucune ne doit montrer de bande ni de motif fixe.

#### 3.11bis.5 Reflets du sol

**REQ-SCN-084** (P3) — **Reflets du sol** (`spl.reflections` = 1).

*Intention.* Le sol reflète la scène déjà rendue comme une surface polie ; le reflet s'estompe
avec la distance parcourue et au bord de l'écran, sans bord franc.

*Mise en œuvre.* L'implémentation DEVRAIT utiliser une technique publique et classique de **tracé
de rayon réfléchi dans l'espace écran** : **marche linéaire** le long de la projection du rayon,
puis **affinement par dichotomie** autour du premier franchissement de la profondeur — la famille
décrite par **McGuire et Mara, « Efficient GPU Screen-Space Ray Tracing », JCGT 3(4), 2014**. Cette
référence **oriente sans être normative** : le repère et la loi de progression de la marche, le
nombre et la forme des affinements, la forme et la largeur exactes des atténuations, et la forme du
noyau de flou restent des **choix d'implémentation libres**, et ce sont les paramètres et les
invariants ci-dessous qui font foi.

*Définition du sol.* Une **surface de sol** est une face dont la normale, exprimée dans le **repère
de scène avant courbure**, a une composante **+Y ≥ 0,9**, et qui appartient à une **instance**
(SPEC-04B §2) dont la **boîte englobante dans le plan `xz` du repère de scène, avant courbure**,
mesure au moins `6 × échelle de scène` unités **dans ses deux dimensions**. Un bloc isolé de
8 × 8 unités dans le plan `xz` (une shape d'une seule tile, `D` = 8, sans rotation ni mise à
l'échelle) réfléchit donc. En revanche la dimension en `z` est la **profondeur** de l'objet : la
même shape avec `D` = 3, ou avec `scale = (1 ; 1 ; 0,5)`, mesure 3 ou 4 unités en `z` et ne
réfléchit pas. Un sol de jeu, qui s'étend sur de nombreuses tiles, dépasse le seuil dans les deux
dimensions dès que sa profondeur atteint 6 unités.
*Caméra locale* (REQ-SCN-009, v1.4). Quand le profil active la caméra locale, le plan de l'image
NES est posé à plat et tient lieu de sol : la verticale de cette définition est alors la **normale
du plan image tournée vers la caméra**, `(0 ; 0 ; −1)` dans le repère de scène avant courbure, à
la place de `+Y`, **quelle que soit l'inclinaison de la caméra** ; l'emprise se mesure de même
dans le plan `xy`. En caméra ordinaire, la définition ci-dessus s'applique sans changement (le gros
plan R et ses invariants sont inchangés). Mesure qui fonde cette règle : sur « Punch-Out!! »,
profil à caméra locale inclinée de −60°, la référence fait réfléchir le tapis du ring (normale de
scène (0 ; 0 ; −1)), environ 10,5 % de la frame changeant à `spl.reflections` = 1.

*Paramètres normatifs*, avec `n(niveau) = 2 · niveau − 1` (niveaux 1 à 5 → `n` de 1 à 9) :
- **longueur maximale** du rayon : `40 × n(ssr.length) × échelle de scène` unités de scène, soit
  40, 120, 200, 280 et 360 pour les réglages 1 à 5 (200 par défaut) ;
- **intensité** du reflet (part de la couleur réfléchie mélangée à la couleur de la surface) :
  `0,045 × n(ssr.shine)`, soit 0,045 / 0,135 / 0,225 / 0,315 / 0,405 (0,225 par défaut ; v1.5, voir Q-099). Le mélange se
  fait **en espace linéaire, avant l'encodage sRGB de REQ-SCN-028** ;
- **flou** : le rayon de flou croît **linéairement** avec la distance parcourue par le rayon, de 0
  au point d'origine jusqu'à `0,003 × (10 − n(ssr.clarity))` de la **hauteur d'écran** à la
  longueur maximale, soit 0,027 / 0,021 / 0,015 / 0,009 / 0,003 pour les réglages 1 à 5 (0,015,
  soit 16 pixels à 1080p, par défaut) ;
- **atténuation** : l'intensité DOIT décroître continûment vers 0 quand le rayon approche sa
  longueur maximale et quand sa projection approche le bord de l'écran. La largeur et la forme de
  ces deux transitions sont libres, mais **aucun bord franc de reflet ne DOIT être visible** ;
- **qualité** : l'implémentation DEVRAIT prendre de l'ordre de 12, 24, 48 et 48 pas de marche et 2,
  3, 5 et 5 raffinements pour Low, Medium, High et Ultra, et tracer à la moitié de chaque dimension
  de la cible de rendu courante sauf en Ultra. Ces nombres ne sont pas observables (voir
  REQ-SCN-083) ;
- seule la **scène déjà rendue** peut être reflétée : rien de ce qui est hors champ ou caché ;
- **portée** : la skybox, le HUD, le tunnel, les modèles de téléviseur et le rectangle du mode 2D
  ne sont jamais des surfaces de sol et ne sont jamais modifiés par les reflets ; les objets
  transparents ne portent pas de reflets (REQ-SCN-080). Sont **reflétés** ceux de ces éléments qui
  sont déjà dessinés au moment de la passe — skybox, tunnel, modèles de téléviseur, rectangle du
  mode 2D et tiles opaques ; ne le sont pas les objets transparents et le HUD, dessinés après, ni
  les tiles « ombre seule » (REQ-SCN-041), qui ne sont pas dessinées (REQ-SCN-080, 4ᵉ tiret).

*Gros plan R* (configuration commune aux invariants de §2.5, exécutable par un tiers) : harnais,
mode 3D, capture **1920 × 1080**, `video.anti_alias` = 0, `video.res_scale` = 0 (Native), échelle de
scène 1, courbure nulle, TV Box = None, skybox `Color` noire, `render.shadow` = **faux**, **tous les
autres effets de §2.3 à §2.5 à Off** (`spl.voxel_grid` = 0, `spl.pixel_shadow` = 0, `spl.ao` = 0),
`spl.reflections` = 1 et les quatre réglages `ssr.*` à leur défaut (`ssr.shine` = 3, donc une
intensité de 0,225 ; `ssr.clarity` = 3 ; `ssr.length` = 3, donc 200 unités ; `ssr.quality` = 2).
Lumière blanche `Light.Direction` = (1, −1, 1), intensité 1. Valeurs lues en sRGB 8 bits puis
converties en linéaire.
- **Sol** : une shape de **8 × 8 tiles pleines** (64 × 64 unités), `D` = 8, avec `rot = (90, 0, 0)`,
  donc de normale de scène (0, 1, 0), centrée sur C : sa boîte englobante `xz` mesure 64 × 64
  unités, c'est donc une surface de sol. Texture personnalisée au format direct de couleur
  RVB **(124, 124, 124)**, soit un albédo linéaire de **0,202 ± 0,002** — un sol sombre, pour que
  l'invariant du canal rouge ait de la marge (voir Notes).
- **Cube rouge** : une tile pleine de `sizeZ` = 8 (8 × 8 × 8 unités), texture au format direct de
  couleur RVB (255, 0, 0), soit un albédo linéaire de (1 ; 0 ; 0), posée sur le sol, son empreinte
  verticale centrée sur le centre du sol.
- **Caméra** : à **40 unités** du centre du sol, dans la direction (−0,5 ; +0,5 ; −0,707) depuis ce
  centre, visant ce centre, FOV 70° : les faces du cube de normales (0, 0, −1) et (−1, 0, 0) sont
  visibles et éclairées (`N·L` = 0,577, REQ-SCN-025).
- **Variante R-m** (mur) : le sol est remplacé par la même shape **sans rotation**, dont la face
  avant a la normale de scène (0, 0, −1) ; le cube rouge est posé devant elle, à 1 unité.
- **Variante R-s** (étendue) : le sol est remplacé par une shape d'**une seule tile pleine**,
  `D` = 8, **sans rotation** (8 × 8 × 8 unités : sa face supérieure a la normale (0, 1, 0) et sa
  boîte `xz` mesure 8 × 8), centrée sur C ; le cube rouge est conservé, **à côté** du bloc : sa base
  au niveau de la face supérieure du bloc, sa face (−1, 0, 0) à 2 unités du bord `+x` du bloc, de
  sorte que le reflet de la face rouge atteigne la face supérieure du bloc. La même variante est
  reprise avec `scale = (1 ; 1 ; 0,5)` sur le bloc (4 unités en `z`).
- **Variante R-h** : gros plan R avec le réglage `Skybox` = 1 (Gradient) et un motif HUD visible au
  centre de l'image (REQ-SCN-044), pour l'invariant de portée.

*Invariants (auto).* Chaque invariant compare deux captures de la configuration indiquée qui ne
diffèrent que par le réglage cité.
- Gros plan R : sur les pixels du sol visibles au contact de la base du cube, le canal rouge
  **linéaire** augmente d'au moins **10 %** par rapport à la même capture avec
  `spl.reflections` = 0.
- Variante R-m : aucun pixel de la face avant du mur (normale (0, 0, −1)) n'est modifié (±1 niveau
  sRGB).
- Variante R-s : le bloc de 8 × 8 unités **est modifié** sur au moins un pixel de sa face
  supérieure ; le même bloc à `scale = (1 ; 1 ; 0,5)` (4 unités en `z`, donc moins de
  `6 × échelle de scène` dans une de ses deux dimensions) n'est modifié sur **aucun** pixel.
- Gros plan R : `ssr.shine` croissant de 1 à 5 fait croître de façon monotone l'écart entre les
  captures avec et sans reflets ; `ssr.length` croissant allonge la zone modifiée ; `ssr.clarity`
  croissant réduit la largeur de transition mesurée sur un bord de reflet.
- `spl.reflections` = 0 : capture identique à une capture Classic de la même scène, à ±2 niveaux
  sRGB (REQ-SCN-080 ; les autres effets sont à Off dans le gros plan R).
- Variante R-h : les pixels de la skybox et du HUD sont identiques à ±1 niveau sRGB entre
  `spl.reflections` = 0 et 1.

*Critères de revue* (catégorie **revue** de SPEC-12, captures archivées dans `verify/`).
- Gros plan R : le reflet est net près du contact, se brouille et s'efface avec la distance, et n'a
  pas de bord franc, ni au bout du rayon ni au bord de l'écran.
- « Super Mario Bros. » 1-1, reflets à On, réglages par défaut : le sol de briques montre un reflet
  reconnaissable des buissons et de Mario, plus marqué au contact, sans que le sol perde sa
  texture ; le ciel et le HUD sont inchangés.
- Les quatre valeurs de `ssr.quality` donnent le même aspect général ; en Ultra, le tracé ne manque
  aucun objet plus large que 2 pixels d'écran.

- Notes (§2.4 et §2.5) : les constantes (8 + 6 (`ao.radius` − 1), le plafond de 50 %, 40, 0,09,
  0,003, le seuil +Y ≥ 0,9 et les 6 unités de la boîte englobante) sont mesurées sur 3dSen 2.0 ;
  **les critères font foi**. Le gain `0,78 × ao.strength` et le plafond « au plus de 40 % » de la
  v1.0 sont retirés : ils n'avaient pas de solution commune (55-10). Les plafonds et invariants
  ci-dessus les remplacent et sont simultanément satisfaisables, parce qu'ils ne contraignent plus
  qu'une **borne** et une **monotonie**, pas une valeur calculée par une loi non spécifiée.
- Note, **marge de l'invariant du canal rouge** : le mélange valant
  `(1 − i) · surface + i · reflet` avec `i` = 0,225, la condition « +10 % » s'écrit
  `R_reflet ≥ 1,444 · R_sol`. Avec les albédos du gros plan R et le modèle de REQ-SCN-028, le sol
  donne `R_sol = 0,202 × (0,1812 + 0,577) = 0,153` et la face rouge réfléchie
  `R_reflet = 1,000 × (0,1812 + 0,577) = 0,759`, soit un facteur **4,9**. Il suffit donc que la part
  rouge effectivement rapportée au contact dépasse **30 %** de sa valeur pleine : le seuil de 10 %
  reste franchi malgré l'atténuation et le flou, quelle que soit leur forme. C'est pour cette marge
  que le sol du gros plan R est sombre (albédo 0,202) et le cube rouge pur.


### 3.12 HUD

**REQ-SCN-044** (P1) [ROM] — Les motifs HUD DOIVENT être rendus avec le modèle d'éclairage de REQ-SCN-029, en mélange alpha comme en REQ-SCN-040 (α = alpha du texel × alpha du motif, avec passage de profondeur préalable), après les objets transparents, sans découpe par le rectangle visible (REQ-SCN-036 ne s'applique pas au HUD).
- Critère d'acceptation : en harnais, un motif HUD placé hors du rectangle visible (par exemple en y = 245) reste entièrement dessiné après les objets transparents ; [ROM] le score de Super Mario Bros reste lisible et entier même quand le rectangle visible est réduit.

**REQ-SCN-045** (P1) — Les motifs HUD DOIVENT être placés dans un plan attaché à la caméra : orientation identique à la caméra, origine à E + d_HUD · f, avec d_HUD = 60 / atan(FOV_rad / 2) (109,42 à 70°), et facteur d'échelle 0,5. Un élément HUD situé en p dans le plan NES apparaît au décalage (p − C) × 0,5 selon les axes droite, haut et avant de la caméra. Le HUD reste fixe à l'écran quand la caméra bouge, mais il est soumis au test de profondeur : les objets de la scène plus proches que d_HUD le masquent.
- Critère d'acceptation : à FOV 70°, le cadre de référence 256 × 240 projeté par le HUD occupe en hauteur environ 78 % de la hauteur de l'image (60 / (109,42 × tan 35°)), centré ; un élément HUD placé au centre de l'image NES (128, 128) apparaît 4 unités (8 × 0,5) au-dessus du centre de l'écran ; en zoomant à moins de 110 unités, le décor passe devant le HUD.
- Notes : C est le centre du cadre de référence (REQ-E3D-410), et non le centre de l'image NES ; ce décalage est celui de l'original.

### 3.13 Couleur de fond et backdrop

**REQ-SCN-046** (P1) [ROM] — Quand aucune skybox n'est affichée (réglage Skybox ∈ {Default, Color}, REQ-SCN-048), l'image DOIT être effacée avec la couleur de fond suivante :

| Situation | Couleur de fond |
|---|---|
| Skybox = Default, jeu non en cours | noir |
| Skybox = Default, jeu en cours, réglage de profil `Setting.BgColor` vrai | couleur NES universelle de fond de la frame courante (entrée 0 de la palette), mise à jour à chaque frame |
| Skybox = Default, jeu en cours, `Setting.BgColor` faux | `Setting.CustomBgColor` |
| Skybox = Color | `Setting.CustomBgColor` (que le jeu soit en cours ou non) |

`Setting.BgColor` DOIT être forcé à vrai à chaque chargement de profil ; Lua PEUT le modifier ensuite. `Setting.CustomBgColor` est un réglage utilisateur (défaut noir, choisi par une palette de couleurs dans l'UI quand Skybox = Color), que Lua PEUT modifier pendant la partie.
- Critère d'acceptation : en harnais, frame synthétique d'entrée 0 de palette 0x22, Skybox Default, jeu en cours : le fond de la capture est sRGB (104, 136, 252), sans éclairage ; entrée 0 = 0x0F → noir ; Skybox = Color avec `CustomBgColor` rouge → rouge. [ROM] Super Mario Bros 1-1 : fond bleu 0x22 ; niveau souterrain : noir.

**REQ-SCN-047** (P1) — Quand le jeu n'est pas en cours (menu, pause), la scène 3D (tiles, HUD, backdrop, boîtier TV) NE DOIT PAS être affichée : seul le fond (REQ-SCN-046) ou la skybox reste visible derrière l'UI (SPEC-10).
- Critère d'acceptation : ouvrir le menu de pause en Skybox Default montre l'UI sur fond noir.

**REQ-SCN-048** (P1) — Le **backdrop** est un pavé optionnel placé derrière (ou dans) la scène, piloté uniquement par Lua :

| Propriété Lua | Effet | Défaut |
|---|---|---|
| `Backdrop.Enable` | affiché ou non | faux |
| `Backdrop.Color` | couleur (sRGB) | noir |
| `Backdrop.SizeX`, `SizeY`, `SizeZ` | dimensions du pavé | 256, 240, 1 |
| `Backdrop.OfsX`, `OfsY`, `OfsZ` | position du centre du pavé par rapport à C | 0, 0, 0 |
| `Backdrop.RecvShadow` | reçoit les ombres | vrai |

Le pavé est un parallélépipède rectangle aligné sur les axes, centré en C + (OfsX, OfsY, OfsZ), de côtés (SizeX, SizeY, SizeZ). Il DOIT être éclairé selon REQ-SCN-075 (albédo = `Backdrop.Color`, AMD-049 §3 point 1), ne jamais projeter d'ombre, et recevoir les ombres si `RecvShadow` est vrai et que les ombres sont activées. Il n'est pas découpé par le rectangle visible. Toutes ces propriétés DOIVENT revenir à leurs valeurs par défaut à la fin de chaque partie.
- Critère d'acceptation (amendé par AMD-049 §3 point 2) : `Backdrop.Enable = true; Backdrop.Color = Color(0,0,0); Backdrop.OfsZ = 26` fait apparaître derrière la scène un panneau **presque noir** (sRGB ≈ 21 hors ombre, ≈ 5 dans l'ombre, avec la lumière par défaut et l'intensité 1, REQ-SCN-075) de 256 × 240 dont la face avant est à z = 25,5, sur lequel se dessinent les ombres des couches avant.
- Notes : valeurs relevées dans les scripts embarqués : OfsZ de 0 à 61, SizeX 240 à 384, SizeY 120 à 256, SizeZ 1 à 120. Par défaut le pavé couvre le cadre de référence (y ∈ [0, 240]), soit 8 unités plus bas que l'image NES (y ∈ [8, 248]) : comportement de l'original (REQ-E3D-410).

**REQ-SCN-075** (P2, AMD-049) — **Éclairage du backdrop** (remplace, dans REQ-SCN-048, « éclairé de façon diffuse (REQ-SCN-028, albédo = `Backdrop.Color`) ») : le backdrop DOIT être éclairé ainsi, **en espace linéaire**, pour chaque point de normale unitaire N (normale de la face du pavé, transportée par la courbure de REQ-SCN-068 quand elle est active), avec V le vecteur unitaire du point vers l'œil ; L = −`Light.Direction` normalisé ; H = (L + V) normalisé, et H = N si L + V = 0 ; c l'albédo (`Backdrop.Color` converti en linéaire) ; E = lumière_lin de REQ-SCN-028 (couleur de la lumière multipliée par l'intensité, convertie en linéaire) ; s le facteur d'ombre de REQ-SCN-031 ; Amb l'ambiante de REQ-SCN-027 (0,1812 en linéaire) :

- produits : nl = max(N·L, 0), nv = |N·V|, nh = max(N·H, 0), lh = max(L·H, 0) ;
- constantes : rugosité perceptuelle ρ = 0,99, α = ρ² = 0,9801 ;
- terme diffus : F90 = 0,5 + 2 · lh² · ρ ; Fd = (1 + (F90 − 1)(1 − nl)⁵) · (1 + (F90 − 1)(1 − nv)⁵) ;
- terme spéculaire : Vis = 0,5 / (nl · (nv · (1 − α) + α) + nv · (nl · (1 − α) + α) + 10⁻⁵) ; D = α² / (π · (nh² · (α² − 1) + 1)²) ; Spec = Vis · D · π · nl ; F = 0,04 + 0,96 · (1 − lh)⁵ ;
- couleur linéaire : `c · 0,96 · (Amb + s · E · Fd · nl) + s · E · Spec · F`, puis encodage identique à REQ-SCN-028 (sRGB à l'affichage, saturé à 1, sans tone mapping) ; le backdrop est **opaque**.

Il n'y a pas de reflet de l'environnement (skybox) : dans l'ombre, la part spéculaire est réduite dans le même rapport s que la part diffuse directe — le spéculaire est de la lumière directe au sens de REQ-SCN-031. Le reste de REQ-SCN-048 (pavé, ombres reçues, pas d'ombre projetée, pas de découpe, propriétés Lua) est inchangé. Clause de priorité (AMD-049, T-1) : si cette exigence n'est pas retenue, l'éclairage diffus d'origine de REQ-SCN-048 (P1) reste valable.
- Critère d'acceptation (auto, valeurs linéaires avant encodage, ±0,001) : N = (0, 0, −1), œil sur l'axe (V = (0, 0, −1)), `Light.Direction` = (0, 0, 1), lumière blanche d'intensité 1, hors ombre : nl = nv = nh = lh = 1 ; Fd = 1 ; Vis = 0,25 ; D = 0,3314 ; Spec = 0,2603 ; F = 0,04 ; pour un albédo linéaire c, la couleur vaut c · 0,96 · 1,1812 + 0,01041 par canal : avec c = 0,2, 0,2372 ; dans l'ombre (s = 0,2) : c · 0,96 · (0,1812 + 0,2) + 0,2 · 0,01041, soit 0,0753.
- Critère d'acceptation (auto, sRGB 8 bits ±2) : même configuration, albédo nul : 0,01041 en linéaire, soit sRGB (26, 26, 26), hors ombre ; 0,00208, soit sRGB (7, 7, 7), dans l'ombre. **Un albédo nul n'est donc plus rendu noir**, ce qui rend enfin observables les ombres que le critère de REQ-SCN-048 demande d'y voir.
- Critère d'acceptation [ROM] : « Punch-Out!! », scène de combat (`Backdrop.Enable = true`, `Backdrop.Color = Palette.FrameColor(0)` = NES `0x0078FC`, `Light.Color = Color(1, 0.8, 0.8)`, `Setting.LocalCam = true`), lumière du profil `Light.Direction = (1, 1, 2)`, intensité 1, ombres actives, hors ombre : la **face avant** du pavé (N = (0, 0, −1)), mesurée en sRGB, vaut (26 ; 100 ; 209) ±3 pour toute plongée de caméra de 0° à 45°.
- Notes : le modèle est celui du programme d'origine depuis la version 1.x (réflexion non métallique presque mate). Par rapport à l'éclairage diffus, la part diffuse perd 4 % et un reflet spéculaire faible, **de la couleur de la lumière et non de l'albédo**, s'y ajoute ; l'écart dominant n'est pas l'assombrissement mais l'apparition de la teinte de la lumière là où l'albédo n'en a aucune (R2, point 49-7 : sur la face avant de Punch-Out!! le rouge passe de 0 à 26 et le bleu perd 3 niveaux). Fd n'est pas borné à 1 : F90 ∈ [0,5 ; 2,48] donc Fd ∈ [0,25 ; 6,15] en incidence rasante. En style Enhanced, la grille de voxels de REQ-SCN-081 s'applique aux faces du pavé, en modulant l'albédo `c` avant le calcul ci-dessus (le terme spéculaire n'est pas assombri par une rainure) — AMD-055 §3 point 5, qui remplace la phrase « La grille de voxels du style « Enhanced » de 3dSen 2.0 n'est pas reprise. » ; à `spl.voxel_grid` = 0, qui est le défaut, cette exigence et ses trois critères ne sont pas affectés.

### 3.14 Skyboxes

**REQ-SCN-049** (P2) — Le réglage utilisateur **Skybox** DOIT offrir quatre valeurs, stockées sous forme d'entier : −1 « Default » (défaut), 0 « Color », 1 « Gradient », 2 « Fantasy ». Gradient et Fantasy affichent une skybox à distance infinie, centrée sur la caméra, immobile par rapport au repère scène, non éclairée, dessinée derrière toute la scène. Le choix de skybox est un réglage utilisateur, jamais pris dans le profil.
- Critère d'acceptation : en Gradient, orbiter la caméra fait défiler le ciel de façon cohérente avec la rotation ; la scène se dessine devant.

**REQ-SCN-050** (P2) [ROM] — La skybox **Gradient** DOIT afficher, pour une direction de vue unitaire d de composante verticale y, la couleur g(d) suivante (couleurs converties en linéaire, puis ré-encodées en sRGB à l'affichage) : avec M = couleur NES universelle de fond de la frame courante (sRGB), Hc = 1,4 × M (non saturée avant conversion) et Bc = 0,6 × M, c = M + (Hc − M) × max(y, 0)^2,5 puis g = c + (Bc − c) × max(−y, 0)^2,5. Des **étoiles** s'y ajoutent quand le niveau de gris de M (0,299 R + 0,587 G + 0,114 B sur les valeurs sRGB 0–1) est inférieur ou égal à 0,4 : avec s = canal rouge de la cubemap `Stars_LP_01` dans la direction d × canal bleu de la même cubemap dans la direction d tournée autour de Y d'un angle de 3° par seconde de temps réel, et k = s × min(|0,2 × y|, 1), la couleur devient g × (1 − k) + k × E, où E est un blanc légèrement grisé (sRGB 0,5 × 2 par canal, soit ≈ 0,98 en linéaire). Le soleil, la lune et les nuages du paquet de skybox d'origine sont désactivés.
- Critère d'acceptation : en harnais, frame synthétique de couleur de fond M = 0x22 : direction horizontale → sRGB (104, 136, 252) ±3 ; zénith → srgb(lin(1,4 × M)) composante par composante (non saturé avant conversion, puis saturé à l'affichage) ; nadir → srgb(lin(0,6 × M)) ; pas d'étoiles ; M = 0x0F (noir) → étoiles présentes là où les cubemaps `Stars_LP_01` le prévoient. [ROM] Super Mario Bros 1-1 (M = 0x22) : horizon sRGB (104, 136, 252), zénith plus clair et saturé, nadir assombri à environ 60 % ; pas d'étoiles. Dans un niveau souterrain (M noir), le ciel est noir avec de rares étoiles scintillant plus nettement vers le zénith et le nadir.
- Notes : les cubemaps sont des données linéaires (non sRGB).

**REQ-SCN-051** (P2) [R2] — La skybox **Fantasy** DOIT afficher un ciel de nébuleuse à partir des cubemaps `Starfield`, `NebulaDensity` et `NebulaDiffusion`, recolorées à chaque frame d'après la palette de fond de la frame. Notations : d direction de vue ; F0 = couleur NES universelle de fond, F1, F2, F3 = couleurs 1 à 3 de la palette de fond 0 de la frame (sRGB) ; γ = niveau de gris de F0 (formule de REQ-SCN-050) ; toutes les teintes sont converties en linéaire, leurs alphas non ; « ×2 » signifie multiplier la teinte linéarisée par 4,595 (doublement en sRGB) ; lissé(u) = 3u² − 2u³ appliqué à u saturé dans [0, 1] ; dens(x) = canal rouge de `NebulaDensity` dans la direction x ; Δ = canaux de `NebulaDiffusion` dans la direction d ; seuils bas 0,4 et haut 0,8, u(x) = (dens(x) − 0,4) / 0,4.
1. Fond : F0, mélangé vers la teinte d'étoiles ×2 avec le poids (canal rouge de `Starfield` remappé de [bmin, bmax] vers [0, 1]) × alpha de la teinte d'étoiles. Si γ > 0,4 : teinte d'étoiles transparente, bmin = bmax = 1 (pas d'étoiles) ; sinon teinte = F1 / 2, alpha = (1 − γ), bmin = 0, bmax = 1.
2. Halo ambiant : mélange vers F0 ×2 × Δ.vert, avec le poids 2 × Δ.vert × lissé(1 − u(d)) (fort hors des zones denses).
3. Socle : mélange vers moy × (F2 + F3) / 2 ×2, où moy = (Δ.vert + Δ.bleu) / 2, avec le poids 2 × moy × (1 − γ) / 4 × lissé(u(d)) (dans les zones denses).
4. Deux couches de « vaguelettes » : la première échantillonne `NebulaDiffusion` (canal rouge ρ1) et la densité dans la direction d + 0,75 × (0,2 Δ.bleu, −0,1 Δ.bleu, 1 Δ.bleu), et mélange vers ρ1 × F2 ×2 avec le poids ρ1 × 2 × (5/255) × lissé(u) ; la seconde fait de même dans la direction d + (1, 0,1, 0,2) × Δ.vert avec la teinte F3.
5. Exposition 1. Pas de rotation de la densité.
- Critère d'acceptation (round 1, harnais, cubemaps de test uniformes injectées à la place des cubemaps de `assets-neutral/`) : (a) toutes les cubemaps nulles → la couleur affichée est F0 dans toutes les directions ; (b) `NebulaDiffusion` = (0 ; 0,5 ; 0), `NebulaDensity` et `Starfield` nulles, F0 = gris sRGB 0,2 → couleur srgb(2,2975 × lin(F0)) ≈ sRGB 0,306 (±2/255) dans toutes les directions (seul le halo ambiant de l'étape 2 contribue, avec le poids 1) ; (c) F0 clair (γ > 0,4) → aucune contribution de `Starfield`. [R2] Comparaison avec l'original sur 3 jeux de fonds différents (sombre, moyen, clair) : mêmes masses de nébuleuse aux mêmes directions, teintes dominantes identiques, étoiles présentes seulement sur fond sombre.

**REQ-SCN-052** (P3) — Les skyboxes et le fond DOIVENT suivre la couleur de fond de la frame **affichée** (et non d'une frame émulée en avance), afin que fond, skybox et boîtier TV changent de couleur en même temps que les tiles.
- Critère d'acceptation : lors d'un flash de fond (changement de l'entrée 0 de la palette sur une seule frame), fond et skybox changent sur la même image que les tiles.

### 3.15 Boîtier TV décoratif

**REQ-SCN-053** (P2) — Le réglage utilisateur **TV Box** DOIT offrir 0 « None » (défaut), 1 « TV 1 », 2 « TV 2 », 3 « TV 3 ». Quand il est non nul et que le jeu est en cours, la scène DOIT être entourée d'un **tunnel** et du modèle de téléviseur choisi. Le tunnel est formé de cinq pavés de couleur unie égale à la couleur de fond de REQ-SCN-046, non éclairés et sans ombre, recalculés à chaque frame d'après le clipping (coordonnées scène) :

| Pavé | x | y | z |
|---|---|---|---|
| gauche | [−8, c0] | [c1 − 4, 244 − c3] | [−16, 112] |
| droite | [256 − c2, 264] | [c1 − 4, 244 − c3] | [−16, 112] |
| bas | [−8, 264] | [−4, c1] | [−16, 112] |
| haut | [−8, 264] | [240 − c3, 244] | [−16, 112] |
| fond | [0, 256] | [0, 240] | [112, 120] |

Les coordonnées sont celles du cadre de référence de REQ-E3D-410 (valeurs de l'original) : les parois intérieures du tunnel coïncident donc avec les bords du rectangle visible R, et tout ce qui se trouve au-delà de z = 112 ou hors du tunnel est masqué. Le fond du tunnel couvre le cadre de référence (y ∈ [0, 240]) et non l'image NES (y ∈ [8, 248]).
- Critère d'acceptation : TV Box = 1, Skybox Default : vue de face, la scène apparaît dans une ouverture aux bords nets égale au rectangle visible ; en orbite, les côtés du tunnel masquent les objets qui dépassent latéralement.

**REQ-SCN-054** (P2) — Les modèles de téléviseur DOIVENT être placés ainsi. Un repère « TV » a son origine en C + ((c0 − c2) / 256, (c1 − c3) / 240, −16) et des facteurs d'échelle (256 − c0 − c2) / 256 en x, (240 − c1 − c3) / 240 en y et 1 en z. Dans ce repère, chaque modèle est tourné de 180° autour de Y puis placé ainsi :

| Réglage | Mesh (`assets-neutral/meshes/`) | Position | Échelle (x, y, z) | Matériau |
|---|---|---|---|---|
| TV 1 | `Object003.obj` | (32,7 ; −136 ; 104) | (320 ; 340 ; 320) | diffus `Tv_3_4_diff` + carte de normales `Tv_3_4_normal`, peu brillant |
| TV 2 | `Object004.obj` | (−0,5 ; −139 ; 124) | (9,35 ; 9,4 ; 13) | diffus `Tv_1_2_diff`, normales `Tv_1_2_normal`, reflet spéculaire modulé par `Tv_1_2_gloss` et léger reflet d'environnement (`Cubemap`, teinte 0,5) |
| TV 3 | `2000s_CRT_TV_LOD0.obj` | (0 ; −156 ; 110) | (437 ; 540 ; 700) | albédo = `2000s_CRT_TV_black_AO` × gris 0,21, normales `…_Normal`, rugosité `…_Roughness`, émission `…_Emissive` × blanc |

Pour TV 3, les meshes `Button_play_LOD0`, `Button_rec_LOD0`, `Button_vol_LOD0`, `Buttons_channel_LOD0` (position locale (−0,042 ; 0,045 ; 0,172)) et `Button_power_LOD0` (position (0 ; 0,045 ; 0,172)), à l'échelle 0,732, sont attachés au modèle dans son repère d'origine. Les fichiers OBJ ayant l'axe X inversé, un sommet OBJ (xo, yo, zo) d'un modèle est placé, après la rotation de 180°, en (sx·xo, sy·yo, −sz·zo) + position, dans le repère TV. Les modèles sont éclairés par la lumière de la scène (REQ-SCN-028, avec spécularité selon leur matériau) et ne reçoivent pas d'ombre.
- Critère d'acceptation : avec chacun des trois réglages, l'écran du téléviseur encadre le rectangle visible, vue de face, sans recouvrir l'image NES.
- Notes : les textures citées (`Tv_3_4_diff`, `Tv_3_4_normal`, `Tv_1_2_diff`, `Tv_1_2_normal`, `Tv_1_2_gloss`, `2000s_CRT_TV_black_*`) sont présentes dans `assets-neutral/textures/`, la cubemap `Cubemap` dans `assets-neutral/cubemaps/`. Le facteur (c0 − c2) / 256 de l'origine du repère TV (et non / 2) est celui de l'original.

### 3.15bis Courbure de l'espace (AMD-043)

La **courbure** est un effet purement géométrique, appliqué au rendu. Elle ne touche ni les profils ni la conversion 2D → 3D, et elle est distincte de la **déformation animée** des motifs (REQ-SCN-043).

**REQ-SCN-066** (P2) — **Réglages de courbure** (portée U, persistés, SPEC-09 §4.1.4) : Courbure gauche-droite (`scene.bend_lr`, id Lua `spl.bend_zx`), Courbure haut-bas (`scene.bend_tb`, `spl.bend_zy`), Arc horizontal (`scene.arc_h`, `spl.arc_yx`), tous trois de −180° à 180° par pas de 30°, défaut 0 ; et Centre de l'arc (`scene.arc_center`, `spl.arc_center`), 0 = centre de l'écran, 1 = bord bas du cadre de référence (y = 0, huit unités sous le bas de l'image NES), défaut 0. Ils s'appliquent à chaud, dès la frame rendue suivante, quel que soit `Setting.VisualStyle`, et se lisent et s'écrivent aussi par `Setting:Get/SetVisualStyleParameter(1, id, …)` (REQ-LUA-114) avec la normalisation d'AMD-040 §3. Ils DEVRAIENT être présentés dans un groupe « Géométrie », marqué avancé.
- Critère d'acceptation (auto) : écrire 45 dans Courbure gauche-droite donne 60 ; la valeur survit à un redémarrage.

**REQ-SCN-067** (P2) — **Transformation des points** : avec s l'échelle de scène (1 sur desktop, REQ-SCN-002) et C = (128, 120, 0) le centre du cadre de référence (REQ-E3D-410), chaque point P = (x, y, z) de la scène courbée (REQ-SCN-068) DOIT être rendu à la position B(P) obtenue ainsi :

1. Pour chacun des trois réglages d'angle θ (en radians) et sa longueur de référence L (256 pour Bend gauche-droite et pour Arc horizontal, 240 pour Bend haut-bas) : courbure k = θ / (L · s) ; avec φ = θ / 4 : facteur de recadrage f = φ / sin φ et décalage h = 2 sin²(φ / 2) / k.
2. L'**enroulement** E(k, f, h) d'un couple (u, w) — u le long de l'axe courbé, w la profondeur — sur un cylindre de rayon 1/k est : w′ = 0,95 / k si w · k > 0,95, sinon w′ = w ; a = u · k ; u ← f · (1 − w′ k) · sin(a) / k ; w ← f · ((1 − cos a) / k + w′ cos a − h).
3. Partant de (u, v, w) = P − C, appliquer dans cet ordre : Bend gauche-droite, E sur (u, w) ; Bend haut-bas, E sur (v, w) ; Arc horizontal, avec le pivot p = 0 (centre de l'écran) ou p = −120 · s (bord bas du cadre de référence), E sur (u, p − v), puis v ← p − (la seconde composante obtenue).
4. B(P) = (u, v, w) + C.

B est continue en θ et vaut l'identité en θ = 0 (limites k → 0, f → 1, h → 0). Une implémentation PEUT traiter comme nul tout |θ| inférieur à un seuil de son choix ne dépassant pas 10⁻⁴ rad. L'axe z est la profondeur de la scène : un angle positif éloigne les bords (bombé), un angle négatif les rapproche (enveloppant).
- Critère d'acceptation (auto) : les treize vecteurs du tableau d'AMD-043 §3, à ±0,05 unité.

**REQ-SCN-068** (P2) [ROM] — **Portée** : la courbure DOIT s'appliquer à la géométrie de la scène de jeu — tiles et sprites convertis (opaques et transparents) et fond (backdrop) — **après** la déformation animée (REQ-SCN-043) et après la position, l'échelle et la rotation propres à chaque objet, donc sur la position finale dans la scène ; elle DOIT s'appliquer aussi à la passe d'ombres, pour que les ombres suivent la surface courbée ; elle NE DOIT PAS s'appliquer au HUD (plan attaché à la caméra, REQ-SCN-045), à la skybox ni au boîtier TV, qui restent plats ; les surfaces planes assez grandes pour se courber visiblement DEVRAIENT être subdivisées, le fond en au moins 32 × 32 cellules par face. **Ordre avec les calculs en coordonnées de scène** : la découpe par le rectangle visible (REQ-SCN-036), le point visé P\* de la caméra (REQ-SCN-008) et le segment d'occultation du FPV (REQ-SCN-022) DOIVENT être évalués sur les positions **avant** courbure ; la courbure n'intervient qu'au placement final des sommets. Une tile gardée par la découpe reste donc visible même si son image courbée sort du rectangle R. **REQ-SCN-028 amendée** : la normale N utilisée pour l'ombrage DEVRAIT être celle du maillage transportée par la courbure (la parenthèse « inchangée par la déformation de REQ-SCN-043 » reste vraie pour la déformation animée).
- Critère d'acceptation (auto) : harnais, Courbure gauche-droite à 90° ; les sommets exportés d'une tile située en (256, 120, 0) sont à B(P) près ; le HUD exporté est inchangé par rapport à une courbure nulle ; avec Arc horizontal = 60° (bord) et les marges par défaut, la tile du coin P = (0, 8, 0) est rendue (B(P) hors de R, mais P dans R).

### 3.16 Mode 2D et réglage « TV Mode »

**REQ-SCN-055** (P1) — Le « mode 2D » NE DOIT PAS être un chemin de rendu séparé : c'est le profil embarqué spécial intitulé « 2D Mode » (fichier `profiles/empty.bytes`, SPEC-03), sans motif, rendu par le même pipeline 3D, avec la même caméra et les mêmes réglages de scène. L'aspect obtenu (tiles converties avec la géométrie par défaut) est défini par SPEC-04.
- Critère d'acceptation : lancer une ROM en « 2D Mode » montre l'image du jeu dans la scène, orbitable avec les commandes de caméra.
- Notes : le réglage « TV Mode » (NTSC, PAL, DENDY) choisit la région émulée (SPEC-02/07) et n'a aucun effet sur le rendu.

Le **rectangle du mode 2D** est la surface plane qui porte l'image en pixels du cœur en mode 2D (AMD-009, réponse à Q-022) : le rectangle z = 0, x ∈ [0, 256], y ∈ [8, 248] du repère de REQ-E3D-410, texturé par l'image NES 256 × 240. Il ne projette pas d'ombre et n'en reçoit pas, et il est éclairé comme une tile (REQ-SCN-028, albédo = couleur du pixel).

**REQ-SCN-076** (P2, AMD-050) — **Transparence du fond universel en mode 2D** : dans l'image du rectangle du mode 2D, un pixel DOIT être transparent (non dessiné) quand le bit correspondant du masque d'opacité du cœur (SPEC-02 REQ-CORE-049) vaut 0, et dessiné quand il vaut 1. Un pixel opaque de même couleur que le fond universel reste donc dessiné. La transparence vaut que la courbure (REQ-SCN-074) soit active ou non ; le rectangle ne projette ni ne reçoit d'ombre. Le pixel transparent DOIT être obtenu par **rejet du fragment**, dans le passage des objets opaques de REQ-SCN-058 : le rectangle n'entre pas dans la classe transparente de REQ-SCN-040 (ni tri, ni passage de profondeur séparé, ni mélange alpha), et le résultat NE DOIT PAS dépendre de `video.anti_alias`.
- Critère d'acceptation (auto) : harnais, mode 2D, mise en condition `scene.use_nes_bg_color` = faux et `Setting.CustomBgColor` = magenta — les réglages par défaut ne permettent pas d'observer la transparence (REQ-PERS-005 point 3 force `scene.use_nes_bg_color` à vrai à chaque chargement de profil). Image synthétique dont une tile de fond a deux pixels de valeur 0 et deux de valeur 3, l'entrée de palette 3 étant égale à l'entrée 0 : les deux pixels de valeur 0 donnent la couleur de fond de la scène (±2 niveaux sRGB) ; les deux pixels de valeur 3 sont dessinés avec la couleur du fond universel.
- Critère d'acceptation (auto) : bits 3 et 4 de PPUMASK à 0 → la capture est la couleur de fond de la scène sur tout le rectangle. Bit 1 de PPUMASK à 0, fond rendu : les 8 colonnes de gauche sont transparentes et le reste du rectangle visible est inchangé (mise en condition de Q-085 : marges (0,1 ; 8,1 ; 0,1 ; 0,1), rangées 8 et 239 pour les bords).
- Critère d'acceptation (auto) : même scène rendue avec `video.anti_alias` = 0, 1 et 2 : l'ensemble des pixels dessinés du rectangle est identique dans les trois cas.
- Notes : vue de face, sans courbure et avec la couleur de fond de la scène égale au fond NES, l'image reste visuellement inchangée et la transparence n'est **pas** observable — c'est le cas de la configuration par défaut. Quand le fond et les sprites sont tous deux désactivés, le rectangle entier est transparent et seule la scène est visible ; c'est le comportement attendu, le programme d'origine n'ayant alors aucune géométrie non plus. Le rectangle est **éclairé comme une tile** (REQ-SCN-028, albédo = couleur du pixel).

**REQ-SCN-074** (P2, AMD-046) — **Courbure du mode 2D** (réglage propre à Zaxnes, divergence acceptée **(D)**) : en mode 2D, le rectangle du mode 2D DOIT être subdivisé en au moins 32 × 32 cellules. Quand le réglage « Courber le mode 2D » (`scene.bend_2d_mode`) est actif (défaut), la courbure de REQ-SCN-067 DOIT s'appliquer au rectangle comme à la géométrie de la scène de jeu (REQ-SCN-068), y compris la clause d'ordre de REQ-SCN-068 : la découpe par le rectangle visible (REQ-SCN-036) est évaluée sur les positions avant courbure, de sorte que l'ensemble des lignes et colonnes NES visibles ne dépend pas du réglage. Quand le réglage est inactif, la position de chaque sommet du rectangle DOIT être sa position plate (B = identité), quels que soient les réglages de REQ-SCN-066. Si un backdrop est présent (REQ-SCN-048, piloté par Lua), il reste soumis à REQ-SCN-068, que le réglage soit actif ou non. En mode 3D, le réglage n'a aucun effet. Le réglage s'applique à chaud, dès la frame rendue suivante ; il est persisté. Il n'a pas d'accès Lua : le programme d'origine n'en a pas.
- **REQ-INP-124 amendée** : en mode 2D, quand le réglage est inactif, REQ-INP-124 NE S'APPLIQUE PAS et la visée est celle de REQ-INP-063, comme à courbure nulle ; quand le réglage est actif, REQ-INP-124 s'applique sans changement.
- Critère d'acceptation (auto) : harnais, mode 2D, Courbure gauche-droite à 90°. Réglage actif : chaque sommet exporté du rectangle, de position plate P, est à moins de 0,05 unité de B(P) ; le rectangle compte au moins 1 089 sommets distincts. Réglage inactif : tous les sommets exportés sont dans le plan z = 0, à leur position plate à 10⁻³ unité près. Les lignes NES visibles sont les mêmes dans les deux cas, pour les marges par défaut. En mode 3D, basculer le réglage ne change aucun sommet exporté.
- Notes : cela ne crée pas de chemin de rendu séparé au sens de REQ-SCN-055 : seule la subdivision d'une surface change, le pipeline reste unique.

### 3.17 Pipeline de rendu de la réimplémentation

**REQ-SCN-056** (P1) — Le rendu DOIT fonctionner avec **OpenGL 3.3 core** sans extension (les textures 2D en tableau, l'instanciation et les cartes de profondeur font partie du cœur 3.3).
- Critère d'acceptation : exécution sur un contexte 3.3 core strict (profil de compatibilité désactivé) sans erreur GL.

**REQ-SCN-057** (P1) — Le moteur de rendu DOIT dessiner dans une **cible fournie par l'hôte** à chaque frame (identifiant de framebuffer et dimensions en pixels), sans supposer le framebuffer par défaut, sans état global de processus, et sans relire l'image produite. Plusieurs instances indépendantes DOIVENT pouvoir coexister.
- Critère d'acceptation : le harnais headless (PLAN, `Sen.Harness`) rend une frame dans un framebuffer hors écran de 640 × 480 et obtient une image identique à celle de la fenêtre redimensionnée à la même taille.

**REQ-SCN-058** (P1, amendée par AMD-055 §3 point 3) — L'ordre de rendu d'une frame DOIT être : carte d'ombre ; effacement ou skybox ; objets opaques (tiles, backdrop, tunnel et modèles TV) ; **en style Enhanced, les reflets du sol (REQ-SCN-084) puis l'occlusion ambiante (REQ-SCN-083)** ; objets transparents triés ; HUD ; puis l'UI de SPEC-10 par-dessus. Les objets transparents, dessinés après les deux passes d'écran de REQ-SCN-080 (leur passage de profondeur compris), n'occultent pas pour l'occlusion ambiante et ne sont pas reflétés ; ils ne sont pas davantage assombris par l'occlusion ni porteurs de reflets. Il en va de même du HUD, dessiné encore après, et des tiles « ombre seule » (REQ-SCN-041), qui n'écrivent aucune profondeur ; la skybox peut être reflétée mais n'occulte jamais (REQ-SCN-080). Le rendu se fait en espace linéaire avec une précision suffisante pour éviter les bandes (au moins 10 bits par canal, ou encodage sRGB matériel).
- Critère d'acceptation : un motif transparent placé devant le backdrop laisse voir celui-ci ; le HUD se dessine par-dessus les objets transparents.

**REQ-SCN-059** (P1) — Le rendu DOIT être indépendant de la cadence d'émulation : les contrôles et le lissage de caméra (REQ-SCN-014 à 020) et le scintillement des étoiles utilisent le temps réel écoulé ; la déformation animée (REQ-SCN-043) utilise le temps émulé. La caméra continue d'être lissée pendant les pauses (la scène, elle, n'est pas affichée, REQ-SCN-047).
- Critère d'acceptation : à 60, 90 et 120 images/s, une rotation de 3 s couvre le même angle (±2 %).

**REQ-SCN-060** (P2) — Avec le réglage utilisateur **saut de rendu** activé (défaut : activé), quand aucune nouvelle frame émulée n'a été produite depuis le dernier rendu et que le jeu est en cours, le rendu de la scène PEUT être omis et l'image précédente conservée. Désactivé, la scène est rendue à chaque rafraîchissement.
- Critère d'acceptation : à 120 Hz, saut activé, la scène n'est pas redessinée plus de 60 fois par seconde en NTSC ; désactivé, l'orbite de la caméra est fluide à 120 Hz.

**REQ-SCN-061** (P2) — La cadence d'affichage DOIT être pilotée par les réglages utilisateur **VSync** (défaut : désactivée) et **fréquence d'images cible** (60, 90 ou 120 images/s, défaut 90 ; toute autre valeur persistée est ramenée à 90) ; leur application relève de l'hôte (SPEC-01/07). Le moteur de rendu DOIT accepter n'importe quelle cadence.
- Critère d'acceptation : réglage 60 images/s sans VSync ⇒ 60 ±2 images/s mesurées.

**REQ-SCN-062** (P3) — Aucun anticrénelage ni post-traitement (bloom, flou, correction colorimétrique) n'est appliqué par défaut. Le réglage utilisateur **AntiAlias** (None, FFAA, SMAA ; défaut None) est conservé mais n'a aucun effet visible dans l'original ; la réimplémentation PEUT lui associer un anticrénelage réel, à condition que la valeur None produise une image sans anticrénelage. Le réglage **qualité graphique** n'a aucun effet visible et PEUT être absent.
- Critère d'acceptation : en None, sur une capture hors écran d'un cube incliné, chaque pixel de bord prend soit la couleur du cube, soit celle du fond (aucune valeur intermédiaire due à un anticrénelage).

**REQ-SCN-063** (P3) — La réimplémentation PEUT proposer un rendu à résolution réduite suivi d'un agrandissement bilinéaire, avec les facteurs 1, 2/3, 1/2, 1/3 et 1/4 (libellés « Native », « 2/3 », « 1/2 », « 1/3 », « 1/4 ») ; le facteur 1 est le défaut, seul disponible sous Windows dans l'original. Statut : le réglage `video.res_scale` est conservé dans le modèle de réglages (SPEC-09) mais n'est **pas** exposé dans l'UI desktop (SPEC-10 REQ-UI-062, DEC-10-Q2) ; sa prise en compte par le rendu est facultative.
- Critère d'acceptation : facteur 1/2 en 1920 × 1080 ⇒ scène rendue en 960 × 540, UI à pleine résolution.

**REQ-SCN-064** (P2) — Le rendu DEVRAIT tenir **60 images/s en 1920 × 1080** avec ombres activées, sur la machine de référence de SPEC-12, pour une scène de 2048 tiles et 1024 shapes. L'instanciation géométrique est recommandée.
- Critère d'acceptation : mesure sur le jeu de référence le plus chargé de SPEC-12 : 99ᵉ centile du temps de frame ≤ 16,7 ms.

## 4. Formats et interfaces

### 4.1 Réglages utilisateur concernés (persistance : SPEC-09)

| Réglage | Type | Défaut | Bornes / valeurs | Exigence |
|---|---|---|---|---|
| FOV | réel (degrés) | 70 | 45 à 95, pas de 5 | REQ-SCN-005 |
| Intensité lumineuse | réel | 1,0 | 0,50 à 1,50, pas de 0,01 | REQ-SCN-026 |
| Ombres | booléen | vrai | — | REQ-SCN-030 |
| Skybox | entier | −1 | −1 Default, 0 Color, 1 Gradient, 2 Fantasy | REQ-SCN-049 |
| Couleur personnalisée | couleur sRGB | noir | — | REQ-SCN-046 |
| TV Box | entier | 0 | 0 à 3 | REQ-SCN-053 |
| Saut de rendu | booléen | vrai | — | REQ-SCN-060 |
| VSync | entier | 0 | 0 désactivée, 1 chaque rafraîchissement, 2 un rafraîchissement sur deux (SPEC-09) | REQ-SCN-061 |
| Fréquence d'images | entier | 90 | 60, 90, 120 | REQ-SCN-061 |
| AntiAlias | entier | 0 | 0 None, 1 FFAA, 2 SMAA | REQ-SCN-062 |
| Facteur de résolution | entier (indice) | 0 | 0 à 4 → 1, 2/3, 1/2, 1/3, 1/4 | REQ-SCN-063 |
| Style visuel (`render.visual_style`) | entier | 0 | 0 Classic, 4 Enhanced (1 accepté en écriture, relu 4) ; autre → 0 | REQ-SCN-078 |
| `render.style.ao` | entier | 0 | 0 Off, 1 MSVO, 2 Depth (pas 1) | REQ-SCN-079, REQ-SCN-083 |
| `render.style.ao_strength` | entier | 3 | 1 à 5, pas 1 | REQ-SCN-079, REQ-SCN-083 |
| `render.style.ao_radius` | entier | 3 | 1 à 5, pas 1 | REQ-SCN-079, REQ-SCN-083 |
| `render.style.ao_thickness` | entier | 3 | 1 à 5, pas 1 (sans effet hors du mode MSVO effectif) | REQ-SCN-079, REQ-SCN-083 |
| `render.style.ao_quality` | entier | 1 | 0 Low, 1 Medium, 2 High (pas 1) | REQ-SCN-079, REQ-SCN-083 |
| `render.style.pixel_shadow` | entier | 0 | 0 à 1, pas 1 | REQ-SCN-079, REQ-SCN-082 |
| `render.style.voxel_grid` | entier | 0 | 0 à 3, pas 1 (`N` = 2^valeur) | REQ-SCN-079, REQ-SCN-081 |
| `render.style.reflections` | entier | 0 | 0 à 1, pas 1 | REQ-SCN-079, REQ-SCN-084 |
| `render.style.ssr_shine` | entier | 3 | 1 à 5, pas 1 | REQ-SCN-079, REQ-SCN-084 |
| `render.style.ssr_clarity` | entier | 3 | 1 à 5, pas 1 | REQ-SCN-079, REQ-SCN-084 |
| `render.style.ssr_length` | entier | 3 | 1 à 5, pas 1 | REQ-SCN-079, REQ-SCN-084 |
| `render.style.ssr_quality` | entier | 2 | 0 Low, 1 Medium, 2 High, 3 Ultra (pas 1) | REQ-SCN-079, REQ-SCN-084 |

Les treize lignes ci-dessus sont celles d'AMD-055 §3 point 8 et §4 (portée U, effet immédiat, non
exposées en libretro) ; les quatre réglages de courbure d'AMD-043 REQ-SCN-066 restent hors du
style (REQ-SCN-078 : ils agissent en Classic comme en Enhanced).

### 4.2 Réglages de profil concernés (stockage : SPEC-03)

| Réglage | Défaut hors profil | Modifiable par Lua | Exigence |
|---|---|---|---|
| Direction de la lumière | (1, −1, 1) | `Light.Direction` | REQ-SCN-025 |
| Couleur de la lumière | blanc | `Light.Color` | REQ-SCN-025 |
| Clipping (4 marges) | (0,1 ; 8,1 ; 8,1 ; 0,1) | `Setting.Clipping[i]`, `Clipping` | REQ-SCN-003 |
| Couleur de fond NES | forcé à vrai au chargement | `Setting.BgColor` | REQ-SCN-046 |
| Caméra locale | faux | `Setting.LocalCam`, `Setting.TopDownCam` | REQ-SCN-009 |
| Drapeaux d'ombre des couches | réinitialisés au chargement (REQ-SCN-033) | `Setting.Layer[i].CastShadow / RecvShadow` | REQ-SCN-033 |

Valeurs propres à la session, non persistées : temps de lissage (`Setting.SmoothTime`, 0,4 s), état du backdrop, état de la caméra (sauf dans les sauvegardes d'état, REQ-SCN-024).

### 4.3 Constantes de scène

| Constante | Valeur | Justification |
|---|---|---|
| Centre C du cadre de référence | (128, 120, 0) | centre du cadre 256 × 240 de REQ-E3D-410, 8 unités sous le centre de l'image NES (128, 128, 0) ; valeur de l'original |
| Angles par défaut | (0,81° ; 0,81° ; 0) | vue quasi frontale, très légèrement en hauteur et à gauche |
| Distance par défaut | 104 / atan(FOV_rad / 2) | cadrage de l'original (≈ 87 % de la hauteur à 70°) |
| Vitesse de rotation | 30°/s | |
| Vitesse de zoom | 70 unités/s ; 5 unités par cran de molette | |
| Glissement souris | 0,1° par pixel | |
| Distance bornée | [0, 512] | |
| Fenêtre de transition de mode | 3 s | |
| Temps de lissage | 0,4 s | |
| Plans proche / lointain | 0,3 / 5000 | |
| Ambiante | sRGB 118 (0,4627) | |
| Force d'ombre | 0,8 | |
| Distance d'ombre | 512 | |
| Carte d'ombre | 1024 texels par défaut | DEC-R2-D4 |
| Distance du plan HUD | 60 / atan(FOV_rad / 2) ; échelle 0,5 | |
| Décalage des sprites | `0,001 × (i mod 64) ∓ 0,1` | appliqué par le rendu (REQ-SCN-037, DEC-R2-T4, AMD-057) |

## 5. Vecteurs de test

### 5.1 Caméra

| Cas | Entrées | Résultat attendu (régime établi) |
|---|---|---|
| C1 | FOV 70, clipping (0,1 ; 8,1 ; 8,1 ; 0,1), vue par défaut | d = 189,65 ; T′ = (124 ; 124 ; 0) ; E = (121,32 ; 126,68 ; −189,61) ; P* ≈ (123,4 ; 124,6) |
| C2 | idem, clipping (0 ; 7,9 ; 0 ; 0,1) (Super Mario Bros) | T′ = (128 ; 123,9 ; 0) ; E = (125,32 ; 126,58 ; −189,61) ; P* ≈ (127,3 ; 124,5) |
| C3 | C2 puis rx = 30°, ry = −20° | E ≈ (184,17 ; 218,73 ; −154,34) ; P* ≈ (145,1 ; 145,7) |
| C4 | FOV 90, réinitialisation | d = 156,21 ; d_HUD = 90,12 |
| C5 | ZoomIn maintenu 3 s depuis C1 | distance de consigne 0 (bornée) |

Tolérances : positions ±0,5 ; P* ±0,6. Vecteurs recalculés en v1.1 dans le repère de REQ-E3D-410 (cible par défaut C = (128, 120, 0), rectangle R du cadre de référence) : valeurs inchangées par rapport à la v1.0, qui utilisait déjà ce repère pour la caméra.

### 5.2 Ombrage (lumière (1, −1, 1), blanche, intensité 1, ambiante 0,1812 linéaire)

Facteurs linéaires : face éclairée (N·L = 0,577) k = 0,7585 ; même face à l'ombre k = 0,2966 ; face à l'ambiante seule k = 0,1812. Sortie = srgb(lin(couleur) × k).

| Couleur NES | sRGB d'entrée | Éclairée | À l'ombre | Ambiante seule |
|---|---|---|---|---|
| 0x30 | (252, 248, 252) | (223, 219, 223) | (146, 144, 146) | (117, 115, 117) |
| 0x22 | (104, 136, 252) | (91, 120, 223) | (57, 76, 146) | (44, 60, 117) |
| 0x17 | (228, 96, 24) | (202, 84, 20) | (132, 52, 9) | (105, 40, 5) |
| 0x29 | (188, 248, 24) | (166, 219, 20) | (108, 144, 9) | (85, 115, 5) |
| blanc pur | (255, 255, 255) | (226, 226, 226) | (148, 148, 148) | (118, 118, 118) |

Tolérance : ±3 par canal. Les valeurs RVB d'entrée supposent la palette NES du §4.1 de SPEC-04.

### 5.3 Boîtier TV (clipping (0,1 ; 8,1 ; 8,1 ; 0,1), cadre de référence de REQ-E3D-410 ; valeurs recalculées en v1.1, inchangées)

Paroi gauche : x ∈ [−8 ; 0,1], y ∈ [4,1 ; 243,9], z ∈ [−16 ; 112]. Paroi droite : x ∈ [247,9 ; 264]. Paroi basse : y ∈ [−4 ; 8,1]. Paroi haute : y ∈ [239,9 ; 244]. Fond : z ∈ [112 ; 120]. Repère TV : origine C + (−0,03125 ; 0,03333 ; −16), échelles (0,96797 ; 0,96583 ; 1).

### 5.4 Captures de référence (comparaison avec l'original en boîte noire) — [ROM] [R2]

Ces vues exigent la ROM de l'utilisateur ([ROM]) ; la comparaison avec l'original n'est faisable qu'au round 2 ([R2]). Au round 1, les descriptions qualitatives ci-dessous servent de critère de revue ; les exigences qu'elles illustrent ont par ailleurs leurs propres critères exécutables.

Conditions communes : fenêtre 1920 × 1080, FOV 70, intensité 1,0, ombres activées, AntiAlias None, TV Box None, Skybox Default sauf mention, caméra laissée immobile 2 s après le début de la partie. Comparaison : superposition des captures ; les contours des objets DOIVENT coïncider à ±2 pixels et les couleurs des grandes surfaces à ±4 par canal.

- **R1 — Super Mario Bros, monde 1-1, premier écran, vue par défaut.** Vue presque frontale, très légèrement plongeante depuis la gauche. L'image NES occupe environ 87 % de la hauteur et 52 % de la largeur de la fenêtre, centrée. Le fond de la fenêtre entière est bleu NES 0x22 (104, 136, 252), sans dégradé. Les blocs et tuyaux en relief montrent leurs faces avant, supérieure et gauche de même luminosité (≈ 88 % de la couleur NES en sRGB), leurs faces droite et inférieure nettement plus sombres (≈ 46 %). Les ombres portées partent vers la droite, le bas et la profondeur. Le score en haut reste net et plat face à la caméra.
- **R2 — Même scène, Skybox Gradient.** Le fond n'est plus uni : bleu 0x22 à l'horizon de la skybox (hauteur des yeux), plus clair vers le haut de l'image, plus sombre vers le bas ; aucune étoile.
- **R3 — Même scène après rotation RotRight 3 s puis RotDown 1 s.** Caméra décalée d'environ 90° sur la gauche de l'écran et de 30° vers le haut : on voit l'écran NES par la tranche, découpé net à ses bords (REQ-SCN-036) ; l'écran reste centré dans l'image grâce au visé corrigé (REQ-SCN-008).
- **R4 — Super Mario Bros, monde 1-2 (souterrain), Skybox Gradient.** Ciel noir parsemé d'étoiles fines qui scintillent lentement.
- **R5 — Super Mario Bros 1-1, TV Box = 3.** La scène est vue à travers une ouverture rectangulaire aux bords nets, entourée par le téléviseur cathodique noir, dont les touches sont visibles sous l'écran.
- **R6 — Super Mario Bros 1-1, Skybox Fantasy.** Nébuleuse aux teintes dérivées du bleu de fond, sans étoiles (fond clair).
- **R7 — Super Mario Bros 1-1, ombres désactivées.** Identique à R1 sans aucune ombre portée ; les faces gardent les mêmes luminosités.

## 6. Questions ouvertes

- **Q1 (P1 assets)** — *Close par DEC-06-Q1* (cubemaps réextraites avec leurs 6 faces, conventions dans `cubemaps.json`). Texte d'origine : les cubemaps de `assets-neutral/cubemaps/` (`Stars_LP_01`, `Starfield`, `NebulaDensity`, `NebulaDiffusion`, `Cubemap`, `Clouds_LP_01`) ne contiennent qu'une image carrée alors que les originaux ont 6 faces. Il faut les réextraire avec les 6 faces et documenter l'ordre et l'orientation des faces dans INVENTORY.md, sinon REQ-SCN-050, -051 et -054 (TV 2) sont irréalisables à l'identique.
- **Q2** — *Close par DEC-06-Q2* (même conversion sRGB → linéaire, différentiel au round 2). Courbe exacte appliquée à l'intensité lumineuse différente de 1 (hypothèse : même conversion sRGB → linéaire que les couleurs). À confirmer par captures de l'original à intensité 0,5 et 1,5.
- **Q3** — *Close par DEC-R2-D4* (1024 texels par défaut). Résolution exacte de la carte d'ombre de l'original (estimée à environ 512 texels). À mesurer sur capture (taille des marches d'escalier sur une ombre oblique) ; REQ-SCN-030 tolère l'écart.
- **Q4 (SPEC-04B)** — *Close par DEC-06-Q4, DEC-R2-T4 et DEC-04B-Q2* (décalage appliqué par le rendu ; déformation avant échelle et rotation). Repère local des sommets pour la déformation animée (REQ-SCN-043) et application éventuelle du décalage de ±0,1 des sprites (REQ-SCN-037) déjà dans les positions livrées : à aligner avec SPEC-04B.
- **Q5 (SPEC-09)** — *Close par DEC-06-Q5* (couche de session non persistée, D). Dans l'original, les modifications Lua de `Setting.CustomBgColor` ou `Light.Intensity` pendant une partie modifient le réglage utilisateur lui-même et peuvent donc être persistées. Faut-il reproduire ce comportement ou isoler les valeurs de session ?
- **Q6 (SPEC-08/10)** — *Close par DEC-06-Q6* ; il n'y a pas de conflit de bouton : la gâchette du Zapper est le bouton droit (REQ-INP-062), l'orbite le bouton gauche (REQ-SCN-017). Le glissement souris de l'original n'est pas restreint à l'état « jeu en cours » (sans effet visible puisque la scène est masquée hors jeu). REQ-SCN-017 le laisse libre : SPEC-08 doit préciser le bouton exact et le conflit éventuel avec la Zapper à la souris.

## Historique

- Copie de travail du côté propre, 2026-11-03 (lot RU-5) : **AMD-055** écrit dans ce fichier — nouvelle section **§3.11bis « Style visuel Enhanced »** (REQ-SCN-078 à REQ-SCN-084, texte de l'amendement §2) après REQ-SCN-043, REQ-SCN-058 amendée (ordre de frame : reflets du sol puis occlusion ambiante en style Enhanced), la phrase de REQ-SCN-075 sur la grille de voxels remplacée (§3 point 5) et treize lignes en §4.1. Les renvois « §2.x » à l'intérieur de §3.11bis sont ceux de l'amendement. Le texte normatif reste celui de `changes/`.
- 1.1-draft (AMD-001, relecture R2) : repère de scène réécrit d'après SPEC-04B REQ-E3D-410 (DEC-R2-T2) : image NES en y ∈ [8, 248], cadre de référence y ∈ [0, 240] et C = (128, 120, 0) conservés (vérifiés dans l'original), REQ-SCN-001/003/012/045/048/053 et §4.3 corrigés, vecteurs §5.1/§5.3 recalculés (inchangés) ; décalage des sprites au rendu (REQ-SCN-037) ; REQ-SCN-043 renvoie à REQ-E3D-568 ; carte d'ombre 1024 (DEC-R2-D4) ; REQ-SCN-059, VSync, palette SPEC-04, statut de REQ-SCN-063 ; étiquettes [ROM]/[R2] et critères exécutables (030, 044, 046, 050, 051, 062, §5.4) ; questions closes.
