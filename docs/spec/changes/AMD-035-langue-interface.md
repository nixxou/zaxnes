# AMD-035 — Langue de l'interface : anglais ou français

Version : 1.0 | 2026-10-23 | **Normatif** (R0, demande de l'utilisateur). Complète SPEC-10 (interface) et SPEC-09 (réglages).

## 1. Règle

- Tous les textes **de l'émulateur** existent en **anglais** et en **français** : menus, fenêtre d'options, boîtes de dialogue, écran d'accueil, notifications, messages d'erreur affichés, info-bulles, fenêtre « À propos », configuration des touches.
- Un nouveau réglage `ui.language` (portée utilisateur, persisté) prend les valeurs `en` (« English ») ou `fr` (« Français »). **La valeur par défaut est `en`.**
- **Aucune détection de la langue du système** : seule la valeur du réglage compte, et au premier lancement c'est l'anglais.
- Le réglage se trouve dans la fenêtre d'options (onglet Interface). Le changement s'applique **immédiatement**, sans redémarrer.
- Une valeur inconnue dans le fichier de réglages est ramenée à `en`, sans erreur.

## 2. Hors périmètre

- Les journaux de la console (préfixe « Zaxnes: »), la sortie de `--help` et les messages de la ligne de commande restent en anglais.
- La documentation (README, HOWTO-RUN, notices), la spec et les tickets ne sont pas traduits.
- Les textes qui viennent des profils (titres, descriptions) ou des scripts Lua sont affichés tels quels.

## 3. Organisation

- Les textes sont regroupés dans une table par langue, avec une clé par texte. Une clé absente dans une langue retombe sur l'anglais : jamais de texte vide ni de clé brute affichée.
- Les textes français existants servent de base pour le français. L'anglais est écrit à partir d'eux, dans un style court et naturel.
- La police d'interface couvre déjà les accents ; il n'y a rien à changer de ce côté.

## 4. Critères d'acceptation

- **Premier lancement**, avec un dossier utilisateur vierge : l'interface est entièrement en anglais.
- **Passage à « Français »** dans les options : toute l'interface bascule immédiatement, et le choix est conservé au lancement suivant.
- **Les deux tables sont complètes** : un test vérifie que les deux langues ont exactement les mêmes clés et qu'aucune valeur n'est vide.
- **Captures d'interface** (`--ui-shots`) produites dans les deux langues.
