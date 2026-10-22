# AMD-013 — Réponses aux tickets du lot L6

Version : 1.0 | 2026-10-21 | **Normatif** (complète la spec v1.1 gelée)

| Ticket | Document | Amendement |
|---|---|---|
| Q-027 | SPEC-07 / SPEC-09 | Vignette d'un emplacement sauvegardé en cours de jeu : choix provisoire validé. |
| Q-028 | SPEC-07 REQ-LOOP-023 | `.zip` pris en charge ; `.7z` refusé avec `Uncompression is not successful` (pas de décodeur autorisé). |
| Q-029 | SPEC-07, AMD-010 | En pause, la scène reste affichée, figée sur la dernière frame ; à l'arrêt, écran d'accueil (REQ-UI-207). |
| Q-030 | AMD-010 REQ-UI-204 | Charger l'état n : **Ctrl+Fn** ; sauvegarder : Maj+Fn ; Fn seul reste aux touches caméra de SPEC-08. AMD-010 est corrigé. |
| Q-031 | SPEC-07 REQ-LOOP-017, DEC-03-Q01 | Région automatique inactive : ordre fidèle (le profil l'emporte). Région automatique active : la région détectée depuis la ROM l'emporte sur le `TVMode` du profil (choix provisoire validé, **D**). |
