# AMD-023 — Réponses aux tickets de la vérification finale

Version : 1.0 | 2026-10-21 | **Normatif**

| Ticket | Amendement |
|---|---|
| Q-060 | Les observables proposés par le vérificateur pour les critères [ROM] décrits comme une inspection humaine sont validés : ils deviennent des mesures automatiques (frames converties, absence de plantage, statistiques de shapes et de pixels), complétées par des captures archivées pour revue. |
| Q-061 | Validé : faute de ROM NES de Quarth, la part MARCHING de REQ-ACC-020 est couverte par des jeux de référence rendus avec l'algorithme MARCHING forcé en réglage global (Tetris, SMB, Metroid). Quarth reste à vérifier si une ROM Famicom devient disponible. |
| Q-062 | Ce plantage est traité comme un défaut à corriger avant la clôture (P6), et non comme un simple risque : une exception, même rare, est incompatible avec REQ-ACC-041 et REQ-ACC-020. Le lot de clôture doit le reproduire (exécutions répétées du banc, éventuellement avec un rendu sur un autre fil que l'émulation), en identifier la cause (accès concurrent, cycle de vie d'un objet, pilote GL) et le corriger, ou démontrer de façon probante qu'il est extérieur au programme. |
