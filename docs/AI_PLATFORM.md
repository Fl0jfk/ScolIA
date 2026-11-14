# Plateforme IA native

Objectif : offrir reformulation, correction, rédaction de réponses, résumés, aide administrative et recherche sur les données autorisées, afin de limiter la copie de messages de parents et d'informations élèves dans des IA publiques.

## Fournisseur
Mistral prioritaire à étudier, sans présumer conformité sur la seule base de la marque. Vérifier contrats et technique : lieux de traitement/stockage, sous-traitants, transferts hors EEE, rétention, entraînement, options no-retention, chiffrement, DPA et notification d'incident. Aucun secret API dans le frontend ; appels backend Rust. Aucun fallback externe non autorisé.

## Architecture et permissions
Orchestration Rust. L'IA passe par les mêmes services métier et autorisations que l'utilisateur. Elle ne possède aucun privilège propre. Contrôles tenant/rôle/dossier à chaque lecture/action. Limites de débit, quotas, suivi coûts, logs minimaux, versionnement prompts et évaluations. Traiter documents, messages et résultats comme données non fiables ; défense contre prompt injection et exfiltration.

## Actions humaines
Toute action sensible, irréversible ou externe est soumise à confirmation. Aucun message/email/notification envoyé sans validation explicite. Afficher destinataire, texte final et pièces jointes avant envoi. Garde-fous renforcés pour finances, discipline et droits des personnes.

## Minimisation
N'envoyer que les champs nécessaires. Évaluer pseudonymisation/minimisation sans promettre anonymisation parfaite. Réintroduire des marqueurs uniquement si mécanisme fiable et autorisé.

## Cas prioritaires
Correction/reformulation, réponse parent, résumé de conversation/dossier autorisé, extraction structurée, rédaction administrative, recherche langage naturel, automatisations proposées puis validées. Évaluer exactitude, robustesse, coût, latence et validation humaine.
