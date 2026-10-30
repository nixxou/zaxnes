# Rapport de vérification — round 1, V2, lot v2-1-import

**Rôle** : R4, vérificateur indépendant, côté propre
**Date** : 2026-10-30
**Branche vérifiée** : `delta/v2-1-import`, commit `c5d9f6a` (« V2-1: import the 3dSen 2.0
profile bundle (AMD-039) »)
**Texte normatif** : `spec/changes/AMD-039-import-3dsen-2.md` v1.1 du 2026-10-30, qui amende
`AMD-030` §3, §4 et §6. C'est bien la dernière version : AMD-039 n'a **pas** de v1.2 — la
seconde relecture R2 l'a accepté en v1.1 sans réserve de fond, contrairement à AMD-040, 041,
043 et 045, passés en v1.2. Relectures R2 lues pour le raisonnement seulement :
`R2-review-AMD-039-045.md` (v1.0) et `R2-review-AMD-039-045-v1.1.md` (AMD-039 **accepté**,
5 réserves levées, 1 partielle de housekeeping).
**Tests écrits pour cette vérification** : `impl/tests/Acceptance/R4V2ImportTests.cs`,
14 tests (13 exécutés, 1 ignoré faute d'installation 2.0). Les tests de l'implémenteur n'ont
pas été modifiés.
**Méthode** : les structures du bundle (en-tête, alignements, blocs LZ4, fichier sérialisé,
objets `TextAsset`) sont construites par le test à partir de la description publique
d'AMD-030 §4 et d'AMD-039 §2, jamais à l'aide du lecteur vérifié ; les charges utiles sont des
profils SPEC-03 écrits par le test, de sorte que les octets qui ressortent se comparent aux
octets entrés.

---

## 1. Décompte par verdict

AMD-039 n'introduit **aucun identifiant `REQ-`** : il énonce un changement de format (§2), une
liste de ce qui ne change pas (§3) et cinq critères d'acceptation (§4), dont deux facultatifs.
Le tableau porte donc une ligne par énoncé vérifiable, numéroté par paragraphe.

| Verdict | Lignes |
|---|---|
| **conforme** | 14 |
| **non conforme** | 0 |
| **non testable** | 2 |
| **ambigu** | 1 |
| **Total** | **17** |

Aucune non-conformité. Deux observations mineures en §4.

---

## 2. Tableau de conformité

Commande commune à toutes les preuves :

```
dotnet test tests/Acceptance/Zax.Acceptance.Tests.csproj -c Release \
  --filter "FullyQualifiedName~R4V2ImportTests"
→ Passed! - Failed: 0, Passed: 13, Skipped: 1, Total: 14
```

Les noms de test ci-dessous sont ceux de `Zax.Acceptance.Tests.R4V2ImportTests`.

| # | Énoncé (AMD-039) | Verdict | Preuve |
|---|---|---|---|
| §2-1 | Version de format **8** acceptée ; AMD-030 §4.1 accepte toujours 6 à 8 et rien d'autre | conforme | `Amd039_format_versions_6_to_8_are_accepted_and_9_is_refused` (format 6 lu sans alignement, format 9 refusé avec un message citant « 9 ») ; `Amd039_the_serialized_file_is_read_in_version_22_with_its_64_bit_fields` (`FormatVersion` = 8) |
| §2-2 | Version du moteur `6000.3.21f1` lue, sans effet sur la lecture | conforme | `Amd039_the_serialized_file_is_read_in_version_22_with_its_64_bit_fields` : `EngineVersion` = `6000.3.21f1`, l'en-tête finit bien à 50 comme en 1.1.2 (les deux chaînes font 11 octets) |
| §2-3 | Drapeaux **0x243** : avec le bit 0x200 et le bloc d'information après l'en-tête (0x80 absent), les blocs de données commencent à la position suivante alignée sur 16 | conforme | `Amd039_c1_a_format_8_bundle_with_bit_0x200_gives_the_expected_text_assets` : en-tête fini à 50, bloc d'information lu à 64, fini à une position ≡ 5 (mod 16) comme la mesure (1493), premier bloc de données 11 octets plus loin et multiple de 16 (comme 1504) |
| §2-4 | Le bourrage se compte après les octets **compressés** du bloc d'information (champ de l'en-tête) | conforme | même test : la taille compressée du bloc d'information (LZ4HC) diffère de sa taille décompressée ; les trois `TextAsset` ne ressortent que si la position est prise après les octets compressés |
| §2-5 | « la position suivante alignée sur 16 » est la position elle-même quand elle est déjà alignée (lecture imposée par la 2ᵉ puce : « position déjà multiple de 16 ») | conforme | `Amd039_an_information_block_ending_on_a_boundary_needs_no_padding` : bloc d'information fini sur un multiple de 16 → aucun octet de bourrage, assets lus |
| §2-6 | Avec le bit **0x80**, le bit 0x200 est **sans effet** | conforme | `Amd039_bit_0x200_has_no_effect_when_the_information_block_is_at_the_end` : le même bundle avec et sans le bit donne les mêmes assets ; `Amd039_c3_…` : données à 64, juste après l'en-tête aligné |
| §2-7 | « les autres bits de drapeaux restent sans effet sur la lecture » | conforme | `Amd039_a_header_bit_the_amendments_do_not_name_is_without_effect` : 0x243 \| 0x100 lit exactement comme 0x243 |
| §2-8 | Fichier sérialisé **version 22** (champs 64 bits) rencontré et lu ; AMD-030 §4.3 accepte 17 à 22 | conforme | `Amd039_the_serialized_file_is_read_in_version_22_with_its_64_bit_fields` : le test laisse les trois champs 32 bits de l'en-tête à **zéro** et ne renseigne que les champs 64 bits ; un lecteur qui les ignorerait ne trouverait aucun objet. `Amd039_a_serialized_file_of_version_23_is_refused_with_a_message` |
| §2-9 | Disposition exacte des champs repris en version 22 (largeurs, champ réservé) | **ambigu** | Q-071 (classe B, clos sur un choix provisoire) : AMD-030 §4.3 « ces tailles et décalages sont repris en champs 64 bits » autorise 24 octets (3 × i64) ou 28 (u32 + i64 + i64 + i64 réservé, format public). AMD-039 §2 déclare la branche vivante **sans trancher la disposition**. L'implémentation garde le format public ; mes tests le reprennent, donc ils ne départagent pas. Voir §4, observation O-1 |
| §2-10 | Plateforme du fichier sérialisé **19** : ignorée | conforme | `Amd039_the_serialized_file_is_read_in_version_22_with_its_64_bit_fields` (`TargetPlatform` = 19, lecture inchangée) ; `Amd039_s3_…` avec la plateforme 5 |
| §2-11 | Motif de chemin **inchangé** en 2.0 : `…_Data/StreamingAssets/aa/StandaloneWindows64/defaultprofiles_assets_all_*.bundle` | conforme | `Amd039_the_2_0_installation_layout_imports_and_a_second_import_adds_nothing` : le bundle est trouvé depuis le dossier d'installation, `BundlePath` contient `StandaloneWindows64` |
| §3 | Une installation 1.1.2 s'importe **comme avant** (format 7, `2019.4.41f2`, 0x43, fichier sérialisé 21, plateforme 5) | conforme | `Amd039_s3_a_1_1_2_shaped_bundle_still_gives_the_expected_text_assets` (aucun bourrage : données collées au bloc d'information) ; et sur le bundle réel de `verify/p8-input`, `Amd039_c5_…` |
| §4-C1 | Bundle synthétique format 8 / 0x243 dont le bloc d'information compressé ne finit pas sur un multiple de 16 → les `TextAsset` attendus | conforme | `Amd039_c1_a_format_8_bundle_with_bit_0x200_gives_the_expected_text_assets` : 3 assets, noms exacts, charges utiles **octet pour octet** celles entrées, chacune relue par le décodeur SPEC-03 |
| §4-C2 | Le même bundle sans le bit 0x200, le bourrage toujours devant les données → **n'importe aucun profil et ne plante pas** (AMD-030 §5) | conforme | `Amd039_c2_the_same_bundle_without_bit_0x200_imports_nothing_and_does_not_crash` : les deux fichiers ne diffèrent qu'à l'intérieur du mot de drapeaux (vérifié octet par octet) ; l'import lève `UnityAssetFormatException` avec un message, 0 profil importé, **dossier de destination vide** (pas même un `.part`) |
| §4-C3 | Bundle format 8 avec **0x80 et 0x200**, données après l'en-tête aligné sans bourrage → les `TextAsset` attendus | conforme | `Amd039_c3_a_format_8_bundle_with_bits_0x80_and_0x200_gives_the_expected_text_assets` : `DataStart` = 64, 3 assets conformes |
| §4-C4 | Installation réelle **3dSen 2.0** (facultatif) : 101 importés, 0 en échec ; second import 101 déjà présents ; empreintes non comparées à l'inventaire | **non testable** | Aucune installation 2.0 n'est accessible côté propre (`ZAXNES_3DSEN_INSTALL` non défini, le témoin `verify/p8-input` est une 1.1.2, format 7). Test écrit et **ignoré** : `Amd039_c4_a_real_2_0_installation_imports_the_101_profiles` [SKIP] — il s'activera dès qu'une 2.0 sera présente |
| §4-C5 | Installation réelle **3dSen 1.1.2** (facultatif) : **inchangé**, comparaison octet pour octet avec l'inventaire de référence comprise | conforme | `Amd039_c5_a_real_1_1_2_installation_imports_the_101_profiles_of_the_inventory` [PASS] sur le témoin `verify/p8-input/fake-3dsen-install` : format de bundle 7, **101 importés / 0 échec**, les 101 empreintes SHA-256 des fichiers écrits appartiennent toutes aux 101 entrées `Profile` d'`assets-neutral/inventory.json`, second import **0 importés / 101 déjà présents / 0 échec** |
| §3-R | Robustesse d'AMD-030 §5 conservée sur la forme 2.0 | conforme | `Amd039_a_truncated_format_8_bundle_gives_a_message_and_writes_nothing` : bundle format 8 tronqué → message, aucun fichier écrit, aucun fichier partiel |

**Non testable (2)** : §4-C4 et, au titre du même manque, la moitié « réelle » de §2-9.
L'absence d'installation 2.0 est la seule cause ; les deux critères sont déclarés facultatifs
par AMD-039 §4.

---

## 3. Tickets Q du lot

Un seul ticket est ouvert sur ce lot : **Q-076** (« Le critère “aucun jeu en double” d'AMD-037 §4
sur une installation 2.0 », R3, 2026-10-30, référencé dans
`tests/Zax.Profiles.Tests/ThreeDSenRealInstallTests.cs`). Q-077 à Q-079 portent sur les lots
V2-2, V2-4 et V2-5.

**Le choix provisoire de Q-076 est conforme au texte.** Trois raisons :

1. AMD-039 **§3** écrit noir sur blanc que « les règles d'import (AMD-030 §5), de doublons et de
   noms, et AMD-037 » ne changent pas. La règle de doublon d'AMD-030 §2 est **octet pour
   octet** ; puisque §5 constate que quatre profils de la 2.0 diffèrent de ceux de la 1.1.2, ces
   quatre-là ne peuvent, à règles inchangées, qu'être conservés comme profils utilisateur.
   Inventer un rapprochement par titre ou par contenu partiel aurait été, lui, une violation
   de §3.
2. Le critère d'AMD-037 §4 (« aucun jeu n'apparaît deux fois ») n'est pas une règle mais un
   **constat**, et son énoncé le justifie explicitement par « chacun des 101 fichiers importés
   est, octet pour octet, un profil intégré » — prémisse qu'AMD-039 §5 rend fausse sur une 2.0.
   Le tenir pour décrit sur la 1.1.2 est la seule lecture qui n'oppose pas deux amendements
   normatifs.
3. C'est exactement la symétrie du traitement qu'AMD-039 §4 réserve déjà aux empreintes :
   exigibles sur une 1.1.2, écartées sur une 2.0 parce que l'inventaire décrit la 1.1.2.

**Réserve de forme** (pour R0, pas pour l'implémenteur) : l'en-tête d'AMD-039 annonce « Amende
AMD-030 §3, §4 et §6 » — correction apportée en v1.1 sur la réserve 39-1 de R2 — mais ne cite
pas **AMD-037 §4**, dont le critère subit pourtant exactement la même supersession partielle.
La phrase manquante est du même ordre que celle que R2 a fait ajouter pour AMD-030 §6.

---

## 4. Observations (aucune non-conformité)

- **O-1 — mineur, documentation.** `src/Zax.Profiles/Import/UnitySerializedFile.cs:111-112`
  porte encore : « *Q-071: … No measured file uses this branch (the installation carries
  version 21), so the public layout is the one implemented.* » AMD-039 §2 dit l'inverse : la
  version 22 « est désormais **rencontrée en pratique** ». Le ticket Q-071 est de surcroît clos
  sur la mention « à revoir seulement si **un vrai fichier de version 22 apparaît** » — condition
  que le texte normatif déclare maintenant remplie. *Observé* : commentaire et statut de ticket
  affirmant que la branche n'est exercée par aucune donnée réelle. *Attendu* : le commentaire et
  Q-071 signalent que la branche est désormais empruntée en pratique et que sa disposition
  (28 octets, format public) reste un choix provisoire non départagé par la spec. *Effet sur le
  comportement* : **aucun** — AMD-039 §2 ne modifie pas la disposition, seulement le constat.
  Sévérité : **mineur**.

- **O-2 — mineur, housekeeping.** La réserve 39-6 de R2 reste partiellement ouverte au-delà de
  ce lot : le miroir `impl/docs/spec/changes/` contient bien AMD-039 et Q-076, **identiques
  octet pour octet** à `spec/changes/` (vérifié par `diff`), mais pas AMD-040 à AMD-045. Rien à
  corriger dans ce lot-ci ; à solder par les lots suivants avant le gate.

- **Remarque de couverture** (pas un défaut) : la commande `--import-3dsen` n'est pas pilotée
  par mes tests — `ThreeDSenImportCommand` est `internal` à `Zax.App` et n'est pas visible du
  projet d'acceptation. Elle est couverte par les tests de l'implémenteur
  (`tests/Zax.App.Tests/ThreeDSenCliTests.cs`, 403 tests du projet au vert) ; mes tests passent
  par l'API publique `ThreeDSenImporter.ImportFrom`, qui emprunte la même localisation de bundle
  (AMD-030 §3) et le même écrivain, et vérifient les fichiers `.3dn` réellement écrits.

---

## 5. Exécutions

Toutes en **Release**, au premier plan, une suite à la fois.

| Projet | Résultat |
|---|---|
| `tests/Acceptance` — filtre `R4V2ImportTests` | **Failed: 0, Passed: 13, Skipped: 1, Total: 14** |
| `tests/Zax.Profiles.Tests` | **Failed: 0, Passed: 259, Skipped: 0** |
| `tests/Zax.App.Tests` | **Failed: 0, Passed: 403, Skipped: 0** |
| `tests/Acceptance` — suite entière | Failed: 1, Passed: 1386, Skipped: 16, Total: 1403 |

L'unique échec de la suite entière est
`AcceptanceGateTests.Converting_a_full_frame_fits_the_time_budget` :
`mean 10.714 / p99 37.011 | mean 3.381 / p99 18.960 | mean 1.348 / p99 15.697 ms` pour un budget
de 4 ms de moyenne et 8 ms au 99ᵉ centile. **Relancé seul, il passe** (`Passed: 1`). C'est le
gabarit de performance connu, sensible à la charge CPU de la machine ; il porte sur
`Zax.Engine3D`, que le commit vérifié ne touche pas (les 8 fichiers modifiés sont le lecteur
`UnityBundle.cs`, quatre fichiers de test et deux documents de spec miroir). **Sans rapport avec
ce lot.**

---

## 6. Conclusion

Le lot `v2-1-import` est **conforme à AMD-039**. Les trois critères d'acceptation « sans
installation réelle » de §4 sont tenus, chacun par un bundle que le test construit lui-même, et
le troisième critère (installation 1.1.2 inchangée, comparaison octet pour octet comprise) est
tenu sur le témoin disponible. Le critère « installation 2.0 » reste non testable côté propre,
faute d'installation ; le test est écrit et s'activera tel quel. Une seule zone d'ombre
subsiste, déjà connue et sans effet observable : la disposition exacte des champs 64 bits de la
version 22 du fichier sérialisé (Q-071), qu'AMD-039 déclare désormais exercée sans la préciser.
