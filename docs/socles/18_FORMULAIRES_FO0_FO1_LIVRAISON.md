# Socle 18 — lots FO0 (correctifs urgents) et FO1 (accords et port) : livraison

Conception suivie : `docs/socles/18_FORMULAIRES.md` (branche `ent-mc0hoe`) §2.2, §5–§10, §19, §22, §23, §27, §29, §31, §33 ; règles de pilotage 17 à 30.
Code : `ent-db/src/formulaires.rs` (FO0), `accords.rs` (types, calcul du verdict, port de signature), `accords_service.rs`
(objets, versions, décisions, « mes accords », reprise), `accords_verifier.rs` (port `verifier`) ; `ent-api/src/accords_s18.rs` (routes) ;
`ent-auth/src/actions.rs` (`socle_18_formulaires`). Migrations **2360** à **2363**. Tests : `tests_formulaires_fo0.rs`, `tests_accords_fo1.rs`,
tests unitaires de `accords.rs` et de `formulaires.rs`.

**Niveau** : implémenté, testé en local (PostgreSQL 16, base de test, mode normal et mode strict de RLS) ; **non validé par Florian** ; pas de revue de
sécurité indépendante (auto-revue) ; pas d'écran (le front de ces lots est hors périmètre). Une seule CI, sur la tête de la pile (FO1).

## Lot FO0 — correctifs urgents (migration 2360)

| Défaut | Correction | Test |
|---|---|---|
| **FD-01** (critique) un accord sur la v1 vaut pour la v2 | chaque version est qualifiée `forme` / `fond` / `restriction` (colonne `changement`, API `publier_formulaire_qualifie`) ; un accord antérieur à la dernière version de **fond** est `a_renouveler` ; les refus restent protecteurs ; sans qualification, `fond` par prudence ; une version de fond ne sollicite que ceux dont aucune décision ne couvre la version | `fd01_ck30_ck31_…`, unitaires `etat_effectif` |
| **FD-02** parent sous interdiction de communication lit et signe | lecture, signature, sollicitation et décision effective passent par `ent_eleve_lien_communicant` | `fd02_ck06_…` |
| **FD-03** un parent voit la décision de l'autre | `etat` toujours `None` pour les familles ; `repondre_formulaire` renvoie **sa** décision | `fd03_…` |
| **FD-04** majeur non prévenu, parents prévenus | notification directe au compte de l'élève qui décide seul ; ses parents ne sont ni sollicités ni décideurs | `fd04_ck19_…` |
| **FD-05** majorité sans capacité | `ent_eleve_decide_seul` (majorité canonique, émancipation, tutelle ; date contestée ou approximative ⇒ non présumée) | `fd05_ck15_ck17_ck18_…` |
| **FD-08** titre en clair dans l'e-mail et le push | type `formulaire` neutre sur tous les canaux externes ; titre de la notification interne neutre | `fd08_…` |
| **FD-10** autorisation signée sans `autoriser_sorties` | invisible, non signable, non sollicitée sans ce droit ; sa décision cesse de compter si le droit est retiré | `fd10_ck08_…` |
| **FD-11** publication / archivage sans acteur | `publie_par`, `archive_par` | `fd11_…` |
| **FD-12** aucune immuabilité en base | déclencheurs : décision en ajout seul, texte publié figé, brouillon seul supprimable | `fd12_…` |
| **FD-14** publier un numéro inférieur archive le plus récent | refusé par le code et par la base | `fd14_fd15_…` |
| **FD-15** nature non verrouillée | refusée par le code et par la base | `fd14_fd15_…` |
| **FD-17** un seul accord suffit toujours | règle `tous` pour `droit_image`, `voix`, `mention_nominative`, `enregistrement_visio`, `voyage_etranger` ; silence ⇒ `en_attente` ; refus ⇒ `refuse` ; `un` : refus + accord ⇒ `desaccord` (la photo reste masquée) | `fd17_ck03_ck07_…` |
| **FD-19** objet non validé en lecture | contrôlé | `fd19_frm261_…` |
| FRM-261 / R-FC23 santé | `SANTE_ACTIVEE = false` : aucun objet de santé ni de donnée sensible (D-034) | `fd19_frm261_…` |
| FRM-267 / R-FC12 consentements parentaux à la majorité | cessent (seules les décisions de l'élève comptent) | `fd04_ck19_…` |
| FRM-275 / R-FC36 relances | publication aux seuls titulaires **sollicitables et communicants** (déjà le cas pour `responsables_de`) | — |

## Lot FO1 — accords et port (migrations 2361–2363)

| Sujet | Où | Test |
|---|---|---|
| Catalogue de référence (19 objets, usages), objets d'établissement, usages (missions = `non_requis`) | 2361, `accords_service.rs` | `catalogue_activation_…`, `creation_d_objet_…` |
| Versions à usages, publication qualifiée ; **un usage ajouté force le fond** (CK-32) ; relecture du texte (texte provisoire, R-FC72) | 2362 | `ck30_ck31_ck32_…` |
| Décisions **en ajout seul**, usage par usage, empreinte à longueurs encodées, qualités (titulaire, avis de l'enfant, co-signature, majeur, personne) | 2362 | `ck03_ck04_ck05_…`, `ck01_ck02_ck35_…` |
| Avis de l'élève dès l'âge fixé, **son refus l'emporte**, par son propre compte (R-FC51) | `calculer_verdict` | `ck03_ck04_ck05_…` |
| Retrait sans passkey, effet immédiat (le verdict est calculé à chaque appel), après l'événement : demande à l'établissement (R-FC14) | `accords_retirer` | `ck01_ck02_ck35_…` |
| « Mes accords » : seulement **ma** décision, usages à renouveler | `accords_mes_accords` | `mes_accords_jamais_…` |
| Port `verifier` : lot, 9 verdicts, **jamais `autorise` par défaut** (`inconnu` sur erreur, objet, usage ou sujet inconnu, sujet d'un autre établissement), raison codée réservée à `autorisations.gerer`, journal agrégé | `accords_verifier.rs` | `ck36_expiration_protege_…`, `verdicts_par_acteur_…` |
| Élève protégé ⇒ `protege` pour tout usage d'image hors personnels (R-FC11) ; expiration au 31 août (CK-36) | idem | `ck36_…` |
| Capacité : majeur, émancipé, tutelle, curatelle, date contestée | `ent_eleve_decide_seul` | `ck15_ck16_ck17_ck18_ck19_…` |
| Lecture détaillée journalisée (FD-09), verdicts de classe sans motif | 2363 | `verdicts_par_acteur_…` |
| **Reprise de 193** (rejouable, `reprise_dossier`, refus hérité global, accord ancien jamais valable pour un usage précis) et de **275** (consentements du socle 4) | `accords_reprendre_*` | `reprise_193_…`, `reprise_275_…` |
| **RLS forcée** sur 7 tables, FK composites, plus aucune cascade depuis `ent_user` ; registre E6 ; compte RLS de `tests_rls.rs` (+7) | 2361–2363 | `separation_entre_etablissements_…`, `tests_rls` |
| Droits : `autorisations.lire` (portées de classe), `.gerer`, `.parametrer` — fonction `socle_18_formulaires`, une seule ligne d'appel | `ent-auth` | `socle_18_droits_par_defaut` |
| **Port de signature** `PortSignature` (socle 7, lot D6) ; adapteur `SignatureAbsente` ; S2 refusé tant que la brique manque | `accords.rs` | `le_port_de_signature_…` |

## Limites et écarts à valider (faits, pas des hypothèses)

1. **Signature** : aucune signature écrite. Sans brique D6, une décision est `en_ligne` sans identifiant de preuve ; S1 n'exige pas encore « session < 15 min ou code » (FO3) ; **S2 est refusé** (jamais simulé).
2. **Droits du lien** : le socle 4 (lot A4) n'a pas encore livré `signer_quotidien`, `signer_documents`, `repondre_formulaires` comme colonnes. Seuls l'autorité parentale et `autoriser_sorties` (`peut_autoriser_sorties`) sont distingués ; l'adaptation se fait dans `condition_droit_lien`.
3. **Désaccord déclaré du socle 4**, **délégataire de l'image** (R-FC60), **annulation par l'établissement**, **sorties**, **guichet / papier / lien nominatif** : FO3. La qualité `delegataire_image` et la décision `annulee_par_etablissement` existent en base sans parcours.
4. **Projection `ent_acc_etat`** non créée : le verdict est calculé à chaque appel (R-FC4, FRM-276 couvert sans cache). À ajouter si la mesure de performance l'exige.
5. **Fusion de fiches élève** : `ent_acc_decision.sujet_eleve_id` n'est pas nommée `eleve_id` ; le registre `TABLES_ELEVE` ne la voit pas. Le déclencheur autorise la réaffectation du seul sujet, mais la commande de fusion ne la déplace pas encore.
6. **Ancien module (193)** : toujours en écriture jusqu'au retrait (FO13) ; la reprise est rejouable par la direction (`POST /accords/reprise/formulaires`). Le trombinoscope lit encore l'ancien module (`eleves_photo_masquee`) : il bascule au `verifier` avec FO5.
7. **Événements** : agrégat `Annonce` réutilisé (comme FO0 historique) ; un agrégat dédié suppose une modification d'`ent-events`.
8. **`afaire_sources.rs`** (socle 29) lit encore l'ancien état par objet (« à répondre ») sans tenir compte de la version de fond ni de l'interdiction : à reprendre avec FO2.
9. **Notifications** : seule la nouvelle demande de fond est émise (enveloppe neutre) ; relances, plage calme et campagne de renouvellement : FO2 / FO8.
10. **Tests** : le comportement des deux modes (`ENT_RLS_MODE` absent et `strict`) a été lancé en local ; la RLS stricte complète tourne en CI sur `ENT` seulement (règle 27).
