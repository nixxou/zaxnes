# AMD-001 — Application de la relecture R2 (spec v1.1-draft)

Rôle : R1 (rédacteur, côté SALE) | Date : 2026-10-20 | Entrées : `changes/R2-review-v1.md`, `DECISIONS-R0.md` (arbitrages DEC-R2-*)
Documents modifiés (en-tête passé à « Version : 1.1-draft », ligne « Historique » ajoutée) : SPEC-00, 01, 02, 03, 04, 04B, 05, 06, 07, 08, 09, 10, 11, 12. `DECISIONS-R0.md` : seul ajout d'un sous-point « Constat R1 » sous DEC-R2-T2.

## 1. Synthèse

- **Repère de scène (DEC-R2-T2)** : définition unique dans **SPEC-04B REQ-E3D-410** (nouvelle exigence). Vérification dans l'original : l'image NES est placée en `y ∈ [8, 248]` (`y = 248 − py`), mais le rectangle visible, le visé corrigé, la cible par défaut, le HUD, le backdrop et le boîtier TV utilisent le **cadre de référence** `[0, 256] × [0, 240]` de centre **C = (128, 120, 0)**, 8 unités sous le centre de l'image. Le rectangle visible est `R = [c0, 256 − c2] × [c1, 240 − c3]`, soit les lignes NES `[8 + c3, 248 − c1]`. Les valeurs fidèles l'emportent sur la proposition R2 (C = (128, 128, 0)) : les vecteurs caméra et TV de SPEC-06 v1.0 étaient déjà calculés dans ce repère et sont **inchangés** après recalcul ; seuls la position de l'image (REQ-SCN-001, §2) et les critères qui en dépendaient (REQ-SCN-003, 045) étaient faux. Visée du Zapper : relation fidèle `py = tronc(240 − (y − Oy))`, qui vise **8 lignes au-dessus** du pixel affiché (SPEC-08 REQ-INP-063, question Q-9 ouverte pour R0).
- **Correction factuelle hors constat R2** : l'avance rapide de l'original applique le réglage de vitesse `s = 2`, soit **k = 1,5** (et non 2×) ; SPEC-07 REQ-LOOP-016, SPEC-08 REQ-INP-103, SPEC-02 REQ-CORE-072 et SPEC-11 REQ-AUD-011 corrigées. Le libellé « 2× » de DEC-07-Q2 est à corriger par R0.
- Étiquettes **[ROM]** / **[R2]** posées dans le titre des exigences concernées ; catégories de vérification et compatibilité de REQ-ACC-050 dans SPEC-12.
- Outils de cet amendement (côté sale) : `tools/amd001/*.py` (remplacements exacts, à occurrence unique), `tools/scene_frame_check.py` (recalcul des vecteurs caméra/TV/Zapper). Scan anti-fuite : aucun nouvel identifiant interne (`tools/out/leak_scan_before_amd001.txt` et `…_after_amd001.txt` : mêmes ensembles d'identifiants, tous visibles des profils ou de l'UI).

Décompte : le rapport R2 annonce 114 constats ; l'énumération ci-dessous compte 117 lignes (T-1 à T-9, 107 constats numérotés par document, point 25 sur REQ-E3D-521/522), certains constats étant rappelés sous plusieurs numéros. **Toutes les lignes sont traitées, aucune n'est rejetée sans correction** : 112 selon la proposition du relecteur ou la décision R0 applicable ; 3 (T-2, 06-1, D-2) corrigées **autrement que proposé**, parce que l'original fixe d'autres valeurs (C = (128, 120, 0), rectangle visible dans le cadre [0, 240]) ; 2 (07-6, D-5) corrigées selon DEC-R2-D5, qui écarte la proposition du relecteur.

## 2. Table des constats

| Constat | Cat. | Document / exigence | Action |
|---|---|---|---|
| T-1 | INC | SPEC-09 §4.1.5, Q-2 ; SPEC-04 REQ-E3D-002 ; SPEC-04B REQ-E3D-410 | Corrigé (DEC-R2-T1) : `clip.bottom` = `Clipping[1]`/`y`, `clip.top` = `Clipping[3]`/`w` ; texte de l'effet réécrit ; composantes nommées dans SPEC-04 ; Q-2 close. |
| T-2 | INC | SPEC-04B REQ-E3D-410 (nouvelle), 401 ; SPEC-06 §2, REQ-SCN-001/003/012/045/048/053/054, §4.3, §5.1, §5.3 ; SPEC-08 REQ-INP-063/123, §5.3 ; SPEC-00 ; SPEC-04 §2 ; SPEC-05 REQ-LUA-088 | Corrigé **autrement que proposé** : vérification dans l'original ; repère unique défini dans REQ-E3D-410 ; C = (128, 120, 0) et R dans le cadre [0, 240] conservés (fidèles), image en y ∈ [8, 248] ; vecteurs recalculés (outil `scene_frame_check.py`) : inchangés ; conversion scène → pixel du Zapper explicitée. Règle « 240 → 248 » retirée. |
| T-3 | INC | SPEC-07 §2, REQ-LOOP-010, 015, §5 ; SPEC-11 REQ-AUD-004, 007 | Corrigé (DEC-R2-T3) : cadence nominale 60/50 images/s ; 800 / 960 échantillons par frame ; vecteurs T1, T4 recalculés. |
| T-4 | INC | SPEC-04B REQ-E3D-405, 641, §4.2, TV-W2 ; SPEC-06 REQ-SCN-037, §4.3 | Corrigé (DEC-R2-T4) : décalage ±0,1 appliqué par le rendu seul ; le moteur 3D livre les indicateurs sprite/priorité. |
| T-5 | INC | SPEC-01 REQ-ARCH-013 ; SPEC-07 REQ-LOOP-040 ; SPEC-09 REQ-PERS-056 ; SPEC-04 REQ-E3D-008 ; SPEC-00 | Corrigé (DEC-R2-T5, D) : l'état appris de la conversion fait partie du savestate ; borne de taille valable pour la session. |
| T-6 | INC | SPEC-07 REQ-LOOP-044 | Corrigé (DEC-R2-T6) : profil rechargé avant l'affectation de `PData` ; critère complété. |
| T-7 | UNT | SPEC-03 REQ-PROF-001, 051 ; SPEC-04 REQ-E3D-060 ; SPEC-05 REQ-LUA-004, 051, 100, 101, §5.1 | Corrigé (DEC-R2-T7) : critères exécutables sur `assets-neutral/profiles` ; §5.1 de SPEC-05 marqué informatif ; noms d'outils retirés des critères. |
| T-8 | UNT | SPEC-02 REQ-CORE-033, 039, 062, 100, 114, 115 ; SPEC-03 REQ-PROF-064 ; SPEC-04 REQ-E3D-233, 252, 322 ; SPEC-04B REQ-E3D-426, 566 ; SPEC-05 REQ-LUA-072, 074, 086, 088, 110, 111, TV-LUA-16 ; SPEC-06 REQ-SCN-030, 044, 046, 050, 051, §5.4 ; SPEC-08 REQ-INP-060, 062, 064, 066 ; SPEC-12 REQ-ACC-020, 021, 031 | Corrigé (DEC-R2-T8) : étiquettes [ROM]/[R2] ; partie « auto » ajoutée quand c'était possible (vecteurs synthétiques, harnais) ; REQ-E3D-426/566 et REQ-PROF-064 rendus exécutables sans ROM commerciale. |
| T-9 | INC | §6 de SPEC-02, 03, 04, 04B, 05, 06, 07, 08, 09, 10, 11 | Corrigé (DEC-R2-T9) : chaque question tranchée est marquée « close par DEC-… » ; la proposition ordinale de SPEC-03 Q-02 est explicitement rejetée. |
| 00-1 | INC | SPEC-00 glossaire | Corrigé : shape = `shape2D`, instance de frame = `shape`. |
| 00-2 | UNC | SPEC-00 glossaire | Corrigé : valeurs numériques 0–5 de SPEC-03. |
| 01-1 | INC | SPEC-01 REQ-ARCH-013 | Corrigé avec T-5. |
| 01-2 | INC | SPEC-01 REQ-ARCH-033 | Corrigé : racine portable `userdata\` (REQ-PERS-071 fait foi). |
| 01-3 | UNC | SPEC-01 REQ-ARCH-013 | Corrigé : borne pour la ROM et le profil chargés, valable pour la session. |
| 02-1 | LEAK | SPEC-02 REQ-CORE-120 | Corrigé : phrase supprimée. |
| 02-2 | UNT | SPEC-02 REQ-CORE-070 | Corrigé : repliement ≤ −60 dBFS sur un balayage 100 Hz–10 kHz, critère spectral. |
| 02-3 | UNT | SPEC-02 REQ-CORE-039 | Corrigé : vecteur synthétique MMC2 ; partie Punch-Out!! étiquetée [ROM]. |
| 02-4 | UNT | SPEC-02 REQ-CORE-002 | Corrigé : matrice opération → test automatisé. |
| 02-5 | UNC | SPEC-02 REQ-CORE-037 | Corrigé : justification ajoutée (règle vérifiée dans l'original, sans motif fonctionnel, reproduite car elle change les lots). |
| 02-6 | UNC | SPEC-02 REQ-CORE-044 | Corrigé : départage par numéro OAM ; l'original emploie un tri non stable, écart classé D. |
| 02-7 | UNC | SPEC-02 REQ-CORE-061 ; SPEC-08 §4.1 | Corrigé : polarité inscrite dans SPEC-08 §4.1 (DEC-02-Q1). |
| 03-1 | INC | SPEC-03 REQ-PROF-025 | Corrigé (DEC-03-Q03, D) : listes 14/15 reconstituées. |
| 03-2 | INC | SPEC-03 REQ-PROF-024, Q-02 | Corrigé : renvoi à REQ-LUA-004 ; Q-02 close. |
| 03-3 | UNC | SPEC-03 §2 | Corrigé : noms de messages déclarés noms de commodité non normatifs. |
| 03-4 | UNC | SPEC-03 REQ-PROF-003 | Corrigé : reformulé en règle de lecture. |
| 03-5 | UNC | SPEC-03 REQ-PROF-052 | Corrigé : aléa exclu du périmètre de REQ-ARCH-012. |
| 04-1 | LEAK | SPEC-04 REQ-E3D-280 | Corrigé : mention des trois tampons supprimée. |
| 04-2 | LEAK | SPEC-04 REQ-E3D-342 | Corrigé : « conservé pour la compatibilité des données ; aucun effet ». |
| 04-3 | LEAK | SPEC-04 REQ-E3D-207 | Corrigé : « `Age` vaut toujours 1 ». |
| 04-4 | INC | SPEC-04 REQ-E3D-205, §4.4 | Corrigé : notation `Bmin`/`Bmax` ; `BL`/`TR` exposés définis explicitement ; critère complété. |
| 04-5 | INC | SPEC-04 REQ-E3D-002, §4.2 | Corrigé : correspondance (g, b, d, h) ↔ x, y, z, w ↔ indices 0–3. |
| 04-6 | INC | SPEC-04 REQ-E3D-021 ; SPEC-04B REQ-E3D-454 | Corrigé : définition unique dans SPEC-04 (valeurs numériques) ; SPEC-04B y renvoie ; note « couches automatiques » supprimée. |
| 04-7 | UNC | SPEC-04 REQ-E3D-300 | Corrigé : reformulé en résultat ; la règle « toutes les shapes qui précèdent s dans la liste fond puis sprites » est celle de l'original (vérifiée) et est conservée. |
| 04-8 | UNC | SPEC-04 REQ-E3D-381 | Corrigé : résultat normatif = bande retenue ; sort des tiles libérées non spécifié. |
| 04-9 | UNC | SPEC-04 REQ-E3D-361 | Corrigé : budget = durée d'une frame de rendu cible (aligné sur REQ-E3D-600, REQ-LOOP-061). |
| 04-10 | UNT | SPEC-04 REQ-E3D-345 | Corrigé : transformée en note informative. |
| 04-11 | UNT | SPEC-04 REQ-E3D-007 | Corrigé : « temps constant amorti » retiré de l'exigence (reste une recommandation dans REQ-E3D-360). |
| 04B-1 | LEAK | SPEC-04B REQ-E3D-603 | Corrigé : parenthèse de pré-allocation supprimée. |
| 04B-2 | LEAK | SPEC-04B REQ-E3D-601 | Corrigé : « tâche de fond » supprimé, seul le retard observable reste. |
| 04B-3 | LEAK | SPEC-04B REQ-E3D-564 | Corrigé : reformulation proposée par R2. |
| 04B-4 | LEAK | SPEC-04B §6 Q-2 | Corrigé : reformulation proposée par R2 ; close par DEC-04B-Q2. |
| 04B-5 | LEAK | SPEC-04B REQ-E3D-507 | Corrigé : exigence **retirée** (ordre sans effet observable ; REQ-E3D-523 ignore l'ordre). |
| 04B-6 | INC | SPEC-04B REQ-E3D-405 | Corrigé avec T-4. |
| 04B-7 | INC | SPEC-04B REQ-E3D-560 ; SPEC-06 REQ-SCN-034, §5.2 | Corrigé : table `NES_RGB` = §4.1 de SPEC-04. |
| 04B-8 | INC | SPEC-04B REQ-E3D-567 ; SPEC-09 §4.1.6 | Corrigé : état de premier lancement « non reproduit (D) ». |
| 04B-9 | UNC | SPEC-04B REQ-E3D-455 | Corrigé : `fillrate` fixé par le hook de construction. |
| 04B-10 | UNC | SPEC-04B REQ-E3D-442 | Corrigé : recherche dans `[y, y + k]`, toujours terminée (vérifié). |
| 04B-11 | UNC | SPEC-04B REQ-E3D-443 | Corrigé : boîte sur toute l'image, ouverte aux cases vides (vérifié). |
| 04B-12 | UNC | SPEC-04B REQ-E3D-568 ; SPEC-05 REQ-LUA-083 | Corrigé : écart `OriPos − Pd = (−q.x, h₀ − q.y, 0)` documenté et déclaré voulu (fidélité, vérifié). |
| 04B-13 | UNT | SPEC-04B REQ-E3D-643, 566, 426 | Corrigé : critères propres (harnais). |
| 05-1 | INC | SPEC-07 REQ-LOOP-044 | Corrigé avec T-6. |
| 05-2 | INC | SPEC-05 REQ-LUA-095 ; SPEC-09 REQ-PERS-011 | Corrigé : l'écriture de `Video.SceneScale` déclenche la réinitialisation de vue (comportement vérifié), conformément à REQ-SCN-002. |
| 05-3 | UNT | SPEC-05 REQ-LUA-051, 100, 101 | Corrigé avec T-7. |
| 05-4 | UNT | SPEC-05 REQ-LUA-111 | Corrigé : instrumentation définie (pauses du ramasse-miettes, allocation imputée aux hooks). |
| 05-5 | UNC | SPEC-05 REQ-LUA-004 | Corrigé : garantie normative restreinte aux noms ASCII ; règle recommandée au-delà. |
| 05-6 | UNC | SPEC-05 REQ-LUA-083 | Corrigé : écart d'un pixel `Overlap`/`OverlapRect` confirmé voulu (vérifié). |
| 05-7 | UNC | SPEC-05 §4.2 (`Instance.pattern`, type `Pattern`) | Corrigé : « pattern de frame » défini (grille, instances, durée de vie, `layer`/`geo`/`sizeZ`). |
| 06-1 | INC | SPEC-06 | Corrigé avec T-2 (autrement que proposé : valeurs fidèles). |
| 06-2 | INC | SPEC-06 REQ-SCN-059 | Corrigé : « continue d'être lissée ». |
| 06-3 | INC | SPEC-06 REQ-SCN-030 | Corrigé (DEC-R2-D4) : 1024 texels par défaut. |
| 06-4 | INC | SPEC-06 §4.1 | Corrigé : VSync 0/1/2. |
| 06-5 | INC | SPEC-06 REQ-SCN-043 | Corrigé : renvoi normatif à REQ-E3D-568. |
| 06-6 | INC | SPEC-06 REQ-SCN-034, §5.2 | Corrigé avec 04B-7. |
| 06-7 | UNT | SPEC-06 REQ-SCN-051 | Corrigé : critère « auto » sur cubemaps de test uniformes ; comparaison visuelle étiquetée [R2]. |
| 06-8 | UNT | SPEC-06 §5.4, REQ-SCN-030, 046 | Corrigé avec T-8 ; critères synthétiques ajoutés à 030 et 046. |
| 06-9 | UNC | SPEC-06 REQ-SCN-054 | Corrigé : présence des textures et de la cubemap vérifiée dans `assets-neutral/` ; note ajoutée. |
| 06-10 | UNC | SPEC-06 REQ-SCN-063 ; SPEC-09 §4.1.1 ; SPEC-10 REQ-UI-062 | Corrigé : réglage conservé dans le modèle, absent de l'UI desktop, application facultative. |
| 07-1 | LEAK | SPEC-07 REQ-LOOP-033 | Corrigé : reformulation proposée par R2. |
| 07-2 | INC | SPEC-07 REQ-LOOP-044 | Corrigé avec T-6. |
| 07-3 | INC | SPEC-07 REQ-LOOP-040 | Corrigé avec T-5. |
| 07-4 | INC | SPEC-07 | Corrigé avec T-3. |
| 07-5 | INC | SPEC-07 REQ-LOOP-016 ; SPEC-08 REQ-INP-103 | Corrigé : modèle de SPEC-08 retenu (demandes réarmées, reliquat 0–30 frames), conforme à l'original ; vitesse fidèle ×1,5 (vérifiée) au lieu de ×2. |
| 07-6 | INC | SPEC-07 REQ-LOOP-063 ; SPEC-05 REQ-LUA-075 | Corrigé selon **DEC-R2-D5** (proposition du relecteur — restreindre DEC-07-Q4 aux savestates manuels — non retenue par R0) : `Save`/`Load` appelés par `retro_serialize`/`retro_unserialize`. |
| 07-7 | UNC | SPEC-07 REQ-LOOP-012 | Corrigé : limite en frames émulées, période `1/f` au facteur `k` courant (vérifié). |
| 08-1 | INC | SPEC-08 REQ-INP-041 | Corrigé : DOIT (DEC-08-1, D). |
| 08-2 | INC | SPEC-08 REQ-INP-060 ; SPEC-09 REQ-PERS-012 | Corrigé (DEC-09-Q6, D). |
| 08-3 | INC | SPEC-08 REQ-INP-103 | Corrigé avec 07-5. |
| 08-4 | UNC | SPEC-08 §4.1 | Corrigé : octet du port Zapper (bit 4 = 1 relâchée). |
| 08-5 | UNC | SPEC-08 REQ-INP-063 | Corrigé avec T-2 : conversion explicite et vecteurs §5.3 recalculés. |
| 08-6 | UNC | SPEC-08 REQ-INP-100 | Corrigé : horloge d'affichage, hors périmètre de déterminisme. |
| 08-7 | UNC | SPEC-08 §6 ; SPEC-06 Q6 | Corrigé : absence de conflit de bouton signalée. |
| 09-1 | INC | SPEC-09 §4.1.5 | Corrigé avec T-1. |
| 09-2 | INC | SPEC-09 REQ-PERS-056 | Corrigé avec T-5. |
| 09-3 | INC | SPEC-09 REQ-PERS-057 | Corrigé : aligné sur REQ-UI-051. |
| 09-4 | INC | SPEC-09 REQ-PERS-071 ; SPEC-01 REQ-ARCH-033 | Corrigé avec 01-2. |
| 09-5 | INC | SPEC-09 §4.1.6 | Corrigé avec 04B-8. |
| 09-6 | INC | SPEC-09 REQ-PERS-066 | Corrigé : P2 DOIT (l'original persiste ce dossier). |
| 09-7 | UNC | SPEC-09 §4.1.5 | Corrigé : domaine des marges verticales [0 ; 248]. |
| 09-8 | UNC | SPEC-09 REQ-PERS-005 | Corrigé : renvoi normatif à REQ-PROF-041. |
| 09-9 | UNC | SPEC-09 REQ-PERS-003 | Corrigé : au moins un cœur, exactement un en v1. |
| 10-1 | LEAK | SPEC-10 REQ-UI-011, 122 | Corrigé : dimensions en proportions du cadre et de la surface de référence. |
| 10-2 | INC | SPEC-10 REQ-UI-002, 010 | Corrigé (DEC-10-Q6, D) : onglet initial GAME. |
| 10-3 | INC | SPEC-10 REQ-UI-039, 121, 122, 140 | Corrigé (DEC-10-Q5, D) : bouton Site retiré. |
| 10-4 | INC | SPEC-10 REQ-UI-076 | Corrigé (DEC-10-Q3) : bloc « à propos » réécrit. |
| 10-5 | INC | SPEC-10 REQ-UI-084 ; SPEC-09 REQ-PERS-041 | Corrigé : texte vérifié dans l'original (sans espace) ; SPEC-10 seule source. |
| 10-6 | INC | SPEC-09 REQ-PERS-057 | Corrigé avec 09-3. |
| 10-7 | INC | SPEC-09 REQ-PERS-066 | Corrigé avec 09-6. |
| 10-8 | UNT | SPEC-10 REQ-UI-120, 122 ; SPEC-12 §2 | Corrigé : catégorie « revue » définie dans SPEC-12 et citée ; mesure de couleur ajoutée à REQ-UI-120. |
| 10-9 | UNC | SPEC-10 REQ-UI-002 | Corrigé : clause de fermeture assouplie (REQ-UI-007, 035, 095). |
| 10-10 | UNC | SPEC-10 REQ-UI-062 ; SPEC-09 §4.1.1 | Corrigé : `Graphic` conservé dans le modèle, absent de l'UI desktop. |
| 11-1 | LEAK | SPEC-11 §6 Q5 | Corrigé : reformulation proposée par R2 ; close par DEC-11-Q5. |
| 11-2 | INC | SPEC-11 REQ-AUD-007 | Corrigé avec T-3 (800 / 960). |
| 11-3 | INC | SPEC-11 REQ-AUD-020, 022 | Corrigé (DEC-11-Q2, D) : fondus de durée fixe 200/60 s, critères discriminants. |
| 11-4 | UNC | SPEC-11 REQ-AUD-004 | Corrigé : renvoi à SPEC-02 REQ-CORE-070/072, formule unique. |
| 11-5 | UNC | SPEC-11 REQ-AUD-005 ; SPEC-02 REQ-CORE-070 | Corrigé : le cœur accepte un ajustement fin du rapport (± 0,5 %). |
| 12-1 | UNT | SPEC-12 §2, REQ-ACC-050 | Corrigé : catégories auto / [ROM] / [R2] / revue ; REQ-ACC-050 compatible. |
| 12-2 | INC | SPEC-12 REQ-ACC-031 ; SPEC-04 REQ-E3D-360 | Corrigé (DEC-R2-12-2) : moyenne ≤ 4 ms et 99ᵉ centile ≤ 8 ms. |
| 12-3 | UNC | SPEC-12 §2, §4 | Corrigé : ROM de Quarth attendue de l'utilisateur. |
| 12-4 | UNC | SPEC-12 REQ-ACC-021 | Corrigé : retrait des captures ou masquage des régions affectées par un écart D. |
| D-1 | INC | SPEC-04B REQ-E3D-405 | Corrigé avec T-4. |
| D-2 | INC | SPEC-06 ; DECISIONS (constat R1) | Corrigé avec T-2 (C fidèle = (128, 120, 0), non (128, 128, 0)). |
| D-3 | INC | SPEC-09 | Corrigé avec T-1 (DEC-R2-T1). |
| D-4 | INC | SPEC-06 REQ-SCN-030 | Corrigé avec 06-3 (DEC-R2-D4). |
| D-5 | INC | SPEC-07 REQ-LOOP-063 ; SPEC-05 REQ-LUA-075 | Corrigé avec 07-6 (DEC-R2-D5). |
| D-6 | UNC | SPEC-04B REQ-E3D-560 ; SPEC-06 REQ-SCN-034, §5.2 ; SPEC-04 Q-03 | Corrigé : renvois de propriété appliqués (DEC-04B-Q1, DEC-04-Q03). |
| D-7 | UNC | SPEC-09 REQ-PERS-017 | Corrigé (DEC-R2-D7) : aucun effet. |
| Point 25 (REQ-E3D-521/522) | statut | SPEC-04B REQ-E3D-521, 522 | Corrigé (DEC-R2-521) : numérotation normative, indice de configuration, texel du coin intérieur de plus grand numéro. |

## 3. Constats « borderline » du rapport R2

Acceptés tels quels par R2 ; seule reformulation faite : SPEC-05 REQ-LUA-028 (« la référence … objets réutilisés »), comme suggéré. Les autres (REQ-CORE-013, 032, 040, 081 ; REQ-E3D-323, 503, 523, 524 ; REQ-LOOP-007, 032 ; REQ-SCN-008 ; SPEC-09 §4.4) sont inchangés, sauf la forme de SPEC-09 §4.4 (« journal d'exécution »).

## 4. Points à arbitrer par R0

1. **Visée du Zapper décalée de 8 lignes** (SPEC-08 Q-9) : spécifiée fidèlement ; une divergence (D) viserait le pixel affiché.
2. **Libellé de DEC-07-Q2** : la vitesse fidèle de l'avance rapide est ×1,5 (constat R1 sous DEC-R2-T2) ; la spec suit la fidélité.
3. **Borne de taille du savestate** avec l'état appris de la conversion (REQ-ARCH-013) : une capacité fixe réservée est permise ; sa valeur n'est pas fixée par la spec.
