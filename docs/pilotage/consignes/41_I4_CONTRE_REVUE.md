# Contre-revue des correctifs I4 (PR 122, tête 79b436cb) — revue indépendante du pilote

Numéros de ligne : ceux de 79b436cb ; les relire sur la base actuelle.

Contre-revue lecture seule du lot I4 (correctifs de revue). Verdict : 1 Élevé confirmé, 7 Moyens, 7 Faibles. Le lot n'est pas prêt tant que É1 n'est pas corrigé.

Chemins, tous sous /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/i4c/ent-rust/ : crates/ent-db/src/internat_sorties/groupes.rs, crates/ent-db/src/internat_sorties/vie.rs, crates/ent-db/src/internat_sorties.rs, crates/ent-db/src/internat_cas.rs, crates/ent-db/src/registre_donnees.rs, crates/ent-db/src/tests_internat_i4_correctifs.rs, migrations/2829_internat_i4_vie_quotidienne.sql, crates/ent-api/src/main.rs.

ÉLEVÉ (confirmé par lecture)
É1. Un élève « en activité » disparaît si l'activité est annulée ou s'il est désinscrit (règle 1).
- La veille ne regarde que les temps t.statut='prevu' et les inscrits retire_le IS NULL (groupes.rs:2930-2931). Le marquage « en activité » de l'appel a les mêmes filtres (groupes.rs:2818-2819).
- annuler_temps_a (vie.rs:1600-1649) n'a aucune garde sur les inscrits déjà pointés 'present' sans rentre_le. desinscrire_temps_a (vie.rs:1801-1820) n'en a pas non plus pour le personnel de gestion, jusqu'à la fin du temps.
- pointer_temps_a refuse le retour d'un temps annulé (« ce temps est annulé », vie.rs:2440) : le cas ne peut plus être fermé par le retour.
- Scénario : 3 élèves pointés « en activité » à 19 h, la gestion annule le temps à 19 h 30. Aucune veille ne les voit. La marque « activité » déjà posée dans l'appel ouvert reste, sans cas associé.
- Non testé (le test m9_* ne couvre que le chemin nominal).
- Correctif : refuser annulation et désinscription tant qu'un inscrit est pointé 'present' sans retour. Ou faire suivre l'élève par la veille quel que soit le statut du temps ou le retrait de l'inscription, et autoriser le pointage de retour sur un temps annulé.

MOYEN
M1. Un cas fermé est rouvert toutes les minutes tant que le retour n'est pas pointé (règle 1).
- veille_non_rentres_a (groupes.rs:2923-2954) n'a aucune garde « un cas existe déjà pour cette sortie ou ce temps, quelle que soit son issue ». L'unicité ne porte que sur les cas ouverts (migration 2824:525), et ouvrir_cas_tx fait ON CONFLICT DO NOTHING.
- Deux fermetures laissent l'élève « parti » : le CPE ferme le cas (autorite_saisie, annule_erreur), ou l'appel pointe l'élève présent via clore_cas_retour_tx (internat_cas.rs:337-341), qui ferme tout cas ouvert de l'élève et de la nuit sans regarder l'origine.
- La veille crée alors un nouveau cas à la minute suivante, avec un événement à chaque fois : bruit qui masque les vraies alertes.
- Le test d'idempotence ne couvre que le cas ouvert (tests_internat_i4_correctifs.rs:109-117).
- Correctif : exclure l'élève si un cas existe déjà pour (élève, sortie ou temps, nuit), quelle que soit son issue.
- Variante : si un cas est déjà ouvert par une autre source (appel), ouvrir_cas_tx renvoie None et le cas n'est pas rattaché à la sortie ni à l'heure prévue ; clore_cas_origine_tx ne le fermera pas au retour.

M2. La veille échoue en silence et l'établissement entier est bloqué (« que se passe-t-il si elle ne tourne pas »).
- Branchement réel : main.rs:423-435 lance la boucle chaque minute et appelle internat_sorties_tache, qui appelle les deux nouvelles veilles (internat_sorties.rs:4566-4573). Calendrier correct : instants absolus, ent_nuit_etab, pas de jour UTC.
- Une erreur sur une seule ligne annule toute la transaction de l'établissement, sans rattrapage ligne par ligne, et n'est journalisée qu'en warn.
- Seule la veille des bilans alimente suivre_echecs_veille (alerte « en échec répété », internat_sorties.rs:60-81). Les deux nouvelles veilles n'y sont pas : aucune alerte si la tâche est bloquée.
- Si le processus ne tourne pas, rien ne rattrape (conforme à « pas un point de lecture »), mais l'absence de supervision ne l'est pas.
- Correctif : un point de reprise par élève, comme veille_sorties_bilan_a, et un appel à suivre_echecs_veille pour les deux veilles.

M3. Les signaux n'arrivent à personne (règle 1, « signalé »).
- internat.etude_non_pointee et internat.alerte n'ont pas de consommateur (le rapport le reconnaît).
- La veille de l'étude ne signale que les non pointés (vie.rs:2567, tp.pointage IS NULL). Un inscrit pointé « absent » ne reçoit qu'un événement d'inscription, sans cas ni signal. Le test m9_* confirme : seul e3 est signalé, pas e2.
- Hypothèse à vérifier : INTERNAT_CAS_OUVERT n'avertit les humains que si I3 le consomme déjà.

M4. Un accord annuel « soirée » couvre une sortie de 00 h 30 à 02 h 00 (règle 2).
- dans_les_bornes_soiree_tx (groupes.rs:399-419) exige même jour local, durée max et zéro nuit couverte. nuits_couvertes_tx (internat_sorties.rs:1993-1997) exige fin >= lendemain 06:00.
- Une sortie 00 h 30 → 02 h 00 passe : même jour, aucune nuit « couverte ». Elle devient autorise par accord annuel alors que l'élève sort en pleine nuit.
- Correctif : refuser dès que la sortie recoupe l'intervalle [ancre de la nuit, matin 06 h], pas seulement quand elle le couvre entièrement.

M5. Export de portabilité : la cause du blocage reste exportée (règle 3, exports).
- registre_donnees.rs:849 déclare ent_internat_sortie_groupe_participant en .export() sans hors_export. La colonne motif_code contient interdiction, desaccord_connu, majorite. Seule la table des tiers autorisés masque son motif (ligne 858).
- Le lecteur d'extrait n'existe pas encore (extrait_personne.rs ne fait que déclarer) : risque latent, mais le contrat déclaré est faux. Le test m10_* est purement déclaratif et ne regarde pas cette colonne.
- Correctif : .hors_export(&["motif_code", "reponse_user_id"]).
- Hypothèse : ent_internat_cas (issue_note, nature interdiction_auteur) est aussi exporté ; ce n'est pas dans ce diff, mais c'est la même règle.

M6. Tests : plusieurs règles ne seraient pas détectées si elles régressaient.
- Le branchement internat_sorties_tache → nouvelles veilles n'est testé nulle part : supprimer les deux appels ne fait échouer aucun test.
- Non couverts : la veille sur une sortie à cheval sur minuit (f6 ne couvre que le départ) ; le cas déjà ouvert par un appel ; l'annulation ou la désinscription avec un élève en activité (É1) ; la rouverture de cas (M1).

M7. Visibilité des protégés (règle 6) : cohérente avec la sécurité, mais incomplète.
- Pour les sorties, groupe_dto_tx (groupes.rs:621-648) liste les participants protégés avec eleve_id, état et parti_le, et ne masque que les noms. Pour les temps du soir (M8), ils sont retirés des inscrits (vie.rs:2339).
- Le choix peut se défendre (l'accompagnateur doit compter ses enfants), mais « protégé introuvable partout » n'est pas tenu pour les sorties et les événements. À trancher explicitement.

FAIBLE
- F-a. accord_tient_tx renvoie « interdiction » pour toute chute de l'accord annuel (hors bornes, type désactivé, verdict) : l'événement part avec la cause interdiction_auteur, fausse et alarmiste pour le CPE (groupes.rs:1936, 1958-1962, 3010-3014).
- F-b. Malaise : un signal par couple (élève, rôle), pas par élève ; un élève déclaré victime et présent reçoit deux signaux (vie.rs:927).
- F-c. M7 : si un élève est déjà « parti » dans la fenêtre d'avance de départ, inviter_eleves_groupe_a (groupes.rs:1121-1131) tente de repasser la sortie en « soumise » ; le déclencheur de la base refuse. C'est sûr, mais l'appelant reçoit une erreur base opaque au lieu d'un conflit lisible.
- F-d. Fusion (M5) : en cas de doublon sur la même sortie, « le conservé l'emporte ». Si la ligne « parti » est celle de l'absorbé, elle reste sur la fiche absorbée et le cas s'ouvrirait sur cet identifiant. Hypothèse, non testée.
- F-e. Les deux veilles font un INSERT ... ON CONFLICT sur les paramètres, par établissement et par minute (surcoût d'écriture inutile).
- F-f. Accord annuel sans contrôle de majorité (accord_tient_tx, branche « annuelle ») : sans conséquence de protection ; hypothèse sur le socle 18.
- F-g. Le refus téléphonique est refusé si le parent saisi est interdit (« cet accord ne peut pas être enregistré ») ; un refus, plus restrictif, devrait peut-être toujours s'enregistrer.

CONFORME (vérifié)
- Règle 1 : tâche branchée et planifiée chaque minute ; tolérance lue en base (15 min par défaut) ; cas I3 « non_rentre », auteur système, avec sortie ou temps, heure prévue et comptes à joindre (internat_cas.rs:434-486) ; instants absolus, nuit du fuseau de l'établissement, donc pas de décalage UTC ni de problème à minuit ; appel « en activité » à l'ouverture et au départ, annulation du retour qui remarque l'élève ; l'attendu ne retire l'élève qu'après départ constaté (migration 2829:529-536).
- Règle 2 : bornes de durée, de jour et de nuit contrôlées à l'invitation et au départ ; majorité caduque pour un titulaire ou un accord hors ligne ; retrait de l'attendu après départ constaté.
- Règle 3, DTO de sortie de groupe : participant_dto ne donne le motif qu'avec internat.lire, internat.gerer ou internat.sorties.decider ; voit_la_cause est lu dans le détail, la réponse du départ et l'annulation du retour ; retour et liste des sorties sans motif ; vue famille neutre.
- Règle 4 : titulaire_pour_tx est la fonction unique (compte ou personne), avec règle M-A et interdictions remise, visite, contact_eleve.
- Règle 5 : annulation du retour par fait motivé ; refus hors ligne enregistrable ; transitions gardées par les déclencheurs de base (migration 2828:246-293).
- Règle 6 : droit avant existence dans repondre, completer, inscrire ; accord_hors_ligne contrôle personnel/droit, portée, puis visibilité de l'élève.
- Règle 7 : malaise sans élève non témoin refusé ; accusé minimal en retour des écritures sur événement ; lectures journalisées ; dérivation de nuit (F6) ; chevauchements sous verrou consultatif (F9) ; nouvelle décision après ajout (M7) ; encadrant obligatoire pour un temps du soir (M6).
- Règle 8 : 2829 n'élargit plus lecture_sensible_objet_check ni cas_nature_check ; « registre » et « sorties » figurent dans la liste recréée par 2826 (2826:880-883) ; le test de rejeu appelle bien rejouer_tout ; le libellé famille « en_cours_d_examen » est inchangé (tiers.rs:97). L'explication du second échec CI (rejeu avorté qui laisse une ancienne fonction) est plausible mais non vérifiée sans exécution.
- Règle 9 : jours fixes d'avril 2026 et instants explicites, pas de jour UTC ; concurrence couverte (départ simultané, réponse et départ, double inscription) ; interdictions visite et contact_eleve couvertes.
- Routes : les quatre nouveaux chemins sont ajoutés au test « aucune route sans session ».

Limite : revue par lecture seule, sans build ni exécution des tests.

