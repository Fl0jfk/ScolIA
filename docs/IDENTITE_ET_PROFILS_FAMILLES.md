# Identité unique, comptes parents et préinscription pré-remplie

Date : 2026-11-02 — Statut : **principes validés** (D-013, D-014), **non implémentés**.
À faire valider par le DPO (délégué à la protection des données) avant développement.

## 1. Principes

1. **Une personne = une identité** au niveau de l'ENT (e-mail vérifié, passkey liée à `scolia.fr`), rattachée à un ou plusieurs établissements. Pas de fusion de comptes : un second établissement **rattache** la même personne, avec sa confirmation.
2. **Comptes parents uniquement**, jamais de compte « famille ». Chaque parent a son propre espace, même au sein d'un même foyer.
3. **Cloisonnement total entre établissements** : un établissement ne voit que ses propres familles. Il ne sait pas qu'une famille est connue ailleurs.
4. **L'ENT sait** qu'une identité est rattachée à plusieurs établissements, uniquement pour la connexion, éviter les doublons et le pré-remplissage voulu par la personne. Pas de statistiques, pas de prospection, pas de croisement de données. Le personnel de l'ENT ne consulte pas ces liens.
5. **Rien n'est transmis sans l'accord du parent** : ni d'un établissement à un autre, ni du profil vers un établissement.

## 2. Deux niveaux de données

| | Identité / profil de la personne (niveau ENT) | Fiche famille dans chaque établissement |
|---|---|---|
| Contenu | e-mail vérifié, passkeys, nom, prénom, coordonnées que la personne a choisi d'y garder | copie propre à l'établissement : responsables, coordonnées, scolarité, justificatifs, paiements… |
| Contrôlé par | la personne | l'établissement (et le parent pour ses propres coordonnées) |
| Responsable RGPD | **l'ENT** (pour cette couche « compte ») | **l'établissement** ; l'ENT est sous-traitant |
| Transmis à un autre établissement | seulement à l'initiative de la personne | **jamais** |

Modification ultérieure : quand le parent change ses coordonnées, on lui propose « Mettre aussi à jour chez : ☑ établissement A ☑ établissement B ». Il choisit ; chaque établissement concerné reçoit une notification « coordonnées modifiées par le parent ». Rien ne change en silence.

## 3. Parcours de préinscription

1. **« Votre adresse e-mail » → Continuer.** L'écran suivant est identique que le compte existe ou non : on ne révèle jamais l'existence d'un compte.
2. **Vérification** : passkey si la personne en a une ; sinon code à 6 chiffres (ou lien) envoyé par e-mail. **Aucune information n'est affichée avant cette étape.**
3. Personne déjà connue : « Nous avons déjà vos informations. Les utiliser pour cette préinscription ? »
   - **Oui** : formulaire pré-rempli ; le parent vérifie et corrige tout avant d'envoyer.
   - **Non** : formulaire libre (sauf nom et e-mail), **même compte**. À la fin : « Enregistrer ces informations dans votre profil ? » (non par défaut).
4. Message affiché dans les deux cas, en clair :
   > « Les informations saisies ici ne sont transmises qu'à cet établissement. Aucune donnée n'est partagée avec un autre établissement sans votre accord. »
5. L'établissement reçoit **sa propre copie**, et rien d'autre que ce qui figure dans le formulaire envoyé. Les autres établissements ne sont ni modifiés ni prévenus.

## 4. Second parent

- Le parent qui fait la démarche peut indiquer l'e-mail de l'autre parent. Cet e-mail **n'est pas vérifié** à ce moment-là.
- À la **validation du dossier par l'établissement**, l'autre parent reçoit une **invitation** : il confirme son adresse (vérification par e-mail) et crée son compte, ou rattache son identité existante.
- **Même foyer, second parent sans e-mail** : il figure sur la fiche famille de l'établissement, **sans compte** et sans connexion. S'il fournit un e-mail plus tard, invitation et création du compte.
- **Parents séparés** : **un e-mail par parent obligatoire** et une fiche par parent. Si la séparation intervient en cours de scolarité, l'établissement sépare les fiches ; chaque parent doit alors avoir son propre e-mail et son propre compte.

## 5. Coordonnées bancaires

- **Pas demandées à la préinscription**, mais à la **validation de l'inscription**.
- Si le parent a déjà donné un IBAN dans un autre établissement : à la fin de la démarche, et seulement s'il a accepté de réutiliser son identité, on lui propose « Souhaitez-vous reprendre également vos coordonnées bancaires ? ». L'IBAN est affiché **masqué** (`FR76 •••• •••• 1234`). S'il en a plusieurs, il choisit.
- S'il accepte, l'IBAN est copié **dans le dossier du nouvel établissement**, et seulement là.
- **Nouveau mandat SEPA à signer dans tous les cas** : chaque établissement (ou OGEC) est un créancier distinct (identifiant créancier SEPA propre) ; un mandat ne se transfère pas.

## 6. Règles de sécurité

- Jamais de pré-remplissage ni de message « compte existant » sans vérification préalable (e-mail ou passkey) : sinon n'importe qui pourrait découvrir qu'une personne a un compte, donc des enfants scolarisés, et lire ses informations.
- Réponses identiques et délais similaires que le compte existe ou non ; limitation du nombre d'envois de codes.
- Codes de vérification à usage unique, de courte durée.
- Journal des rattachements, des réutilisations d'informations et des copies d'IBAN (qui, quand, vers quel établissement).

## 7. Ce qui existe déjà et ce qui manque

| Existant (vérifié) | À construire |
|---|---|
| `ent_user` : un compte par e-mail (unique) | vérification d'e-mail par code ; parcours de préinscription |
| `ent_membership` : un compte peut appartenir à plusieurs établissements, avec un rôle par établissement | invitation qui **rattache** une identité existante au lieu d'en créer une |
| sessions liées à un établissement (`ent_session.etablissement_id`) | profil personnel (coordonnées choisies par la personne) |
| passkeys (`webauthn-rs`) | passkeys liées à la personne (aujourd'hui rangées par établissement) et au domaine `scolia.fr` ; liste des adresses autorisées |
| | table des établissements avec leur sous-domaine (D-012) |
| | invitation du second parent à la validation du dossier ; séparation des fiches |
| | proposition d'IBAN masqué à la validation de l'inscription + nouveau mandat |

## 8. Points à faire valider par le DPO

- L'ENT responsable de traitement pour la couche « compte personne » (identité, connexion, profil) ; conditions d'utilisation et notice d'information correspondantes.
- Réutilisation d'informations et d'IBAN d'un établissement vers un autre à l'initiative du parent (base : demande de la personne, consentement explicite, affichage masqué).
- Durées de conservation du profil et des comptes inactifs.
- Analyse d'impact (AIPD) probable : données d'enfants, plateforme multi-établissements.
