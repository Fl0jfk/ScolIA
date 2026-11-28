package fr.scolia.mobile.core

import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail

/** Palier « notes eva » : analyse d'une case identique au serveur, brouillon de saisie, envoi par lot. */
class EvaluationsTest {
    private val dossier: File = File(System.getProperty("scolia.contrat") ?: "../../../contrat/mobile").canonicalFile
    private val lecteur = Json { ignoreUnknownKeys = true }

    @Test
    fun analyse_identique_aux_vecteurs_du_serveur() {
        val doc = lecteur.parseToJsonElement(File(dossier, "regles/analyse_cellule.json").readText()).jsonObject
        val ecarts = mutableListOf<String>()
        val cas = doc.getValue("analyse").jsonArray
        assertTrue(cas.size > 100, "trop peu de vecteurs : ${cas.size}")
        for (v in cas) {
            val o = v.jsonObject
            val texte = o.getValue("texte").jsonPrimitive.content
            val a = SaisieNotes.analyser(
                texte,
                o.getValue("bareme_m").jsonPrimitive.long,
                o.getValue("hors_bareme_autorise").jsonPrimitive.boolean,
                o.getValue("pas_m").jsonPrimitive.long,
            )
            val att = o.getValue("attendu").jsonObject
            val attendu = when (att.getValue("type").jsonPrimitive.content) {
                "vide" -> AnalyseCellule.Vide
                "statut" -> AnalyseCellule.Statut(att.getValue("statut").jsonPrimitive.content)
                "note" -> AnalyseCellule.Note(att.getValue("valeur_m").jsonPrimitive.long)
                else -> AnalyseCellule.Erreur(att.getValue("message").jsonPrimitive.content)
            }
            if (a != attendu) ecarts += "« $texte » ($o) : $a au lieu de $attendu"
        }
        for (v in doc.getValue("format").jsonArray) {
            val o = v.jsonObject
            val m = o.getValue("valeur_m").jsonPrimitive.long
            val t = o.getValue("texte").jsonPrimitive.content
            if (SaisieNotes.formater(m) != t) ecarts += "format $m : ${SaisieNotes.formater(m)} au lieu de $t"
        }
        if (ecarts.isNotEmpty()) fail("analyse divergente du serveur :\n  " + ecarts.joinToString("\n  "))
    }

    private fun cellule(statut: String? = null, valeur: Long? = null, version: Int? = null, ouverte: Boolean = true, concerne: Boolean = true) =
        CelluleEva(concerne, statut, valeur, version, null, ouverte, false, false, false, false)

    private fun evaluation(id: String, etat: String = "ouverte", publiee: String? = null) = EvaluationEva(
        id, "Interro", "2027-03-08", 20_000, 1_000, "normal", false, etat, null, 1, "explicite", null, publiee, null,
        false, null, false, false, false, false, true, false, null,
    )

    private fun grille(vararg lignes: Pair<String, CelluleEva>, peutSaisir: Boolean = true, etat: String = "ouverte") = GrilleEva(
        EnseignementEva("c1", "Maths 4B", "4B", null, false, "saisie"),
        PeriodeEva("p1", "T1", "Trimestre 1", "2027-01-01", "2027-03-31"),
        "2027-03-08", peutSaisir, false, 20_000, 10, null,
        listOf(evaluation("ev0"), evaluation("ev1", etat)),
        lignes.map { (id, c) -> LigneEleveEva(id, "Nom$id", "Prénom", null, listOf(cellule(), c), null, 0) },
        null, emptyList(),
    )

    @Test
    fun seules_les_cases_modifiees_comprises_et_ouvertes_partent() {
        val b = BrouillonSaisie(
            grille(
                "e1" to cellule(),
                "e2" to cellule("note", 12_000, 3),
                "e3" to cellule(ouverte = false),
                "e4" to cellule(concerne = false),
                "e5" to cellule("absent", null, 2),
            ),
            "ev1",
        )
        assertEquals(listOf("e1", "e2", "e3", "e5"), b.eleves.map { it.eleveId }) // non concerné : jamais montré
        assertEquals("12", b.texte("e2"))
        assertEquals("abs", b.texte("e5"))
        b.taper("e1", " 15,5 ")
        b.taper("e2", "12") // inchangé
        b.taper("e3", "10") // période arrêtée : ignoré
        b.taper("e5", "21") // hors barème : refusé localement, non envoyé
        assertFalse(b.modifiable("e3"))
        assertEquals(listOf(CelluleSaisieEva("ev1", "e1", "15,5", null)), b.aEnvoyer())
        assertEquals(listOf("e5"), b.enErreur())
        b.taper("e5", "")
        assertTrue(b.videeSansEffet("e5")) // une case vidée n'efface rien
        assertTrue(b.aEnvoyer().none { it.eleveId == "e5" })
        b.taper("e2", "nr")
        assertEquals(CelluleSaisieEva("ev1", "e2", "nr", 3), b.aEnvoyer().first { it.eleveId == "e2" })
    }

    @Test
    fun evaluation_annulee_ou_grille_en_lecture_ne_se_saisit_pas() {
        assertFalse(BrouillonSaisie(grille("e1" to cellule(), etat = "annulee"), "ev1").modifiable("e1"))
        val lecture = BrouillonSaisie(grille("e1" to cellule(), peutSaisir = false), "ev1")
        lecture.taper("e1", "12")
        assertTrue(lecture.aEnvoyer().isEmpty())
    }

    @Test
    fun issues_ecrit_conflit_et_refus() {
        val b = BrouillonSaisie(grille("e1" to cellule(), "e2" to cellule("note", 8_000, 1), "e3" to cellule()), "ev1")
        b.taper("e1", "15,5")
        b.taper("e2", "9")
        b.taper("e3", "abs")
        val envoye = b.lot()
        val n = b.appliquer(
            SaisieEvaRetour(
                listOf(
                    CelluleRetourEva("ev1", "e1", "ecrit", 1, "note", 15_500),
                    CelluleRetourEva("ev1", "e2", "conflit", conflit = ConflitEva("note", 11_000, 2, "Mme Autre")),
                    CelluleRetourEva("ev1", "e3", "refuse", message = "période arrêtée"),
                ),
            ),
            envoye,
        )
        assertEquals(1, n)
        assertEquals("15,5", b.texte("e1"))
        assertFalse(b.modifiee("e1"))
        assertEquals(1, b.caseDe("e1")?.version)
        // Conflit : la frappe reste, mais ne repart pas sans un choix explicite.
        assertEquals(11_000, b.conflit("e2")?.valeurM)
        assertEquals("9", b.texte("e2"))
        assertEquals("période arrêtée", b.refus("e3"))
        assertEquals(listOf("e3"), b.aEnvoyer().map { it.eleveId })
        b.garderLaMienne("e2")
        assertEquals(CelluleSaisieEva("ev1", "e2", "9", 2), b.aEnvoyer().first { it.eleveId == "e2" })
        // L'autre choix : reprendre la valeur du serveur.
        val c = BrouillonSaisie(grille("e2" to cellule("note", 8_000, 1)), "ev1")
        c.taper("e2", "9")
        c.appliquer(SaisieEvaRetour(listOf(CelluleRetourEva("ev1", "e2", "conflit", conflit = ConflitEva("note", 11_000, 2)))), c.lot())
        c.reprendreServeur("e2")
        assertEquals("11", c.texte("e2"))
        assertNull(c.conflit("e2"))
        assertTrue(c.aEnvoyer().isEmpty())
    }

    @Test
    fun intention_du_lot_suit_son_contenu() {
        val b = BrouillonSaisie(grille("e1" to cellule(), "e2" to cellule("note", 8_000, 4)), "ev1")
        b.taper("e2", "9")
        b.taper("e1", "15")
        assertEquals("eva-saisie:ev1:e1=15@-;e2=9@4", b.intention(b.lot()))
        b.taper("e1", "16")
        assertEquals("eva-saisie:ev1:e1=16@-;e2=9@4", b.intention(b.lot()))
        assertTrue(b.intention(b.lot()).startsWith(b.famille()))
    }

    @Test
    fun frappe_pendant_l_envoi_gardee_et_conflit_sans_etat_bloque() {
        val b = BrouillonSaisie(grille("e1" to cellule(), "e2" to cellule("note", 8_000, 1)), "ev1")
        b.taper("e1", "12")
        b.taper("e2", "9")
        val envoye = b.lot()
        b.taper("e1", "14") // retapée pendant la requête
        b.appliquer(
            SaisieEvaRetour(
                listOf(
                    CelluleRetourEva("ev1", "e1", "ecrit", 1, "note", 12_000),
                    CelluleRetourEva("ev1", "e2", "conflit", message = "la case a changé depuis votre lecture"),
                ),
            ),
            envoye,
        )
        assertEquals("14", b.texte("e1"))
        assertEquals(listOf(CelluleSaisieEva("ev1", "e1", "14", 1)), b.aEnvoyer())
        assertEquals("la case a changé depuis votre lecture", b.refus("e2"))
        b.taper("e2", "10")
        assertTrue(b.aEnvoyer().none { it.eleveId == "e2" }) // relire la grille d'abord
    }

    @Test
    fun attente_remplies_et_frappes_sauvegardees() {
        val b = BrouillonSaisie(grille("e1" to cellule(), "e2" to cellule("note", 8_000, 1)), "ev1")
        assertFalse(b.enAttente())
        assertEquals(1, b.nbRemplies())
        b.taper("e1", "25") // en erreur : en attente quand même
        assertTrue(b.enAttente())
        val c = BrouillonSaisie(b.grille, "ev1")
        c.reprendreFrappes(b.frappes())
        assertEquals("25", c.texte("e1"))
    }

    @Test
    fun grille_degeneree_sans_plantage() {
        val g = grille("e1" to cellule())
        val tronquee = g.copy(eleves = g.eleves + LigneEleveEva("e9", "X", "Y", null, emptyList(), null, 0))
        assertEquals(listOf("e1"), BrouillonSaisie(tronquee, "ev1").eleves.map { it.eleveId })
    }

    @Test
    fun lot_envoye_avec_la_cle_et_sans_version_nulle() = runTest {
        val recues = mutableListOf<RequeteHttp>()
        val c = ApiClient(FauxServeur.BASE, { r -> recues += r; ReponseHttp(200, """{"cellules":[]}""") }, { "jeton" }, appli = "android/1 (build 1)")
        c.saisirNotes(SaisieEvaBody("c1", listOf(CelluleSaisieEva("ev1", "e1", "12", null), CelluleSaisieEva("ev1", "e2", "abs", 3))), cle = "k1")
        val r = recues.single()
        assertEquals("k1", r.entetes["Idempotency-Key"])
        assertEquals(
            """{"cours_id":"c1","cellules":[{"evaluation_id":"ev1","eleve_id":"e1","texte":"12"},{"evaluation_id":"ev1","eleve_id":"e2","texte":"abs","version_attendue":3}]}""",
            r.corps,
        )
    }

    @Test
    fun periode_du_jour_et_libelles() {
        assertTrue(PeriodeEva("p", "T1", "T1", "2027-01-01", "2027-03-31").contient("2027-03-31"))
        assertFalse(PeriodeEva("p", "T1", "T1", "2027-01-01", "2027-03-31").contient("2027-04-01"))
        assertEquals("12,5 / 20", SaisieNotes.resume("note", 12_500, 20_000))
        assertEquals("Dispensé", SaisieNotes.resume("dispense", null, 20_000))
        assertEquals("vide", SaisieNotes.resume(null, null, 20_000))
    }
}
