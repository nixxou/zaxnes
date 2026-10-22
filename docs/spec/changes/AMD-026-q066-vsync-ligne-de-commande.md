# AMD-026 — Réponse à Q-066 : `video.vsync` = 2 et options de ligne de commande

Version : 1.0 | 2026-10-22 | **Normatif** (R0)

- **`video.vsync` = 2** : l'hôte demande un intervalle d'échange de 2. Si la plateforme ne permet pas cet intervalle, il se replie sur 1 (jamais sur 0) et journalise ce repli. `Setting.TargetFramerate` (REQ-PERS-013, point 1) reste calculé à partir de la valeur demandée. Le choix provisoire de Q-066 §2.1 est adopté tel quel.
- **Options de ligne de commande portant sur un réglage persisté** (`--fov`, `--no-shadows`, `--skybox`, `--tv`, `--no-render-skip`, `--fps`, `--vsync`) : une option non passée n'a aucun effet. Une option passée est une **surcharge de session**. Elle n'est jamais écrite dans le fichier de réglages et disparaît pour une clé dès que l'utilisateur change lui-même ce réglage. Une écriture de script (REQ-PERS-010) la masque le temps de la partie. Le choix provisoire de Q-066 §2.2 est adopté tel quel.
- **Écriture du fichier de réglages** : retirer ou non une clé du fichier se décide sur la seule couche de préférence, jamais sur la valeur en vigueur.
