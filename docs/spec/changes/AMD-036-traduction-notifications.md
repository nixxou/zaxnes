# AMD-036 — Traduction des notifications à texte fixé (clôt Q-075)

Version : 1.0 | 2026-10-23 | **Normatif** (R0). Complète AMD-035 ; modifie la lecture de SPEC-10 REQ-UI-082 à REQ-UI-084, SPEC-09 REQ-PERS-022, REQ-PERS-024, REQ-PERS-042, REQ-PERS-054 et AMD-030 §5.

## 1. Règle

- Les textes que SPEC-10 et SPEC-09 fixent « mot pour mot » deviennent les **entrées anglaises** de la table des textes, inchangées (y compris les particularités de l'original, comme les deux espaces de REQ-UI-084). Chacun reçoit une **traduction française** dans la table française.
- Les critères d'acceptation de ces exigences s'entendent **avec la langue `en`** (valeur par défaut) : rien ne change pour eux.
- Les deux questions de confirmation (REQ-PERS-042, REQ-PERS-054) et leurs boutons suivent la même règle : `Yes` / `No` en anglais, `Oui` / `Non` en français.
- **Résumé d'import de 3dSen (AMD-030 §5)** : `--import-3dsen` l'imprime **en anglais**, toujours (AMD-035 §2). La notification affichée dans l'interface utilise la langue de l'interface. Le texte anglais est la traduction directe du texte français d'AMD-030 §5, qui devient l'entrée française.
- Les journaux de la console restent en anglais (AMD-035 §2).

## 2. Critères d'acceptation

- En `en`, chaque notification à texte fixé est identique, caractère par caractère, au texte de la spec.
- En `fr`, aucune de ces notifications n'apparaît en anglais (hors noms propres et chemins).
- La sortie de `--import-3dsen` est en anglais quel que soit `ui.language`.
- Le test « mêmes clés, aucune valeur vide » couvre les nouvelles entrées.
