package fr.scolia.mobile.app

import android.app.Application
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.core.app.ApplicationProvider
import fr.scolia.mobile.app.ecrans.SectionMesAutresComptes
import fr.scolia.mobile.app.ui.ThemeScolIA
import fr.scolia.mobile.core.ApiClient
import fr.scolia.mobile.core.CacheMemoire
import fr.scolia.mobile.core.Capacites
import fr.scolia.mobile.core.EtatConnexion
import fr.scolia.mobile.core.Individu
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

/** Serveur factice de « Mes autres comptes » : statut et corps par « MÉTHODE /chemin », journal des écritures. */
private class FauxIndividu(var moi: String) : Transport {
    val ecritures = mutableListOf<String>()
    override suspend fun executer(requete: RequeteHttp): ReponseHttp {
        val chemin = "/" + requete.url.substringAfter("://").substringAfter('/')
        return when {
            requete.methode == "GET" && chemin == "/dossier/individu/moi" -> ReponseHttp(200, moi)
            requete.methode == "POST" && chemin == "/dossier/individu/declarations" -> {
                ecritures += "declarer ${requete.corps}"
                ReponseHttp(202, "")
            }
            requete.methode == "POST" && chemin.endsWith("/reponse") -> {
                ecritures += "repondre $chemin ${requete.corps}"
                moi = SANS_DEMANDE
                ReponseHttp(200, if (requete.corps!!.contains("true")) "\"11111111-1111-4111-8111-111111111111\"" else "null")
            }
            else -> ReponseHttp(404, """{"error":"inconnu"}""")
        }
    }
}

private const val SANS_DEMANDE = """{"autres_comptes":0,"fiches_reliees":1,"declarations_recues":[],"liens":[]}"""
private const val AVEC_DEMANDE = """{"autres_comptes":0,"fiches_reliees":1,"declarations_recues":[{"id":"d1",
 "declarant":"Anne Martin","declare_le":"2026-11-27T08:00:00Z","expire_le":"2026-11-28T08:00:00Z"}],"liens":[]}"""

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class MesAutresComptesComposeTest {
    @get:Rule val rule = createComposeRule()
    private lateinit var modele: AppModele
    private val rattachement = Rattachement("https://lpnb.scolia.fr", "lpnb", "Lycée Pierre Nord")

    @Before
    fun prepare() {
        modele = AppModele(ApplicationProvider.getApplicationContext())
    }

    private fun afficher(api: FauxIndividu) {
        val client = ApiClient(rattachement.baseUrl, api, { "jeton-test" })
        val m = Me("u1", "x@lpnb.fr", "e", "prof", null, 1, "complete", "passkey", "mobile", false)
        modele.brancherPourTest(rattachement, client, LecteurHorsLigne(client, CacheMemoire(), { "t" }), EtatConnexion.Connecte(m, Capacites.espacePour(m, emptyList())))
        rule.setContent { ThemeScolIA { SectionMesAutresComptes(modele) } }
    }

    private fun attendreTexte(t: String) = rule.waitUntil(5_000) { rule.onAllNodesWithText(t, substring = true).fetchSemanticsNodes().isNotEmpty() }

    @Test
    fun confirmer_demande_une_confirmation_qui_rappelle_l_effet() {
        val api = FauxIndividu(AVEC_DEMANDE)
        afficher(api)
        attendreTexte("Compte « Anne Martin »")
        rule.onNodeWithContentDescription("C'est moi : le compte Anne Martin est le mien").performClick()
        rule.onNodeWithText("Confirmer votre autre compte").assertIsDisplayed()
        assertTrue(api.ecritures.isEmpty(), "rien n'est envoyé avant la confirmation")
        rule.onNodeWithText("Oui, c'est mon compte").performClick()
        rule.waitUntil(5_000) { api.ecritures.isNotEmpty() }
        assertEquals(1, api.ecritures.size)
        assertTrue(api.ecritures[0].contains("/dossier/individu/declarations/d1/reponse") && api.ecritures[0].contains("true"))
        attendreTexte(Individu.messageReponse(true))
    }

    @Test
    fun refuser_demande_aussi_une_confirmation() {
        val api = FauxIndividu(AVEC_DEMANDE)
        afficher(api)
        attendreTexte("Compte « Anne Martin »")
        rule.onNodeWithContentDescription("Ce n'est pas moi : refuser la demande du compte Anne Martin").performClick()
        rule.onNodeWithText("Refuser cette demande ?").assertIsDisplayed()
        rule.onNodeWithText("Annuler").performClick()
        assertTrue(api.ecritures.isEmpty())
        rule.onNodeWithContentDescription("Ce n'est pas moi : refuser la demande du compte Anne Martin").performClick()
        rule.onNodeWithText("Refuser").performClick()
        rule.waitUntil(5_000) { api.ecritures.isNotEmpty() }
        assertTrue(api.ecritures[0].contains("false"))
        attendreTexte(Individu.messageReponse(false))
    }

    @Test
    fun declarer_controle_l_adresse_puis_affiche_la_reponse_neutre() {
        val api = FauxIndividu(SANS_DEMANDE)
        afficher(api)
        attendreTexte("Aucun autre compte")
        rule.onNodeWithTag("individu-ouvrir-declarer").performClick()
        rule.onNodeWithTag("individu-email").performTextInput("pas-une-adresse")
        rule.onNodeWithTag("individu-declarer").performClick()
        attendreTexte("Cette adresse e-mail n'est pas valide.")
        assertTrue(api.ecritures.isEmpty(), "contrôle local avant tout envoi")
        rule.onNodeWithTag("individu-email").performTextInput("@lpnb.fr")
        rule.onNodeWithTag("individu-declarer").performClick()
        rule.waitUntil(5_000) { api.ecritures.isNotEmpty() }
        assertTrue(api.ecritures[0].contains("pas-une-adresse@lpnb.fr"))
        attendreTexte("Demande enregistrée.")
    }
}
