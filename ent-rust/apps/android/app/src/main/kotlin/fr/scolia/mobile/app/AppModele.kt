package fr.scolia.mobile.app

import android.app.Activity
import android.app.Application
import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import fr.scolia.mobile.app.securite.CacheChiffre
import fr.scolia.mobile.app.securite.CoffreKeystore
import fr.scolia.mobile.app.securite.PasskeyAndroid
import fr.scolia.mobile.core.ApiClient
import fr.scolia.mobile.core.ErreurApi
import fr.scolia.mobile.core.EtatConnexion
import fr.scolia.mobile.core.LecteurHorsLigne
import fr.scolia.mobile.core.LecteurRattachement
import fr.scolia.mobile.core.PasskeyChallenge
import fr.scolia.mobile.core.Rattachement
import fr.scolia.mobile.core.ServiceConnexion
import fr.scolia.mobile.core.TransportOkHttp
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import java.lang.ref.WeakReference

/** Activité au premier plan (nécessaire à Credential Manager), sans fuite mémoire. */
object ActiviteCourante {
    private var ref: WeakReference<Activity>? = null
    fun definir(a: Activity?) {
        ref = a?.let { WeakReference(it) }
    }
    fun exiger(): Activity = ref?.get() ?: error("Aucune activité au premier plan.")
}

/** Établissements mémorisés (non secrets) et dernier e-mail utilisé (D-011 : e-mail mémorisé). */
private class Preferences(context: Context) {
    private val p = context.getSharedPreferences("scolia_prefs", Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true }

    @Serializable
    data class R(val baseUrl: String, val slug: String, val nom: String? = null, val etab: String? = null)

    fun rattachements(): List<Rattachement> = try {
        json.decodeFromString(ListSerializer(R.serializer()), p.getString("rattachements", "[]")!!)
            .map { Rattachement(it.baseUrl, it.slug, it.nom, null, it.etab) }
    } catch (e: Exception) {
        emptyList()
    }

    fun sauver(liste: List<Rattachement>) {
        val r = liste.map { R(it.baseUrl, it.slug, it.nom, it.etablissementIdLabo) }
        p.edit().putString("rattachements", json.encodeToString(ListSerializer(R.serializer()), r)).apply()
    }

    var actif: String?
        get() = p.getString("actif", null)
        set(v) = p.edit().putString("actif", v).apply()

    var dernierEmail: String?
        get() = p.getString("email", null)
        set(v) = p.edit().putString("email", v).apply()
}

/** État global de l'appli : établissement actif, session, verrou. */
class AppModele(app: Application) : AndroidViewModel(app) {
    private val prefs = Preferences(app)
    private val cache = CacheChiffre(app)
    val lecteurLiens = LecteurRattachement(BuildConfig.DOMAINE_SCOLIA, BuildConfig.MODE_LABO)

    var rattachements by mutableStateOf(prefs.rattachements())
        private set
    var actif by mutableStateOf(rattachements.firstOrNull { it.baseUrl == prefs.actif } ?: rattachements.firstOrNull())
        private set
    var etat by mutableStateOf<EtatConnexion>(EtatConnexion.Deconnecte)
        private set
    var verrouille by mutableStateOf(true)
    var occupe by mutableStateOf(false)
        private set
    var erreur by mutableStateOf<String?>(null)
    var invitationEnAttente by mutableStateOf<String?>(null)
        private set
    val dernierEmail: String? get() = prefs.dernierEmail

    var client: ApiClient? = null
        private set
    var lecteur: LecteurHorsLigne? = null
        private set
    private var service: ServiceConnexion? = null

    init {
        // Session mobile existante (jeton au coffre) : reprise directe, verrou biométrique ensuite.
        actif?.let {
            configurer(it)
            reprendre()
        }
    }

    private fun configurer(r: Rattachement) {
        val coffre = CoffreKeystore(getApplication(), r.baseUrl)
        val c = ApiClient(r.baseUrl, TransportOkHttp(userAgent = "ScolIA-Android/${BuildConfig.VERSION_NAME}"), coffre::lire, r.etablissementIdLabo)
        client = c
        service = ServiceConnexion(c, coffre, cache, PasskeyAndroid(ActiviteCourante::exiger), rpId = BuildConfig.DOMAINE_SCOLIA)
        lecteur = LecteurHorsLigne(c, cache, {
            val me = (etat as? EtatConnexion.Connecte)?.me
            "${r.baseUrl}|${me?.userId}"
        })
    }

    /** Lien d'invitation, QR code ou saisie. Vérifie l'établissement auprès du serveur. */
    fun rattacher(saisie: String) = action {
        when (val r = lecteurLiens.lire(saisie)) {
            is LecteurRattachement.Resultat.Refuse -> erreur = r.raison
            is LecteurRattachement.Resultat.Ok -> {
                val brut = r.rattachement
                val sonde = ApiClient(brut.baseUrl, TransportOkHttp(), { null }, brut.etablissementIdLabo)
                val nom = try {
                    sonde.etablissementDeLAdresse().nom
                } catch (e: ErreurApi.Introuvable) {
                    if (brut.etablissementIdLabo == null) throw ErreurApi.Introuvable("Établissement inconnu à cette adresse.")
                    brut.nom
                }
                val ok = brut.copy(nom = nom)
                rattachements = rattachements.filter { it.baseUrl != ok.baseUrl } + ok
                prefs.sauver(rattachements)
                invitationEnAttente = ok.invitation
                choisir(ok)
            }
        }
    }

    fun choisir(r: Rattachement) {
        actif = r
        prefs.actif = r.baseUrl
        etat = EtatConnexion.Deconnecte
        configurer(r)
        reprendre()
    }

    /** Retire l'établissement : session révoquée (si possible), jeton, cache et e-mail effacés. */
    fun oublierEtablissement(r: Rattachement) {
        val s = if (actif?.baseUrl == r.baseUrl) service else null
        val coffre = CoffreKeystore(getApplication(), r.baseUrl)
        viewModelScope.launch {
            s?.deconnexion() // révocation côté serveur au mieux (le jeton est lu avant effacement)
            coffre.effacer()
            cache.toutEffacer()
        }
        prefs.dernierEmail = null
        rattachements = rattachements.filter { it.baseUrl != r.baseUrl }
        prefs.sauver(rattachements)
        if (actif?.baseUrl == r.baseUrl) {
            actif = rattachements.firstOrNull()
            etat = EtatConnexion.Deconnecte
            actif?.let { configurer(it) }
        }
    }

    private var reprise: kotlinx.coroutines.Job? = null

    /** Reprise de session ; annule une reprise en cours pour un autre établissement. */
    fun reprendre() {
        reprise?.cancel()
        val s = service
        val cible = actif?.baseUrl
        reprise = viewModelScope.launch {
            try {
                val e = s?.reprendre() ?: EtatConnexion.Deconnecte
                if (actif?.baseUrl == cible) etat = e
            } catch (e: ErreurApi.HorsLigne) {
                erreur = "Pas de connexion au serveur. Réessayez quand le réseau revient."
            } catch (e: ErreurApi) {
                erreur = e.message
            }
        }
    }

    /** Horodatage de mise en arrière-plan (survit aux recréations de l'activité). */
    var arrierePlanDepuis: Long = 0L

    fun connexionPasskey(email: String) = action {
        prefs.dernierEmail = email.trim()
        etat = service!!.connexionPasskey(email)
        verrouille = false // connexion à l'instant : pas de second déverrouillage
    }

    fun connexionMotDePasse(email: String, mdp: String) = action {
        prefs.dernierEmail = email.trim()
        etat = service!!.connexionMotDePasse(email, mdp)
        verrouille = false
    }

    fun secondFacteur(defi: PasskeyChallenge) = action {
        etat = service!!.terminerPasskey(defi)
        verrouille = false
    }

    fun enroler(mdp: String) = action {
        service!!.enrolerPasskey(mdp, "Téléphone Android")
        etat = service!!.reprendre()
    }

    fun deconnexion() = action {
        service?.deconnexion()
        etat = EtatConnexion.Deconnecte
    }

    /** Session refusée par le serveur (révocation à distance, expiration). */
    fun sessionPerdue() {
        service?.oublier()
        etat = EtatConnexion.Deconnecte
        erreur = "Votre session a pris fin. Reconnectez-vous."
    }

    private fun action(bloc: suspend () -> Unit) {
        viewModelScope.launch {
            occupe = true
            erreur = null
            try {
                bloc()
            } catch (e: ErreurApi.NonAuthentifie) {
                sessionPerdue()
            } catch (e: kotlinx.coroutines.CancellationException) {
                throw e
            } catch (e: Exception) {
                erreur = e.message ?: "Erreur inattendue."
            } finally {
                occupe = false
            }
        }
    }
}
