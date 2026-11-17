# Socle 3 — Lots I3 (pièces réelles) et I4 (accords des responsables) : livraison

Branche `ent-socle-3-code-i3i4` (partie d'`origin/ENT`, fusionnée avec `origin/ENT` avant la CI, règle 28).
Conception : `03_INSCRIPTIONS.md` §4.4, §8.4, §8.6, §11.3, §15.2, §15.4 (branche `ent-mc0hoe`).

Statut : **implémenté et testé en local** (PostgreSQL 16 jetable) ; **non validé** tant que la CI n'a pas rendu son verdict ;
non prêt pilote, non prêt production. Aucun écran (I14b).

## Ce qui est livré

| Lot | Contenu | Migrations |
|---|---|---|
| I3 | règles de pièces par campagne et conditions typées en lignes (D-036) ; dépôt **réel** par le socle 7 (type réel contrôlé sur le contenu, quarantaine, antivirus fermé par défaut, photos → un PDF, PDF désarmé) ; pièce de famille déposée une fois pour la démarche ; réutilisation **proposée** puis confirmée ; contrôle (accepter, refuser avec motif type, dispenser) ; rattachement à l'élève à la validation (mêmes objets) ; purge des fichiers des dossiers non aboutis | 310, 311 |
| I4 | accords par dossier, responsable et objet ; réponse en ligne **sans compte** (lien à usage unique + code à 6 chiffres), depuis un compte, ou sur papier ; désaccord **bloquant** ; levée **avec pièce** par la direction ; dispense motivée ; trace d'ajout seul | 312 |

Plage utilisée : 310 à 312 ; 313–329 libres (vérifié sur toutes les branches distantes). `MIGRATION_TAG` inchangé (3808) ; 310–312 sont appliquées **après 3704** dans `migrate.rs` (311 modifie `ent_doc_piece`).

## Décisions de conception (à relire)

1. **Quatrième sujet « dossier » dans `ent_doc_piece`** (socle 7, §6 : « une démarche (dossier d'admission avant l'existence de l'élève) »). Avant la validation l'élève n'existe pas et le socle 3 n'en crée aucun : sans cela, aucune pièce d'admission ne peut être un fichier du socle 7. Garde d'immutabilité étendue. Le socle 7 reste propriétaire de tout ce qui touche au fichier ; le socle 3 appelle ses commandes (`documents_pieces_dossier.rs`) : la garde `garde_inscriptions_ecritures` n'a reçu **aucune exception**.
2. **Catalogue des types** : celui du socle 7 (fermé). Trois types ajoutés : `accord_responsable`, `decision_orientation`, `notification_affectation`. Un code libre de campagne est traité comme une pièce « autre » motivée par son libellé (jamais ce que le socle 7 interdit). Les pièces d'identité ne se demandent pas par une campagne (non conservées).
3. **Autorité** : seuls les déclarants d'autorité « oui » décident. « inconnue » ne décide pas tant que l'établissement n'a pas tranché. L'**autorité exclusive** déclarée ne vaut que si une **décision de justice acceptée** est au dossier (sinon un parent écarterait l'autre en la déclarant). L'autorité est relue au moment de la réponse (lien reçu avant un retrait d'autorité : refusé).
4. **Jeton et code jamais stockés** : dérivés (HMAC) d'un secret serveur ; le code dépend d'un compteur (cinq envois, dix minutes, cinq essais). Réponses publiques identiques pour tout lien refusé, pour un accord comme pour un désaccord.
5. **Règle `un_seul`** : le dépôt vaut accord du déposant ; mais un désaccord déclaré par l'autre titulaire **bloque quand même** (art. 372-2, §4.4). Réinscription : la confirmation de la famille vaut accord ; on n'exige que l'absence de refus. Dossier antérieur à I4 sans accord ouvert : idem (reprise sans casse).
6. **« À ne pas solliciter »** : aucune demande, relance ni copie ; l'accord se **dispense** par la direction (`inscriptions.decider`), sinon le dossier attend.
7. **Dépôt famille** : déposant vérifié du dossier seulement (comme le reste de l'espace) ; le second parent sans compte ne dépose pas.
8. **Signature (D6)** : non recodée ; port `PortSignatureAccord` (trait sans implémentation) à brancher quand D6 sera fusionné.
9. **Notifications** : le moteur du socle 8 exige un élève et des liens ; une famille d'admission n'en a pas. Les événements `inscription.piece_demandee|deposee|controlee|pieces_purgees` et `inscription.accord_demande|repondu` sont émis (sans donnée personnelle) ; les natures du socle 8 seront déclarées au lot N11. Le lien d'accord part par la file d'envoi des inscriptions (type `information`, reconnu au préfixe de la clé : la contrainte de 309 est rejouée à chaque reprise et ne tolère pas de nouveau type).

## Garde-fous vérifiés
- Décision, validation, admission : refusées pendant `bloque_desaccord` (la vue historique `statut` ne change pas pendant le blocage : c'est `etat` qui fait foi — trou fermé dans `decider_avance`).
- Levée : jamais par le raccourci « motif seul » ; une pièce acceptée, du bon type, non déjà utilisée pour un autre désaccord ; accord écrit → « donné », décision de justice → « dispensé ».
- Pièce bloquante : validation refusée (message nommant la pièce) ; dispense d'une pièce bloquante réservée à la direction ; type sensible : droit du domaine du socle 7 en plus.
- Fichier infecté ou illisible : la pièce repasse « refusée » seule, jamais servie.
- Purge : destruction au magasin **avant** marquage au registre (reprise après panne), objets partagés avec une pièce d'élève jamais détruits, ligne et historique conservés, idempotente, tâche horaire `ent-api`.

## Fichiers
- `ent-db` : `inscriptions_pieces.rs`, `inscriptions_accords.rs`, `documents_pieces_dossier.rs`, extraction de `preparer_depot` / `inserer_depot` dans `documents_pieces.rs`, `inscriptions_reactions.rs` (envoi du lien et du code), garde statique du pool.
- `ent-types` : `inscriptions_pieces.rs` (types neufs ; aucun type existant modifié, le web n'est pas touché hors tests).
- `ent-api` : `inscriptions_pieces.rs` (routes), anti-abus des trois routes publiques `/accords/{id}/consulter|code|repondre`, listes fermées des routes publiques, séparation (`domaine_inscriptions_i3_i4`).
- Registre des données : 5 tables ; compteur RLS **+5** (après fusion : ENT +6, I3/I4 +5).
- Tests : `tests_inscriptions_i3_i4.rs` (19 scénarios), `tests_inscriptions.rs` (dépôts réels). Aucun test ne dépend de l'heure réelle ; aucun `#[ignore]`.

## Écarts avec le périmètre demandé
- **Web** : l'ancienne route `POST /preinscription/dossiers/{id}/pieces/{code}` (nom de fichier seul) subsiste pour ne pas casser l'écran ; elle ne peut plus produire une pièce acceptable. Deux tests web (`demo_p1`, `inscriptions`) ne déposent/acceptent plus de pièce (le serveur de test n'a pas d'antivirus) et la pièce du jeu est non bloquante. L'écran de dépôt réel et l'écran public des accords relèvent de I14b.
- **Traitement RGPD** : aucun nouveau ; le traitement « inscriptions » du registre couvre les nouvelles tables. Durée de conservation des non-aboutis : 180 jours (`PURGE_NON_ABOUTIS_JOURS`) **à valider avec le DPO** (§15.4).
- **Conservation de l'accord papier d'un dossier validé** : rattaché à l'élève comme les autres pièces conservées ; question de minimisation à trancher.
- Mobile (I15) : non traité.
- `expedier_envois` traite une file **commune** à tous les établissements : les tests de l'accord font plusieurs passes.

## Vérifications
- clippy 1.99.0, `--workspace --tests --features ent-db/labo,ent-api/labo,ent-lab/labo` : **0 avertissement** (rustc compris).
- Tests locaux : I3/I4, inscriptions I2, registre, RLS, gardes statiques, migrations, `ent-api` (séparation, inscriptions, recette sécurité).
- `web/Cargo.lock` : `cargo metadata --locked` passe.
- CI : voir le message au pilote (SHA, run).
