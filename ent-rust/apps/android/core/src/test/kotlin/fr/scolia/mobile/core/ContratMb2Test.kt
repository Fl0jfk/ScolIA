package fr.scolia.mobile.core

import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.test.fail

/**
 * Socle 11, palier MB2 : le cœur décode **toutes** les fixtures dorées produites par le serveur
 * (`ent-rust/contrat/mobile/<cas>.json`, banc `ent-api/src/tests_contrat_mobile.rs`) avec ses vraies méthodes, et
 * n'appelle aucune route hors de la liste blanche `routes.txt`.
 *
 * Un DTO serveur qui change régénère les fixtures ; si le modèle Kotlin n'a pas suivi, ce test devient rouge.
 * Une fixture sans entrée ici, ou une méthode publique d'[ApiClient] sans cas, le rend rouge aussi.
 */
class ContratMb2Test {
    private val dossier: File = File(
        System.getProperty("scolia.contrat") ?: "../../../contrat/mobile",
    ).canonicalFile

    private val lecteur = Json { ignoreUnknownKeys = true }

    /** Serveur qui répond à toute requête par le corps et le statut de la fixture, et note ce qu'il a reçu. */
    private class ServeurFixture(private val statut: Int, private val corps: String) : Transport {
        val recues = mutableListOf<RequeteHttp>()
        override suspend fun executer(requete: RequeteHttp): ReponseHttp {
            recues += requete
            return ReponseHttp(statut, corps)
        }
    }

    private fun client(t: Transport) = ApiClient(FauxServeur.BASE, t, { "jeton" }, appli = "android/1.4.2 (build 142)")

    /** Un cas par fixture : l'appel réel du client qui consomme cette réponse. */
    private val cas: Map<String, suspend (ApiClient) -> Unit> = mapOf(
        "auth_etablissement" to { c -> c.etablissementDeLAdresse() },
        "auth_login" to { c -> c.login("persona@ent.test", "x") },
        "auth_passkey_login_start" to { c -> c.passkeyLoginStart("persona@ent.test") },
        // Enrôlement : appelé par `ServiceConnexion.enrolerPasskey` avec le même décodeur.
        "auth_passkey_register_start" to { c -> c.post("/auth/passkey/register/start", "{}", PasskeyChallengeEnrolement.serializer()) },
        "auth_me" to { c -> c.me() },
        "roles_mes_droits" to { c -> c.mesDroits() },
        "auth_sessions" to { c -> c.sessions() },
        "auth_sessions_revoquer" to { c -> c.revoquerSession("s1") },
        "auth_logout" to { c -> c.logout() },
        "devices_push" to { c -> c.enregistrerPush(PushDeviceBody("android", "fcm")) },
        "devices_push_supprimer" to { c -> c.desinscrirePush("p1") },
        "edt_mes_creneaux" to { c -> c.mesCreneaux("2027-03-08") },
        "eleves_edt" to { c -> c.edtEleve("e1", "2027-03-08") },
        "appels_ouvrir" to { c -> c.ouvrirAppel("c1", "2027-03-08") },
        "appel" to { c -> c.appel("a1") },
        "appels_lignes" to { c -> c.marquer("a1", "e1", EtatAppel.ABSENT) },
        "appels_clore" to { c -> c.clore("a1") },
        "famille_enfants" to { c -> c.mesEnfants() },
        "famille_resume" to { c -> c.resumeEnfant("e1") },
        "absences_justifier" to { c -> c.justifierAbsence("ab1", "motif") },
        "vs_absences" to { c -> c.absencesATraiter("2027-03-08") },
        "note_periodes" to { c -> c.periodesNotes() },
        "eleves_notes" to { c -> c.notes("e1") },
        "eleves_moyennes" to { c -> c.moyennes("e1", "p1") },
        "devoirs" to { c -> c.devoirsClasse("p1", "4B") },
        "eleves_devoirs" to { c -> c.devoirsEleve("e1", "p1") },
        "classes_effectif" to { c -> c.effectif("4B", "2027-03-08") },
        "cahier_texte" to { c -> c.cahierTexte(classe = "4B") },
        "cahier_texte_eleve" to { c -> c.cahierTexte(eleveId = "e1") },
        "messages_fils" to { c -> c.fils() },
        "messages_fil" to { c -> c.fil("f1") },
        "messages_repondre" to { c -> c.repondre("f1", "Merci") },
        "annonces" to { c -> c.annonces() },
        "eleves_annonces" to { c -> c.annoncesEleve("e1") },
        "notifications" to { c -> c.notifications() },
        "eva_enseignements" to { c -> c.evaEnseignements() },
        "eva_cours_periodes" to { c -> c.evaPeriodes("c1") },
        "eva_grille" to { c -> c.evaGrille("c1", "p1") },
        "eva_evaluations" to { c -> c.creerEvaluation(CreerEvaluationEvaBody("c1", "Interro", "2027-03-08", 20_000, 1_000)) },
        "eva_saisie" to { c -> c.saisirNotes(SaisieEvaBody("c1", listOf(CelluleSaisieEva("ev1", "e1", "15,5")))) },
        "eva_saisie_conflit" to { c -> c.saisirNotes(SaisieEvaBody("c1", listOf(CelluleSaisieEva("ev1", "e1", "abs")))) },
        "eva_saisie_lot" to { c -> c.saisirNotes(SaisieEvaBody("c1", listOf(CelluleSaisieEva("ev1", "e1", "15,5", 1), CelluleSaisieEva("ev1", "e2", "21")))) },
        "eva_grille_apres_saisie" to { c -> c.evaGrille("c1", "p1") },
        "eva_evaluations_publication" to { c -> c.publierEvaluation("ev1", "maintenant") },
    )

    /** Méthodes publiques d'[ApiClient] qui ne consomment pas une fixture, avec la raison. */
    private val horsFixtures = mapOf(
        "passkeyLoginFinish" to "cérémonie WebAuthn, non jouée par le banc serveur (MB4)",
        "getBrut" to "lecture brute du cache hors ligne : chemins de la liste blanche (HorsLigne)",
        "get" to "primitive générique",
        "post" to "primitive générique (enrôlement passkey dans Session)",
        "decoder" to "primitive générique",
        "getBaseUrl" to "accesseur",
        "getAppli" to "accesseur",
    )

    private fun fixtures(): List<File> {
        assertTrue(dossier.isDirectory, "dossier des fixtures introuvable : $dossier")
        return dossier.listFiles { f -> f.extension == "json" }.orEmpty().sortedBy { it.name }
    }

    private fun routes(): List<Regex> =
        File(dossier, "routes.txt").readLines().map { it.trim() }.filter { it.isNotEmpty() && !it.startsWith("#") }.map { Regex(gabaritVersRegex(it)) }

    private fun dansLaListe(r: RequeteHttp, liste: List<Regex>): Boolean {
        val ligne = "${r.methode} ${r.url.removePrefix(FauxServeur.BASE)}"
        return liste.any { it.matches(ligne) }
    }

    @Test
    fun chaque_fixture_est_decodee_par_le_vrai_client_sur_la_bonne_route() = runTest {
        val liste = routes()
        val vues = fixtures()
        assertTrue(vues.size >= cas.size, "fixtures manquantes : ${cas.keys - vues.map { it.nameWithoutExtension }.toSet()}")
        val echecs = mutableListOf<String>()
        for (f in vues) {
            val nom = f.nameWithoutExtension
            val appel = cas[nom]
            if (appel == null) {
                echecs += "$nom : fixture sans cas dans ContratMb2Test (nouvelle route servie aux applis ?)"
                continue
            }
            val fixture = lecteur.parseToJsonElement(f.readText()).jsonObject
            val statut = fixture.getValue("statut").jsonPrimitive.int
            val corps = fixture["corps"]?.let { if (it is kotlinx.serialization.json.JsonNull) "" else it.toString() } ?: ""
            val route = fixture.getValue("route").jsonPrimitive.content
            val serveur = ServeurFixture(statut, corps)
            try {
                appel(client(serveur))
            } catch (e: Exception) {
                echecs += "$nom : réponse non décodée (${e::class.simpleName}: ${e.message})"
                continue
            }
            val r = serveur.recues.singleOrNull()
            if (r == null) {
                echecs += "$nom : ${serveur.recues.size} requêtes au lieu d'une"
                continue
            }
            if (!Regex(gabaritVersRegex(route)).matches("${r.methode} ${r.url.removePrefix(FauxServeur.BASE)}")) {
                echecs += "$nom : le client appelle ${r.methode} ${r.url} alors que la fixture vient de $route"
            }
            if (!dansLaListe(r, liste)) echecs += "$nom : ${r.methode} ${r.url} hors de routes.txt"
        }
        for (nom in cas.keys - vues.map { it.nameWithoutExtension }.toSet()) echecs += "$nom : cas sans fixture"
        if (echecs.isNotEmpty()) fail("contrat mobile rompu côté Kotlin :\n  " + echecs.joinToString("\n  "))
    }

    @Test
    fun chaque_methode_publique_du_client_est_couverte() {
        val couvertes = setOf(
            "etablissementDeLAdresse", "login", "passkeyLoginStart", "me", "mesDroits", "sessions", "revoquerSession", "logout",
            "enregistrerPush", "desinscrirePush", "mesCreneaux", "edtEleve", "ouvrirAppel", "appel", "marquer", "clore",
            "mesEnfants", "resumeEnfant", "justifierAbsence", "absencesATraiter", "periodesNotes", "notes", "moyennes",
            "devoirsClasse", "devoirsEleve", "effectif", "cahierTexte", "fils", "fil", "repondre",
            "annonces", "annoncesEleve", "notifications",
            "evaEnseignements", "evaPeriodes", "evaGrille", "creerEvaluation", "saisirNotes", "publierEvaluation",
        ) + horsFixtures.keys
        val publiques = ApiClient::class.java.declaredMethods
            .filter { java.lang.reflect.Modifier.isPublic(it.modifiers) && !it.isSynthetic }
            .map { it.name.substringBefore("-") }
            // `internal` compilé en public avec le suffixe `$core` : hors de l'API des écrans.
            .filterNot { it.endsWith("\$default") || it.contains('$') }
            .toSet()
        val oubliees = publiques - couvertes
        assertTrue(oubliees.isEmpty(), "méthodes d'ApiClient sans cas de contrat (MB2) : $oubliees")
    }

    @Test
    fun les_cheminements_hors_fixture_restent_dans_la_liste_blanche() = runTest {
        val liste = routes()
        val s = ServeurFixture(200, "{}")
        val c = client(s)
        runCatching { c.passkeyLoginStart("persona@ent.test") }
        runCatching { c.passkeyLoginFinish(PasskeyLoginFinishBody("t", JsonObject(emptyMap()))) }
        runCatching { c.post("/auth/passkey/register/start", "{}", kotlinx.serialization.json.JsonElement.serializer()) }
        runCatching { c.post("/auth/passkey/register/finish", "{}", kotlinx.serialization.json.JsonElement.serializer()) }
        // Chemins mis en cache hors ligne (Chemins de Session) : lus par getBrut.
        for (chemin in listOf(
            Chemins.mesCreneaux("2027-03-08"), Chemins.edtEleve("e1", "2027-03-08"), Chemins.cahierClasse("4B"),
            Chemins.cahier("e1"), Chemins.notes("e1"), Chemins.FILS, Chemins.ENFANTS,
        )) runCatching { c.getBrut(chemin) }
        assertEquals(11, s.recues.size)
        val hors = s.recues.filterNot { dansLaListe(it, liste) }.map { "${it.methode} ${it.url}" }
        assertTrue(hors.isEmpty(), "requêtes hors de routes.txt : $hors")
    }

    @Test
    fun bandeaux_session_courte_et_appareil_sans_verrou() {
        fun me(assurance: String, portee: String = "complete") =
            Me("u1", "p@ent.test", "e1", "prof", portee = portee, assurance = assurance)
        assertEquals(emptyList(), Avertissement.pour(me("passkey"), appareilSansVerrou = false))
        assertEquals(listOf(Avertissement.SESSION_COURTE), Avertissement.pour(me("mot_de_passe"), false))
        assertEquals(listOf(Avertissement.SESSION_COURTE), Avertissement.pour(me("sso"), false))
        // Enrôlement exigé : l'écran d'enrôlement suffit, pas de bandeau en plus.
        assertEquals(emptyList(), Avertissement.pour(me("mot_de_passe", portee = "enrolement"), false))
        assertEquals(
            listOf(Avertissement.SESSION_COURTE, Avertissement.APPAREIL_SANS_VERROU),
            Avertissement.pour(me("mot_de_passe"), appareilSansVerrou = true),
        )
    }

    @Test
    fun cle_gardee_apres_un_echec_incertain_et_oubliee_apres_succes_ou_refus() = runTest {
        var n = 0
        val cles = ClesIntention { "cle-${++n}" }
        val vues = mutableListOf<String>()
        // Réseau coupé : le nouvel essai reprend la même clé.
        runCatching { cles.executer("marquer:a1:e1:absent") { c -> vues += c; throw ErreurApi.HorsLigne(null) } }
        runCatching { cles.executer("marquer:a1:e1:absent") { c -> vues += c; throw ErreurApi.Refus(409, "en cours", "idempotence.en_cours") } }
        cles.executer("marquer:a1:e1:absent") { c -> vues += c }
        assertEquals(listOf("cle-1", "cle-1", "cle-1"), vues)
        // Après le succès, la même intention est une nouvelle demande.
        assertEquals("cle-2", cles.executer("marquer:a1:e1:absent") { it })
        // Refus définitif (403) : clé oubliée.
        runCatching { cles.executer("clore:a1") { throw ErreurApi.Interdit("non") } }
        assertEquals("cle-4", cles.cle("clore:a1"))
        // Changer d'avis (autre état) : autre intention, autre clé.
        assertTrue(cles.cle("marquer:a1:e1:present") != cles.cle("marquer:a1:e1:retard"))
        assertTrue(ClesIntention.incertaine(ErreurApi.Serveur(503)))
        assertTrue(ClesIntention.incertaine(ErreurApi.DebitDepasse("lent", 5)))
        assertFalse(ClesIntention.incertaine(ErreurApi.Refus(422, "invalide")))
        assertFalse(ClesIntention.incertaine(ErreurApi.AppliTropAncienne("vieille")))
        assertFalse(ClesIntention.incertaine(ErreurApi.Refus(409, "appel clos", "appel.clos")))
    }

    @Test
    fun un_succes_oublie_les_intentions_soeurs() = runTest {
        var n = 0
        val cles = ClesIntention { "cle-${++n}" }
        val f = "marquer:a1:e1:"
        // « Absent » incertain (clé gardée), puis « Retard » réussi, puis « Absent » à nouveau : clé neuve.
        runCatching { cles.executer("${f}absent", famille = f) { throw ErreurApi.HorsLigne(null) } }
        cles.executer("${f}retard", famille = f) { }
        assertEquals("cle-3", cles.executer("${f}absent", famille = f) { it })
    }

    private fun gabaritVersRegex(route: String): String {
        val (methode, gabarit) = route.split(" ", limit = 2)
        return "^$methode " + gabarit.split("/").joinToString("/") { seg ->
            if (seg.startsWith("{") && seg.endsWith("}")) "[^/?]+" else Regex.escape(seg)
        } + "(\\?.*)?$"
    }
}
