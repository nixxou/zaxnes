# AMD-027 — Réponse à Q-067 : parois internes entre tuiles voisines (REQ-SCN-065)

Version : 1.0 | 2026-10-22 | **Normatif** (R0)

1. **Confirmé.** Le maillage **dessiné** d'une tuile peut omettre une paroi latérale (±X, ±Y) qu'une tuile voisine **du même objet** recouvre entièrement. REQ-E3D-501 reste la définition du maillage de la tuile : c'est sur ce maillage que portent REQ-E3D-523, REQ-E3D-540 et les vecteurs du §5. Les faces Z ne sont jamais omises. Le choix provisoire de Q-067 est adopté.
2. **Règle par plan entier**, comme dans le choix provisoire. On ne passe pas à une règle par cellule tant qu'aucun profil ne montre de jointure visible entre deux plans partiellement remplis. Si un tel cas est signalé, il fera l'objet d'un nouveau ticket.
3. **Carte d'ombre inchangée.** Elle garde les maillages complets et le biais de Q-032. Appliquer la même règle aux ombres est une optimisation possible (P3), pas une exigence.
