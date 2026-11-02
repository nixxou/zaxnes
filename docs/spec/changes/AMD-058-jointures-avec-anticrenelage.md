# AMD-058 — Jointures sans fissure quand l'anticrénelage est actif

Version : 1.0 | 2026-11-01 | **Normatif** (R1, suite de la vérification de V2-29 dans la passe R5). **Complète SPEC-06 REQ-SCN-065** (AMD-012) et **AMD-032 §2** (anticrénelage, divergence D). AMD-057 est inchangé.
Priorité : P2.

## 1. Constat

Après V2-29, les fines lignes claires d'un pixel à la jointure de tuiles voisines restent visibles, sans changement, sur la façade de la tour de l'intro de Mega Man 2 (frame 599) et sur le mur bleu derrière la piste d'Excitebike (frame 599 après Start aux frames 200 et 300). Mesures en 2560 × 1440, style Classic :

- avec `video.anti_alias` = 1 (MSAA 4x, valeur d'usine d'AMD-052) : lignes claires horizontales et verticales au pas de la tuile ;
- avec `video.anti_alias` = 0, **même binaire, même scène** : aucune ligne, sur les deux scènes ;
- la référence n'en montre aucune. Elle rend toujours sans anticrénelage (AMD-032 §1).

Le défaut vient donc du seul chemin anticrénelé. Ni les parois conservées par AMD-057, ni la géométrie, ni l'éclairage n'en sont la cause, puisque le rendu sans anticrénelage est propre.

## 2. Exigence

**REQ-SCN-065** (complétée, P2) — L'exigence de REQ-SCN-065 DOIT tenir pour **chaque** valeur de `video.anti_alias` (0, 1 et 2) : ni grille sombre, ni fissure claire, ni discontinuité d'ombre à la jointure de deux tuiles voisines d'une surface plane, que ces tuiles appartiennent au même objet ou à deux objets différents de même profondeur et de même couche. Les moyens sont **libres** (traitement des échantillons, de la profondeur des parois internes, de la résolution des échantillons, etc.), à l'exclusion :
- du retrait des parois (AMD-057) ;
- de la désactivation de l'anticrénelage ou de la baisse du nombre d'échantillons : le réglage choisi doit garder son effet sur les bords des objets (AMD-032 §2).

- Critère d'acceptation (auto) : le critère de REQ-SCN-065 (surface de 4 × 4 tuiles identiques, éclairée, recevant l'ombre) passe avec `video.anti_alias` = 0, 1 et 2.
- Critère d'acceptation (auto) : la même surface faite de tuiles appartenant à **deux objets** voisins (deux colonnes de 2 × 4 tuiles) passe aussi, pour les trois valeurs.
- Critère d'acceptation (revue) : façade de la tour de Mega Man 2 (frame 599, sans entrée) et mur bleu d'Excitebike (frame 599, Start aux frames 200 et 300), rendus en 2560 × 1440 avec `video.anti_alias` = 1 : aucune ligne claire ; la capture avec anticrénelage diffère de celle sans anticrénelage seulement au bord des objets (contours adoucis).

## 3. Notes

- L'écart reste une divergence D assumée (la référence rend sans anticrénelage) ; le présent amendement exige seulement qu'elle n'ajoute pas de défaut.
- Hors de ce lot : la teinte de la face latérale des bottes de foin d'Excitebike (cylindres `HCYLINDER`). Géométrie et éclairage y sont identiques à la référence ; seul le texel retenu pour la face de bout diffère (référence : le texel clair, Zaxnes : le texel orange foncé). À instruire séparément.

## Historique

- 1.0 (2026-11-01) : rédaction après la comparaison avec et sans anticrénelage (`verify/delta-v2/R5-report.md`, « Cause des fissures claires »).
