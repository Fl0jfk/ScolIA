package fr.scolia.mobile.core

import kotlinx.coroutines.test.runTest
import java.time.LocalDateTime
import java.time.ZoneId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

private const val APPLI = "android/1.4.2 (build 142)"
private const val ME = """{"user_id":"u1","email":"prof@lpnb.fr","etablissement_id":"e1","role":"prof",
 "passkey_count":1,"portee":"complete","assurance":"passkey","client":"mobile","passkey_obligatoire":true}"""

private fun enveloppe(code: String, message: String = "msg", extra: String = "") =
    """{"erreur":{"code":"$code","message":"$message","champs":[],"reessayer_apres_s":null,"trace":"tr-1"$extra}}"""

/** Contrat mobile MB1 côté appli (§5.2, §5.3) et correctifs d'appli de MB0 (DA1, DS4). */
class ContratMb1Test {
    private val paris = ZoneId.of("Europe/Paris")

    private fun client(s: FauxServeur, coffre: CoffreJeton = CoffreMemoire().apply { ecrire("j") }, cles: Iterator<String>? = null) =
        ApiClient(FauxServeur.BASE, s, coffre::lire, appli = APPLI, nouvelleCle = { cles?.next() ?: java.util.UUID.randomUUID().toString() })

    // ---------- En-têtes ----------

    @Test
    fun toute_requete_declare_l_appli() = runTest {
        val s = FauxServeur(mapOf("GET /auth/me" to ReponseHttp(200, ME), "GET /auth/etablissement" to ReponseHttp(200, """{"id":"e1","slug":"lpnb","nom":"Lycée"}""")))
        val c = client(s)
        c.me()
        runCatching { c.etablissementDeLAdresse() }
        assertEquals(2, s.recues.size)
        s.recues.forEach { assertEquals(APPLI, it.entetes["X-ScolIA-Appli"]) }
    }

    @Test
    fun valeur_d_appli_sans_identifiant_d_appareil() {
        assertEquals("android/0.1.0 (build 1)", ApiClient.valeurAppli("android", "0.1.0", 1))
    }

    @Test
    fun ecriture_authentifiee_porte_une_cle_uuid_et_lecture_non() = runTest {
        val s = FauxServeur(
            mapOf(
                "POST /appels/a1/lignes" to ReponseHttp(204, ""),
                "GET /auth/me" to ReponseHttp(200, ME),
                "DELETE /devices/push/p1" to ReponseHttp(204, ""),
            ),
        )
        val c = client(s)
        c.marquer("a1", "el1", EtatAppel.PRESENT)
        c.me()
        c.desinscrirePush("p1")
        val cle = s.recues[0].entetes["Idempotency-Key"]
        assertNotNull(cle)
        assertEquals(36, cle.length)
        java.util.UUID.fromString(cle) // UUID valide
        assertNull(s.recues[1].entetes["Idempotency-Key"])
        assertNotNull(s.recues[2].entetes["Idempotency-Key"])
    }

    @Test
    fun deux_intentions_deux_cles_et_un_nouvel_essai_garde_la_sienne() = runTest {
        val s = FauxServeur(mapOf("POST /appels/a1/lignes" to ReponseHttp(204, "")))
        val c = client(s)
        c.marquer("a1", "el1", EtatAppel.ABSENT)
        c.marquer("a1", "el2", EtatAppel.ABSENT)
        assertTrue(s.recues[0].entetes["Idempotency-Key"] != s.recues[1].entetes["Idempotency-Key"])
        // Nouvel essai de la même intention (après un délai dépassé) : même clé, le serveur rejoue.
        c.marquer("a1", "el1", EtatAppel.ABSENT, cle = "6f1c1a8e-0e8b-4a43-9c55-3a9f2c7a2b10")
        c.marquer("a1", "el1", EtatAppel.ABSENT, cle = "6f1c1a8e-0e8b-4a43-9c55-3a9f2c7a2b10")
        assertEquals(s.recues[2].entetes["Idempotency-Key"], s.recues[3].entetes["Idempotency-Key"])
    }

    @Test
    fun authentification_hors_registre_sans_cle() = runTest {
        val s = FauxServeur(
            mapOf(
                "POST /auth/login" to ReponseHttp(401, enveloppe("auth.refusee")),
                "POST /auth/logout" to ReponseHttp(204, ""),
                "DELETE /auth/sessions/s1" to ReponseHttp(204, ""),
            ),
        )
        val c = client(s)
        runCatching { c.login("a@b.fr", "x") }
        c.logout()
        c.revoquerSession("s1")
        s.recues.forEach { assertNull(it.entetes["Idempotency-Key"], it.url) }
    }

    // ---------- Enveloppe d'erreur codée ----------

    @Test
    fun session_expiree_est_une_session_perdue() = runTest {
        val s = FauxServeur(mapOf("GET /auth/me" to ReponseHttp(401, enveloppe("session.expiree", "Session expirée."))))
        val e = assertFailsWith<ErreurApi.NonAuthentifie> { client(s).me() }
        assertEquals("session.expiree", e.code)
        assertEquals("tr-1", e.trace)
    }

    @Test
    fun mot_de_passe_actuel_refuse_n_est_pas_une_session_perdue() = runTest {
        // Enrôlement passkey avec un mauvais mot de passe actuel : 401 `auth.refusee`, la session reste.
        val s = FauxServeur(mapOf("POST /auth/passkey/register/finish" to ReponseHttp(401, enveloppe("auth.refusee", "Identifiants refusés."))))
        val c = client(s)
        val e = assertFailsWith<ErreurApi.Refus> {
            c.post("/auth/passkey/register/finish", "{}", PasskeyChallengeEnrolement.serializer())
        }
        assertEquals(401, e.statut)
        assertEquals("auth.refusee", e.code)
        assertEquals("Identifiants refusés.", e.message)
    }

    @Test
    fun jeton_mort_sur_une_route_auth_est_une_session_perdue() = runTest {
        // Le serveur pose `auth.refusee` sur tout 401 de `/auth/*` : un jeton révoqué sur `/auth/me` en reçoit un.
        val s = FauxServeur(mapOf("GET /auth/me" to ReponseHttp(401, enveloppe("auth.refusee", "Identifiants refusés."))))
        val coffre = CoffreMemoire().apply { ecrire("jeton-mort") }
        assertFailsWith<ErreurApi.NonAuthentifie> { client(s, coffre).me() }
        val cache = CacheMemoire().apply { ecrire("k", EntreeCache("[]", System.currentTimeMillis(), Long.MAX_VALUE)) }
        val etat = ServiceConnexion(client(s, coffre), coffre, cache, FauxPasskey()).reprendre()
        assertIs<EtatConnexion.Deconnecte>(etat)
        assertNull(coffre.lire())
        assertEquals(0, cache.taille)
    }

    @Test
    fun connexion_refusee_avec_enveloppe() = runTest {
        val s = FauxServeur(mapOf("POST /auth/login" to ReponseHttp(401, enveloppe("auth.refusee", "Identifiants refusés."))))
        val coffre = CoffreMemoire()
        val e = assertFailsWith<ErreurApi.Refus> { client(s, coffre).login("a@b.fr", "x") }
        assertEquals("auth.refusee", e.code)
    }

    @Test
    fun message_neutre_du_serveur_jamais_le_json_brut() = runTest {
        val s = FauxServeur(
            mapOf(
                "GET /eleves/x/notes" to ReponseHttp(404, enveloppe("objet.introuvable", "Élément introuvable.")),
                "GET /annonces" to ReponseHttp(403, """{"autre":"json"}"""),
            ),
        )
        val c = client(s)
        val e = assertFailsWith<ErreurApi.Introuvable> { c.notes("x") }
        assertEquals("objet.introuvable", e.code)
        assertEquals("Élément introuvable.", e.message)
        // JSON qui n'est pas l'enveloppe : jamais affiché tel quel.
        val i = assertFailsWith<ErreurApi.Interdit> { c.annonces() }
        assertEquals("Action non autorisée.", i.message)
        assertNull(i.code)
    }

    @Test
    fun champs_et_regle_metier() = runTest {
        val s = FauxServeur(
            mapOf(
                "POST /absences/ab1/justifier" to ReponseHttp(
                    400,
                    """{"erreur":{"code":"requete.invalide","message":"Motif manquant.","champs":[{"champ":"motif","code":"champ.requis"}],"reessayer_apres_s":null,"trace":"t"}}""",
                ),
                "POST /eleves/el1/notes" to ReponseHttp(422, enveloppe("periode.arretee", "Période arrêtée.")),
            ),
        )
        val c = client(s)
        val e = assertFailsWith<ErreurApi.Refus> { c.justifierAbsence("ab1", "") }
        assertEquals("requete.invalide", e.code)
        assertEquals(listOf(ErreurChampCodee("motif", "champ.requis")), e.champs)
        val r = assertFailsWith<ErreurApi.Refus> { c.saisirNote("el1", SaisirNoteBody(matiereId = "m1", valeur = 12.0, dateNote = "2027-03-14")) }
        assertEquals(422, r.statut)
        assertEquals("periode.arretee", r.code)
    }

    @Test
    fun idempotence_en_cours_donne_le_delai() = runTest {
        val s = FauxServeur(
            mapOf("POST /appels/a1/clore" to ReponseHttp(409, """{"erreur":{"code":"idempotence.en_cours","message":"En cours.","champs":[],"reessayer_apres_s":2,"trace":"t"}}""")),
        )
        val e = assertFailsWith<ErreurApi.Refus> { client(s).clore("a1") }
        assertEquals("idempotence.en_cours", e.code)
        assertEquals(2, e.reessayerApresS)
    }

    @Test
    fun appli_trop_ancienne_et_debit() = runTest {
        val s = FauxServeur(
            mapOf(
                "GET /auth/me" to ReponseHttp(426, enveloppe("appli.trop_ancienne", "Mettez l'application à jour pour continuer.")),
                "GET /annonces" to ReponseHttp(429, """{"erreur":{"code":"debit.depasse","message":"Trop de demandes.","champs":[],"reessayer_apres_s":30,"trace":"t"}}"""),
            ),
        )
        val c = client(s)
        assertIs<ErreurApi.AppliTropAncienne>(runCatching { c.me() }.exceptionOrNull())
        val d = assertFailsWith<ErreurApi.DebitDepasse> { c.annonces() }
        assertEquals(30, d.reessayerApresS)
    }

    @Test
    fun ancien_texte_court_toujours_lu() = runTest {
        // Route sans enveloppe (serveur plus ancien) : repli sur le texte court.
        val s = FauxServeur(mapOf("POST /absences/ab1/justifier" to ReponseHttp(422, "Délai dépassé.")))
        val e = assertFailsWith<ErreurApi.Refus> { client(s).justifierAbsence("ab1", "malade") }
        assertEquals("Délai dépassé.", e.message)
        assertNull(e.code)
    }

    @Test
    fun erreur_serveur_codee() = runTest {
        val s = FauxServeur(mapOf("GET /annonces" to ReponseHttp(503, enveloppe("service.maintenance"))))
        val e = assertFailsWith<ErreurApi.Serveur> { client(s).annonces() }
        assertEquals(503, e.statut)
        assertEquals("service.maintenance", e.code)
    }

    // ---------- DA1 : expiration du cache ----------

    private fun ms(t: String) = LocalDateTime.parse(t).atZone(paris).toInstant().toEpochMilli()

    @Test
    fun edt_et_appel_expirent_a_minuit_le_reste_a_24h() {
        val ecrit = ms("2027-03-14T22:30:00")
        assertEquals(ms("2027-03-15T00:00:00"), DureeCache.expiration(Chemins.mesCreneaux("2027-03-14"), ecrit, paris))
        assertEquals(ms("2027-03-15T00:00:00"), DureeCache.expiration(Chemins.edtEleve("el1", "2027-03-14"), ecrit, paris))
        assertEquals(ms("2027-03-15T00:00:00"), DureeCache.expiration("/appels/a1", ecrit, paris))
        assertEquals(ecrit + DureeCache.MAX_MS, DureeCache.expiration(Chemins.FILS, ecrit, paris))
        assertEquals(ecrit + DureeCache.MAX_MS, DureeCache.expiration(Chemins.notes("el1"), ecrit, paris))
        assertFalse(DureeCache.duJour("/eleves/el1/notes"))
    }

    @Test
    fun cache_de_25_h_non_servi_et_efface() = runTest {
        val s = FauxServeur(mapOf("GET /messages/fils" to ReponseHttp(200, "[]")))
        val cache = CacheMemoire()
        var maintenant = ms("2027-03-14T08:00:00")
        val l = LecteurHorsLigne(client(s), cache, { "u1|e1" }, { maintenant }, { paris })
        l.lire(Chemins.FILS, Chemins.listeFils)
        s.horsLigne = true
        maintenant += 23L * 3600 * 1000
        assertTrue(l.lire(Chemins.FILS, Chemins.listeFils).horsLigne) // 23 h : encore servi
        maintenant += 2L * 3600 * 1000
        assertFailsWith<ErreurApi.HorsLigne> { l.lire(Chemins.FILS, Chemins.listeFils) } // 25 h : refusé
        assertEquals(0, cache.taille)
    }

    @Test
    fun edt_de_la_veille_non_servi_apres_minuit() = runTest {
        val chemin = Chemins.mesCreneaux("2027-03-14")
        val s = FauxServeur(mapOf("GET $chemin" to ReponseHttp(200, "[]")))
        val cache = CacheMemoire()
        var maintenant = ms("2027-03-14T23:50:00")
        val l = LecteurHorsLigne(client(s), cache, { "u1|e1" }, { maintenant }, { paris })
        l.lire(chemin, Chemins.listeCreneaux)
        s.horsLigne = true
        maintenant = ms("2027-03-15T00:00:01")
        assertFailsWith<ErreurApi.HorsLigne> { l.lire(chemin, Chemins.listeCreneaux) }
    }

    @Test
    fun horloge_reculee_ou_plafond_depasse_rend_l_entree_echue() {
        val e = EntreeCache("[]", 1_000_000, Long.MAX_VALUE)
        assertFalse(e.echue(1_000_000 + 1))
        assertTrue(e.echue(999_999)) // horloge reculée avant l'écriture
        assertTrue(e.echue(1_000_000 + DureeCache.MAX_MS)) // plafond de 24 h revérifié à la lecture
    }

    @Test
    fun purge_au_demarrage_ne_garde_que_le_valide() {
        val cache = CacheMemoire()
        cache.ecrire("a", EntreeCache("[]", 0, 100))
        cache.ecrire("b", EntreeCache("[]", 0, 1_000))
        cache.purgerEchues(500)
        assertNull(cache.lire("a"))
        assertNotNull(cache.lire("b"))
        val l = LecteurHorsLigne(ApiClient(FauxServeur.BASE, FauxServeur(emptyMap())), cache, { "x" }, { 2_000L })
        l.purger()
        assertEquals(0, cache.taille)
    }

    // ---------- DS4 : désinscription des notifications ----------

    private class PushFaux(private val jeton: String?) : FournisseurPush {
        override suspend fun jetonAppareil() = jeton
        override val plateforme = "android"
    }

    @Test
    fun push_inscrit_garde_l_identifiant_et_desinscrit_a_la_deconnexion() = runTest {
        val s = FauxServeur(
            mapOf(
                "POST /auth/login" to ReponseHttp(200, """{"mfa_required":false,"token":"t","user_id":"u1","email":"a@b.fr","etablissement_id":"e1","role":"prof","expires_at":"2027-02-01T00:00:00Z"}"""),
                "GET /auth/me" to ReponseHttp(200, ME),
                "GET /roles/mes-droits" to ReponseHttp(200, "[]"),
                "POST /devices/push" to ReponseHttp(201, """{"id":"p1","platform":"android"}"""),
                "DELETE /devices/push/p1" to ReponseHttp(204, ""),
                "POST /auth/logout" to ReponseHttp(204, ""),
            ),
        )
        val coffre = CoffreMemoire()
        val memoire = MemoirePushMemoire()
        val svc = ServiceConnexion(client(s, coffre), coffre, CacheMemoire(), FauxPasskey(), PushFaux("jeton-fcm"), memoire)
        svc.connexionMotDePasse("a@b.fr", "x")
        assertEquals("p1", memoire.lire())
        val inscription = s.recues.first { it.methode == "POST" && it.url.endsWith("/devices/push") }
        assertNotNull(inscription.entetes["Idempotency-Key"])
        svc.deconnexion()
        val ordre = s.recues.map { "${it.methode} ${it.url.removePrefix(FauxServeur.BASE)}" }
        assertTrue(ordre.indexOf("DELETE /devices/push/p1") in 0 until ordre.indexOf("POST /auth/logout"))
        assertNull(memoire.lire())
        assertNull(coffre.lire())
    }

    @Test
    fun deconnexion_sans_inscription_ne_desinscrit_rien() = runTest {
        val s = FauxServeur(mapOf("POST /auth/logout" to ReponseHttp(204, "")))
        val coffre = CoffreMemoire().apply { ecrire("j") }
        ServiceConnexion(client(s, coffre), coffre, CacheMemoire(), FauxPasskey()).deconnexion()
        assertEquals(listOf("POST"), s.recues.map { it.methode })
    }

    // ---------- Transport ----------

    @Test
    fun transport_refuse_le_texte_clair_hors_labo() = runTest {
        val t = TransportHttps()
        assertFailsWith<java.io.IOException> { t.executer(RequeteHttp("GET", "http://lpnb.scolia.fr/auth/me", emptyMap())) }
    }

    @Test
    fun transport_ne_suit_pas_les_redirections() = runTest {
        val serveur = com.sun.net.httpserver.HttpServer.create(java.net.InetSocketAddress("127.0.0.1", 0), 0)
        var cibleAtteinte = false
        serveur.createContext("/depart") { ex ->
            ex.responseHeaders.add("Location", "/cible")
            ex.sendResponseHeaders(302, -1)
            ex.close()
        }
        serveur.createContext("/cible") { ex ->
            cibleAtteinte = true
            ex.sendResponseHeaders(200, -1)
            ex.close()
        }
        serveur.createContext("/erreur") { ex ->
            val b = enveloppe("objet.introuvable").toByteArray()
            ex.sendResponseHeaders(404, b.size.toLong())
            ex.responseBody.use { it.write(b) }
        }
        serveur.createContext("/refus") { ex ->
            ex.requestBody.readBytes()
            val b = enveloppe("auth.refusee").toByteArray()
            ex.sendResponseHeaders(401, b.size.toLong())
            ex.responseBody.use { it.write(b) }
        }
        serveur.createContext("/ecrire") { ex ->
            val corps = ex.requestBody.readBytes().toString(Charsets.UTF_8)
            val ok = ex.requestHeaders.getFirst("X-ScolIA-Appli") == APPLI && corps == "{\"a\":1}"
            val b = (if (ok) "ok" else "ko").toByteArray()
            ex.sendResponseHeaders(200, b.size.toLong())
            ex.responseBody.use { it.write(b) }
        }
        serveur.start()
        try {
            val base = "http://127.0.0.1:${serveur.address.port}"
            val t = TransportHttps(autoriserHttp = true)
            assertEquals(302, t.executer(RequeteHttp("GET", "$base/depart", emptyMap())).statut)
            assertFalse(cibleAtteinte)
            val e = t.executer(RequeteHttp("GET", "$base/erreur", emptyMap()))
            assertEquals(404, e.statut)
            assertTrue(e.corps.contains("objet.introuvable"))
            val w = t.executer(RequeteHttp("POST", "$base/ecrire", mapOf("X-ScolIA-Appli" to APPLI, "Content-Type" to "application/json"), "{\"a\":1}"))
            assertEquals("ok", w.corps)
            // Un 401 sur un POST est rendu tel quel (pas d'exception « hors ligne »).
            val r = t.executer(RequeteHttp("POST", "$base/refus", mapOf("Content-Type" to "application/json"), "{\"email\":\"a\"}"))
            assertEquals(401, r.statut)
            assertTrue(r.corps.contains("auth.refusee"))
        } finally {
            serveur.stop(0)
        }
    }
}
