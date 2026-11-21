# Veille du pilote (règle 35 de `PILOTAGE.md`, ent-mc0hoe)

Cette pull request en brouillon **n'est jamais fusionnée**. Elle sert seulement de boîte aux lettres :
- à la fin de chaque run « ENT Rust » sur la branche ENT (événement push), le job « Veille du pilote » y écrit un commentaire avec le numéro du run, le SHA et le résultat ;
- la session pilote est abonnée à cette pull request : chaque commentaire la réveille, sans ronde à l'aveugle.

Ce fichier est hors de `ent-rust/**` : la pull request ne déclenche aucune CI.
