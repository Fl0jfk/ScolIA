# Tableau de maturité des 51 socles

Mis à jour chaque semaine par le pilote. **Estimation du pilote**, à recaler avec la visite guidée et l'avis de Florian.

- **Refonte** : part approximative du plan de lots de la refonte Rust « masterclass » versée sur ENT (bien avancé ≈ 80 %, moitié ≈ 50 %, tiers ≈ 33 %, faible ≈ 15 %, — = pas commencé).
- **Écrans** : écrans Leptos de la refonte (oui / partiels / non ; « anciens » = seulement l'ancienne interface `ui.rs` ou le Next.js).
- **Existant de départ** : état du code hérité au moment de la carte (`docs/SOCLES.md` : I implémenté, P partiel, E embryonnaire, A absent). Un socle « pas commencé » dans la refonte peut donc déjà avoir du code ancien.
- Les **numéros** viennent de la carte (rangement par étage), pas de l'ordre de travail.

## Semaine du 20 novembre 2026 (ENT = f7b9421c)

**En bref** : 3 socles bien avancés, 4 à moitié, 11 au tiers, 5 faibles, 28 pas commencés dans la refonte. Environ **15 à 20 %** de l'ENT complet. Les plus gros manques : **applications mobiles (11)**, obligatoires pour le MVP, et **bulletins (23)**.

| N° | Socle | Étage | Refonte | Écrans | Existant de départ | Ce qui reste (principal) |
|---|---|---|---|---|---|---|
| 1 | Sécurité | Cœur | **bien avancé** | oui | fait | invitation en masse |
| 2 | L'élève | Cœur | moitié | liste et fiche | P | mobile (E10), exports (E11), écrans E9c-E9f |
| 3 | Inscriptions et réinscriptions | Cœur | tiers | anciens | P | I5 et suivants ; fiches de rentrée, réinscriptions (création pure, D-068) |
| 4 | Dossier administratif | Cœur | tiers | anciens | I | vocabulaire contrôlé des liens (« Maman »…), écrans |
| 5 | Structure et année scolaire | Cadre | **bien avancé** | partiels | I | S12, S13 ; année « préparation » créée d'office (à décider) |
| 6 | Personnels, services et équipes | Cadre | — | anciens | P | à concevoir |
| 7 | Documents, pièces et signatures | Cadre | moitié | non | P | D7 et suivants |
| 8 | Notifications et diffusion | Cadre | faible | non | P | destinataires réels (internat, notes), canaux |
| 9 | Interopérabilité nationale, identité fédérée | Cadre | — | — | P | à concevoir |
| 10 | Emploi du temps | Journée | tiers | partiels | I | EDT réel (T10) |
| 11 | **Applications mobiles** | Journée | **—** | — | P | **conception à faire (Opus, avec Florian)**, une appli par plateforme pour tous les rôles |
| 12 | Appel et assiduité | Journée | moitié | oui | I | suites (tests à l'heure fixe, AS6) |
| 13 | Discipline et suivi éducatif | Journée | faible | anciens | I | refonte |
| 14 | Accueil, sorties et sûreté | Journée | — | anciens | P | à concevoir |
| 15 | Absences des personnels et remplacements | Journée | **bien avancé** | oui | P | A10 reprise |
| 16 | Messagerie et conversations | Échanger | tiers | partiels | I | M9, M10, M12 ; pièce jointe sur message différé |
| 17 | Informations, annonces et campagnes | Échanger | — | anciens | P | refonte |
| 18 | Formulaires, autorisations et consentements | Échanger | tiers | partiels | A | suites |
| 19 | Agenda, réunions et rendez-vous | Échanger | tiers | oui | E | suites |
| 20 | Cahier de textes et travail personnel | Apprendre | moitié | partiels | E | suites (prochain lot proposé) |
| 21 | Évaluations et notes | Apprendre | tiers | grille | I | V3c+V4 en cours ; V5 à V13 |
| 22 | Compétences et livrets | Apprendre | — | — | P | à concevoir |
| 23 | **Bulletins, conseils et passage** | Apprendre | **—** | — | P | après 21 V8 |
| 24 | Facturation | Argent | faible | non | I | F3 et suivants |
| 25 | Paiements, prélèvements, recouvrement | Argent | — | anciens | I | refonte |
| 26 | Bourses et aides financières | Argent | — | — | P | à concevoir |
| 27 | RGPD, conservation et archives | Premier étab. | tiers | non | E | suites |
| 28 | Mise en service, reprise, support | Premier étab. | tiers | non | E | outils de reprise par domaine |
| 29 | Portails, « Aujourd'hui », à-faire | Premier étab. | faible | partiels | E | navigation par domaines (conception avec Florian) |
| 30 | Assistant IA et « chat first » | Transverse | — | — | E | à concevoir |
| 31 | Automatisations et règles | Transverse | — | — | E | à concevoir |
| 32 | Espaces de travail et bureautique | Transverse | faible | non | A | ArtCraft (D-006) |
| 33 | Visioconférence et continuité | Transverse | — | — | A | à concevoir |
| 34 | Santé et infirmerie | Chaque élève | — | anciens | I | refonte (remise par la fonction unique déjà branchée) |
| 35 | Besoins éducatifs particuliers | Chaque élève | — | — | E | à concevoir |
| 36 | Suivi social, psy, protection de l'enfance | Chaque élève | — | — | P | à concevoir |
| 37 | Orientation et parcours | Chaque élève | — | — | P | à concevoir |
| 38 | Examens et certifications | Chaque élève | — | — | A | à concevoir |
| 39 | Stages, PFMP et entreprises | Chaque élève | — | anciens (Next.js) | I | module du Next.js à épaissir et relier (D-068) |
| 40 | Restauration | Services | — | anciens | I | refonte |
| 41 | Internat | Services | tiers | non | I | I4 (vie quotidienne, 2828-2829) et suivants ; écrans |
| 42 | Sorties et voyages scolaires | Services | tiers | partiels | A | V3 et suivants |
| 43 | Périscolaire et transports | Services | — | — | E | à concevoir |
| 44 | Ressources matérielles, prêts, maintenance | Services | — | — | E | à concevoir |
| 45 | CDI et ressources documentaires | Services | — | — | A | à concevoir |
| 46 | Instances et vie démocratique | Services | — | — | A | à concevoir |
| 47 | Pilotage et indicateurs | Services | — | — | E | à concevoir |
| 48 | Comptabilité, budget et achats | Organisme | — | — | E | à concevoir |
| 49 | Paie et administration du personnel | Organisme | — | — | A | question ouverte : états d'heures ou paye complète ? |
| 50 | Premier degré (écoles) | Extensions | — | — | E | plus tard |
| 51 | Post-bac et apprentissage | Extensions | — | — | A | plus tard |
