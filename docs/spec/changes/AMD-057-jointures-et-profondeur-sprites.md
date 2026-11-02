# AMD-057 — Parois entre tuiles voisines conservées, profondeur des sprites par indice

Version : 1.0 | 2026-11-01 | **Normatif** (R1, suite du point 3 de la passe R5 : comparaison Classic sur dix jeux). **Retire AMD-027 §1 et §2** (parois latérales omises du maillage dessiné) ; **amende SPEC-06 REQ-SCN-037** et la ligne « Décalage des sprites » du tableau des constantes de SPEC-06 ; SPEC-04B REQ-E3D-405 suit (renvoi). REQ-SCN-065 (AMD-012) est **inchangée** et reste le critère des jointures.
Priorité : P2.

## 1. Constat

Comparaison en boîte noire avec 3dSen 2.0, style Classic, sur Super Mario Bros. 3, The Legend of Zelda, Castlevania, Mega Man 2, Metroid, Contra, Excitebike, Ninja Gaiden III, Zelda II et Blaster Master (21 captures, mêmes frames, mêmes profils ; maillages relevés tuile par tuile) :

1. **Maillages des tuiles.** Sur 9 866 tuiles dessinées, 4 899 ont dans Zaxnes les mêmes bornes que la référence mais **moins de sommets** (toujours un multiple de 4 en moins : 8, 12, 16 ou 20 au lieu de 24 pour une tuile pleine). Ce sont les parois latérales qu'AMD-027 retire entre deux tuiles voisines du même objet. La référence dessine **toutes** les faces de REQ-E3D-501 : un maillage de tuile fermé sur lui-même, y compris entre deux tuiles pleines voisines.
2. **Fissures.** Sur les grandes faces planes faites de tuiles voisines, Zaxnes montre de fines lignes claires d'un pixel le long de certaines jointures : façade de la tour de l'intro de Mega Man 2 (frame 599), mur bleu derrière la piste d'Excitebike (frame 599). La référence ne montre ni ces fissures ni la grille sombre qui avait motivé AMD-027 (Q-067 : herbe de Zelda II, piste d'Excitebike), alors qu'elle garde les parois.
3. **Profondeur des sprites.** La référence ne décale pas les tuiles de sprite de ±0,1 exactement : le décalage dépend de l'indice du sprite. Mesure : tuiles de sprite d'indices 5, 9, 20, 24, 35, 39, 50, 54 (Contra, frame 599) à z = −0,095, −0,091, −0,080, −0,076, −0,065, −0,061, −0,050, −0,046 ; la moitié basse d'un 8×16 (indice 64 + n) a le même décalage que sa moitié haute.

## 2. Exigences

**Parois entre tuiles** (retire AMD-027 §1-2). Le maillage **dessiné** d'une tuile DOIT être son maillage de REQ-E3D-501, toutes faces comprises : aucune paroi latérale n'est omise au motif qu'une tuile voisine la recouvre. REQ-SCN-065 reste exigée : ni grille sombre, ni fissure claire, ni discontinuité d'ombre à la jointure de deux tuiles voisines d'une surface plane. Les moyens d'y parvenir avec des parois confondues (traitement de la profondeur, de l'éclairage ou de la rastérisation des parois internes) sont **libres**, à l'exclusion du retrait des parois.
- Critère d'acceptation (auto) : `--dump-meshes` d'une scène où deux tuiles pleines du même objet se touchent → chacune a 24 sommets (boîte complète).
- Critère d'acceptation (auto) : le critère de REQ-SCN-065 (surface de 4 × 4 tuiles identiques, éclairée, recevant l'ombre) passe avec les parois conservées.
- Critère d'acceptation (revue) : façade de la tour de Mega Man 2 (frame 599 de l'intro, sans entrée) et mur bleu d'Excitebike (frame 599 après Start aux frames 200 et 300) sans ligne claire ; herbe de Zelda II et piste d'Excitebike sans grille sombre.

**REQ-SCN-037** (amendée, P2) — Le décalage de profondeur d'une tuile de sprite DOIT valoir `n = 0,001 × (i mod 64) − 0,1` pour un sprite devant le fond et `n = 0,001 × (i mod 64) + 0,1` pour un sprite « derrière le fond », où `i` est l'**indice du sprite** de REQ-CORE-043 (0 à 63, ou 64 + n pour la moitié basse d'un 8×16). Le décalage s'applique **par tuile**, au même endroit qu'avant (repère local, avant échelle et rotation, une fois). Les tuiles de fond ne sont pas décalées. Conséquence : à recouvrement égal, le sprite d'indice le plus faible — le plus prioritaire sur la NES — est devant.
- Critère d'acceptation (auto) : TV-W2 devient : face avant d'un sprite d'indice 0 en L1 FRONT à z = −0,1 ; d'indice 5, à −0,095 ; moitié basse d'indice 69, à −0,095.
- Critère d'acceptation (auto) : deux sprites plats qui se recouvrent, indices 3 et 10 → le sprite 3 est dessiné devant, sans scintillement.

## 3. Notes

- AMD-027 §3 (carte d'ombre sur les maillages complets) est sans objet : les maillages dessinés sont désormais les maillages complets.
- La teinte des faces latérales des bottes de foin d'Excitebike (frame 599) diffère aussi de la référence ; elle est à remesurer après ce lot, les parois pouvant en être la cause.

## Historique

- 1.0 (2026-11-01) : rédaction après la comparaison R5 sur dix jeux (`verify/delta-v2/R5-report.md`, « Point 3 »).
