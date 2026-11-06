package fr.scolia.mobile.core

import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** Faux serveur : répond selon « MÉTHODE chemin » et garde les requêtes reçues. */
class FauxServeur(private val routes: Map<String, ReponseHttp>) : Transport {
    val recues = mutableListOf<RequeteHttp>()
    var horsLigne = false
    override suspend fun executer(requete: RequeteHttp): ReponseHttp {
        if (horsLigne) throw java.io.IOException("réseau coupé")
        recues += requete
        val chemin = requete.url.removePrefix(BASE)
        return routes["${requete.methode} $chemin"] ?: ReponseHttp(404, "introuvable")
    }

    companion object {
        const val BASE = "https://lpnb.scolia.fr"
    }
}

private fun ok(json: String) = ReponseHttp(200, json)

private const val ME_PROF = """{"user_id":"u1","email":"prof@lpnb.fr","etablissement_id":"e1","role":"prof",
 "passkey_count":1,"portee":"complete","assurance":"passkey","client":"mobile","passkey_obligatoire":true}"""

private const val OPTIONS_CONNEXION = """{"publicKey":{"challenge":"Y2hhbGxlbmdl","timeout":60000,
 "rpId":"scolia.fr","allowCredentials":[{"type":"public-key","id":"aWQx"}],"userVerification":"required"},"mediation":null}"""

private const val ASSERTION_NATIVE = """{"id":"aWQx","rawId":"aWQx","type":"public-key",
 "response":{"authenticatorData":"YXV0aA","clientDataJSON":"Y2xpZW50","signature":"c2ln","userHandle":"dXNlcg"},
 "clientExtensionResults":{},"authenticatorAttachment":"platform"}"""

class FauxPasskey(private val reponse: String = ASSERTION_NATIVE) : AuthentificateurPasskey {
    var optionsRecues: String? = null
    override suspend fun signer(optionsJson: String): String {
        optionsRecues = optionsJson
        return reponse
    }
    override suspend fun creer(optionsJson: String): String = error("non utilisé")
}

class ModelesTest {
    private val json = Json { ignoreUnknownKeys = true }

    @Test
    fun login_reponse_a_plat_donne_une_session() {
        val r = json.decodeFromString(
            LoginResponse.serializer(),
            """{"mfa_required":false,"token":"t","user_id":"u","email":"a@b.fr","etablissement_id":"e","role":"parent","expires_at":"2027-02-01T00:00:00Z"}""",
        )
        assertEquals("parent", r.session()?.role)
        assertFalse(r.toString().contains("\"t\""))
    }

    @Test
    fun login_mfa_ne_donne_pas_de_session() {
        val r = json.decodeFromString(
            LoginResponse.serializer(),
            """{"mfa_required":true,"passkey":{"token":"x","options":$OPTIONS_CONNEXION,"expires_at":"2026-11-06T10:00:00Z"}}""",
        )
        assertNull(r.session())
        assertEquals("x", r.passkey?.token)
    }

    @Test
    fun le_mot_de_passe_n_apparait_jamais_dans_toString() {
        assertFalse(LoginBody("a@b.fr", "Secret123!").toString().contains("Secret123!"))
        val corps = Json.encodeToString(LoginBody.serializer(), LoginBody("a@b.fr", "p"))
        assertTrue(corps.contains("\"client\":\"mobile\""), corps)
    }

    @Test
    fun me_tolere_les_champs_absents_et_inconnus() {
        val me = json.decodeFromString(
            Me.serializer(),
            """{"user_id":"u","email":"e@x.fr","etablissement_id":"e","role":"eleve","eleve_id":"el1","passkey_count":0,"nouveau":1}""",
        )
        assertEquals("complete", me.portee)
        assertEquals("el1", me.eleveId)
    }

    @Test
    fun appel_et_etats() {
        val a = json.decodeFromString(
            Appel.serializer(),
            """{"id":"a","classe":"3A","jour":"2026-11-06","statut":"clos","creneau_id":null,"matiere_libelle":"Maths",
               "lignes":[{"eleve_id":"e","eleve_label":"Lou B.","etat":"retard"}]}""",
        )
        assertTrue(a.estClos)
        assertEquals(EtatAppel.RETARD, EtatAppel.depuis(a.lignes[0].etat))
        assertNull(EtatAppel.depuis("inconnu"))
    }
}

class CapacitesTest {
    private fun me(role: String, eleveId: String? = null, portee: String = "complete") =
        Me("u", "x@y.fr", "e", role, eleveId, 1, portee, "passkey", "mobile", false)

    @Test
    fun professeur_appel_en_premier() {
        val e = Capacites.espacePour(me("prof"), listOf(Droit("notes.saisir", "mes_classes")))
        assertEquals(Profil.PROFESSEUR, e.profil)
        assertEquals(listOf(Module.ACCUEIL, Module.APPEL), e.modules.take(2))
        assertTrue(e.contient(Module.SAISIE_NOTES))
        assertFalse(e.contient(Module.ABSENCES))
        assertEquals(Module.PROFIL, e.modules.last())
    }

    @Test
    fun famille_absences_et_notes() {
        val e = Capacites.espacePour(me("parent"), emptyList())
        assertEquals(Profil.FAMILLE, e.profil)
        assertTrue(e.contient(Module.ABSENCES))
        assertTrue(e.contient(Module.NOTES))
        assertFalse(e.contient(Module.APPEL))
        assertFalse(e.contient(Module.SAISIE_NOTES))
    }

    @Test
    fun eleve_sans_appel_ni_absences_a_justifier() {
        val e = Capacites.espacePour(me("eleve", "el1"), emptyList())
        assertEquals(Profil.ELEVE, e.profil)
        assertFalse(e.contient(Module.APPEL))
        assertFalse(e.contient(Module.ABSENCES))
        assertTrue(e.contient(Module.EDT))
    }

    @Test
    fun role_libre_avec_droit_absences_etablissement_devient_vie_scolaire() {
        val e = Capacites.espacePour(me("administratif"), listOf(Droit("absences.lire", "etablissement")))
        assertEquals(Profil.VIE_SCOLAIRE, e.profil)
        assertTrue(e.contient(Module.VIE_SCOLAIRE))
        assertFalse(e.contient(Module.APPEL))
    }

    @Test
    fun direction() {
        val e = Capacites.espacePour(me("direction"), emptyList())
        assertEquals(Profil.DIRECTION, e.profil)
        assertTrue(e.contient(Module.VIE_SCOLAIRE))
    }

    @Test
    fun enrolement_obligatoire_bloque_tout_sauf_le_compte() {
        val e = Capacites.espacePour(me("prof", portee = "enrolement"), emptyList())
        assertTrue(e.enrolementRequis)
        assertEquals(listOf(Module.PROFIL), e.modules)
    }

    @Test
    fun au_plus_quatre_onglets() {
        val e = Capacites.espacePour(me("parent"), emptyList())
        assertEquals(4, e.onglets.size)
        assertEquals(e.modules, e.onglets + e.autres)
    }
}

class RattachementTest {
    private val l = LecteurRattachement()

    @Test
    fun lien_d_invitation() {
        val r = l.lire("https://lpnb.scolia.fr/rejoindre?invitation=AbCdEf123456")
        val ok = assertIs<LecteurRattachement.Resultat.Ok>(r).rattachement
        assertEquals("https://lpnb.scolia.fr", ok.baseUrl)
        assertEquals("lpnb", ok.slug)
        assertEquals("AbCdEf123456", ok.invitation)
    }

    @Test
    fun saisie_courte() {
        val ok = assertIs<LecteurRattachement.Resultat.Ok>(l.lire(" LPNB ")).rattachement
        assertEquals("https://lpnb.scolia.fr", ok.baseUrl)
        assertIs<LecteurRattachement.Resultat.Ok>(l.lire("lpnb.scolia.fr"))
    }

    @Test
    fun refus() {
        listOf(
            "http://lpnb.scolia.fr",
            "https://scolia.fr.evil.com",
            "https://evilscolia.fr",
            "https://lpnb.scolia.fr:8443",
            "https://user:pw@lpnb.scolia.fr",
            "scolia://rattacher?h=lpnb",
            "https://www.scolia.fr",
            "https://a..scolia.fr",
            "",
            "javascript:alert(1)",
        ).forEach { assertIs<LecteurRattachement.Resultat.Refuse>(l.lire(it), it) }
    }

    @Test
    fun invitation_malformee_ignoree() {
        val ok = assertIs<LecteurRattachement.Resultat.Ok>(l.lire("https://lpnb.scolia.fr/rejoindre?invitation=abc")).rattachement
        assertNull(ok.invitation)
        assertIs<LecteurRattachement.Resultat.Refuse>(l.lire("https://lpnb.scolia.fr/rejoindre?invitation=<script>"))
    }

    @Test
    fun labo_seulement_en_mode_labo() {
        assertIs<LecteurRattachement.Resultat.Refuse>(l.lire("http://10.0.2.2:3001"))
        val ok = assertIs<LecteurRattachement.Resultat.Ok>(
            LecteurRattachement(modeLabo = true).lire("http://10.0.2.2:3001?etablissement_id=abc"),
        ).rattachement
        assertEquals("http://10.0.2.2:3001", ok.baseUrl)
        assertEquals("abc", ok.etablissementIdLabo)
    }
}

class PasskeyTest {
    private val json = Json

    @Test
    fun options_connexion_extrait_public_key() {
        val o = json.parseToJsonElement(OPTIONS_CONNEXION).jsonObject
        val s = PasskeyPont.optionsConnexion(o, "scolia.fr")
        val pk = json.parseToJsonElement(s).jsonObject
        assertEquals("Y2hhbGxlbmdl", pk["challenge"]!!.jsonPrimitive.content)
    }

    @Test
    fun rp_id_etranger_refuse() {
        val o = json.parseToJsonElement(OPTIONS_CONNEXION.replace("scolia.fr", "evil.fr")).jsonObject
        assertFailsWith<PasskeyPont.ErreurPasskey> { PasskeyPont.optionsConnexion(o, "scolia.fr") }
    }

    @Test
    fun assertion_au_format_serveur() {
        val c = PasskeyPont.assertionPourServeur(ASSERTION_NATIVE)
        assertEquals("aWQx", c["rawId"]!!.jsonPrimitive.content)
        assertTrue(c.containsKey("extensions"))
        assertFalse(c.containsKey("clientExtensionResults"))
        val r = c["response"]!!.jsonObject
        assertEquals(setOf("authenticatorData", "clientDataJSON", "signature", "userHandle"), r.keys)
    }

    @Test
    fun assertion_incomplete_ou_mal_encodee_refusee() {
        assertFailsWith<PasskeyPont.ErreurPasskey> { PasskeyPont.assertionPourServeur("""{"id":"a","response":{}}""") }
        assertFailsWith<PasskeyPont.ErreurPasskey> {
            PasskeyPont.assertionPourServeur(ASSERTION_NATIVE.replace("c2ln", "c2ln+/="))
        }
        assertFailsWith<PasskeyPont.ErreurPasskey> { PasskeyPont.assertionPourServeur("pas du json") }
    }

    @Test
    fun attestation_au_format_serveur() {
        val c = PasskeyPont.attestationPourServeur(
            """{"id":"aWQx","rawId":"aWQx","type":"public-key","response":{"attestationObject":"YXR0","clientDataJSON":"Y2Q","transports":["internal","hybrid"]}}""",
        )
        assertEquals(setOf("attestationObject", "clientDataJSON", "transports"), c["response"]!!.jsonObject.keys)
    }
}

class ApiEtConnexionTest {
    private fun client(s: FauxServeur, coffre: CoffreJeton) = ApiClient(FauxServeur.BASE, s, coffre::lire)

    @Test
    fun connexion_passkey_complete() = runTest {
        val s = FauxServeur(
            mapOf(
                "POST /auth/passkey/login/start" to ok("""{"token":"defi1","options":$OPTIONS_CONNEXION,"expires_at":"2026-11-06T10:00:00Z"}"""),
                "POST /auth/passkey/login/finish" to ok("""{"token":"jeton-secret","user_id":"u1","email":"prof@lpnb.fr","etablissement_id":"e1","role":"prof","expires_at":"2027-02-01T00:00:00Z"}"""),
                "GET /auth/me" to ok(ME_PROF),
                "GET /roles/mes-droits" to ok("""[{"action":"notes.saisir","portee":"mes_classes","cibles":[]}]"""),
            ),
        )
        val coffre = CoffreMemoire()
        val pk = FauxPasskey()
        val svc = ServiceConnexion(client(s, coffre), coffre, CacheMemoire(), pk)
        val etat = assertIs<EtatConnexion.Connecte>(svc.connexionPasskey("prof@lpnb.fr"))
        assertEquals(Profil.PROFESSEUR, etat.espace.profil)
        assertEquals("jeton-secret", coffre.lire())

        val start = s.recues.first()
        assertTrue(start.corps!!.contains("\"client\":\"mobile\""))
        assertNull(start.entetes["Authorization"], "pas de jeton avant connexion")
        val finish = s.recues[1]
        assertTrue(finish.corps!!.contains("\"token\":\"defi1\""))
        assertTrue(finish.corps!!.contains("\"authenticatorData\""))
        assertEquals("Bearer jeton-secret", s.recues[2].entetes["Authorization"])
    }

    @Test
    fun mot_de_passe_puis_second_facteur() = runTest {
        val s = FauxServeur(
            mapOf(
                "POST /auth/login" to ok("""{"mfa_required":true,"passkey":{"token":"d2","options":$OPTIONS_CONNEXION,"expires_at":"2026-11-06T10:00:00Z"}}"""),
            ),
        )
        val coffre = CoffreMemoire()
        val svc = ServiceConnexion(client(s, coffre), coffre, CacheMemoire(), FauxPasskey())
        val etat = assertIs<EtatConnexion.SecondFacteur>(svc.connexionMotDePasse("p@x.fr", "pw"))
        assertEquals("d2", etat.defi.token)
        assertNull(coffre.lire(), "aucun jeton tant que le second facteur n'est pas fait")
    }

    @Test
    fun session_revoquee_efface_tout() = runTest {
        val s = FauxServeur(mapOf("GET /auth/me" to ReponseHttp(401, "session invalide")))
        val coffre = CoffreMemoire().apply { ecrire("ancien") }
        val cache = CacheMemoire().apply { ecrire("k", EntreeCache("[]", 1)) }
        val svc = ServiceConnexion(client(s, coffre), coffre, cache, FauxPasskey())
        assertEquals(EtatConnexion.Deconnecte, svc.reprendre())
        assertNull(coffre.lire())
        assertNull(cache.lire("k"))
    }

    @Test
    fun erreurs_http_traduites() = runTest {
        val coffre = CoffreMemoire().apply { ecrire("j") }
        val s = FauxServeur(
            mapOf(
                "GET /famille/enfants" to ReponseHttp(403, "accès refusé"),
                "GET /annonces" to ReponseHttp(500, "<html>boom</html>"),
                "POST /absences/a1/justifier" to ReponseHttp(422, "motif trop court"),
            ),
        )
        val c = client(s, coffre)
        assertEquals("accès refusé", assertFailsWith<ErreurApi.Interdit> { c.mesEnfants() }.message)
        assertEquals(500, assertFailsWith<ErreurApi.Serveur> { c.annonces() }.statut)
        assertEquals("motif trop court", assertFailsWith<ErreurApi.Refus> { c.justifierAbsence("a1", "x") }.message)
        s.horsLigne = true
        assertFailsWith<ErreurApi.HorsLigne> { c.annonces() }
    }

    @Test
    fun chemins_encodes() = runTest {
        val coffre = CoffreMemoire().apply { ecrire("j") }
        val s = FauxServeur(mapOf("GET /classes/3e%20A/effectif?jour=2026-11-06" to ok("""{"classe":"3e A","jour":"2026-11-06","eleves":[]}""")))
        assertEquals("3e A", client(s, coffre).effectif("3e A", "2026-11-06").classe)
    }

    @Test
    fun appel_marquer_puis_clore() = runTest {
        val coffre = CoffreMemoire().apply { ecrire("j") }
        val appelJson = """{"id":"a1","classe":"3A","jour":"2026-11-06","statut":"clos","lignes":[]}"""
        val s = FauxServeur(
            mapOf(
                "POST /appels/a1/lignes" to ReponseHttp(204, ""),
                "POST /appels/a1/clore" to ok(appelJson),
            ),
        )
        val c = client(s, coffre)
        c.marquer("a1", "el1", EtatAppel.ABSENT)
        assertTrue(s.recues[0].corps!!.contains("\"etat\":\"absent\""))
        assertTrue(c.clore("a1").estClos)
    }

    @Test
    fun lecture_hors_ligne_depuis_le_cache() = runTest {
        val coffre = CoffreMemoire().apply { ecrire("j") }
        val s = FauxServeur(mapOf("GET /messages/fils" to ok("[]")))
        val cache = CacheMemoire()
        val l = LecteurHorsLigne(client(s, coffre), cache, { "u1|e1" }, { 42L })
        assertFalse(l.lire(Chemins.FILS, Chemins.listeFils).horsLigne)
        s.horsLigne = true
        val hl = l.lire(Chemins.FILS, Chemins.listeFils)
        assertTrue(hl.horsLigne)
        assertEquals(42L, hl.horodatageMs)
        // Autre utilisateur : pas de lecture du cache d'autrui.
        val autre = LecteurHorsLigne(client(s, coffre), cache, { "u2|e1" })
        assertFailsWith<ErreurApi.HorsLigne> { autre.lire(Chemins.FILS, Chemins.listeFils) }
    }

    @Test
    fun deconnexion_hors_ligne_oublie_quand_meme() = runTest {
        val s = FauxServeur(emptyMap()).apply { horsLigne = true }
        val coffre = CoffreMemoire().apply { ecrire("j") }
        val svc = ServiceConnexion(client(s, coffre), coffre, CacheMemoire(), FauxPasskey())
        svc.deconnexion()
        assertNull(coffre.lire())
    }
}
