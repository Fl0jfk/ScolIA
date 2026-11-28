package fr.scolia.mobile.app

import android.app.Application
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextClearance
import androidx.compose.ui.test.performTextInput

import androidx.test.core.app.ApplicationProvider
import fr.scolia.mobile.app.ui.RacineScolIA
import fr.scolia.mobile.app.ui.ThemeScolIA
import fr.scolia.mobile.core.ApiClient
import fr.scolia.mobile.core.CacheMemoire
import fr.scolia.mobile.core.Capacites
import fr.scolia.mobile.core.Droit
import fr.scolia.mobile.core.EtatConnexion
import fr.scolia.mobile.core.LecteurHorsLigne
import fr.scolia.mobile.core.Me
import fr.scolia.mobile.core.Rattachement
import fr.scolia.mobile.core.ReponseHttp
import fr.scolia.mobile.core.RequeteHttp
import fr.scolia.mobile.core.Transport
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/** Serveur factice : réponses par « MÉTHODE /chemin » (sans la requête), journal des appels. */
private class FauxApi(private val reponses: Map<String, String>) : Transport {
    val recues = mutableListOf<String>()
    override suspend fun executer(requete: RequeteHttp): ReponseHttp {
        val chemin = requete.url.substringAfter("://").substringAfter('/').let { "/" + it }.substringBefore('?')
        val cle = "${requete.methode} $chemin"
        recues += cle
        return reponses[cle]?.let { ReponseHttp(200, it) } ?: ReponseHttp(404, """{"error":"inconnu"}""")
    }
}

private const val CRENEAU = """[{"id":"c1","classe":"5A","matiere_libelle":"Maths","prof_user_id":"u1","jour_semaine":1,
 "heure_debut":"08:00:00","heure_fin":"09:00:00","salle":"B12","semaine_ab":""}]"""
private fun appel(statut: String, etat: String) = """{"id":"a1","classe":"5A","jour":"2026-11-06","statut":"$statut",
 "creneau_id":"c1","matiere_libelle":"Maths","lignes":[{"eleve_id":"e1","eleve_label":"Léa Martin","etat":"$etat"}]}"""

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class EcransComposeTest {
    @get:Rule val rule = createComposeRule()
    private lateinit var modele: AppModele
    private val rattachement = Rattachement("https://lpnb.scolia.fr", "lpnb", "Lycée Pierre Nord")

    @Before
    fun prepare() {
        modele = AppModele(ApplicationProvider.getApplicationContext())
    }

    private fun me(role: String, eleveId: String? = null, portee: String = "complete") =
        Me("u1", "x@lpnb.fr", "e", role, eleveId, 1, portee, "passkey", "mobile", false)

    private fun connecter(api: FauxApi, role: String, droits: List<Droit>, eleveId: String? = null, portee: String = "complete") {
        val client = ApiClient(rattachement.baseUrl, api, { "jeton-test" })
        val lecteur = LecteurHorsLigne(client, CacheMemoire(), { "test" })
        val m = me(role, eleveId, portee)
        modele.brancherPourTest(rattachement, client, lecteur, EtatConnexion.Connecte(m, Capacites.espacePour(m, droits)))
    }

    private fun afficher() = rule.setContent { ThemeScolIA { RacineScolIA(modele, onDeverrouiller = {}) } }

    @Test
    fun rattachement_refuse_une_adresse_hors_scolia() {
        afficher()
        rule.onNodeWithText("Rejoindre mon établissement").assertIsDisplayed()
        rule.onNodeWithText("Adresse de l'établissement").performTextInput("http://pirate.example.com")
        rule.onNodeWithText("Continuer").performClick()
        rule.waitUntil(5_000) { modele.erreur != null }
        assertTrue(modele.actif == null, "aucun établissement ne doit être mémorisé")
    }

    @Test
    fun connexion_propose_la_passkey_d_abord_et_le_mot_de_passe_en_secours() {
        modele.brancherPourTest(
            rattachement, ApiClient(rattachement.baseUrl, FauxApi(emptyMap())),
            LecteurHorsLigne(ApiClient(rattachement.baseUrl, FauxApi(emptyMap())), CacheMemoire(), { "t" }),
            EtatConnexion.Deconnecte,
        )
        afficher()
        rule.onNodeWithText("Lycée Pierre Nord").assertIsDisplayed()
        rule.onNodeWithText("Se connecter avec une passkey").assertIsDisplayed()
        rule.onNodeWithText("Mot de passe").assertDoesNotExist()
        rule.onNodeWithText("Pas encore de passkey ? Utiliser mon mot de passe").performClick()
        rule.onNodeWithText("Mot de passe").assertIsDisplayed()
    }

    @Test
    fun verrou_affiche_avant_les_donnees() {
        connecter(FauxApi(emptyMap()), "prof", emptyList())
        modele.verrouille = true
        afficher()
        rule.onNodeWithText("ScolIA est verrouillée").assertIsDisplayed()
        rule.onNodeWithText("Accueil").assertDoesNotExist()
    }

    @Test
    fun enrolement_exige_la_passkey_et_masque_le_reste() {
        connecter(FauxApi(emptyMap()), "prof", emptyList(), portee = "enrolement")
        modele.verrouille = false
        afficher()
        rule.onNodeWithText("Créer votre passkey").assertIsDisplayed()
        rule.onNodeWithTag("module-APPEL").assertDoesNotExist()
    }

    @Test
    fun navigation_du_professeur_selon_ses_droits() {
        val api = FauxApi(mapOf("GET /edt/mes-creneaux" to CRENEAU, "GET /annonces" to "[]"))
        connecter(api, "prof", listOf(Droit("absences.lire", "mes_classes"), Droit("notes.saisir", "mes_classes")))
        modele.verrouille = false
        afficher()
        rule.onNodeWithTag("module-APPEL").assertIsDisplayed()
        rule.onNodeWithTag("module-ABSENCES").assertDoesNotExist()
        rule.onNodeWithTag("module-VIE_SCOLAIRE").assertDoesNotExist()
        // « Plus » : cahier de textes (mes classes) et saisie de notes par la grille `/eva/*` (palier « notes eva »).
        rule.onNodeWithText("Plus").performClick()
        rule.onNodeWithTag("module-AGENDA").assertIsDisplayed()
        rule.onNodeWithTag("module-SAISIE_NOTES").assertIsDisplayed()
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("Maths") > 0 }
        assertTrue("GET /edt/mes-creneaux" in api.recues)
    }

    @Test
    fun navigation_de_la_famille_sans_appel() {
        val api = FauxApi(mapOf("GET /annonces" to "[]"))
        connecter(api, "parent", emptyList())
        modele.verrouille = false
        afficher()
        rule.onNodeWithTag("module-ABSENCES").assertIsDisplayed()
        rule.onNodeWithTag("module-APPEL").assertDoesNotExist()
        rule.onNodeWithTag("module-SAISIE_NOTES").assertDoesNotExist()
    }

    @Test
    fun un_role_inconnu_recoit_les_ecrans_de_ses_droits() {
        val api = FauxApi(mapOf("GET /annonces" to "[]", "GET /absences/a-traiter" to "[]"))
        connecter(api, "role_invente_par_l_etablissement", listOf(Droit("appel.verifier", "etablissement")))
        modele.verrouille = false
        afficher()
        rule.onNodeWithTag("module-VIE_SCOLAIRE").assertIsDisplayed()
        rule.onNodeWithTag("module-APPEL").assertDoesNotExist()
    }

    @Test
    fun appel_marquer_absent_puis_cloture_avec_confirmation() {
        val api = FauxApi(
            mapOf(
                "GET /edt/mes-creneaux" to CRENEAU,
                "GET /annonces" to "[]",
                "POST /appels" to appel("ouvert", "present"),
                "POST /appels/a1/lignes" to "",
                "POST /appels/a1/clore" to appel("clos", "absent"),
            ),
        )
        connecter(api, "prof", listOf(Droit("absences.lire", "mes_classes")))
        modele.verrouille = false
        afficher()
        rule.onNodeWithTag("module-APPEL").performClick()
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("Faire l'appel") > 0 }
        rule.onNodeWithText("Faire l'appel").performClick()
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("Léa Martin") > 0 }
        rule.onNodeWithContentDescription("Léa Martin : Absent").performClick()
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("1 élèves · 1 absent(s) · 0 retard(s)") > 0 }
        // Action sensible : rien n'est clos tant que l'on n'a pas confirmé.
        rule.onNodeWithText("Valider l'appel").performClick()
        rule.onNodeWithText("Valider l'appel ?").assertIsDisplayed()
        assertTrue(api.recues.none { it.endsWith("/clore") })
        rule.onNodeWithText("Valider").performClick()
        rule.waitUntil(5_000) { api.recues.any { it.endsWith("/clore") } }
        assertEquals(1, api.recues.count { it.endsWith("/clore") })
    }

    @Test
    fun saisie_de_notes_par_la_grille_puis_publication_confirmee() {
        val api = FauxApi(
            mapOf(
                "GET /edt/mes-creneaux" to "[]",
                "GET /annonces" to "[]",
                "GET /eva/enseignements" to ENSEIGNEMENTS,
                "GET /eva/cours/c1/periodes" to PERIODES,
                "GET /eva/grille" to GRILLE,
                "POST /eva/saisie" to """{"cellules":[{"evaluation_id":"ev1","eleve_id":"e1","issue":"ecrit","version":1,"statut":"note","valeur_m":15500}]}""",
                "POST /eva/evaluations/ev1/publication" to EVALUATION.replace("\"publiee_le\":null", "\"publiee_le\":\"2027-03-08T10:00:00Z\""),
            ),
        )
        connecter(api, "prof", listOf(Droit("notes.saisir", "mes_classes")))
        modele.verrouille = false
        afficher()
        // Cinq modules : « Saisir des notes » est rangé dans « Plus » (barre limitée).
        if (rule.onAllNodesWithTag("module-SAISIE_NOTES").fetchSemanticsNodes().isEmpty()) rule.onNodeWithText("Plus").performClick()
        rule.onNodeWithTag("module-SAISIE_NOTES").performClick()
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("Maths 4B") > 0 }
        rule.onNodeWithText("Maths 4B").performClick()
        // La période du jour s'ouvre d'elle-même, puis la grille.
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("Interrogation") > 0 }
        rule.onNodeWithText("Interrogation").performClick()
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("Martin Léa") > 0 }
        // Hors barème : compris localement comme une erreur, rien ne part.
        rule.onNodeWithContentDescription("Note de Martin Léa").performTextInput("25")
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("1 case(s) non comprise(s) : corrigez-les avant d'enregistrer.") > 0 }
        rule.onNodeWithTag("eva-enregistrer").performClick()
        assertTrue(api.recues.none { it == "POST /eva/saisie" })
        rule.onNodeWithContentDescription("Note de Martin Léa").performTextClearance()
        rule.onNodeWithContentDescription("Note de Martin Léa").performTextInput("15,5")
        rule.onNodeWithTag("eva-enregistrer").performClick()
        rule.waitUntil(5_000) { rule.onAllNodesWithTextCount("1 case(s) enregistrée(s).") > 0 }
        assertEquals(1, api.recues.count { it == "POST /eva/saisie" })
        // Publication : action sensible, rien ne part sans confirmation.
        rule.onNodeWithText("Publier").performClick()
        rule.onNodeWithText("Publier les notes ?").assertIsDisplayed()
        assertTrue(api.recues.none { it.endsWith("/publication") })
        rule.onNodeWithTag("eva-publication-confirmer").performClick()
        rule.waitUntil(5_000) { api.recues.any { it.endsWith("/publication") } }
    }
}

private const val ENSEIGNEMENTS = """[{"cours_id":"c1","libelle":"Maths 4B","cible_code":"4B","matiere_id":null,"sans_matiere":false,"droit":"saisie"},
 {"cours_id":"c2","libelle":"Latin (lecture)","cible_code":"3A","matiere_id":null,"sans_matiere":true,"droit":"lecture"}]"""
private const val PERIODES = """[{"id":"p1","code":"T1","libelle":"Toute l'année","date_debut":"2000-01-01","date_fin":"2999-12-31"}]"""
private const val EVALUATION = """{"id":"ev1","titre":"Interrogation","date_eval":"2027-03-08","bareme_m":20000,"coef_m":1000,
 "comptage":"normal","hors_bareme_autorise":false,"etat":"ouverte","motif_annulation":null,"version":1,"publication":"explicite",
 "publication_prevue_le":null,"publiee_le":null,"retiree_le":null,"en_cours_de_correction":false,"sous_matiere_id":null,
 "restreinte":false,"commune":false,"annoncee":false,"a_des_resultats":false,"peut_modifier":true,"retrait_possible":false,"stats":null}"""
private val GRILLE = """{"enseignement":{"cours_id":"c1","libelle":"Maths 4B","cible_code":"4B","matiere_id":null,"sans_matiere":false,"droit":"saisie"},
 "periode":{"id":"p1","code":"T1","libelle":"Toute l'année","date_debut":"2000-01-01","date_fin":"2999-12-31"},"jour":"2027-03-08",
 "peut_saisir":true,"peut_corriger":false,"echelle_m":20000,"pas_m":10,"delai_publication_max_jours":null,
 "evaluations":[$EVALUATION],
 "eleves":[{"eleve_id":"e1","nom":"Martin","prenom":"Léa","mouvement":null,"moyenne_m":null,"nb_comptees":0,
   "cellules":[{"concerne":true,"statut":null,"valeur_m":null,"version":null,"resultat_id":null,"saisie_ouverte":true,
     "rattrapage_autorise":false,"hors_periode":false,"corrige":false,"commentaire":false}]}],
 "moyenne_classe":null,"avertissements":[]}"""
private fun androidx.compose.ui.test.junit4.ComposeContentTestRule.onAllNodesWithTextCount(t: String): Int =
    onAllNodesWithText(t).fetchSemanticsNodes().size
