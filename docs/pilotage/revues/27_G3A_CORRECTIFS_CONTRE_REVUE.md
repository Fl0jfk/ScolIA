# Contre-revue — socle 27, G3a, correctifs du guichet des droits RGPD — 2026-11-26

Branche `ent-socle-27-code-g3a`, PR 158 (brouillon vers `ENT`). Code relu à **`245020e9`** (CI run 727 : success). Tête finale **`aaa6fb83`** : ce commit ne touche que `docs/socles/27_G3A_LIVRAISON.md` (+4 lignes, vérifié par `git diff --stat 245020e9 aaa6fb83`), il ne change donc pas le verdict.

Relecteur indépendant, lecture seule : le diff `git diff origin/ENT...245020e9` a été lu, rien n'a été exécuté, rien n'a été poussé sur la branche du lot ni sur `ENT`. Les chemins `fichier:ligne` sont ceux de `245020e9`.

## Verdict : **versable**, avec trois réserves moyennes à solder avant la mise en service (pas de blocage pour G3a)

Les deux élevés de la première contre-revue (E1, E2) sont réellement corrigés et testés. Aucun nouvel élevé, aucune régression de sécurité, aucune fuite entre établissements trouvée. Les réserves M-a, M-b, M-c ci-dessous sont des trous résiduels de l'esprit des correctifs (R-RG16, CL27-04, accusé de réception), pas des failles d'isolation ; elles gagnent à être corrigées dans la passe G3b ou un petit correctif avant tout usage avec de vraies pièces.

## Contrôles de cadre

- **Règle 25** : `git log 245020e9 --not origin/ENT` = 11 commits, tous du lot (guichet G3a, ses correctifs, 2 fusions d'`origin/ENT`). Aucun ne touche `docs/POINT_QUOTIDIEN.md`, `docs/PILOTAGE.md` ni `migrations_versees.txt`.
- **Migration 2544** : `migrate.rs` ajoute `m!("2544_rgpd_g3_demandes")` **en fin de tableau**, après `2227` (règle 38). Les 4 fonctions et déclencheurs de la migration portent `SET search_path = pg_catalog, public, pg_temp` ; `rgp_demande_numero_suivant` qualifie `public.rgp_demande_compteur` ; les autres fonctions ne lisent aucune relation. Aucun diff sur 2540 à 2543.
- **RLS** : `ent_rls_poser` sur les 5 tables (`2544…sql`, dernières lignes) ; compteur de `tests_rls.rs` +5 ; registre des données et catégorie de conservation `rgpd.demande` à jour ; `TOUTES` 282 → 286 cohérent avec 4 actions ajoutées ; `Droits*` portée établissement seulement ; secrétariat sans remise ni arbitrage. Chaque service passe par `exiger_action_etab` ou un contrôle équivalent ; 404 neutres et test de cloisonnement présents (`tests_rgpd_g3a.rs:1321`).
- Aucun `#[ignore]`, aucun test à l'horloge réelle dans les fichiers du lot (`now()` SQL seulement pour des horodatages d'écriture, jamais pour un délai).

## Élevés

Aucun.

### E1 — destruction réelle de la pièce : **corrigé**
`droits_demandes.rs:3011-3184`. La clôture et le refus écrivent une demande durable (`piece_destruction_demandee_le`) dans leur transaction ; après commit, `executer_destruction_pieces` supprime les objets reçu et servi du magasin (`objet_supprimer`), les marque supprimés au registre `ent_objet`, puis seulement alors efface la référence et journalise « détruite ». Un échec laisse la pièce référencée, compte la tentative, journalise une seule fois « en attente », et la tâche horaire (câblée `ent-api/src/main.rs`, hors du chemin de requête) la reprend. Un objet partagé avec une autre pièce n'est jamais détruit. `piece_document_id` est validé (`valider_piece`, 3250-3292 : établissement, type `piece_identite`, non retirée, jamais déjà rattachée, objet non partagé) et la FK composite et l'index unique de la migration l'imposent. Tests : `tests_rgpd_g3a_correctifs.rs:137` (l'objet a disparu du magasin et du registre, ordre du journal) et `:190` (panne, tentatives, une seule ligne de journal, reprise, idempotence). La méthode suit celle de `documents_pieces.rs:1948` et de la purge des pièces infectées (le registre `ent_doc_piece` reste, comme pour toute pièce d'identité du socle 7).

### E2 — qualité du titulaire : **corrigé**
`droits_demandes.rs:886-904` (`qualite_compatible`), `:1004-1010` (dépôt en ligne), `:1488-1500` (guichet). `ConfirmeeSansAutorite`, `Restreinte`, `Absente` et `NonVerifiable` ne donnent jamais « verifiee » : en ligne, refus neutre unique (`ORIENTER_GUICHET`, comparé par `Debug` aux cas « sujet inexistant » dans le test) ; au guichet, résultat `a_arbitrer`, demande bloquée en `identite_a_verifier` avec drapeau, levée par `droits_arbitrer_identite` (`droits.arbitrer`, motif obligatoire, tracé). Test `:358` couvre autorité absente, protection au dossier, refus et acceptation d'arbitrage, et les gardes M1. Écart d'écriture mineur avec la consigne : le refus neutre est un 400 (`SaisieInvalide`) et non un 403 ; la consigne laissait le choix et le message est identique dans tous les cas, donc aucun oracle.

## Moyens (restants)

- **M-a. Pièce d'une vérification échouée : jamais rattachée, donc jamais détruite** (`droits_demandes.rs:1493-1495`, `.bind(if reussie { piece_document_id } else { None })` ; l'écran fait de même, `ent-web-app/.../rgpd_droits/mod.rs:897-913`). Si le guichet a scanné une pièce, la vérification échoue (`echouee`) : la référence est silencieusement ignorée, la pièce reste dans le coffre, hors du circuit de E1. R-RG16 (« jamais gardée au-delà de la clôture ») n'est plus tenu pour ce cas. Correctif : rattacher la pièce aussi en cas d'échec (validation comprise) pour que la clôture la détruise, ou refuser une référence de pièce avec `reussie = false`. Test à ajouter.
- **M-b. Dépôt public : une seconde demande distincte est abandonnée en silence** (`droits_demandes.rs:1126-1141`). Le contact normalisé déjà vu depuis la veille renvoie `MESSAGE_SANS_COMPTE` (« votre demande est enregistrée ») sans rien enregistrer. Un ancien élève qui dépose un accès puis, le lendemain, un effacement perd la seconde demande, dont le délai légal aurait couru. Correctif prudent : conserver la demande (sans numéro) ou la fusionner dans la première en ajoutant les droits, la même réponse restant servie.
- **M-c. CL27-04 contournable par la qualité** (`droits_demandes.rs:2942`, `exiger_coherence_qualite`). Un élève **mineur** qui dépose avec la qualité par défaut de l'écran (`soi_meme`) ou `ancien_eleve` n'est pas contrôlé sur sa minorité (seule `eleve_mineur` l'est) : `exiger_titulaires_decides` (`:2983`) ne s'applique qu'à `qualite = 'eleve_mineur'`, la décision d'informer les titulaires est donc sautée. Correctif : pour toute qualité « pour soi » dont le sujet est un élève mineur à la date, traiter la demande comme `eleve_mineur` (ou refuser `soi_meme` et `ancien_eleve` à un mineur).

## Faibles

- **Pf1.** Plafond de dépôts publics par établissement (`droits_demandes.rs:1105-1121`, constante `:66`) : 30 dépôts à contacts distincts saturent la journée et bloquent les vrais demandeurs ; le contrôle (compte puis insertion) n'est pas verrouillé, un pic concurrent le dépasse de quelques unités. Compromis noté D-069 ; à valider par Florian, la purge est en G6 (F6, non fait comme demandé).
- **Pf2.** Seau HTTP par contact non normalisé (`anti_abus.rs`, champ `contact` lu tel quel) : le contournement par `+étiquette` ou casse est arrêté par la couche base, pas par le seau HTTP. Sans conséquence de sécurité.
- **Pf3.** Dépôt connecté : la détection « protégée » (`:828-851`) regarde toute adresse protégée ou « ne pas solliciter » parmi les personnes liées à l'élève, pas seulement le demandeur ; la protection de l'autre parent bloque donc le dépôt en ligne de tous les titulaires. Direction sûre (guichet, aucune fuite), mais à noter pour l'expérience des familles.
- **Pf4.** Le code d'arbitrage `restreinte` / `sans_autorite` est visible de tout porteur de `droits.traiter` (secrétariat compris) sur la fiche. À confirmer avec le socle 4 : un secrétariat a-t-il le droit de savoir qu'une protection existe ?
- **Pf5.** Numéro officiel attribué dès `reussie = true`, y compris pour un résultat `a_arbitrer` (`:1519`) : cohérent avec « demande confirmée » mais antérieur à la reconnaissance de la qualité ; à acter dans la doc.
- **Pf6.** Destruction : pas de verrou entre la passe de clôture et la tâche horaire (double ligne `piece_detruite` possible, sans effet) ; une pièce « partagée » reste en attente indéfiniment sans alerte autre que la première ligne de journal. `droits_refuser` est bloqué par `exiger_titulaires_decides` pour un mineur, ce qui empêche de refuser un dépôt non identifiable tant que la décision d'information n'est pas prise.
- **Pf7.** F7 : la référence de pièce est un UUID à saisir à la main (`mod.rs:879-897`), sans lien depuis un dépôt du coffre ; utilisable, peu ergonomique. Pour les dépôts sans compte de qualité « pour soi », aucune corrélation entre la fiche demandeur et le sujet rapproché n'est exigée à la vérification (confiance au guichet, tracée).
- **Pf8.** F5 : `Utc::now()` reste passé à `notifier_personne` (`:2724`) pour l'horodatage de l'avis ; sans effet sur le délai, qui n'utilise que le jour de l'établissement.

## Moyens 1 à 6 et faibles F1 à F8 de la première revue

| Réf. | État | Preuve |
|---|---|---|
| M1 identité/qualité avant décision et clôture | corrigé | `:2080-2105`, `:2250-2275`, test `:511` |
| M2 qualité sans compte, rapprochement tracé | corrigé | `:763-846`, `:1654-1740`, test `:584` |
| M3 compte membre de l'établissement | corrigé (404 neutre) | `:1226-1240`, test `:717` |
| M4 normalisation, plafond, numéro à la confirmation | corrigé, avec Pf1, Pf2 et M-b | `:570`, `:1105-1141`, `:3204`, test `:780` |
| M5 `sans_objet` sous arbitrage et motivé | corrigé | `:2020-2045`, test `:900` |
| M6 clôture bloquée, compteur | corrigé pour `eleve_mineur`, **contournable** (M-c) | `:2983`, test `:979` |
| F1 oracle 403/404 | corrigé (`neutre`) | `:2828`, test E2 |
| F2 accès suspendu ou fermé | corrigé | `:945-951`, test `:1055` |
| F3 réouverture d'un refus | corrigé | `:2386-2405`, test `:1093` |
| F4 alertes par palier | corrigé | `:2670-2700`, test `:1174` |
| F5 heure réelle | corrigé pour les délais (Pf8) | test `:1262` |
| F6 purge des dépôts spam | non fait, noté G6 (conforme à la consigne) | — |
| F7 écran de saisie de pièce, page publique | corrigé (Pf7) | `mod.rs`, test web serveur |
| F8 validation de la pièce | corrigé | `:3250` |

## Isolation RLS, refus neutres, régressions

- Cloisonnement : toutes les requêtes portent `etablissement_id` et tournent dans `TenantTx` ; `rgp_demande_numero_suivant` est `SECURITY INVOKER` : un `p_etab` étranger est rejeté par la RLS. Aucune table sans RLS forcée.
- Refus neutres : dépôt connecté (même message pour sujet inexistant, hors établissement, lien absent, sans autorité, restreint) ; dépôt public (réponse invariable, aucune confirmation d'existence, identifiant interne non rendu) ; `mes_demandes` n'expose ni journal ni gestionnaire ni code d'arbitrage.
- Pas de régression : les actions existantes et leurs compteurs sont inchangés hors ajouts ; `migrations_versees.txt` non touché.

## Limites de cette revue

Rien n'a été compilé ni exécuté : les affirmations « testé » reposent sur la lecture des tests et sur la CI verte du run 727 à `245020e9`. Le frontend a été lu seulement aux points sensibles (saisie de la pièce, alertes).
