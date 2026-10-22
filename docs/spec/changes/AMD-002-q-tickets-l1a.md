# AMD-002 — Réponses aux tickets du lot L1a

Version : 1.0 | 2026-10-20 | **Normatif** (complète la spec v1.1 gelée)

| Ticket | Document | Amendement |
|---|---|---|
| Q-001 | SPEC-02 REQ-CORE-011, -091 | Les phrases entre guillemets français sont des descriptions de cause. Seules les chaînes en `police de code` sont littérales. Le cœur renvoie une cause typée et un message technique ; l'hôte choisit le texte affiché. |
| Q-002 | SPEC-02 REQ-CORE-011 ; SPEC-07 | Cœur : instance laissée sans ROM après un échec de chargement. **Nouvelle exigence REQ-LOOP-018 (P2)** : l'ouverture d'une ROM depuis une partie en cours DOIT valider le fichier avant d'interrompre la partie. En cas d'échec, la partie en cours continue (dans l'état où elle était, en pause si le menu était ouvert) et le message d'erreur de SPEC-07 s'affiche. Critère d'acceptation : partie en cours, ouverture d'un fichier tronqué → message d'erreur, puis la partie reprend avec la RAM inchangée. |
| Q-003 | SPEC-02 REQ-CORE-015, -016 | La tolérance de durée de frame s'applique à partir de la deuxième frame qui suit un changement de région. La première ne doit pas dépasser deux frames nominales de la nouvelle région. |
