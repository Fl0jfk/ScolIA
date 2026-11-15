# Procédures RGPD utilisables dès le premier jour (socle 27, lot G0)

> **Statut : proposé, à valider par l'établissement et son DPO.** Aucun écran n'existe encore (D-063 : les écrans viennent en dernier lot du socle 27). Cette page sert **avant** les écrans, sur papier ou par courriel, jusqu'aux lots G3 (guichet des droits) et G5 (violations). Les délais viennent du RGPD ; les points marqués [NV] restent à faire confirmer (voir `docs/socles/27_RGPD_ARCHIVES.md` §4 et §24).

## 1. Qui fait quoi
- **Responsable de traitement** : l'établissement (chef d'établissement ; dans le privé, l'organisme gestionnaire ou le chef d'établissement selon la répartition écrite, Q-RG1).
- **ScolIA** : sous-traitant (art. 28) : aide l'établissement, n'instruit pas les demandes à sa place.
- **DPO** : **peut ne pas être nommé** (cas actuel de Florian). Tant qu'aucun DPO n'est nommé, ces rôles sont tenus par le chef d'établissement, et le logiciel affiche « en attente de DPO » : cela **ne bloque rien**, sauf la **purge réelle** (aucune donnée n'est détruite tant qu'une catégorie n'est pas validée).

## 2. Demande de droits d'une personne (accès, rectification, effacement, limitation, opposition, portabilité)
1. **Enregistrer** la demande le jour de réception : date, canal, demandeur, objet. Le délai d'**un mois** court dès la réception (prolongeable de deux mois si la demande est complexe, avec information motivée dans le premier mois).
2. **Vérifier l'identité et la qualité** de façon proportionnée : compte connecté, ou courriel connu au dossier avec code ; une pièce d'identité n'est consultée que pour vérifier, puis **détruite**. Seul un titulaire de l'autorité parentale à la date de la demande parle pour un mineur ; un parent n'obtient **jamais** les données propres de l'autre parent ni une adresse protégée.
3. **Instruire** : lister les domaines concernés (scolarité, vie scolaire, finances, messagerie, santé…), rassembler les données, **masquer les tiers**, faire relire par une deuxième personne. Rien ne part sans relecture.
4. **Exclusions** : une demande d'effacement ne s'applique pas à une donnée qu'une obligation légale impose de garder (registre des élèves, pièces comptables…) ; le refus est motivé par écrit.
5. **Remettre** la réponse par un canal protégé (jamais en pièce jointe non chiffrée d'un courriel ordinaire).
6. **Clore et conserver la preuve** (demande, décision, date de remise) ; durée proposée : 5 ans [NV].

## 3. Violation de données
1. **Dès la découverte**, noter l'heure (le délai de **72 heures** pour notifier la CNIL, si le risque pour les personnes n'est pas improbable, part de la prise de connaissance).
2. **Contenir** : couper l'accès concerné, changer les mots de passe ou révoquer les sessions (un administrateur peut le faire), conserver les traces.
3. **Prévenir** immédiatement le chef d'établissement et, le cas échéant, le DPO ; ScolIA (sous-traitant) prévient l'établissement **sans retard injustifié** s'il est à l'origine ou témoin de l'incident.
4. **Qualifier** : quelles données, combien de personnes, quels risques (usurpation, atteinte à la vie privée, données de mineurs, santé).
5. **Notifier la CNIL** par le téléservice si le risque n'est pas improbable ; **informer les personnes** si le risque est élevé.
6. **Inscrire** toute violation (même non notifiée) au registre des violations : faits, effets, mesures.

## 4. Conservation et destruction
- Le **catalogue de conservation** (`ent-db/src/conservation.rs`) est une **proposition**. Il n'applique **aucune durée** et ne détruit **rien** tant que la catégorie n'a pas été validée par l'établissement (et son DPO).
- Toute destruction irréversible exigera une **confirmation humaine** et un **journal** (lots G6 à G8, pas avant l'été 2027).
- Le tableau des sanctions à effacer (R511-13 IV) est tenu en attendant l'exécuteur du socle 13.

## 5. Traces d'accès
- Une lecture d'une donnée sensible laisse une trace **avant** d'être servie ; si la trace ne peut pas être écrite, la lecture est refusée (RG-D01).
- Le journal des modifications est en **ajout seul** : ni modification, ni suppression, ni `TRUNCATE`.
