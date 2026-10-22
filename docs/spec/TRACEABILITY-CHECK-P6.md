# Contrôle de couverture de la matrice de traçabilité (phase P6)

Version : 1.0 | 2026-10-21 | **Documentation seulement**.

Produit par `impl/tools/traceability-check.ps1`, qui relève tous les identifiants
`REQ-<MOD>-NNN` de `spec/*.md` et de `spec/changes/*.md` et les compare à la première
colonne de `verify/round1/traceability.csv`. Voir aussi `SPEC-v1.2-INDEX.md`.

## 1. Résultat

| Grandeur | Valeur |
|---|---|
| Identifiants relevés dans `spec/` et `spec/changes/` | 676 |
| Lignes d'exigence dans la matrice | 673 |
| Présents dans la spec, **absents de la matrice** | 3 dont 0 réel(s) |
| Présents dans la matrice, **absents de la spec** | 0 |
| Listés plus d'une fois dans la matrice | 0 |

**Couverture complète** : chaque exigence de la spécification et des amendements a exactement une ligne dans la matrice, et la matrice ne contient aucun identifiant qui ne soit pas dans la spécification.

## 2. Différences relevées

| Identifiant | Différence | Cité dans | Lecture |
|---|---|---|---|
| `REQ-E3D-512` | absente de la matrice | SPEC-04B-geometrie.md, Q-063.md | Renvoi mort : REQ-E3D-406 cite « REQ-E3D-504, REQ-E3D-512 », mais REQ-E3D-512 n'est définie nulle part (le maillage local d'une tuile est REQ-E3D-500 à -507 et -520 à -524). Sans effet normatif ; consigné dans `changes/Q-063.md`. Le texte gelé n'est pas modifié. |
| `REQ-INP-03` | absente de la matrice | AMD-020-q-tickets-p4c.md | Faux positif : la spec écrit `REQ-INP-03x` pour désigner la famille REQ-INP-030/031 (AMD-020, Q-048). Ce n'est pas un identifiant. |
| `REQ-INP-04` | absente de la matrice | AMD-015-q-tickets-l8.md | Faux positif : la spec écrit `REQ-INP-04x` pour désigner la famille de la capture, REQ-INP-042/045/047 (AMD-015, Q-033). Ce n'est pas un identifiant. |

## 3. Répartition de la matrice par document

| Famille | Document | Lignes |
|---|---|---|
| REQ-ACC-… | SPEC-12 | 11 |
| REQ-ARCH-… | SPEC-01 | 14 |
| REQ-AUD-… | SPEC-11 | 18 |
| REQ-CORE-… | SPEC-02 | 65 |
| REQ-E3D-… | SPEC-04 / SPEC-04B | 138 |
| REQ-INP-… | SPEC-08 | 65 |
| REQ-LOOP-… | SPEC-07 | 42 |
| REQ-LUA-… | SPEC-05 | 72 |
| REQ-PERS-… | SPEC-09 | 61 |
| REQ-PROF-… | SPEC-03 | 38 |
| REQ-SCN-… | SPEC-06 | 65 |
| REQ-UI-… | SPEC-10 (+ AMD-010) | 84 |
| **Total** | | **673** |

## 4. Reproduire

```
powershell -File impl/tools/traceability-check.ps1 -Report spec/TRACEABILITY-CHECK-P6.md
```
