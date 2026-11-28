package fr.scolia.mobile.core

import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** Signaler un incident (registre des violations, socle 27, complément G5) : écran du personnel, règles du serveur. */
class SignalementTest {
    private fun me(role: String) = Me("u", "x@y.fr", "e", role, null, 1, "complete", "passkey", "mobile", false)

    @Test
    fun module_pour_le_personnel_qui_a_le_droit_de_declarer() {
        val prof = Capacites.espacePour(me("prof"), listOf(Droit("absences.lire", "mes_classes"), Droit("violations.declarer", "etablissement")))
        assertTrue(prof.contient(Module.SIGNALER))
        assertEquals(Module.PROFIL, prof.modules.last(), "« Mon compte » reste en dernier")
        assertFalse(prof.onglets.contains(Module.SIGNALER), "rangé dans « Plus » : l'appel et les messages d'abord")
        assertFalse(Capacites.espacePour(me("prof"), listOf(Droit("absences.lire", "mes_classes"))).contient(Module.SIGNALER))
        // Élève et famille : guichet des droits, jamais ce module (le serveur refuserait de toute façon : 403).
        assertFalse(Capacites.espacePour(me("parent"), emptyList()).contient(Module.SIGNALER))
        assertFalse(Capacites.espacePour(me("eleve"), emptyList()).contient(Module.SIGNALER))
    }

    @Test
    fun memes_bornes_que_le_serveur() {
        assertTrue(Signalement.erreurIntitule("  ") != null)
        assertTrue(Signalement.erreurIntitule("ab") != null)
        assertNull(Signalement.erreurIntitule("Clé USB perdue"))
        assertTrue(Signalement.erreurIntitule("x".repeat(201)) != null)
        assertNull(Signalement.erreurDescription(""))
        assertTrue(Signalement.erreurDescription("ab") != null)
        assertTrue(Signalement.erreurDescription("x".repeat(4001)) != null)
        // Le serveur compte des caractères Unicode : 200 emoji (400 unités UTF-16) restent recevables.
        assertNull(Signalement.erreurIntitule("🔑".repeat(200)))
        assertTrue(Signalement.erreurIntitule("🔑".repeat(201)) != null)
    }

    @Test
    fun caracteres_de_controle_changes_comme_le_serveur_les_attend() {
        assertEquals("a b\nc\nd", Signalement.net(" a\tb\r\nc\rd "))
        assertEquals("Clé b", Signalement.corps("Clé\u0007b", "", false).intitule)
    }

    @Test
    fun corps_sans_precision_vide_et_intention_stable() {
        val c = Signalement.corps("  Clé USB perdue ", "  ", true)
        assertEquals(DeclarationIncidentBody("Clé USB perdue", null, true), c)
        val json = Json { explicitNulls = false; encodeDefaults = true }
        val o = json.parseToJsonElement(json.encodeToString(DeclarationIncidentBody.serializer(), c)).jsonObject
        assertEquals(setOf("intitule", "incident_securite"), o.keys, "précisions vides : non envoyées")
        assertEquals(Signalement.intention(c), Signalement.intention(Signalement.corps("Clé USB perdue", "", true)))
        assertNotEquals(Signalement.intention(c), Signalement.intention(Signalement.corps("Clé USB perdue", "", false)))
        assertEquals("Traitée", Signalement.libelleAvancement("close"))
    }

    @Test
    fun declarer_porte_une_cle_d_idempotence_et_lit_la_reference() = runTest {
        val s = FauxServeur(
            mapOf(
                "POST /rgpd/violations/declarations" to ReponseHttp(
                    201,
                    """{"id":"v1","reference":"V-2027-003","intitule":"Clé USB perdue","connue_le":"2027-03-01T08:00:00Z",
                       "avancement":"recue","message":"Merci"}""",
                ),
            ),
        )
        val c = ApiClient(FauxServeur.BASE, s, { "jeton" }, appli = "android/1.0.0 (build 1)", nouvelleCle = { "cle-1" })
        val d = c.declarerIncident(Signalement.corps("Clé USB perdue", "", false))
        assertEquals("V-2027-003", d.reference)
        assertEquals("cle-1", s.recues.single().entetes["Idempotency-Key"])
    }
}
