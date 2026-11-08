# Passe « personas » sur le socle 8 — Notifications et diffusion

**Date :** 2026-11-08 — **Statut :** proposition, **non décidée** (à relire par Florian). Aucun code écrit ni modifié ; aucun accès à la base, aux secrets ni aux services de production ; l'application Next.js n'a pas été ouverte.
**Branche :** `ent-personas-socle-8` (base `ent-mc0hoe` `17fedea`, qui porte `08_NOTIFICATIONS.md` relu).
**Éprouve :** `docs/socles/08_NOTIFICATIONS.md` (lu en entier), avec `CLAUDE.md`, `VISION.md`, `DECISIONS.md` (D-014, D-017, D-019 à D-027), `SOCLES.md`, `MATRICE_CHARLEMAGNE.md` (§1, §3.9, lignes VS-07, EDT-12, COM-09 à 11, COM-18, SAN-18, T-29, G25, G26), `docs/recette/RECETTE_PERSONAS_1.md` (lu sur la branche `ent-recette-1-correctifs`, absent de la base : constats RP1-06, RP1-11, RP1-12, RP1-13), `PERSONAS_07.md` (méthode et format repris).
**Ajouts :** faits **directement** dans `08_NOTIFICATIONS.md`, marqués **[personas]** : §0 point 11 ; natures complémentaires (§4.1 bis) ; cas limites `CN-51` à `CN-80` (§4.2) ; règles `R-N50` à `R-N72` (§9.9) ; notes aux §14 et §15 ; actions `NTF-119` à `NTF-164` (§17.14) ; réglages signalés `S-56` à `S-67` (§18, liste à part) ; compléments de modèle (§19) ; droits (§21.3) ; événements (§22.3) ; contrat IA (§23.5) ; écrans (§24) ; répartition dans les lots (§27) ; questions `Q-N32` à `Q-N43` (§28) ; contradictions (§29). Rien n'est supprimé ; quand un ajout contredit le texte d'origine, une note [personas] est posée à côté.
**Numérotation :** les manques continuent `PERSONAS_07.md` (qui s'arrête à M-333) : **M-334 à M-404**. Les réglages signalés continuent aussi : **S-56 à S-67**.

---

## 0. À lire en premier : ce qu'il faut trancher avant le premier lot de code

Aucun lot du socle 8 n'est commencé. La conception est solide sur le cœur (personnes plutôt que comptes, enveloppe neutre, cascade, preuve, diffusion confirmée). En la faisant vivre par celles et ceux qui préviennent, reçoivent et doivent prouver, six points apparaissent qui changent les **premiers** lots (N1, N2, N3, N8, N9).

| # | Point | Pourquoi | Ce que le lot doit faire | Gravité | Lots |
|---|---|---|---|---|---|
| A | **Une fermeture décidée la veille arrive après le car.** `edt.fermeture_exceptionnelle` est U2 ; une U2 attend la fin des heures calmes de nuit, donc 7 h 00 (§9.3) | Vigilance rouge annoncée à 21 h 40, premier ramassage à 6 h 30 : les enfants partent vers un collège fermé, sur des routes dangereuses ; la direction publie sur un réseau social à la place (P03) | **R-N50** : fenêtre « veille jusqu'à 22 h 30, matin dès 6 h 00 » pour les fermetures et transports annulés, qui n'est pas une U3 | **bloquant** | N3, N9 |
| B | **L'aperçu d'une diffusion révèle une mesure de protection.** §15 : « le professeur qui diffuse voit "1 personne exclue (décision)" » | Dans une classe de 25, le professeur sait quelle famille fait l'objet d'une interdiction ou d'un « ne pas solliciter » ; c'est une fuite sur une situation judiciaire (P11) | **R-N51** : exclusions sensibles fondues avec les non-joignables, « moins de 3 » en dessous de trois, détail aux seuls habilités | **bloquant** | N2, N9 |
| C | **Rien n'interdit de prévenir la famille d'une information préoccupante.** Les natures sont déclarées librement par chaque brique ; aucune garde ne refuse qu'un fait du socle 36 alimente une nature adressée aux titulaires | Prévenir un parent maltraitant qu'un signalement est en cours met l'enfant en danger (P12) | **R-N58** : classe d'événements `confidentiel_famille`, refusée par l'assemblage pour toute nature à destinataire famille ou élève, test généré | **bloquant** | N1 |
| D | **« Transmis » est compté comme « prévenu ».** La pastille « famille prévenue » et la liste « à prévenir autrement » ne disent pas qu'un SMS transmis sans accusé n'est pas reçu | La vie scolaire n'appelle pas une famille qui n'a rien reçu (P01) | **R-N52** : pour U2/U3, joint = remis, lu, accusé ou appel « joint » ; colonne « à vérifier » | important | N8 (parcours minimal) |
| E | **Un appel jamais clôturé bloque l'avis d'absence.** Le délai de confirmation part de la clôture de l'appel | Un professeur sur trois oublie de clôturer [H] : l'avis ne part jamais, sans que personne le sache (P01) | **R-N53** : départ à la clôture ou à la fin de la séance + 10 min, au premier des deux | important | N8 |
| F | **L'élève peut détourner les avis d'absence.** Rien ne dit comment un numéro d'envoi change ; un code envoyé au **nouveau** numéro ne prouve rien si l'élève le tient | Absentéisme caché aux parents, pratique connue [H] (P01) | **R-N56**, NTF-123, NTF-128 : changement vérifié par l'ancien canal ou au guichet, avis de sécurité à l'ancien, double canal 30 jours, alerte si le numéro est celui d'un élève | important | N2 |

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_07.md` §1 : 19 personas réalistes et variées, prises dans la consigne et dans le tableau « acteurs » du socle 8 (§3), complétées pour couvrir public et privé, école, collège rural à ramassage, lycée sur deux sites, internat ; bureau, guichet, direction, terrain, sortie scolaire ; familles séparées, non francophones, sans internet, protégées ; élève mineur et majeur ; personnes extérieures (grand-parent, inspecteur). Pour chacune : son année (rentrée, absences du matin, réunions, relances, intempéries, exercices, voyages, fin d'année), puis ses **incidents** (SMS non remis, adresse invalide, envoi groupé erroné, panne du fournisseur, notification la nuit, coût des SMS qui explose). Chaque fois qu'elle voulait faire quelque chose que la conception ne prévoit pas, ou que c'était trop long, ambigu, risqué ou frustrant, le manque est noté.

**Grille transversale** posée à chaque persona : qui reçoit et pourquoi ? ; qui ne reçoit pas et le sait-on ? ; « prévenu » est-il vrai ? ; heure d'arrivée ; sur quel canal, avec quoi d'écrit ; coût ; langue et capacité de lecture ; parents séparés et personnes protégées ; élève mineur, majeur ; erreur et annulation ; panne ; preuve un an après ; « le ferait-elle encore avec son téléphone personnel ? ».

**Couverture Charlemagne** (D-023, D-024) : les lignes de la matrice qui touchent le socle 8 (VS-07, EDT-12, COM-09, 10, 11, 18, SAN-18, T-29, G25, G26) et les fonctions de communication qu'un utilisateur de Charlemagne et d'École Directe attend [H : la documentation de l'éditeur n'a pas pu être lue, même limite que la matrice §0] ont été relues ; celles qui manquaient sont au §4.

**Gravité** (identique aux passes précédentes).
- **bloquant** : sans lui, l'établissement contourne de façon dangereuse (sécurité d'un enfant, fuite d'une donnée, pratique probablement illégale) ou ne peut pas traiter un cas réel ; ou la conception se contredit au point d'empêcher de coder.
- **important** : cas fréquent sans solution propre, perte de temps forte ou risque d'erreur ; ou fonction de Charlemagne absente.
- **confort** : améliore nettement le quotidien ; un contournement existe.
- **différenciant** : peu de logiciels du marché le font [H, comparaison de mémoire] ; argument de vente.

**Conventions.** [H] hypothèse ; **[NV, à vérifier]** règle réglementaire citée de mémoire, à vérifier sur le texte officiel avant de coder ; [R] recommandation. Numéros de socles : `SOCLES.md`.

**Limites.** Les personas sont construites, pas interviewées. Les règles citées (information des familles un jour de grève, usage du téléphone au collège, signalement d'absentéisme, information d'une personne dont on donne le numéro, actes usuels, conservation) le sont **de mémoire** : marquées [NV]. Aucune page officielle n'a été consultée ; **aucun sous-agent n'a été utilisé** : la passe ne demandait pas de recherche au-delà des documents lus.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Nadia**, CPE | Collège public en éducation prioritaire, 650 élèves | Prévenir les familles des absences du matin ; savoir qui est vraiment prévenu |
| P02 | **Fatou**, agent d'accueil | Lycée public | Téléphone, enfant qui attend, parent qui récupère, appelant inconnu |
| P03 | **M. Delmas**, principal | Collège rural, 70 % d'élèves transportés | Fermeture pour intempéries décidée la veille au soir ; car annulé |
| P04 | **Mme Haddad**, proviseure | Lycée sur deux sites, internat | Confinement, points de situation, panne pendant l'alerte, exercice |
| P05 | **Mme Lemoine**, professeure principale de 3ᵉ | Collège privé sous contrat | Réunion de rentrée, coupon-réponse, document joint, téléphone |
| P06 | **M. Rigaud**, comptable de l'organisme de gestion | Ensemble privé (école, collège, lycée) | Relances d'impayés, payeurs, coût des SMS |
| P07 | **Marc et Céline**, parents séparés de Lou (5ᵉ) | Collège de P05 | Chacun son foyer ; garde alternée ; nouveau conjoint |
| P08 | **M. Bernard**, père qui ne lit pas ses e-mails | Collège de P01 | Canal qui ne marche pas sans que personne le sache |
| P09 | **Mme Kovalenko** (ukrainienne) ; **M. Diallo** (ne lit pas) | Collège de P01, avec UPE2A | Langue, coût des SMS non latins, appel |
| P10 | **Mme Rossi**, grand-mère sans compte | École et collège de P05 | Vient chercher l'enfant ; personne ne la prévient |
| P11 | **M. Leroy**, père « ne pas solliciter » ; **Mme Lefort**, mère protégée | Lycée de P04 | Ce que les aperçus et les exclusions révèlent |
| P12 | **Sonia**, assistante sociale | Collège de P01 | Information préoccupante qui ne doit jamais arriver à la famille |
| P13 | **Hugo**, 18 ans, terminale ; **Léo**, 13 ans, 4ᵉ | Lycée de P04 ; collège de P01 | Majorité ; téléphone en cours et la nuit |
| P14 | **M. Petit**, professeur de six classes ; Nadia (P01) côté réception | Collège de P01 | Trop de notifications ; suivre un élève fragile |
| P15 | **M. Fabre**, inspecteur ; référent absentéisme de la direction académique | — | « Prouvez que la famille a été prévenue » |
| P16 | **Yann**, référent numérique | Ensemble de P06 | Panne silencieuse, envoi nocturne, fuite à rappeler, numéros faux |
| P17 | **Sylvie**, secrétaire de scolarité sous Charlemagne depuis douze ans | Lycée privé sous contrat | Envois groupés « comme avant » : nouveaux inscrits, papier, validation |
| P18 | **Mme Faure**, professeure accompagnatrice | Collège de P05, voyage en Espagne | Donner des nouvelles aux familles sans son téléphone personnel |
| P19 | **Mme Perret**, directrice d'école | École publique de cinq classes | Jour de grève, accueil des élèves |

---

## 3. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait (identifiant dans `08_NOTIFICATIONS.md`) ou le socle ultérieur où il est renvoyé.

### P01 — Nadia, CPE (collège en éducation prioritaire)

**Année.** *Chaque matin* : 25 à 40 absences à 8 h 20 ; elle veut, à 9 h, savoir qui est vraiment prévenu et appeler les autres. *Octobre* : trois élèves présents à 8 h, absents à 10 h ; l'un est à l'infirmerie, l'autre en permanence, le troisième est parti. *Novembre* : une mère arrive furieuse : « je n'ai jamais rien reçu », alors que la fiche indique « prévenue ✓ ». *Janvier* : un élève de 4ᵉ cumule les absences ; la mère ne réagit jamais ; Nadia découvre que le « portable de la mère » est celui de l'élève. *Toute l'année* : les mêmes messages tapés vingt fois (« merci de passer à la vie scolaire », « justificatif attendu »).
**Incidents.** Un professeur fait l'appel à 8 h mais ne le clôture pas : aucune famille de sa classe n'est prévenue. Douze SMS « transmis » à 10 h n'ont pas d'accusé. Une mère répond à l'avis : « Il est parti à 7 h 30 ! ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-334 | **Délai de confirmation borné** même si l'appel n'est jamais clôturé | §0 point E : sans lui, une classe entière n'est pas prévenue, en silence | important | 8, 12 | R-N53, CN-57 |
| M-335 | **« Transmis » n'est pas « joint »** : colonne « à vérifier » | §0 point D : la pastille ment, la famille n'est pas appelée | important | 8 | R-N52, CN-56 |
| M-336 | **Absence après présence** : avis interne immédiat, famille 15 min après sauf « localisé » | Un élève qui disparaît en cours de journée est un risque réel ; le délai et l'envoi direct à la famille ne conviennent pas | important | 12, 8 | R-N67, CN-58, §4.1 bis |
| M-337 | **« Il devrait être présent »** : la famille signale depuis l'avis que l'enfant est parti de chez lui | L'information la plus urgente de la journée arrive par la famille ; aujourd'hui elle téléphone, souvent au standard saturé [H] | important | 8, 12 | NTF-120, R-N68, CN-59 |
| M-338 | **Changement de numéro vérifié**, alerte si c'est celui d'un élève | §0 point F | important | 8, 4 | R-N56, NTF-128, CN-52 |
| M-339 | **Avis de sécurité** quand un canal ou une catégorie à plancher change, sur l'ancien canal | Même détournement par le compte du parent ; la personne doit pouvoir annuler | important | 8 | NTF-123, §4.1 bis |
| M-340 | **Bilan du matin** (prévenues, lues, à vérifier, non jointes) | La liste existe (NTF-63) ; la vue d'ensemble à 10 h manque | confort | 8 | NTF-146 |
| M-341 | **Modèles de messages** de l'établissement (texte seul) | Charlemagne a des modèles de SMS [H] ; sans eux, la CPE retape ou copie depuis un fichier | important | 8 | NTF-131, S-59 |

### P02 — Fatou, agent d'accueil (lycée public)

**Année.** *Chaque jour* : vingt appels de parents (« vous avez reçu mon message ? », « mon fils est-il absent ? ») ; un enfant malade attend ; un grand-père vient chercher sa petite-fille. *Mars* : un homme appelle, se dit le père de Léa, demande si elle est là aujourd'hui.
**Incidents.** Fatou ne sait pas répondre « oui, la famille a été prévenue » sans ouvrir un journal qu'elle n'a pas le droit de lire (RP1-06). Elle n'a aucun moyen de vérifier l'identité de l'appelant.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-342 | **Statut « famille prévenue »** pour l'accueil, sans contenu ni destination | Répondre au téléphone sans ouvrir le journal ; droit minimal (RP1-06) | important | 8 | NTF-147, §21.3 |
| M-343 | **« Rappeler au numéro connu »** quand un appelant non vérifié demande une information | Protège l'enfant d'un parent sous interdiction qui se fait passer pour autre chose ; aucun logiciel ne le propose [H] | différenciant | 8, 4 | NTF-147 |
| M-344 | **Avis de sortie effectuée** (« Léa a quitté l'établissement à 14 h 10, prise en charge par une personne autorisée ») | Les familles veulent savoir que la remise a eu lieu ; absent du catalogue §4.1 | important | 14, 8 | §4.1 bis |
| M-345 | **Demande de modification de coordonnées** reçue au téléphone, transmise au secrétariat | L'accueil n'a pas à modifier ; aujourd'hui un post-it | confort | 4 | renvoyé (§5) |

### P03 — M. Delmas, principal (collège rural à ramassage)

**Année.** *Décembre* : à 21 h 40, la préfecture annonce une vigilance rouge ; il décide de fermer ; le premier car passe à 6 h 30. *Février* : le transporteur annule la ligne 12 à 6 h 15. *Mars* : exercice d'évacuation ; des parents paniquent en voyant passer l'information sur un réseau social.
**Incidents.** L'avis de fermeture, U2, partirait à 7 h 00 (§9.3). Les grands-parents qui récupèrent les enfants à 16 h 30 ne sont prévenus de rien.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-346 | **Fenêtre veille / matin** pour fermetures et transports | §0 point A | **bloquant** | 8 | R-N50, NTF-144, CN-51, Q-N32 |
| M-347 | **Audience « usagers d'une ligne de transport »** | Ne déranger que les familles concernées par le car annulé | important | 8, 43 | NTF-132, CN-67 |
| M-348 | **Personnes qui viennent chercher** dans l'audience des fermetures et alertes non médicales | La grand-mère arrive devant un collège fermé ou confiné | important | 8, 4, 14 | R-N60, NTF-132 |
| M-349 | **Information des familles** avant ou après un exercice | Évite l'inquiétude et les appels ; pratique courante [H] | confort | 8, 14 | NTF-143, §4.1 bis |

### P04 — Mme Haddad, proviseure (lycée sur deux sites, internat)

**Année.** *Octobre* : fuite de gaz sur le site B : il faut alerter le site B seulement. *Mars* : confinement réel à 10 h 12 ; à 10 h 40, elle veut dire « situation sous contrôle, restez à l'écoute » ; à 11 h 30, « levée ». Le réseau du lycée tombe pendant l'alerte. *Toute l'année* : prévenir la mairie, le transporteur, la cellule académique.
**Incidents.** Les familles appellent toutes en même temps et saturent le standard.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-350 | **Points de situation** pendant une alerte, même audience, deux gestes | Une crise dure ; entre l'alerte et la levée, le silence nourrit les rumeurs | important | 8, 14 | NTF-139, §4.1 bis |
| M-351 | **Alerte par site, bâtiment ou ligne** | Lycées multi-sites, cités scolaires [H] ; alerter tout le monde inquiète inutilement | important | 8, 5 | NTF-140, CN-66 |
| M-352 | **Plan B hors ligne** si ScolIA est injoignable | Une alerte qui dépend d'un seul serveur ne part pas quand le réseau tombe | important | 8, 11 | NTF-142, CN-65, Q-N37 |
| M-353 | **Contacts externes de crise** dans les modèles | Mairie, transporteur, cellule académique : aujourd'hui un annuaire papier [H] | confort | 8, 14 | NTF-141 |
| M-354 | Phrase « n'appelez pas l'établissement » dans les modèles d'urgence | Lignes libres pour les secours | confort | 8 | §14 point 8 |

### P05 — Mme Lemoine, professeure principale de 3ᵉ (collège privé)

**Année.** *Septembre* : réunion de rentrée ; elle veut l'ordre du jour en PDF et savoir combien de parents viendront. *Novembre* : rencontre parents-professeurs (socle 19). *Mars* : la sortie au théâtre est déplacée ; elle est dans le car, sans ordinateur. *Toute l'année* : la direction exige de relire tout message aux familles.
**Incidents.** Sans réponse dans l'ENT, elle crée un groupe de messagerie personnel avec les parents [H, pratique répandue].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-355 | **Coupon-réponse** : accusé demandé, une question fermée, résultats, relance des non-répondants | Le coupon détachable est encore la norme [H] ; un formulaire complet (socle 18) est trop lourd pour « viendrez-vous ? » | important | 8, 18, 19 | NTF-134 |
| M-356 | **Document joint** à une diffusion (lien, ou pièce jointe C0) | Circulaires, ordres du jour, listes de fournitures : Charlemagne envoie des pièces jointes [H] | important | 8, 7 | NTF-133, R-N65, Q-N35 |
| M-357 | **Diffusion légère depuis le mobile**, bornée à une classe ou aux participants | Remplace les groupes de messagerie personnels des enseignants, hors de tout contrôle | différenciant | 8, 11 | NTF-136, R-N64 |
| M-358 | « Les deux responsables légaux ont été informés » sur les convocations | Coupe court au « vous favorisez l'autre parent » | confort | 8 | NTF-160, Q-N41 |

### P06 — M. Rigaud, comptable de l'organisme de gestion

**Année.** *Octobre* : relance n° 1 à 140 familles ; trois paient par un tiers ; deux couples séparés paient chacun la moitié. *Décembre* : une relance part le 24 au soir et provoque des appels le 26, quand personne ne répond. *Mars* : 85 % du budget SMS est consommé ; il veut savoir qui consomme et rapprocher la facture d'OVH. *Toute l'année* : les familles demandent « avez-vous reçu mon paiement ? ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-360 | **Audience « payeurs »** dans les diffusions | Charlemagne cible le « responsable financier » [H] ; NTF-46 ne connaît que responsables et élèves | important | 8, 24 | NTF-132 |
| M-361 | **Avis « paiement enregistré »** au seul payeur | Évite les appels « avez-vous reçu ? » | confort | 25, 8 | §4.1 bis |
| M-362 | **Plage d'envoi métier** (jours ouvrés 9 h – 17 h, jamais le 24 ni le 31 décembre) | La famille doit pouvoir rappeler la comptabilité | confort | 25, 8 | R-N70, NTF-157 |
| M-363 | **Consommation par auteur et service, projection, rapprochement** avec la facture | Comprendre pourquoi le budget explose avant qu'il soit épuisé | confort | 8 | NTF-156, CN-80 |

### P07 — Marc et Céline, parents séparés (garde alternée, autorité conjointe)

**Année.** *Septembre* : Marc veut tout recevoir, même ses semaines « sans » ; Céline ne veut rien recevoir la semaine de Marc. *Novembre* : le calendrier de garde n'a pas été saisi ; une absence tombe un lundi de changement. *Février* : le nouveau compagnon de Céline récupère Lou à l'accueil ; Marc reçoit « Lou a été prise en charge par M. Durand ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-364 | **Choix du parent en garde alternée** (« aussi les semaines de l'autre » / « seulement les miennes ») | La règle d'établissement (CN-05) ne connaît pas la volonté de chaque parent | important | 8, 4 | NTF-122, CN-68, S-58 |
| M-365 | **Calendrier manquant ou ambigu → les deux** | Sans règle de repli, la règle « foyer du jour » peut ne prévenir personne | important | 4, 8 | R-N61, CN-69 |
| M-366 | **Nom d'un adulte tiers** dans un avis de sortie seulement s'il appartient au foyer du destinataire | La vie privée de chaque foyer (RP1-11) vaut aussi pour les avis | important | 14, 8 | R-N72 |

### P08 — M. Bernard, père qui ne lit pas ses e-mails

**Année.** Il a donné une adresse e-mail en 2023, n'y va jamais ; il a l'appli mais a coupé ses notifications dans les réglages du téléphone. *Octobre* : il lit une absence dans l'aperçu d'un e-mail, sans ouvrir l'ENT ; 30 min après, il reçoit le SMS d'escalade et appelle, agacé.
**Incidents.** Pour le moteur, l'e-mail est « transmis » et la notification « transmise » : tout semble en ordre.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-367 | **« J'ai pris connaissance »** dans l'e-mail et le SMS, sans connexion | Arrête l'escalade inutile ; donne une preuve sans pixel espion | important | 8 | NTF-119, R-N71, CN-55 |
| M-368 | **Joignabilité effective** (taux d'ouverture, dernier canal efficace) et proposition de changer de canal | Aucun logiciel ne dit « cette famille ne lit pas nos e-mails » [H] | différenciant | 8 | NTF-126, §4.1 bis |
| M-369 | **Vérification de joignabilité à la rentrée** | Nettoyer les coordonnées avant la première absence, pas après | différenciant | 8, 4 | NTF-125 |
| M-370 | **Autorisation push rapportée par l'appli** | Un push bloqué compte comme joignable : le plancher est faux | important | 8, 11 | R-N57, NTF-124, CN-54 |

### P09 — Mme Kovalenko (ukrainienne) et M. Diallo (ne lit pas)

**Année.** *Septembre* : Mme Kovalenko reçoit tout en français. *Octobre* : la CPE lui écrit un message individuel, en français. *Novembre* : son SMS d'absence en ukrainien coûte 3 segments. *Toute l'année* : M. Diallo ne lit ni le français ni le peul écrit ; seul un appel fonctionne ; il répond par SMS vocal ou en peul [H].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-371 | **Traduction proposée d'un message individuel** (NTF-39, 41) | §9.7 ne la prévoit que pour les diffusions ; le message individuel est le plus important | important | 8, 30 | NTF-130 |
| M-372 | **« Préfère un appel » / « sans accès internet »** | Pour un non-lecteur, tout texte est inutile ; l'appel doit être prévu, pas improvisé | important | 4, 8 | NTF-127, CN-73 |
| M-373 | **SMS en une seule langue** | Le français joint double le coût et la longueur | confort | 8 | R-N62, CN-72 |
| M-374 | **Traduction des réponses entrantes** pour le personnel | La réponse en ukrainien reste sans suite | confort | 8, 16, 30 | NTF-163 |
| M-375 | **Tuile « langue non renseignée »** à la rentrée | Sans langue, tout part en français | confort | 8, 4 | NTF-164 |
| M-376 | **Message vocal pré-enregistré** dans la langue pour les gabarits critiques | Le seul canal qui atteint un non-lecteur sans mobiliser un personnel [H] | différenciant | 8 | renvoyé (Q-N7, appel vocal hors V1) |

### P10 — Mme Rossi, grand-mère sans compte

**Année.** Elle vient chercher Léo tous les soirs ; elle n'est ni titulaire ni payeuse ; sa fille a donné son numéro comme « personne autorisée ». *Novembre* : le cours de 16 h est annulé ; Léo sort à 15 h ; elle arrive à 16 h 30. *Décembre* : fermeture pour neige : elle se déplace pour rien.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-377 | **Personne de confiance** désignée par un titulaire, catégories fermées | La personne qui vient chercher l'enfant est celle qui doit savoir l'heure de sortie | important | 8, 4 | NTF-121, R-N60, CN-60, Q-N33 |
| M-378 | **Information de la personne désignée** à son premier message (qui l'a désignée, comment arrêter) | On écrit à quelqu'un qui n'a rien demandé [NV] | confort | 8 | R-N60 |
| M-379 | **Avis « sortie anticipée »** (heure de sortie modifiée selon le régime de sortie) | `edt.cours_annule` vise l'élève ; la famille et la personne de confiance ne sont pas prévenues de l'heure de sortie | important | 10, 14, 8 | §4.1 bis |

### P11 — M. Leroy (« ne pas solliciter ») et Mme Lefort (mère protégée)

**Année.** La direction a décidé « ne pas solliciter » pour M. Leroy en attendant une décision du juge. Le professeur de théâtre prépare une diffusion à son atelier de 12 élèves ; l'aperçu affiche « 1 personne exclue (décision) ». En juin, la décision du juge tombe ; personne ne revoit le statut.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-380 | **Exclusions sensibles agrégées** dans les aperçus | §0 point B | **bloquant** | 8 | R-N51, CN-61, §15 note |
| M-381 | **Révision datée** d'une décision « ne pas solliciter » | Une mesure provisoire devient définitive par oubli | confort | 8, 4 | NTF-161 |

### P12 — Sonia, assistante sociale

**Situation.** Elle enregistre une information préoccupante au socle 36. Une brique déclarera un jour une nature « un document vous attend » ou « rendez-vous avec l'assistante sociale » sur un événement voisin.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-382 | **Classe `confidentiel_famille`** refusée par l'assemblage | §0 point C | **bloquant** | 8, 13, 34, 35, 36 | R-N58, CN-70, §4.1 bis |

### P13 — Hugo (18 ans, terminale) et Léo (13 ans, 4ᵉ)

**Année.** *Hugo* : le jour de ses 18 ans, il demande que ses parents ne reçoivent plus ses absences ; ses parents l'apprennent par l'absence de messages. *Léo* : son téléphone vibre en cours pour une note publiée ; le soir, à 23 h, pour un devoir.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-383 | **Information unique des parents** quand l'élève majeur limite leur accès | Le silence soudain inquiète ou fait croire à une panne (RP1-12) | confort | 2, 8 | NTF-159, CN-74 |
| M-384 | **Élèves mineurs** : jamais la nuit, différé pendant les cours au collège | La règle de l'établissement sur le téléphone [NV] ne doit pas être contredite par l'ENT | différenciant | 8 | R-N66, NTF-158, CN-75, Q-N38 |

### P14 — M. Petit (six classes) et Nadia côté réception

**Année.** M. Petit reçoit chaque message, chaque appel à faire, chaque remplacement ; il voudrait suivre de près deux élèves fragiles de sa classe de 4ᵉ. Nadia reçoit les tâches, le parapheur, les messages, les alertes : tout dans la même catégorie « Messagerie » faute de catégories pour les personnels.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-385 | **Catégories des personnels** | Les dix catégories du §9.2 sont celles des familles | important | 8, 6 | NTF-152, S-65 |
| M-386 | **Suivre un élève** (absences, retards, punitions), durée limitée | Le PP et le CPE le font aujourd'hui de mémoire [H] | différenciant | 8, 12, 13 | NTF-153, Q-N43 |
| M-387 | **Rendre muet un fil ou une nature** sans plancher | Un fil bavard ne doit pas couvrir le reste | confort | 8, 16 | NTF-154 |
| M-388 | **Indicateurs de charge** des personnels (T-29) | Droit à la déconnexion mesuré, pas seulement promis | confort | 8, 47 | NTF-155 |

### P15 — M. Fabre (inspecteur) et le référent absentéisme

**Situation.** En juin, pour un élève signalé pour absentéisme [NV : procédure et pièces attendues], on demande la preuve que la famille a été informée de chaque absence depuis octobre, et des convocations.
**Incident.** Le texte rendu des avis courants n'est gardé que 90 jours (§19) : on sait qu'un avis est parti, pas ce qu'il disait.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-389 | **Dossier de preuve d'information** par élève et période, scellé | NTF-66 ne prouve qu'un avis à la fois | important | 8, 7, 12 | NTF-145, §21.3 |
| M-390 | **Conservation du texte** des avis d'assiduité, convocations et procédures l'année + 1 an | Sans le texte, la preuve est incomplète | important | 8, 27 | R-N59, CN-79, Q-N39 |
| M-391 | **Légende des états** dans la preuve (« transmis » ≠ « remis » ≠ « lu ») | Un tiers lit « transmis » comme « reçu » | confort | 8 | NTF-145 |

### P16 — Yann, référent numérique (incidents)

**Incidents.** *Septembre* : OVH accepte les SMS mais aucun accusé ne revient pendant 40 minutes. *Octobre* : une erreur de fuseau planifie des résumés à 2 h du matin. *Novembre* : une diffusion part avec un lien vers la liste nominative des boursiers. *Toute l'année* : numéros de fixe dans le champ portable, numéros à l'étranger, numéros réattribués. *Janvier* : une AED partie en décembre avait programmé une diffusion pour le 2 janvier.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-392 | **Détection de panne silencieuse** (taux d'accusés) | Le disjoncteur actuel ne voit que les erreurs | important | 8 | R-N69, NTF-149, CN-63 |
| M-393 | **Garde-fou nocturne** dans l'expéditeur | Une seule erreur de planification réveille 800 familles | important | 8 | R-N54, NTF-150, CN-64 |
| M-394 | **Retirer un contenu** déjà remis (fuite) | Le correctif n'efface pas l'élément ni le lien | important | 8, 7, 27 | NTF-148, R-N63, CN-62 |
| M-395 | **Type de numéro** contrôlé (fixe, étranger) | SMS perdus vers des fixes ; coût des SMS à l'étranger | important | 8, 4 | R-N55, NTF-151, CN-53, CN-76, Q-N42 |
| M-396 | **Revérification annuelle** des numéros | Un numéro réattribué fait écrire à un inconnu | confort | 8, 4 | NTF-129, CN-71 |
| M-397 | **Diffusion programmée orpheline** (auteur parti, vacances) suspendue | Personne ne la surveille | confort | 8 | NTF-138, CN-77 |
| M-398 | **Second fournisseur SMS** en secours | D-004 fixe OVH ; une panne longue n'a que le repli push / e-mail | confort | 8 | Q-N40 (question seulement) |

### P17 — Sylvie, secrétaire de scolarité (sous Charlemagne depuis douze ans)

**Année.** *Juin* : elle écrit aux familles des **nouveaux inscrits** de la rentrée (journée d'accueil, fournitures) : ils n'ont pas encore de scolarité. *Septembre* : quinze familles n'ont ni e-mail ni portable : elle imprime la circulaire et la donne aux élèves. *Toute l'année* : la direction relit tous les envois des professeurs. Le coût des SMS augmente parce que des familles ont refusé l'appli.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-399 | **Audience « inscrits de la rentrée » et admis** | Fonction courante de Charlemagne [H] ; ces familles n'ont pas encore de classe | important | 8, 3 | NTF-132 |
| M-400 | **Version papier** par classe avec liste de remise | Le « mot dans le carnet » reste la voie pour les familles sans numérique | important | 8, 7 | NTF-135, Q-N36 |
| M-401 | **Validation préalable** des diffusions des professeurs (option) | Attente fréquente des directions du privé [H] | important | 8 | NTF-137, CN-78, S-56, Q-N34 |
| M-402 | **Proposer l'appli** aux familles servies par SMS | Le push coûte zéro ; personne ne le propose | confort | 8, 11 | NTF-162, §4.1 bis |

### P18 — Mme Faure, professeure accompagnatrice (voyage en Espagne)

**Situation.** Les familles veulent savoir que le car est arrivé, que le programme change ; aujourd'hui, un groupe de messagerie sur son téléphone personnel [H]. NTF-94 ne prévoit que le **retard** de retour.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-403 | **Point de situation d'un voyage** depuis le mobile, aux familles des participants | Le numéro personnel de l'enseignant circule, rien n'est tracé | important | 42, 8 | §4.1 bis, NTF-136 |

### P19 — Mme Perret, directrice d'école (premier degré public)

**Situation.** Jour de grève : elle doit informer les familles des modalités d'accueil [NV : délai et forme à vérifier, service d'accueil de la commune].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-404 | **Avis « modalités d'accueil un jour de grève »** | Obligation probable [NV] ; absent du catalogue | important | 50, 15, 8 | §4.1 bis |

### Manque transverse

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-359 | **Bilan hebdomadaire des internes** aux familles (SAN-18 de la matrice) | Ligne de la matrice sans nature déclarée | confort | 41, 8 | §4.1 bis |

---

## 4. Couverture des fonctions de communication de Charlemagne (D-023, D-024)

Le contenu exact des fonctions de communication de Charlemagne et d'École Directe n'a pas pu être lu [H, même limite que la matrice §0]. La liste est celle qu'une secrétaire, une CPE et une comptable habituées à ces logiciels attendent.

| Fonction attendue [H] | Avant la passe | Après la passe | Où |
|---|---|---|---|
| SMS groupés, crédit, historique des envois | conçu | inchangé ; projection et consommation par auteur | NTF-45–58, NTF-156 |
| **Modèles de SMS / de messages** | absent (NTF-57 duplique une diffusion passée) | conçu, texte seul | NTF-131 |
| Destinataires « responsable 1 / responsable 2 / les deux » | conçu (chaque titulaire en son nom) | inchangé | NTF-46 |
| Destinataire **« responsable financier »** | absent | conçu | NTF-132 |
| E-mails groupés **avec pièce jointe** | refusé (aucune pièce jointe C2+) | lien ou pièce jointe C0 | NTF-133, R-N65 |
| Publipostage papier, étiquettes | courrier individuel (socle 7) | + version papier par classe | NTF-135 |
| Envois aux **nouveaux inscrits** | absent | conçu | NTF-132 |
| Accusé de lecture / coupon-réponse des circulaires | suivi de lecture seulement | accusé demandé, question fermée | NTF-134 |
| Notifications de l'appli (notes, absences, messages, documents, devoirs) | conçu | inchangé | §4.1 |
| SMS d'absence aux seules familles concernées (G25, COM-09) | conçu | précisé : « joint » vrai (R-N52) | NTF-58 |
| Relance automatique des absences non justifiées (VS-07) | conçu (socle 12) | inchangé | §4.1 |
| Relances de facturation par paliers | conçu (socle 25) | + plage d'envoi, audience payeurs | R-N70, NTF-132 |
| Information des familles d'une absence d'enseignant (EDT-12) | conçu | + sortie anticipée, personne de confiance | §4.1 bis |
| « Pourquoi ai-je reçu ce message ? » (COM-11) | conçu | inchangé | NTF-11 |
| Droit à la déconnexion (COM-18), charge (T-29) | conçu | + catégories des personnels, indicateurs | NTF-152, 155 |
| Résumé hebdomadaire des internes (SAN-18) | absent | nature déclarée | §4.1 bis |
| Validation des envois par la direction | quatre yeux au-delà d'un seuil | + option de validation préalable | NTF-137 |
| Carnet de correspondance | socle 16 | renvoyé | §5 |
| Annonces, actualités | socle 17 | renvoyé | §5 |

---

## 5. Manques qui relèvent d'autres socles

| Socle | Manques | Ce qu'il faudra prévoir |
|---|---|---|
| 2 Élève | M-383 | Événement `eleve.acces_parents_modifie` |
| 3 Inscriptions | M-399 | Audience des admis et inscrits de la rentrée (lecture publiée) |
| 4 Dossier administratif | M-338, M-345, M-348, M-364, M-365, M-372, M-377, M-395, M-396 | Lien « personne de confiance » ; préférences `sans_internet`, `prefere_appel` ; choix de garde alternée ; type de numéro et date de vérification ; changement de canal vérifié ; demande de modification transmise par l'accueil |
| 5 Structure | M-351 | Sites et bâtiments comme audiences |
| 6 Personnels | M-385 | Fonctions pour les catégories des personnels |
| 7 Documents | M-356, M-394, M-400 | Document C0 joignable ; révocation des liens au retrait ; lot d'impression par classe |
| 11 Mobile | M-352, M-357, M-370 | Plan B hors ligne ; diffusion légère ; autorisation des notifications rapportée |
| 12 Appel | M-334, M-336, M-337 | Fin de séance comme borne ; « localisé » ; alerte famille sur la fiche d'absence |
| 13, 34, 35, 36 | M-382 | Marquer `confidentiel_famille` les événements concernés |
| 14 Accueil, sûreté | M-344, M-366, M-350 | Nature `accueil.sortie_effectuee` ; personnes autorisées par foyer ; points de situation au registre |
| 16 Messagerie | M-374, M-387 | Traduction des réponses ; muet d'un fil |
| 19 Réunions | M-355 | Lien entre coupon-réponse et réunion |
| 25 Paiements | M-361, M-362 | `finances.paiement_enregistre` ; plage d'envoi des relances |
| 30 IA | M-371, M-374 | Traduction souveraine, relue |
| 41 Internat | M-359 | Bilan hebdomadaire |
| 42 Voyages | M-403 | Point de situation |
| 43 Transports | M-347 | Usagers par ligne, événement d'inscription |
| 50 Premier degré | M-404 | Modalités d'accueil un jour de grève |

---

## 6. Synthèse

### 6.1 Manques par gravité

| Gravité | Nombre |
|---|---|
| bloquant | 3 |
| important | 38 |
| confort | 23 |
| différenciant | 7 |
| **Total** | **71** (M-334 à M-404) |

Comptage fait par script sur les tableaux du §3. Les trois bloquants sont les points A à C du §0 : **M-346** (fermeture décidée la veille qui arrive après le car), **M-380** (aperçu qui désigne la famille sous mesure de protection), **M-382** (rien n'interdit de prévenir une famille d'une information préoccupante). Les points D à F du §0 sont **importants** (M-335, M-334, M-338).

Les sept différenciants : M-343 (rappel au numéro connu), M-357 (diffusion légère depuis le mobile), M-368 (joignabilité effective), M-369 (vérification de rentrée), M-376 (message vocal dans la langue, renvoyé), M-384 (élèves mineurs : nuit et cours), M-386 (suivre un élève).

**69** manques sont traités par un ajout dans `08_NOTIFICATIONS.md` ; **2** sont renvoyés (M-345 au socle 4, M-376 à la question Q-N7) ; M-398 n'est qu'une question (Q-N40), sans alternative proposée (D-004).

### 6.2 Ajouts dans `08_NOTIFICATIONS.md`

| Partie | Ajouts |
|---|---|
| Actions métier | **46** (NTF-119 à NTF-164) |
| Règles | **23** (R-N50 à R-N72) |
| Cas limites | **30** (CN-51 à CN-80) |
| Natures complémentaires | §4.1 bis (17 natures, déclarées par leur socle) + classe `confidentiel_famille` |
| Modèle | §19 (appareils, destinataires, avis, diffusions, réponses, modèles de messages, urgences, suivi d'élève, alertes famille, joignabilité ; compléments au socle 4) |
| Droits | §21.3 (6 actions ; 22 au total) |
| Événements | §22.3 (4 émis, soit 20 ; 9 consommés) |
| Contrat IA | §23.5 |
| Écrans | §24 (note) |
| Lots | §27 (répartition, aucun lot nouveau) |
| Questions | 12 (Q-N32 à Q-N43) |
| Réglages signalés | 12 (S-56 à S-67), §18, dont 2 sur l'existant |

### 6.3 Les dix ajouts les plus importants

1. **Fenêtre veille / matin des fermetures** (R-N50, NTF-144 ; M-346). La fermeture décidée à 21 h 40 arrive avant le car de 6 h 30, sans réveiller tout le monde à minuit.
2. **Classe `confidentiel_famille`** (R-N58 ; M-382). Une information préoccupante ne peut, par construction, produire aucun avis à la famille.
3. **Exclusions sensibles agrégées** (R-N51 ; M-380). L'aperçu d'un professeur ne désigne plus la famille sous interdiction.
4. **« Transmis » n'est pas « joint », et l'appel non clôturé ne bloque rien** (R-N52, R-N53 ; M-334, M-335). La pastille « famille prévenue » dit vrai ; entre dans le parcours minimal vendable.
5. **« Il devrait être présent » et absence après présence** (NTF-120, R-N67, R-N68 ; M-336, M-337). L'enfant qui n'arrive pas ou disparaît en cours de journée est cherché tout de suite.
6. **Canal d'envoi changé de façon vérifiée, avis de sécurité, push bloqué détecté** (R-N56, R-N57, NTF-123, 124, 128 ; M-338, M-339, M-370). L'élève ne détourne plus ses avis ; un canal mort n'est plus compté vivant.
7. **Personne de confiance et sortie anticipée** (NTF-121, R-N60, §4.1 bis ; M-377, M-379, M-348). La grand-mère qui vient chercher l'enfant connaît l'heure de sortie et les fermetures, rien d'autre.
8. **Diffusions « comme sous Charlemagne », en mieux** (NTF-131 à 135, 137 ; M-341, M-355, M-356, M-360, M-399, M-400, M-401). Modèles de messages, payeurs, inscrits de la rentrée, document joint, coupon-réponse, version papier, validation préalable.
9. **Urgence qui dure et qui résiste** (NTF-139 à 142 ; M-350, M-351, M-352). Points de situation, alerte par site, plan B quand le serveur tombe.
10. **Preuve d'information par élève et période, texte conservé** (NTF-145, R-N59 ; M-389, M-390). L'inspection ou le signalement d'absentéisme reçoivent un dossier complet, lisible et scellé.

Suivent de près : la diffusion légère depuis le mobile (NTF-136), « J'ai pris connaissance » sans connexion (NTF-119), la joignabilité effective et la vérification de rentrée (NTF-125, 126), le garde-fou nocturne et la panne silencieuse (R-N54, R-N69), le retrait d'un contenu envoyé à tort (NTF-148), la traduction des messages individuels (NTF-130), les catégories et le suivi d'élève pour les personnels (NTF-152, 153).

### 6.4 Ce que la passe a montré sur la conception

- **Le moteur est juste ; ce sont ses états qui sont trop optimistes.** « Transmis », « appel clôturé », « push actif », « coordonnée vérifiée » sont pris pour des vérités ; sur le terrain, chacun peut être faux sans bruit. La moitié des ajouts importants rendent ces états honnêtes.
- **Les règles de temps sont pensées pour le bruit, pas pour la logistique.** Les heures calmes protègent bien du message de 23 h ; elles empêchent le message de 6 h que le car rend nécessaire.
- **La minimisation s'arrête aux contenus ; elle doit couvrir les nombres et les états.** « 1 exclu », « famille prévenue », « nom de la personne qui a récupéré » sont des informations.
- **Les personnes qui comptent ne sont pas toutes titulaires** : grand-parent, nourrice, accompagnateur, transporteur, mairie. La conception les connaît comme contacts d'urgence ; il leur faut une place bornée dans les envois ordinaires.
- **Les enseignants communiquent déjà hors de l'ENT** [H] ; sans diffusion légère depuis le mobile, la meilleure conception reste inutilisée.
- **D-023 tient** : les ajouts sont des comportements par défaut ; les dix réglages nouveaux sont signalés avec une valeur fixe (§6 bis).

---

## 6 bis. Réglages signalés (D-023)

Liste à part, recopiée dans `08_NOTIFICATIONS.md` §18 ; **rien n'est supprimé**. « Version automatique » = ce que fait le logiciel quand l'établissement ne touche à rien.

| # | Réglage | Pourquoi il pèse | Version automatique proposée |
|---|---|---|---|
| S-56 | Validation préalable des diffusions des professeurs | un interrupteur de plus, un goulot | désactivé, un seul interrupteur |
| S-57 | Catégories d'une personne de confiance | tentation de tout rendre cochable | ensemble fixe |
| S-58 | Choix du parent en garde alternée | peut devenir une matrice | deux choix |
| S-59 | Modèles de messages | textes jamais revus | 50 au plus, revue annuelle |
| S-60 | Heures de la fenêtre veille / matin | sans enjeu local | 22 h 30 et 6 h 00 fixes |
| S-61 | Durée d'un suivi d'élève | confort | 3 mois renouvelables |
| S-62 | Différé des collégiens pendant les cours | ne doit pas être réglable | règle fixe |
| S-63 | Seuil « canal inefficace » | sans enjeu local | 5 avis non ouverts en 30 jours |
| S-64 | Plage d'envoi métier | tentation d'un réglage d'établissement | déclarée par la nature |
| S-65 | Catégories des personnels | multiplication | six fixes |
| S-66 | **Existant** : préréglage, catégorie × canal, heures calmes, pause, budget de bruit, heure du résumé | cinq réglages pour « quand et comment me déranger ? » | façade : préréglage + heures calmes ; le reste replié ; budget de bruit fixe et invisible |
| S-67 | **Existant** : délais d'annulation et de confirmation | trois durées inexplicables | valeurs gardées ; seul le délai de confirmation apparaît |

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** vérifiés dans la conception : U2 attend la fin des heures calmes de nuit, et `edt.fermeture_exceptionnelle` est U2 (§9.3, §4.1) ; « le professeur qui diffuse voit "1 personne exclue (décision)" » (§15) ; aucune garde d'assemblage sur la nature d'un événement source (§6.1) ; délai de confirmation compté « après la clôture de l'appel » (CN-01, R-N12) ; « remise non confirmée » distincte de « remise » mais aucune règle sur ce que vaut « joint » (§8.3, §11.4) ; texte rendu des natures courantes 90 jours (§19) ; catégories de préférence orientées familles (§9.2) ; audiences de NTF-46 sans payeurs, transport, site ni inscrits de la rentrée ; NTF-94 limité au retard de retour ; diffusions de masse réservées au web (§24) ; aucune règle de changement d'un canal d'envoi ; aucun état d'autorisation des notifications sur l'appareil (§19, `ent_ntf_appareil`). Constats RP1-06, RP1-11, RP1-12, RP1-13 lus dans `RECETTE_PERSONAS_1.md` (branche `ent-recette-1-correctifs`).

**Hypothèses [H]** : la vie des personas et les fréquences (appels non clôturés, numéros d'élèves déclarés comme ceux des parents, groupes de messagerie personnels des enseignants, coupons-réponses, familles sans numérique) ; les fonctions de Charlemagne et d'École Directe ; le comportement des fournisseurs de notifications quand l'utilisateur a coupé les alertes ; le coût des SMS en alphabets non latins (UCS-2).

**À vérifier [NV]** : information des familles un jour de grève dans le premier degré public ; règles d'usage du téléphone portable au collège ; procédure et pièces d'un signalement d'absentéisme ; durée de conservation des preuves d'information ; information d'une personne dont le numéro est donné par un tiers ; désignation d'une personne de confiance par un seul titulaire (acte usuel) ; information des familles d'un suivi interne ; plans de numérotation mobile en France et à l'étranger.

**Contradictions signalées** (toutes notées [personas] dans la conception, aucune tranchée) : §15 contre la minimisation (R-N51) ; §9.3 contre le ramassage scolaire (R-N50) ; §19 contre la preuve de fin d'année (R-N59) ; §24 précisé par R-N64 ; NTF-58 suppose R-N52. Les compléments de modèle entrent dans les migrations prévues (3801, 3803, 3804, 3805) ou dans la réserve 3811–3889 ; **à attribuer dans `MIGRATIONS.md`** au moment du code (non fait ici, aucune migration n'est écrite).
