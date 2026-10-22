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

**REQ-SCN-003** (P1) — Les quatre marges de clipping DOIVENT provenir du réglage de profil (SPEC-03) au chargement de la partie. Elles DOIVENT être modifiables pendant la partie par Lua (`Setting.Clipping[i]` et l'alias global `Clipping`, i = 0 gauche, 1 bas, 2 droite, 3 haut), avec un effet dès la frame rendue suivante sur la découpe (REQ-SCN-039), le cadrage de la caméra (REQ-SCN-007, -008) et le boîtier TV (REQ-SCN-053). Valeurs typiques des profils : 0,1, 7,9, 8,1, 16,1, 24,1. Valeurs par défaut en l'absence de profil : (0,1 ; 8,1 ; 8,1 ; 0,1).
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

**REQ-SCN-037** (P2) — Les tiles de sprite (shapes non-fond) DOIVENT être décalées en profondeur, après placement : de +0,1 unité (vers le fond) pour les sprites « derrière le fond » (priorité NES) et de −0,1 unité (vers la caméra) pour les autres. Les tiles de fond ne sont pas décalées. Le rendu est **seul propriétaire** de ce décalage (DEC-06-Q4, DEC-R2-T4) : les transformations livrées par SPEC-04B ne l'incluent pas (REQ-E3D-405, 641) ; il est appliqué dans le repère local de la shape, avant son échelle et sa rotation, une seule fois.
- Critère d'acceptation : un sprite plat posé sur une tile de fond plate de même profondeur reste visible devant elle, sans scintillement ; TV-W2 de SPEC-04B : face avant d'un sprite en L1 FRONT affichée à z = −0,1 (et non −0,2 ni 0).

**REQ-SCN-038** (P2) — Le rendu DOIT appliquer à chaque shape la position, l'échelle et la rotation livrées par le moteur (SPEC-04B), et ne rendre que les shapes actives dont le mesh est prêt.
- Critère d'acceptation : couverture par les vecteurs de SPEC-04B.

**REQ-SCN-039** (P1) — Le rectangle de découpe R DOIT être recalculé à chaque frame rendue depuis le clipping courant (REQ-SCN-003).
- Critère d'acceptation : un script qui modifie `Setting.Clipping[0]` de 0,1 à 16,1 masque dès la frame suivante la bande x < 16,1.

**REQ-SCN-040** (P1) — Un motif d'opacité alpha < 1 (et de style différent de « ombre seule ») DOIT être rendu en **transparence** : d'abord un passage qui n'écrit que la profondeur, puis un passage de couleur mélangé (source × α + destination × (1 − α)), avec α = alpha du texel (canal alpha de la couleur de palette ou de la texture) × alpha du motif. Seule la surface la plus proche de chaque objet transparent est ainsi visible (pas de faces internes). Les objets transparents DOIVENT être dessinés après tous les objets opaques, triés du plus lointain au plus proche. Ils sont éclairés (REQ-SCN-028) mais ne projettent pas d'ombre.
- Critère d'acceptation : un motif d'alpha 0,5 devant un fond rouge laisse voir le fond à environ 50 %, et on ne distingue pas ses faces arrière.

**REQ-SCN-041** (P1) — Le style de rendu d'un motif (SPEC-03/04) DOIT être respecté : 0 = invisible (ni dessiné, ni ombre) ; 1 = normal ; 2 = **ombre seule** (non dessiné, mais projette son ombre, toujours en opaque).
- Critère d'acceptation : un motif de style 2 n'apparaît pas à l'écran mais son ombre est visible sur le décor qui reçoit les ombres.

**REQ-SCN-042** (P3) — Le rendu des tiles NE DOIT comporter ni contour, ni effet CRT, ni lignes de balayage, ni émission propre, ni brouillard.
- Critère d'acceptation : inspection visuelle des vues de référence §5.4.

**REQ-SCN-043** (P2) — Un motif dont la déformation animée est active DOIT voir ses sommets déplacés à chaque frame rendue selon **SPEC-04B REQ-E3D-568, qui est normative** (pivot `Pd` avec le décalage `h₀` dépendant de la version du profil, temps de frame = frames émulées / 60, application avant l'échelle et la rotation de l'objet, DEC-04B-Q2). Le rendu PEUT appliquer ce déplacement lui-même à partir des paramètres livrés (REQ-E3D-641). Les normales ne sont pas modifiées. L'animation suit le temps **émulé** : elle se fige en pause et avance en accéléré.
- Critère d'acceptation : vecteur TV-D1 de SPEC-04B ; avec `deltaSpeed[1][3] = 2π` en plus, l'onde fait un cycle par seconde de jeu et reste figée pendant une pause.

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

Le pavé est un parallélépipède rectangle aligné sur les axes, centré en C + (OfsX, OfsY, OfsZ), de côtés (SizeX, SizeY, SizeZ). Il DOIT être éclairé de façon diffuse (REQ-SCN-028, albédo = `Backdrop.Color`), ne jamais projeter d'ombre, et recevoir les ombres si `RecvShadow` est vrai et que les ombres sont activées. Il n'est pas découpé par le rectangle visible. Toutes ces propriétés DOIVENT revenir à leurs valeurs par défaut à la fin de chaque partie.
- Critère d'acceptation : `Backdrop.Enable = true; Backdrop.Color = Color(0,0,0); Backdrop.OfsZ = 26` fait apparaître derrière la scène un panneau noir de 256 × 240 dont la face avant est à z = 25,5, sur lequel se dessinent les ombres des couches avant.
- Notes : valeurs relevées dans les scripts embarqués : OfsZ de 0 à 61, SizeX 240 à 384, SizeY 120 à 256, SizeZ 1 à 120. Par défaut le pavé couvre le cadre de référence (y ∈ [0, 240]), soit 8 unités plus bas que l'image NES (y ∈ [8, 248]) : comportement de l'original (REQ-E3D-410).

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

### 3.16 Mode 2D et réglage « TV Mode »

**REQ-SCN-055** (P1) — Le « mode 2D » NE DOIT PAS être un chemin de rendu séparé : c'est le profil embarqué spécial intitulé « 2D Mode » (fichier `profiles/empty.bytes`, SPEC-03), sans motif, rendu par le même pipeline 3D, avec la même caméra et les mêmes réglages de scène. L'aspect obtenu (tiles converties avec la géométrie par défaut) est défini par SPEC-04.
- Critère d'acceptation : lancer une ROM en « 2D Mode » montre l'image du jeu dans la scène, orbitable avec les commandes de caméra.
- Notes : le réglage « TV Mode » (NTSC, PAL, DENDY) choisit la région émulée (SPEC-02/07) et n'a aucun effet sur le rendu.

### 3.17 Pipeline de rendu de la réimplémentation

**REQ-SCN-056** (P1) — Le rendu DOIT fonctionner avec **OpenGL 3.3 core** sans extension (les textures 2D en tableau, l'instanciation et les cartes de profondeur font partie du cœur 3.3).
- Critère d'acceptation : exécution sur un contexte 3.3 core strict (profil de compatibilité désactivé) sans erreur GL.

**REQ-SCN-057** (P1) — Le moteur de rendu DOIT dessiner dans une **cible fournie par l'hôte** à chaque frame (identifiant de framebuffer et dimensions en pixels), sans supposer le framebuffer par défaut, sans état global de processus, et sans relire l'image produite. Plusieurs instances indépendantes DOIVENT pouvoir coexister.
- Critère d'acceptation : le harnais headless (PLAN, `Sen.Harness`) rend une frame dans un framebuffer hors écran de 640 × 480 et obtient une image identique à celle de la fenêtre redimensionnée à la même taille.

**REQ-SCN-058** (P1) — L'ordre de rendu d'une frame DOIT être : carte d'ombre ; effacement ou skybox ; objets opaques (tiles, backdrop, tunnel et modèles TV) ; objets transparents triés ; HUD ; puis l'UI de SPEC-10 par-dessus. Le rendu se fait en espace linéaire avec une précision suffisante pour éviter les bandes (au moins 10 bits par canal, ou encodage sRGB matériel).
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
| Décalage des sprites | ±0,1 | appliqué par le rendu (REQ-SCN-037, DEC-R2-T4) |

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

- 1.1-draft (AMD-001, relecture R2) : repère de scène réécrit d'après SPEC-04B REQ-E3D-410 (DEC-R2-T2) : image NES en y ∈ [8, 248], cadre de référence y ∈ [0, 240] et C = (128, 120, 0) conservés (vérifiés dans l'original), REQ-SCN-001/003/012/045/048/053 et §4.3 corrigés, vecteurs §5.1/§5.3 recalculés (inchangés) ; décalage des sprites au rendu (REQ-SCN-037) ; REQ-SCN-043 renvoie à REQ-E3D-568 ; carte d'ombre 1024 (DEC-R2-D4) ; REQ-SCN-059, VSync, palette SPEC-04, statut de REQ-SCN-063 ; étiquettes [ROM]/[R2] et critères exécutables (030, 044, 046, 050, 051, 062, §5.4) ; questions closes.
