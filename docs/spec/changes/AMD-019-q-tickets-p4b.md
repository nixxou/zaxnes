# AMD-019 — Réponses aux tickets du round 1, partie B

Version : 1.0 | 2026-10-21 | **Normatif**

| Ticket | Exigence | Amendement |
|---|---|---|
| Q-044 | voir ticket | Protocole du vérificateur validé : le nombre d'échantillons par frame se mesure comme une moyenne sur une fenêtre d'au moins 600 frames, indépendamment de la taille du tampon de lecture. |
| Q-045 | voir ticket | La courbe sRGB de SPEC-06 §2 fait foi : la valeur exacte est celle que donne la courbe (≈ 4,672), et « ×2 » n'était qu'une approximation. Tolérance de ±2 % sur ce critère. |
| Q-046 | voir ticket | « Cas dégénéré » : l'œil confondu avec l'un des sommets (au moins une des deux distances nulle). Dans ce cas, le visé corrigé est le milieu du segment. La lecture du vérificateur est validée ; l'implémentation doit être corrigée. |
| Q-047 | voir ticket | Budgets de REQ-LUA-110/111 et REQ-SCN-064 : partie « auto » mesurée sur une trame pleine synthétique, selon le protocole de Q-042 ; la mesure sur les jeux de référence est étiquetée [ROM]. REQ-SCN-064 reçoit l'étiquette [ROM] pour cette partie. |
