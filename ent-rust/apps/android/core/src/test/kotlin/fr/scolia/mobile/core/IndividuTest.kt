package fr.scolia.mobile.core

import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import java.time.ZoneId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * « Mes autres comptes » (transverse individu) : décodage des réponses **réelles** du serveur (fixtures
 * `contrat/mobile/individu/`, produites par `ent-web-server/tests/individu_contrat_mobile.rs`), requêtes émises
 * (chemin, corps, clé d'idempotence), liste blanche, textes et contrôles locaux identiques au web.
 */
class IndividuTest {
    private val dossier: File = File(System.getProperty("scolia.contrat") ?: "../../../contrat/mobile", "individu").canonicalFile

    private fun fixture(nom: String): String = File(dossier, "$nom.json").readText()

    private fun client(s: FauxServeur) = ApiClient(FauxServeur.BASE, s, jeton = { "jeton" }, nouvelleCle = { "cle-neuve" })

    @Test
    fun moi_decode_la_reponse_reelle_avec_liens_et_demande() = runTest {
        val s = FauxServeur(mapOf("GET /dossier/individu/moi" to ReponseHttp(200, fixture("moi"))))
        val m = client(s).individuMoi()
        assertTrue(m.autresComptes >= 1)
        assertTrue(m.liens.isNotEmpty(), "la fixture porte au moins un lien")
        assertTrue(m.declarationsRecues.isNotEmpty(), "la fixture porte au moins une demande reçue")
        val d = m.declarationsRecues.first()
        assertTrue(d.declarant.isNotBlank())
        assertNotNull(Individu.dateHeure(d.expireLe).takeIf { it.contains(" à ") }, "date lisible : ${d.expireLe}")
        // Jamais l'autre fiche : le lien vu par la personne n'a que trois champs.
        val brut = Json.parseToJsonElement(fixture("moi")).jsonObject
        val lien = (brut["liens"] as kotlinx.serialization.json.JsonArray).first().jsonObject
        assertEquals(setOf("origine", "motif_code", "pose_le"), lien.keys)
    }

    @Test
    fun moi_d_un_serveur_sans_liens_reste_lisible() = runTest {
        val s = FauxServeur(mapOf("GET /dossier/individu/moi" to ReponseHttp(200, """{"autres_comptes":0,"fiches_reliees":1,"declarations_recues":[]}""")))
        val m = client(s).individuMoi()
        assertEquals(emptyList(), m.liens)
        assertEquals("Aucun autre compte n'est reconnu comme le vôtre dans cet établissement.", Individu.resumeMoi(m))
        assertNull(Individu.resumeFiches(m))
    }

    @Test
    fun declarer_envoie_l_adresse_et_accepte_le_202_vide() = runTest {
        val s = FauxServeur(mapOf("POST /dossier/individu/declarations" to ReponseHttp(202, "")))
        client(s).declarerAutreCompte("autre@ent.test", cle = "k1")
        val r = s.recues.single()
        assertEquals("k1", r.entetes["Idempotency-Key"])
        assertEquals("autre@ent.test", Json.parseToJsonElement(r.corps!!).jsonObject["email"]!!.jsonPrimitive.content)
    }

    @Test
    fun repondre_rend_le_lien_ou_rien() = runTest {
        val s = FauxServeur(
            mapOf(
                "POST /dossier/individu/declarations/d%201/reponse" to ReponseHttp(200, fixture("reponse_confirmer")),
                "POST /dossier/individu/declarations/d2/reponse" to ReponseHttp(200, fixture("reponse_refuser")),
            ),
        )
        val c = client(s)
        assertNotNull(c.repondreDeclaration("d 1", true))
        assertNull(c.repondreDeclaration("d2", false))
        assertTrue(Json.parseToJsonElement(s.recues[0].corps!!).jsonObject["confirmer"]!!.jsonPrimitive.boolean)
        assertFalse(Json.parseToJsonElement(s.recues[1].corps!!).jsonObject["confirmer"]!!.jsonPrimitive.boolean)
        assertEquals("cle-neuve", s.recues[1].entetes["Idempotency-Key"])
    }

    @Test
    fun un_refus_du_serveur_reste_un_refus() = runTest {
        val s = FauxServeur(mapOf("POST /dossier/individu/declarations" to ReponseHttp(429, """{"erreur":{"code":"quota","message":"Trop de demandes aujourd'hui."}}""")))
        val e = assertFailsWith<ErreurApi> { client(s).declarerAutreCompte("a@b.fr") }
        assertEquals("Trop de demandes aujourd'hui.", e.message)
    }

    @Test
    fun chaque_requete_est_dans_la_liste_blanche_individu() = runTest {
        val gabarits = File(dossier, "routes.txt").readLines().map { it.trim() }.filter { it.isNotEmpty() && !it.startsWith("#") }
        val regex = gabarits.map { g ->
            val (m, chemin) = g.split(" ", limit = 2)
            Regex("^$m " + chemin.split("/").joinToString("/") { seg ->
                if (seg.startsWith("{") && seg.endsWith("}")) "[^/?]+" else Regex.escape(seg)
            } + "$")
        }
        val s = FauxServeur(emptyMap())
        val c = client(s)
        runCatching { c.individuMoi() }
        runCatching { c.declarerAutreCompte("a@b.fr") }
        runCatching { c.repondreDeclaration("abc", true) }
        assertEquals(3, s.recues.size)
        for (r in s.recues) {
            val ligne = "${r.methode} ${r.url.removePrefix(FauxServeur.BASE).substringBefore('?')}"
            assertTrue(regex.any { it.matches(ligne) }, "$ligne hors de contrat/mobile/individu/routes.txt")
        }
    }

    @Test
    fun adresse_controlee_comme_sur_le_web() {
        assertEquals("a@b.fr", Individu.adresseAEnvoyer("  A@B.fr ").getOrThrow())
        assertEquals("Saisissez l'adresse e-mail de votre autre compte.", Individu.adresseAEnvoyer(" ").exceptionOrNull()?.message)
        for (faux in listOf("ab.fr", "@b.fr", "a@", "a b@c.fr", "x".repeat(250) + "@b.fr")) {
            assertEquals("Cette adresse e-mail n'est pas valide.", Individu.adresseAEnvoyer(faux).exceptionOrNull()?.message, faux)
        }
    }

    @Test
    fun libelles_de_chaque_motif_et_origine_du_serveur() {
        // Copie de `ent-types::individu::MOTIFS_LIEN` et des origines de la migration 279.
        for (m in listOf("deux_comptes", "fiche_sans_compte", "doublon_agent", "autre")) {
            assertTrue(Individu.libelleMotifLien(m) != "Motif non listé", m)
        }
        for (o in listOf("declaration_direction", "declaration_confirmee", "proposition_acceptee")) {
            assertTrue(Individu.libelleOrigine(o) != "Lien enregistré", o)
        }
        assertEquals("Motif non listé", Individu.libelleMotifLien("nouveau"))
    }

    @Test
    fun resumes_et_tris() {
        val m = MonIndividu(
            autresComptes = 2, fichesReliees = 3,
            declarationsRecues = listOf(
                DeclarationRecue("b", "Paul", "2026-11-27T08:00:00Z", "2026-11-28T08:00:00Z"),
                DeclarationRecue("a", "Anne", "2026-11-27T09:00:00Z", "2026-11-28T07:00:00Z"),
            ),
            liens = listOf(
                MonLienIndividu("declaration_direction", "deux_comptes", "2026-10-01T10:00:00Z"),
                MonLienIndividu("declaration_confirmee", "deux_comptes", "2026-11-01T10:00:00Z"),
            ),
        )
        assertTrue(Individu.resumeMoi(m).startsWith("2 autres comptes"))
        assertTrue(Individu.resumeMoi(m.copy(autresComptes = 1)).startsWith("1 autre compte "))
        assertTrue(Individu.resumeFiches(m)!!.startsWith("3 fiches"))
        assertEquals("declaration_confirmee", Individu.trierLiens(m.liens).first().origine)
        assertEquals("a", Individu.trierDemandes(m.declarationsRecues).first().id)
        val paris = ZoneId.of("Europe/Paris")
        assertEquals("28/11/2026 à 08h00", Individu.dateHeure("2026-11-28T07:00:00Z", paris))
        assertEquals("28/11/2026 à 08h00", Individu.dateHeure("2026-11-28T07:00:00.123456Z", paris))
        assertEquals("pas une date", Individu.dateHeure("pas une date"))
        assertEquals(
            "Demandé le 27/11/2026 à 10h00 · à confirmer avant le 28/11/2026 à 08h00",
            Individu.detailDemande(m.declarationsRecues[1], paris),
        )
    }

    @Test
    fun intentions_distinctes_par_demande_et_par_reponse() {
        assertTrue(Individu.intentionRepondre("d1", true) != Individu.intentionRepondre("d1", false))
        assertTrue(Individu.intentionDeclarer("a@b.fr") != Individu.intentionDeclarer("c@b.fr"))
    }
}
