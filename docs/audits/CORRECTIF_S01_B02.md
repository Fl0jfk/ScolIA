# Correctif S-01 et B-02 (audit Fable 1)

- **Date :** 2026-11-11 — **Branche :** `ent-audit-correctifs-sso` (partie d'`ENT`)
- **Références :** D-009, D-012, D-013, D-056 ; `docs/socles/ORGANISME_ADRESSES.md` §6.5 et §8 ; `docs/audits/AUDIT_FABLE_1.md` S-01 et B-02 (sur `ent-mc0hoe`).
- **État :** implémenté et testé sur branche ; à valider par le pilote puis par Florian.

## 1. Les deux failles (rappel)

- **S-01** : la direction de A déclare un fournisseur OIDC qu'elle contrôle. Il affirme `email = victime`, `email_verified = true`. L'ENT rattache ce `sub` à l'identité **globale** de la victime (membre de A et de B), ouvre une session d'enrôlement et accepte une passkey sans mot de passe. La passkey vaut pour la personne entière : l'attaquant entre dans B.
- **B-02** : un administrateur d'établissement enregistre n'importe quel nom d'hôte (`college-b.scolia.fr`, `app.scolia.fr`…), premier arrivé premier servi, sans preuve. Le slug est aussi modifiable.

## 2. Modèle de confiance

**Faits de départ.** L'identité (`ent_user`, e-mail, mot de passe, passkeys) est globale (D-013). Une appartenance (`ent_membership`) est propre à un établissement. Un fournisseur OIDC est déclaré par un établissement et ne peut ouvrir qu'une session dans cet établissement (déjà vrai : `state.etablissement_id`).

**Règles retenues.**

| # | Règle | Ce qu'elle ferme |
|---|---|---|
| T1 | Un fournisseur OIDC ne fait foi **que pour son établissement** : il ouvre une session dans cet établissement et rien d'autre. Il ne prouve rien sur l'identité globale. | Passage de A vers B |
| T2 | **Pas de rattachement automatique par e-mail** si l'identité trouvée (a) n'est pas membre de l'établissement du fournisseur, (b) a une appartenance dans un **autre** établissement (quel que soit son état), ou (c) a déjà au moins une passkey. Le contrôle d'appartenance a lieu **avant** l'écriture du rattachement (audit S-01 d). Refus uniforme « compte inconnu ». | Prise de l'identité globale par e-mail |
| T3 | **Enrôlement d'une passkey sans mot de passe** (session d'enrôlement ouverte par le SSO) permis **seulement** si l'identité n'appartient qu'à l'établissement du fournisseur. Sinon le mot de passe actuel est exigé (refus `MotDePasseActuelInvalide`, comme pour une session ouverte par mot de passe). | Passkey de l'attaquant posée sur une personne d'un autre établissement, y compris par un rattachement antérieur au correctif |
| T4 | **Domaine validé par la plateforme.** Un administrateur d'établissement ne peut plus ajouter ni retirer un domaine. L'ajout, le retrait et la déclaration d'un établissement passent par l'opérateur ScolIA (`ent-api plateforme-domaine …`, accès à la base requis), qui refuse les hôtes réservés de la plateforme. | Appropriation d'adresse, hameçonnage sur adresse légitime |
| T5 | **Slug immuable** pour l'administrateur d'établissement : il peut renommer le nom affiché, pas changer le slug. | Changement d'identifiant court par un administrateur |

**Ce qui fait foi après le correctif.** Une passkey n'est posée que par le titulaire, prouvé par (i) son mot de passe actuel, ou (ii) une connexion SSO de l'établissement qui est, à cette date, **le seul** auquel l'identité appartient. Dans le cas (ii), l'établissement est l'autorité naturelle du compte : c'est lui qui l'a créé et qui en répond.

**Pourquoi pas une validation plateforme des fournisseurs OIDC (audit S-01 c) dès maintenant.** T1 à T3 retirent au fournisseur toute portée hors de son établissement ; une validation plateforme exigerait une colonne d'état (migration) et un circuit opérateur. Recommandée avant l'ouverture à un deuxième établissement réel (avec le bornage du domaine d'e-mail), pas bloquante ici.

**Preuve DNS (domaines personnalisés).** Prévue, pas codée : aucun résolveur DNS n'est dans les dépendances (`Cargo.lock`) et l'ajout d'une dépendance demande une analyse de licence et l'accord de Florian (CLAUDE.md). Conception proposée : enregistrement `TXT _scolia-verification.<domaine>` portant `HMAC(secret plateforme, établissement | domaine)`, vérifié à l'ajout et revérifié périodiquement ; jamais pour un hôte sous le domaine de la plateforme (réservé à l'opérateur). D-056 n'en a pas besoin : les adresses `<uai>.scolia.fr` sont créées par l'opérateur à la déclaration de l'UAI.

## 3. Hôtes réservés (T4)

Refusés même pour l'opérateur : le domaine de plateforme lui-même (`scolia.fr`, et `ENT_RP_ID` s'il diffère) et ses sous-domaines `www`, `admin`, `api`, `app`, `auth`, `support`, `status`, `demo`, `aide`, `mail`, `static`, `fichiers` (liste de `ORGANISME_ADRESSES.md` §6.5). Les libellés au format UAI restent permis à l'opérateur (c'est lui qui déclare les UAI). Un domaine déjà servi par un autre établissement reste refusé (conflit), jamais réattribué.

## 4. Cas limites

| Cas | Comportement |
|---|---|
| Première connexion SSO d'un professeur créé par A (seulement dans A, sans passkey) | Rattachement par e-mail accepté (inchangé) ; enrôlement de passkey sans mot de passe accepté. |
| Même professeur, également en poste dans B | Rattachement refusé. Il se connecte par mot de passe (lien « mot de passe oublié » si le compte n'en a jamais choisi). Le rattachement depuis une session ouverte est à construire (§6). |
| Compte avec passkey, jamais rattaché | Rattachement refusé : la personne a ses propres secrets. |
| Rattachement **existant** (antérieur au correctif) vers une identité multi-établissements | La connexion SSO ouvre toujours une session dans l'établissement du fournisseur seulement ; si la personne a une passkey, elle est exigée (inchangé) ; sinon l'enrôlement sans mot de passe est refusé (T3). |
| Compte rattaché alors qu'il n'était que dans A, qui rejoint B ensuite | L'enrôlement SSO sans mot de passe devient impossible dès l'appartenance à B. **Risque résiduel** : une passkey posée par A avant l'arrivée dans B reste valable dans B. À traiter avec le rattachement confirmé de D-013 (proposer à la personne, à l'acceptation de l'invitation de B, la revue de ses passkeys ; §6). |
| Appartenance dans B fermée ou suspendue | Compte comme « autre appartenance » (toutes les lignes, quel que soit l'état) : prudence. |
| Oracle d'appartenance | Le refus du rattachement révèle à un fournisseur contrôlé par A un bit (« cette personne de A a une autre appartenance ou une passkey »). Même bit que le 403 déjà renvoyé par `cible_compte_autorisee` pour un changement d'e-mail. Aucun journal d'établissement ne le consigne ; trace technique sans identifiant. |
| Gel après changement d'adresse (RF-01) | Inchangé : rattachement refusé pendant le gel. |
| Administrateur qui retire son propre domaine | Refusé : passe par l'opérateur (un compte d'administrateur volé ne coupe plus l'établissement). |
| Démo et labo | `ent-lab` et les tests utilisent les fonctions de plateforme ; aucun changement de comportement visible. |
| Mot de passe « jamais choisi » (secret aléatoire du provisionnement) | Indiscernable aujourd'hui d'un mot de passe choisi (`password_hash` toujours rempli). T2 ne s'appuie donc pas sur la présence d'un mot de passe ; voir §6. |

## 5. Existant et migration

- **Aucune migration de schéma** : les règles s'appuient sur `ent_membership`, `ent_passkey_credential` et `ent_oidc_identity` existants. `MIGRATION_TAG` et `migrate.rs` sont inchangés.
- **Aucune base ScolIA n'est déployée** (PILOTAGE règle 15) : pas de rattachement de production à reprendre. Pour une base de répétition, requête de contrôle à passer par l'opérateur avant la mise en service :
  ```sql
  SELECT i.provider_id, i.user_id
    FROM ent_oidc_identity i
    JOIN ent_oidc_provider p ON p.id = i.provider_id
   WHERE EXISTS (SELECT 1 FROM ent_membership m
                  WHERE m.user_id = i.user_id AND m.etablissement_id <> p.etablissement_id);
  ```
  Chaque ligne est un rattachement qui ne serait plus créé aujourd'hui ; le retirer si la personne ne l'a pas confirmé.
- **Domaines déjà enregistrés** : conservés. L'opérateur les revoit (`ent-api plateforme-domaine lister`).
- **API** : `POST /etablissement/domaines` et `DELETE /etablissement/domaines/{domaine}` répondent 403 pour un administrateur d'établissement ; `PUT /etablissement/fiche` refuse un changement de slug (403).

## 6. Hors périmètre, recommandé ensuite

1. **Rattachement SSO depuis une session ouverte** (« lier mon compte Google ») : `oidc_start` en mode liaison, état lié à l'utilisateur connecté (migration de la plage 700–709 : colonne `ent_oidc_state.lien_user_id`). Rend T2 confortable pour les comptes multi-établissements.
2. **Colonne « mot de passe choisi le »** pour étendre T2 aux comptes qui ont choisi un mot de passe (protection du psychologue ou de l'infirmière dans A seul ; même famille que l'objection sur `personnels.gerer`).
3. **Validation plateforme des fournisseurs OIDC** et bornage du domaine d'e-mail aux domaines vérifiés de l'établissement (S-01 c).
4. **Preuve DNS** pour les domaines personnalisés (§2), après accord sur un résolveur.
5. **Revue des passkeys à l'acceptation d'un nouvel établissement** (risque résiduel §4).
6. S-03 (réauthentification récente) et S-13 (liste fermée d'origines WebAuthn) sont traités à part.

## 7. Tests de non-régression

`ent-rust/crates/ent-db/src/tests_correctif_s01_b02.rs` (base locale, feature `labo`) :

- scénario S-01 complet : fournisseur de A, victime membre de A et B sans passkey, passkey rendue obligatoire → rattachement refusé, aucun `ent_oidc_identity` écrit, aucune session ;
- victime avec passkey → rattachement refusé ;
- victime non membre de A → refus et aucun rattachement écrit (contrôle avant écriture) ;
- compte de A seul → rattachement accepté (non-régression) ;
- rattachement antérieur vers une identité multi-établissements → session dans A seulement, enrôlement sans mot de passe refusé ; compte de A seul → enrôlement sans mot de passe toujours accepté jusqu'à la vérification de la passkey ;
- scénario B-02 : l'administrateur de A ne peut ni ajouter `college-b.scolia.fr`, ni retirer son domaine, ni changer son slug ; l'opérateur ajoute, refuse les hôtes réservés et les conflits, retire.
