# Revue de sécurité finale — socle 1 (et domaines du socle 2 déjà intégrés)

Date : 2026-11-06. Branche : `ent-socle1-revue-finale` (base `ent-integration-lot4` = `ENT`, 43180f7).
Méthode : revue adverse du code (attaquant élève, parent, professeur, personnel malveillant, personne extérieure, établissement concurrent), en quatre axes menés en parallèle (authentification ; autorisation et routes ; isolation des établissements ; données, HTTP et vulnérabilités classiques), puis vérification à la main des constats élevés, correctifs localisés et tests d'attaque contre un PostgreSQL 16 local jetable (`ent_revue_test`).
Aucun accès à la production, à Next.js, à `main` ni à un secret.

Légende — **FAIT** : vérifié dans le code (et par un test quand c'est indiqué) ; **HYPOTHÈSE** : dépend du déploiement ou d'un enchaînement non rejoué ; « revue » : relevé par lecture du code par un relecteur indépendant, non rejoué par un test.

## 1. Synthèse (verdict par domaine)

| Domaine | Verdict | Confiance |
|---|---|---|
| Authentification (passkeys, sessions, OIDC, anti-abus) | **Solide sur les mécanismes**, aucun contournement de l'obligation de passkey trouvé ; 1 faille élevée de **prise de compte par changement d'e-mail** (RF-01, ouverte, décision) ; 2 failles anti-abus corrigées ici | Élevée sur les mécanismes, moyenne sur l'anti-abus |
| Rôles et droits | Moteur D-017 sain (anti-escalade, familles hors rôles, repli sûr) ; **des chemins annexes oubliaient la portée** : 5 corrigés ici, d'autres restent (exports, bulletins, conseils, devoirs) | Moyenne |
| Isolation des établissements | **Aucune fuite de données exploitable trouvée** ; défense en profondeur incomplète (RLS pilote désactivée, pas de FK composites) ; 2 défauts corrigés (outbox hors RLS, téléphone d'un double membre) | Élevée sur l'applicatif, faible sur la défense en profondeur |
| Données (chiffrement, fichiers, liens, journaux, en-têtes) | Bon niveau ; l'ancienne UI `/app` n'avait **aucune CSP** (corrigé ici par CSP à empreinte) ; champs santé libres en clair, pas de rotation de clé | Moyenne à élevée |
| Socle 2 (dossier, vie scolaire, référentiel, outbox) | Isolation et contrôles d'établissement corrects ; minimisation santé/psy à revoir (métadonnées, journal, dispenses — cette dernière corrigée) | Moyenne |

**Peut-on dire « carré » aujourd'hui ?** Non, pas encore : les mécanismes sont bons, mais RF-01 (élevée) et la liste RF-04 (portée des professeurs sur des chemins annexes) doivent être fermés avant un pilote. Le reste relève de la défense en profondeur et de décisions.

Nombre de constats : **4 élevés** (4 corrigés depuis la branche `ent-socle1-revue-correctifs`), **21 moyens** (5 corrigés, 1 partiellement), **13 faibles** (0 corrigé). Aucun critique.

## 2. Tableau des constats

### Élevés
| # | Constat | Scénario d'attaque | Preuve | Correctif | Statut |
|---|---|---|---|---|---|
| RF-01 | Prise de compte par **changement d'e-mail** | Un Administratif change l'e-mail d'un compte Psy / Infirmerie / Prof / CPE / Compta (ou une Direction celui d'une autre Direction), puis demande « mot de passe oublié » vers sa boîte ; si le compte a une passkey, il lance la récupération D-018 : confirmation, alerte et liens d'annulation **partent tous vers la nouvelle adresse** ; 24 h plus tard il finalise, réinitialise le mot de passe et enrôle sa passkey. Seul le message à l'ancienne adresse peut alerter la victime. | FAIT : `ent-db/src/service.rs` `cible_compte_autorisee` (Administratif → Prof…Psy pour `OperationCompte::Email`, Direction → toute cible non Admin) ; changement d'e-mail sans effet sur passkeys ni demandes de récupération ; la récupération lit l'e-mail courant. Enchaînement complet : HYPOTHÈSE (non rejoué) | Recommandé : (1) appliquer à l'e-mail la hiérarchie des `Identifiants` ; (2) nouvelle adresse effective seulement après confirmation par lien, ancienne adresse prévenue **avec lien d'annulation** ; (3) annuler les demandes de récupération et geler reset / récupération 72 h après un changement d'e-mail fait par un tiers | **Corrigé (branche `ent-socle1-revue-correctifs`, 2026-11-06)** : (1) hiérarchie stricte, jamais un pair — Admin → tout sauf Admin, Direction → ni Admin ni Direction, Administratif → comptes Parent et Élève seulement (choix à valider : l'Administratif n'a plus aucun accès aux comptes du personnel) ; (2) changement en deux temps (migration 255 `ent_email_change`) : lien de confirmation envoyé à la NOUVELLE adresse, ANCIENNE adresse prévenue avec un lien d'annulation qui, jusqu'à 72 h après la confirmation, **rétablit** l'ancienne adresse ; messagerie absente → 503, rien n'est modifié ; (3) un changement demandé par un tiers annule les récupérations de passkey en cours, invalide les liens de mot de passe et **gèle 72 h** (`ent_user.recuperation_gel_jusqu`) la réinitialisation de mot de passe et la récupération de passkey (demande, confirmation, finalisation). Tests : `tests_revue_finale::rf01_prise_de_compte_par_changement_d_email_echoue` (rejoue l'enchaînement d'attaque), `rf01_changement_personnel_confirme_sans_gel`, `tests_l3` adaptés. Reste : page Leptos pour les liens `/email/confirmer` et `/email/annuler` (API : `POST /auth/email/confirmer|annuler`) |
| RF-02 | Parent sans droit « autoriser les sorties » pose des sorties | Parent au droit décoché : `PUT /eleves/{id}/sorties` `{matin,midi,soir:true}` ou demande de sortie exceptionnelle → accepté (sécurité des mineurs) | FAIT : `upsert_autorisation_sortie_for_actor`, `demander_sortie_exceptionnelle_for_actor` n'exigeaient que le lien ; le droit n'était lu que par `ui.rs` | `lien.peut_autoriser_sorties` exigé ; test `tests_revue_finale::rf10_parent_sans_droit_sorties_refuse` | **Corrigé ici** |
| RF-03 | Historique des **e-mails et SMS** lisible par tout le personnel | Un professeur ou un comptable : `GET /mails`, `GET /sms` → 200 derniers envois : adresses et téléphones des familles, sujets nominatifs, corps des SMS (« sanction … pour Prénom Nom ») | FAIT : `list_mails_recents_for_actor` / `list_sms_recents_for_actor` (`is_staff()` seul) | E-mails : `audit.lire` ; SMS : `audit.lire` ou CPE (qui les envoie) ; test `rf12_boites_d_envoi_reservees` | **Corrigé ici** (choix CPE à valider) |
| RF-04 | **Chemins annexes** qui contournent la portée « mes classes » du professeur | (a) `POST /eleves/{id}/appreciations` pour un élève non suivi ; (b) `POST /copilote/ask` « absences non justifiées » → noms de tout l'établissement ; (c) `GET /vs/dispenses` → dispenses médicales de tout l'établissement ; (d) `GET /exports/lsl`, `/exports/lsun` → INE, appréciations de toute classe ; (e) `POST /eleves/{id}/bulletins` → publie un PDF « famille » pour tout élève ; (f) conseils de classe : `conseils.gerer` à l'établissement (mention et envoi aux familles pour tout élève) ; (g) `POST /devoirs`, `DELETE /devoirs/{id}`, `POST /cahier-texte` sans portée de classe | FAIT (a, b, c, d, e : code relu) ; (f, g) revue | (a) `notes.saisir` par élève — test `rf11` ; (b) filtre `absences.lire` (`eleves_autorises`) ; (c) filtre `suivi_eleve` hors gestionnaires — test `rf13` ; (d–g) recommandé : borner par `eleves_autorises` / classes enseignées ; (f) décision produit déjà notée dans `ROLES_ET_DROITS.md` | **Corrigé** (a, b, c avant ; d, e, f, g sur `ent-socle1-revue-correctifs`) : `classes_dans_perimetre` / `exiger_classe_dans_perimetre` (portée `suivi_eleve` ramenée aux classes ; direction et rôles « établissement » inchangés) appliqués aux exports LSL / LSUN (d), à la publication de bulletin par élève suivi (e), aux conseils de classe — création, tenue, dossiers, décisions, listes (f), aux devoirs (création, suppression, liste) et au cahier de textes (g). Tests `rf04d`, `rf04e`, `rf04f`, `rf04g`. Choix (f) : un professeur n'agit que sur ses classes, y compris pour un rôle libre à portée établissement attribué en plus du profil — la portée est celle du profil de base ; à confirmer |

### Moyens
| # | Constat | Scénario | Preuve | Correctif | Statut |
|---|---|---|---|---|---|
| RF-05 | Verrou (compte, IP) **empoisonnable** depuis n'importe quelle IP | « E-mail » `compte-ip:prof@ecole.fr|<IP du collège>` : la clé globale (e-mail brut) coïncidait avec la clé dure de la victime → 6 requêtes la verrouillent 15 min | FAIT : `auth_repo.rs` `verify_credentials` | Clé globale préfixée `email:` ; test `rf03_cle_de_verrou_non_empoisonnable` (échoue sans le correctif) | **Corrigé ici** |
| RF-06 | `POST /auth/password` : mot de passe actuel devinable sans limite | Session volée ou poste laissé ouvert : essais illimités, coût Argon2 libre | FAIT : `change_password` appelait `verify_password` directement | Passe par `verify_current_password` (compteur `reauth:`, 5 / 15 min, n'affecte pas la connexion) ; test `rf05_changement_de_mot_de_passe_limite` | **Corrigé ici** |
| RF-07 | Défi passkey factice discernable (énumération) | `POST /auth/passkey/login/start` : `userVerification` `preferred` (factice) ≠ `required` (vrai) → sépare les comptes membres avec passkey | FAIT : `service.rs` défi factice ; `tests_abus::forme()` ne compare pas les valeurs | Valeur alignée sur `required` | **Partiel** : reste le nombre d'`allowCredentials` (comptes à plusieurs passkeys) — fabriquer le factice avec le moteur réel et comparer les valeurs dans le test |
| RF-08 | Ancienne UI `/app` (+ `/`, `/login`, `/famille`, `/eleve`) **sans CSP**, jeton Bearer en `localStorage` | Un seul oubli d'`esc()` parmi ~2 000 `innerHTML` → vol du jeton (90 j sur mobile) | FAIT : `routes.rs` `ui_page`, `ui.rs` (`localStorage`) ; aucune CSP | CSP à **empreinte SHA-256 du script unique** (`script-src 'sha256-…'`, sans `unsafe-inline` pour les scripts ; `connect-src`/`img-src` `'self'`, `object-src`/`base-uri 'none'`) ; aucun gestionnaire en attribut dans la page (vérifié par test `ui::tests::csp_ancienne_ui_autorise_seulement_le_script_de_la_page`) | **Corrigé ici** (atténuation) ; reste : retirer `/app` de la production au fil du remplacement Leptos, HSTS sur l'API (ou au proxy, HYPOTHÈSE) |
| RF-09 | Consommateurs de l'**outbox hors RLS** | Variable posée `ent.etablissement_id`, la politique lit `app.etablissement_id` : en permissif, aucun bornage ; en strict, consommateurs vides | FAIT : `outbox.rs` `traiter_par`, `100_rls_pilote.sql` ; le test validait le mauvais nom | Pose `app.etablissement_id` (+ `app.rls_strict` en strict), test outbox aligné | **Corrigé ici** |
| RF-10 | **Téléphone** (identité globale) d'un double membre modifiable par l'autre établissement | Administratif de A change le numéro d'un parent aussi membre de B → les SMS de B partent vers le numéro choisi (D-013) | FAIT : `set_user_telephone_for_actor` (`is_member` seul) | Refus si la cible a une autre appartenance (sauf soi-même) ; test `rf15_telephone_d_un_double_membre_non_modifiable` | **Corrigé ici** (même écriture dans le rattachement parent : à traiter avec l'invitation, RF-17) |
| RF-11 | Déni de service par **NAT partagé** | Un élève fait 51 échecs depuis le réseau du collège → toute l'IP verrouillée 15 min ; 6 échecs → un professeur précis verrouillé depuis l'établissement | FAIT : seuils `ENT_LOGIN_MAX_FAILED_IP`=50, compte=5 | Délai croissant ou défi au lieu d'un refus dur ; liste d'IP d'établissement ; contrôle de `ENT_TRUST_PROXY` au démarrage | Ouvert |
| RF-12 | Relais cookie `/web` : l'hôte n'arrive pas à l'API | `ent-web-session` retire `Host` ; seul `X-Forwarded-Host` (si `ENT_TRUST_PROXY` booléen) le transmet : en `souple`, C2 inopérant via `/web` ; en `strict`, `/web/*` → 404 | FAIT (code) ; impact HYPOTHÈSE (aucun test strict via le relais) | Transmettre l'hôte vérifié par le relais ; aligner les deux analyseurs de `ENT_TRUST_PROXY` ; test strict à travers le relais | Ouvert — **bloquant pour passer en `strict`** |
| RF-13 | Déni de service ciblé sur « mot de passe oublié » et récupération | Épuiser les seaux par e-mail (3/h, 2/h) d'une victime chaque heure ; une nouvelle demande annule la confirmation en cours | FAIT : `anti_abus.rs`, `recuperation_repo.rs` | Absorber en 202 sans envoi au lieu de 429 par e-mail ; garder le plafond par IP | Ouvert |
| RF-14 | Direction → **déclaration d'IdP** → prise des comptes sans passkey, dont l'Admin | IdP https quelconque ; rattachement par e-mail vérifié global ; changer l'`issuer` garde les identités liées | FAIT (mécanisme) ; abus HYPOTHÈSE | Déclaration d'IdP réservée Admin / plateforme ; purger les identités au changement d'`issuer` ; pas de rattachement automatique d'un rôle supérieur | Ouvert — décision |
| RF-15 | **Métadonnées santé / psy** lisibles hors `sante.lire` / `psy.lire` | Professeur : `GET /eleves/{id}/documents` (titres et tiroir « santé » de tout élève) ; `GET /eleves/{id}/journal` (traces « consultation du dossier psychologue / du PAI ») | FAIT (`list_documents_meta_eleve_for_actor` : `is_staff` seul) ; journal : revue | Portée `dossier.lire` par élève et masquage du tiroir santé sans `documents.lire_sante` ; exclure le domaine `consultation` du journal hors `audit.lire` | Ouvert |
| RF-16 | Professeur voit motif de **suspension judiciaire**, payeur, part de paiement | `GET /dossier/eleves/{id}/fiche` avec la section famille → lecture `Complete` | revue : `dossier_personnes.rs` `responsables_eleve_for_actor` | Vue réduite pour les professeurs (sans suspension ni paiement) | Ouvert |
| RF-17 | Oracle d'existence d'e-mail **entre établissements** | Rattachement d'un parent, invitation d'un compte élève, changement d'e-mail avec l'adresse d'une personne de B → 409 « e-mail déjà utilisé » | revue : `service.rs` (rattachement, `inviter_compte_eleve`, `set_email`) ; rattachement déjà noté au socle 1 | Réponse neutre + invitation acceptée par la personne | Ouvert |
| RF-18 | Champs de santé libres **en clair** | Sauvegarde ou lecture SQL : `ent_infirmerie_passage.note_soins`, `motif`, `ent_pai.motif`, dispenses médicales | FAIT : `champs_chiffres.rs` (6 tables couvertes) | Étendre `CHAMPS_CHIFFRES` | Ouvert (limite connue) |
| RF-19 | **Pas de rotation** de la clé des champs | Changer `ENT_SECRETS_KEY` rend tout illisible ; une fuite de clé n'est pas remédiable | FAIT : format `enc:v1:` sans identifiant de clé | `enc:v2:<kid>:`, déchiffrement multi-clés, rechiffrement | Ouvert (limite connue) |
| RF-20 | **RLS pilote** désactivée par défaut, 22 tables | Un oubli de filtre sur `ent_personne`, `ent_eleve_lien`, `ent_absence_*`, `ent_note`… n'est rattrapé par rien | FAIT : `ENT_RLS_MODE` défaut `off` ; couverture | Activer `strict` au niveau du rôle applicatif (`ALTER ROLE … SET app.rls_strict='on'`), étendre la couverture, vérifier que le rôle n'est ni propriétaire ni `BYPASSRLS` | Ouvert — décision de calendrier |
| RF-21 | Désignation des **responsables de groupe** sans anti-escalade | Titulaire de `roles.gerer` avec une portée `mes_groupes` s'ajoute responsable de tous les groupes | revue : `droits_repo.rs` `definir_responsables_groupe_for_actor` | Exiger la couverture, ou réserver à l'Admin | Ouvert |
| RF-22 | Règles d'**absence** incohérentes | Élève mineur déclare sa propre absence par `POST /eleves/{id}/absences` (interdit par le parcours famille) ; tout professeur déclare ou annule une absence pour n'importe quel élève | revue : `actor_can_declare_absence`, `annuler_absence_vers_present_for_actor` | Une seule règle (majeur, lien, portée) | Ouvert |
| RF-23 | **Droits cochés** des responsables non appliqués partout | `GET /eleves/{id}/commissions` sans `peut_voir_vs` ; réinscription (confirmer, options, documents) sans droit particulier | revue | Exiger le droit coché correspondant | Ouvert |
| RF-24 | Détection de **PDF actif** contournable | `/#4AavaScript`, jetons dans un flux compressé, `/OpenAction`, `/AA`, `/EmbeddedFile` non vus | FAIT : `ent-object/src/upload.rs` `pdf_a_jeton` | Décoder `#xx`, refuser `/OpenAction`, `/AA`, `/EmbeddedFile` ; ou PDF déposés par des tiers toujours en pièce jointe (aujourd'hui l'affichage en ligne garde `default-src 'none'` + `nosniff`) | Ouvert |
| RF-25 | **SSO web hors cookie** | Le relais interdit `/auth/oidc/` : le SSO web reste sur l'ancienne UI qui stocke le jeton en `localStorage` (contraire à A6) | FAIT : `ent-web-session/src/lib.rs` | Route de relais SSO avec cookie `__Host-` et CSRF | Ouvert |

### Faibles (ouverts, recommandés)
| # | Constat | Preuve |
|---|---|---|
| RF-26 | `LIKE` sans échappement (`%`, `_`) : recherches larges et coûteuses (personnel seulement, `LIMIT`) | FAIT : `foyer_repo.rs` `list_foyers`, `auth_repo.rs` `search_staff` (reprendre `ESCAPE '\'` de `eleve_repo.rs`) |
| RF-27 | Modification de rôle non atomique ; journal écrit après le commit | revue : `droits_repo.rs` `modifier_role_for_actor` |
| RF-28 | Lien signé `/fichiers/{jeton}` : porteur rejouable (60 s, 900 s max), non comparé à l'hôte (D-012) ; préfixe de clé `lab/` accepté sans condition d'environnement | FAIT : `routes.rs` `servir_fichier_signe`, `ent-object/src/presign.rs` |
| RF-29 | `map_db` classe les erreurs par sous-chaîne (« note », « psy »…) : fragile, risque d'exposer un futur message | FAIT : `routes.rs` `map_db` |
| RF-30 | `ENT_CORS_ORIGINS=*` → `CorsLayer::permissive()` possible en production | FAIT : `ent-api/src/lib.rs` |
| RF-31 | `X-Forwarded-Host` : première valeur prise (si le proxy ajoute au lieu de remplacer, le client choisit) | FAIT : `tenant.rs` `hote_brut` |
| RF-32 | Politique de sécurité liée au profil (Admin/Direction), pas à `securite.politique` : une Direction peut retirer l'obligation de passkey de l'Admin | revue : `securite_repo.rs` |
| RF-33 | E-mail en clair dans le `payload` des événements `auth` du journal (lisible par `audit.lire`) | FAIT : `service.rs` (journal `auth`, `json!({ "email": … })`) — remplacer par une empreinte comme `audit_compte` |
| RF-34 | Fil de messagerie et factures partagés entre tous les responsables du foyer (cas des parents séparés) | revue — décision produit |
| RF-35 | `creer_edt_contrainte_for_actor` accepte un `prof_user_id` non membre (intégrité, pas de fuite) | revue |
| RF-36 | OIDC : `link_identity` avant le contrôle d'appartenance (lien orphelin) ; `issuer` de la découverte non comparé à l'attendu ; `reqwest` suit les proxys d'environnement (contourne l'épinglage DNS, HYPOTHÈSE) | revue : `service.rs`, `ent-oidc/src/provider.rs`, `transport.rs` |
| RF-37 | Garde SQL statique : angles morts (table derrière une constante, vues, `USING`, `FROM ONLY`, alias) ; aucune FK composite `(etablissement_id, id)` | FAIT : `ent-db/tests/isolation_sql_statique.rs` |
| RF-38 | `GET /sepa` change l'état (factures « exportées ») : rejouable par un lien, pas idempotent | revue |

## 3. Ce qui a été vérifié et tient (preuves)
**Authentification**
- Session revérifiée en base à chaque requête : révocation, échéance, inactivité, appartenance jointe (retrait de rôle immédiat) — `ent-db/src/auth_repo.rs` `resolve_bearer` ; jetons 256 bits stockés hachés (`ent-auth/src/token.rs`).
- Session « enrôlement » : seules 5 routes exactes (`/auth/me`, `/auth/logout`, `/auth/passkeys`, `register/start|finish`) — `ent-api/src/auth.rs` `ROUTES_ENROLEMENT` ; mot de passe redemandé sauf SSO ; défi lié au compte, usage unique, plafond de 10 atomique.
- Réinitialisation : jeton haché, usage unique atomique, 30 min, toutes sessions révoquées, aucune session créée, 202 neutre ; la passkey reste exigée ensuite.
- WebAuthn : vérification de l'utilisateur exigée, compteur atomique (`FOR UPDATE`), `rp_id` fail-closed hors labo.
- OIDC : state consommé (`DELETE … RETURNING`), cookie de liaison comparé en temps constant, PKCE S256, nonce, RS256 seul (ni `none` ni HS256), `iss`/`aud`/`exp`, `email_verified`, garde SSRF (plages interdites, adresse épinglée, redirections revalidées), secret chiffré.
- Récupération D-018 : transitions atomiques, `finaliser` sous `FOR UPDATE`, annulation par connexion passkey.
- IP fiable : socket par défaut, `X-Forwarded-For` seulement derrière `ENT_TRUST_PROXY`, IPv6 agrégées en /64 ; seaux par e-mail normalisés.
- Relais cookie : `__Host-ent_session` `HttpOnly; Secure; SameSite=Lax`, `Origin` + `X-ENT-CSRF` sur toute méthode non sûre, routes émettrices de jeton interdites, `client=web` imposé.
- `/auth/register` et `/labo/bootstrap` absents du binaire sans la feature `labo` (contrôle CI).

**Autorisation**
- Moteur D-017 : droits relus à chaque requête, familles et élèves hors rôles libres, cibles désignées vérifiées dans l'établissement, `roles.gerer` / `securite.politique` à portée établissement seulement, anti-escalade sur création / modification / attribution, journal ; repli sans contexte = modèle du profil (jamais plus) — `ent-db/src/droits_repo.rs`, `droits_ctx.rs`.
- Lien familial actif (suspension, dates) lu en un point (`ent_eleve_lien_actif`) ; santé / psy contrôlés par élève et tracés ; messagerie, notes, pièces : contrôle par élève de bout en bout.
- Inventaire des routes : les 6 fichiers qui déclarent des routes (`routes.rs`, `dossier.rs`, `referentiel.rs`, `referentiel_s2.rs`, `roles.rs`, `vie_scolaire.rs`) sont lus par `tests/separation/routes_src.rs`, et un test échoue si un autre fichier en déclare (vérifié par grep : aucun autre).

**Isolation**
- Session liée à son établissement, session de A sur l'hôte de B → 401 (strict et souple si l'hôte est connu) ; `etablissement_id` du corps ignoré sur une adresse connue.
- Écritures du socle 2 avec identifiants du client (personne, foyer, élève, contacts, motif, pièce, incident, sanction, site, niveau, MEF, classe, groupe, matière, salle) : établissement contrôlé avant écriture ; déclencheurs `classe_id` / personne.
- Outbox : lectures filtrées par établissement, distributeur `e.etablissement_id = o.etablissement_id` ; export SEPA sous `TenantTx`.
- Liens signés : HMAC sur clé, établissement, destinataire, échéance, disposition, nom et type ; clé d'un autre établissement refusée à l'émission et au service ; 60 s par défaut.
- Politique RLS : valeur absente → NULL, `FORCE ROW LEVEL SECURITY`, `USING` = `WITH CHECK`, aucun `SECURITY DEFINER`.

**Données et HTTP**
- Injection SQL : aucune valeur client concaténée ; `format!` limités à des noms constants ; pas de tri fourni par le client.
- XSS : un seul `inner_html` Leptos (icônes statiques) ; texte échappé par défaut ; aucune réponse HTML avec contenu utilisateur ; iCal échappé.
- `/web` : CSP à nonce sans `unsafe-inline`, `frame-ancestors`, `base-uri 'none'`, `object-src 'none'` ; `/fichiers/{jeton}` : `nosniff`, `no-store`, `default-src 'none'; sandbox`, CORP, disposition assainie, types actifs forcés en pièce jointe.
- Dépôts : type réel par signature, ZIP lu par le répertoire central sans décompression (20 000 entrées max), UTF-16 et HTML refusés, clé d'objet contrôlée (pas de traversée).
- Chiffrement AES-256-GCM, nonce aléatoire, AAD `table.colonne:id`, fail-closed sans clé.
- Journaux : pas de jeton ni d'e-mail dans `tracing` (hors RF-33, journal métier) ; span sur le gabarit de route ; erreurs SQL → « Erreur interne ».
- Déni de service : corps 25 Mio, délai 120 s, `CatchPanicLayer`, listes bornées (`clamp`), limiteur de débit des routes publiques.

## 4. Tests exécutés (local, PostgreSQL 16 jetable `ent_revue_test`, Rust du dépôt)
- Nouveaux tests d'attaque `ent-db/src/tests_revue_finale.rs` : `rf03`, `rf05`, `rf10`, `rf11`, `rf12`, `rf13`, `rf15` — **7 verts**. Leur échec sur le code d'origine n'a pas été rejoué : il est déduit de la lecture (HYPOTHÈSE forte pour `rf03`, `rf05`, `rf10`, `rf12`, `rf15` ; `rf13` ne prouve rien si aucune dispense n'existe ce jour-là dans le cast).
- `ent-db` `tests_outbox` (11) et `copilote` (6) : verts après correctifs.
- `ent-api` `ui::tests::csp_ancienne_ui_autorise_seulement_le_script_de_la_page` (unitaire, sans base).
- Suite complète `cargo test --locked -p ent-api -p ent-db -p ent-lab --features ent-api/labo,ent-lab/labo -- --test-threads=1` (après correctifs) : **354 réussis, 0 échec**, 1 ignoré préexistant (dont matrice de séparation 40, tenant par hôte 8, dépôts et liens de fichiers, 193 tests `ent-db`).
- CSP de `/app` dans Chromium (Playwright, API labo locale) : page de connexion rendue, `<img onerror>` injecté **bloqué** ; écrans admin, professeur, famille et élève chargés **sans aucune violation CSP**.
- **Non fait** : `cargo deny` / `cargo audit` (outils absents de l'environnement ; ils tournent en CI), suites frontend `web/` (non modifiées), clippy, parcours complets de l'ancienne UI au-delà de l'accueil de chaque profil.

## 5. Risques résiduels acceptés et décisions à prendre
1. ~~RF-01~~ — appliqué (voir tableau) ; reste à valider : l'Administratif ne change plus que les e-mails Parent / Élève.
2. ~~RF-04 (d à g)~~ — appliqué (voir tableau).
3. **RF-14** — déclaration d'un IdP par la Direction (recommandation : Admin ou plateforme seulement).
4. **RF-20** — calendrier d'activation de la RLS `strict` et de son extension ; tant qu'elle est `off`, l'isolation repose sur l'applicatif seul (aucune fuite trouvée, garde statique et matrice en place).
5. **RF-34** — partage des fils et des factures entre responsables séparés.
6. Choix fait ici (à valider) : historique des SMS ouvert au CPE en plus de `audit.lire` (RF-03).
7. Déjà connus et toujours vrais : antivirus absent (`AucuneAnalyse`), pas de KMS, type de client `mobile` déclaré par le client (E7), boîte e-mail compromise = récupération possible malgré le délai (compromis D-018).

## 6. Corrections restantes avant un pilote (ordre recommandé)
1. ~~RF-01~~ et ~~RF-04 d–g~~ — corrigés (`ent-socle1-revue-correctifs`) ; plus aucune faille élevée ouverte connue.
2. Page Leptos des liens d'e-mail (confirmation / annulation), puis :
3. RF-15, RF-16 (minimisation santé / psy / judiciaire vis-à-vis des professeurs).
4. RF-12 (hôte à travers le relais) puis passage `ENT_TENANT_PAR_HOTE=strict` ; RF-31.
5. RF-11, RF-13 (anti-abus sans déni de service ciblé) ; RF-07 (reste du défi factice).
6. RF-14, RF-21, RF-32 (escalades d'administration).
7. RF-22, RF-23 (règles familles / absences).
8. RF-20 (RLS strict au rôle applicatif sur les tables pilotes) ; RF-18 (champs santé) ; RF-19 (rotation) avant la production.
9. RF-17, RF-24, RF-25, puis les faibles.

## 7. Remarques hors sécurité relevées en passant
- Ancienne UI : erreur JavaScript `settle is not defined` sur l'accueil professeur (`ui.rs`, `settle` utilisé hors de la fonction qui le définit) — préexistante, sans lien avec la CSP ; enregistrement du service worker refusé (portée `/` pour un script sous `/static/`).
