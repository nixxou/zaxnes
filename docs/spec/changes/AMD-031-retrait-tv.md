# AMD-031 — Retrait du boîtier TV

Version : 1.0 | 2026-10-22 | **Normatif** (R0, décision de l'utilisateur). Remplace AMD-025 et les exigences du boîtier TV et de son tunnel (SPEC-06, AMD-021 pour la partie TV, REQ-PUB-004).

## 1. Décision

Le boîtier TV décoratif est **retiré du produit** : plus de réglage, plus de modèle, plus de rendu du boîtier ni de son tunnel. La scène se compose de l'image du jeu (2D ou 3D) et du fond (skybox ou couleur).

## 2. Ce qui disparaît

- Le réglage TV dans l'interface : fenêtre d'options, menus et raccourcis.
- L'option de ligne de commande `--tv`, de l'application comme du harnais. Une option inconnue suit la règle générale des options inconnues.
- Le modèle CC BY « Retro Crt Tv » : le fichier ne doit plus être livré ni cherché, et il faut retirer son attribution de `ASSETS-LICENSES.md` ainsi que les étapes de copie de `publish.ps1`.
- Le chargeur glTF et le décodeur JPEG, qui ne servaient qu'au boîtier, ainsi que le placement du modèle, le tunnel et le découpage de l'image par le boîtier. Le décodeur PNG reste : il sert à l'icône.
- La documentation correspondante dans le README, `HOWTO-RUN.md` et `THIRD-PARTY-NOTICES.md`.

## 3. Compatibilité (obligatoire)

- **Profils** : le champ du boîtier TV des profils (SPEC-03, `tvBox`, champ 35) reste **lu et réécrit à l'identique**. L'aller-retour d'un profil doit rester exact, octet pour octet, mais la valeur n'a plus aucun effet.
- **Réglages persistés** : une valeur enregistrée pour la clé du boîtier TV est ignorée sans erreur au chargement, et elle n'est plus réécrite.
- **Scripts Lua** : une lecture ou une écriture du réglage TV par un script ne provoque pas d'erreur. La lecture rend toujours « aucun », et l'écriture n'a aucun effet.

## 4. Critères d'acceptation

- Aucune occurrence du boîtier TV dans l'interface ni dans l'aide de la ligne de commande. `git grep -i -e "tv_box" -e "tvbox" -e "television" -e "retro_crt" -e "gltf" -e "jpeg"` ne trouve plus que les éléments de compatibilité du §3 et leurs tests.
- La version publiée ne contient plus de dossier `assets/tv`.
- L'aller-retour des 101 profils reste identique, octet pour octet.
- Les profils et les scripts qui manipulent le réglage TV continuent de fonctionner sans erreur.
