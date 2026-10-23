# AMD-037 — Profils utilisateur sans identifiant (import 3dSen)

Version : 1.0 | 2026-10-23 | **Normatif** (R0, décision de l'utilisateur). Complète SPEC-03 REQ-PROF-062 et AMD-030 §5 ; s'applique aux deux dossiers de profils d'AMD-033.

## 1. Constat

- Dans 3dSen, l'identité d'un jeu intégré vient du catalogue (REQ-PROF-060), pas du profil : sur les 101 profils intégrés, 56 portent un champ `Id` et 45 n'en ont pas (REQ-PROF-022).
- L'import depuis 3dSen (AMD-030) écrit les 101 profils tels quels. Au démarrage suivant, REQ-PROF-062 ignore les 45 fichiers sans `Id` (« le profil ne porte pas d'identifiant ») : ces jeux n'apparaissent pas.
- Mesuré sur une installation réelle, avec la 0.2.0 C# comme avec le portage Rust : 101 profils importés, 45 ignorés au chargement.

## 2. Règle

- **Identifiant dérivé.** Lors de la lecture d'un profil utilisateur (REQ-PROF-062, deux dossiers d'AMD-033), si le champ `Id` est absent ou vide, le jeu reçoit un **identifiant dérivé de son contenu** au lieu d'être ignoré :
  - on calcule l'empreinte SHA-256 de **tous les octets du fichier** ;
  - on encode ses **9 premiers octets** en base64 standard (alphabet `A–Z a–z 0–9 + /`, sans remplissage) : on obtient 12 caractères, le même format que les identifiants de REQ-PROF-052 ;
  - cet identifiant sert partout où sert l'identifiant d'un profil utilisateur : clé de `rom.json` (REQ-PROF-064), couverture, sauvegardes, suppression (REQ-PROF-065), détection des doublons.
- **Aucune réécriture.** Le fichier n'est jamais modifié : l'identifiant est recalculé à chaque lecture. AMD-030 §5 reste vrai (profils écrits sans aucune modification, identiques à l'inventaire de référence).
- **Profil identique à un profil intégré.** Un profil utilisateur dont les octets sont identiques à ceux d'un profil intégré du catalogue en usage (REQ-PROF-060) n'ajoute pas de jeu : le jeu intégré, avec ses métadonnées de catalogue, est montré une seule fois. Le fichier utilisateur est signalé une fois dans le journal. Cette règle vaut que le profil ait un `Id` ou non.
- **Titre.** Inchangé : `Title`, sinon le nom du fichier sans extension (REQ-PROF-062).
- **Import d'un fichier `.3dn` isolé (REQ-PROF-063).** Inchangé : un fichier sans `Id` est refusé, comme dans 3dSen. La présente règle ne concerne que les fichiers déjà présents dans un dossier de profils, dont ceux écrits par l'import 3dSen.

## 3. Ce qui ne change pas

- Les profils qui ont un `Id` gardent cet `Id`.
- Les collisions d'identifiant entre deux profils utilisateur suivent REQ-PROF-062 et AMD-033 (le premier lu gagne ; à identifiant égal entre les deux dossiers, celui de l'utilisateur gagne).
- Un identifiant dérivé ne peut pas recouvrir un identifiant de catalogue : ceux-ci sont courts (`"1"` à `"101"`) alors que l'identifiant dérivé fait 12 caractères.

## 4. Critères d'acceptation

- **Sans installation réelle** (tests unitaires) :
  - un profil synthétique sans `Id` déposé dans le dossier de l'utilisateur apparaît dans la bibliothèque, avec l'identifiant égal aux 12 caractères base64 des 9 premiers octets du SHA-256 de ses octets ; relu après redémarrage, il garde le même identifiant ;
  - le même profil sans `Id` copié sous un autre nom n'apparaît qu'une fois (contenu identique, AMD-033) ;
  - un profil utilisateur identique à un profil intégré du catalogue n'ajoute pas de jeu ;
  - l'import d'un fichier `.3dn` isolé sans `Id` reste refusé (REQ-PROF-063).
- **Sur une installation réelle de 3dSen** (test facultatif, ignoré si l'installation est absente) :
  - sans dossier d'assets : après l'import, les 101 profils apparaissent dans la bibliothèque, aucun n'est ignoré pour absence d'identifiant ;
  - avec le dossier d'assets du catalogue : aucun jeu n'apparaît en double.

## 5. Questions

- Aucune. Q-076 n'est pas ouverte : la décision est prise par l'utilisateur (2026-10-23).
