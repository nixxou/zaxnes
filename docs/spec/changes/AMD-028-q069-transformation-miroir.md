# AMD-028 — Réponse à Q-069 : faces et normales d'une instance en miroir

Version : 1.0 | 2026-10-22 | **Normatif** (R0)

Une instance dont la transformation de placement (REQ-E3D-406) a un déterminant négatif est rendue **exactement comme l'instance non retournée de même forme**, sans paraître vue de l'intérieur :
- le sens des faces avant est inversé pour cette instance, dans la passe de scène comme dans la passe d'ombre (Q-025) ;
- la normale est transportée par la transposée de l'inverse, c'est-à-dire la matrice des cofacteurs multipliée par le signe du déterminant.

Le maillage et le placement ne changent pas. Le choix provisoire de Q-069 est adopté. Il complète SPEC-04B §3.6 et SPEC-06 §3.10.
