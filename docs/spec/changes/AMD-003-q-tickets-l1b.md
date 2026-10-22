# AMD-003 — Réponses aux tickets du lot L1b

Version : 1.0 | 2026-10-20 | **Normatif** (complète la spec v1.1 gelée)

| Ticket | Document | Amendement |
|---|---|---|
| Q-004 | SPEC-02 REQ-CORE-037, -038 | Le scroll d'une rangée capturée est celui du bord gauche de la ligne de capture, lu au début de cette ligne avec fx, les nametables, le mirroring et les banques CHR. La tolérance d'une ligne sur un split est symétrique (une ligne plus tôt ou plus tard). |
| Q-005 | SPEC-02 REQ-CORE-062 | La ligne de pré-rendu appartient à l'image suivante. Sur un recouvrement de fond, le dernier enregistrement capturé l'emporte. La limite x < 239 de la référence est optionnelle (P3). |
