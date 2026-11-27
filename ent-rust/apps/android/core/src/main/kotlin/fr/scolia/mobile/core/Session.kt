package fr.scolia.mobile.core

import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.JsonObject
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.util.concurrent.TimeUnit

/** Stockage sécurisé du jeton (Android : chiffré par une clé du Keystore). */
interface CoffreJeton {
    fun lire(): String?
    fun ecrire(jeton: String)
    fun effacer()
}

class CoffreMemoire : CoffreJeton {
    @Volatile private var j: String? = null
    override fun lire() = j
    override fun ecrire(jeton: String) {
        j = jeton
    }
    override fun effacer() {
        j = null
    }
}

/** Authentificateur natif (Credential Manager / AuthenticationServices). Rend du JSON WebAuthn. */
interface AuthentificateurPasskey {
    suspend fun signer(optionsJson: String): String
    suspend fun creer(optionsJson: String): String
}

/**
 * Notifications push : interface prête, fournisseur non choisi (FCM, APNs direct,
 * UnifiedPush… à décider). L'implémentation par défaut ne fait rien.
 */
interface FournisseurPush {
    /** Jeton de l'appareil chez le fournisseur, ou `null` si indisponible / refusé. */
    suspend fun jetonAppareil(): String?
    val plateforme: String
}

object PushInactif : FournisseurPush {
    override suspend fun jetonAppareil(): String? = null
    override val plateforme = "android"
}

/** État de connexion présenté à l'interface. */
sealed class EtatConnexion {
    data object Deconnecte : EtatConnexion()
    /** Mot de passe accepté, passkey exigée en second facteur. */
    data class SecondFacteur(val defi: PasskeyChallenge) : EtatConnexion()
    data class Connecte(val me: Me, val espace: Espace) : EtatConnexion()
}

/**
 * Parcours de connexion (D-009, D-011) :
 * 1. passkey d'abord (sans mot de passe) ;
 * 2. sinon mot de passe, puis passkey si le compte en a une (`mfa_required`) ;
 * 3. session `client: "mobile"` (longue, révocable) ; jeton au coffre.
 */
class ServiceConnexion(
    private val client: ApiClient,
    private val coffre: CoffreJeton,
    private val cache: Cache,
    private val passkey: AuthentificateurPasskey,
    private val push: FournisseurPush = PushInactif,
    private val rpId: String = LecteurRattachement.DOMAINE_SCOLIA,
    private val appId: String = "fr.scolia.mobile",
) {
    suspend fun connexionPasskey(email: String): EtatConnexion {
        val defi = client.passkeyLoginStart(email)
        return terminerPasskey(defi)
    }

    suspend fun connexionMotDePasse(email: String, motDePasse: String): EtatConnexion {
        val r = client.login(email, motDePasse)
        if (r.mfaRequired) {
            val defi = r.passkey ?: throw ErreurApi.Format("Second facteur demandé sans défi.")
            return EtatConnexion.SecondFacteur(defi)
        }
        val s = r.session() ?: throw ErreurApi.Format("Session incomplète.")
        return ouvrir(s.token)
    }

    suspend fun terminerPasskey(defi: PasskeyChallenge): EtatConnexion {
        val options = PasskeyPont.optionsConnexion(defi.options, rpId)
        val reponse = passkey.signer(options)
        val credential = PasskeyPont.assertionPourServeur(reponse)
        val s = client.passkeyLoginFinish(PasskeyLoginFinishBody(defi.token, credential))
        return ouvrir(s.token)
    }

    /** Session existante (jeton au coffre) : reprise après déverrouillage biométrique. */
    suspend fun reprendre(): EtatConnexion {
        if (coffre.lire() == null) return EtatConnexion.Deconnecte
        return try {
            charger()
        } catch (e: ErreurApi.NonAuthentifie) {
            oublier()
            EtatConnexion.Deconnecte
        }
    }

    /** Enrôlement d'une passkey (obligatoire à terme, D-009). */
    suspend fun enrolerPasskey(motDePasseActuel: String, nom: String) {
        val defi = client.post("/auth/passkey/register/start", "{}", PasskeyChallengeEnrolement.serializer())
        val options = PasskeyPont.optionsEnrolement(defi.options, rpId)
        val reponse = passkey.creer(options)
        val corps = EnrolementFinishBody(defi.token, PasskeyPont.attestationPourServeur(reponse), nom, motDePasseActuel)
        client.post(
            "/auth/passkey/register/finish",
            client.json.encodeToString(EnrolementFinishBody.serializer(), corps),
            kotlinx.serialization.json.JsonElement.serializer(),
        )
    }

    suspend fun deconnexion() {
        try {
            client.logout()
        } catch (_: ErreurApi) {
            // Hors ligne : le jeton est quand même oublié localement ; la session expirera
            // ou pourra être révoquée depuis un autre appareil.
        }
        oublier()
    }

    fun oublier() {
        coffre.effacer()
        cache.toutEffacer()
    }

    private suspend fun ouvrir(jeton: String): EtatConnexion {
        coffre.ecrire(jeton)
        val etat = charger()
        enregistrerPush()
        return etat
    }

    private suspend fun charger(): EtatConnexion {
        val me = client.me()
        val droits = if (me.portee == "enrolement" || me.role in setOf("parent", "eleve")) {
            emptyList()
        } else {
            try {
                client.mesDroits()
            } catch (e: ErreurApi.Interdit) {
                emptyList()
            }
        }
        return EtatConnexion.Connecte(me, Capacites.espacePour(me, droits))
    }

    private suspend fun enregistrerPush() {
        val jeton = try {
            push.jetonAppareil()
        } catch (_: Exception) {
            null
        } ?: return
        try {
            client.enregistrerPush(PushDeviceBody(push.plateforme, jeton, appId))
        } catch (_: ErreurApi) {
            // Non bloquant : l'appli fonctionne sans push.
        }
    }
}

@kotlinx.serialization.Serializable
data class PasskeyChallengeEnrolement(val token: String, val options: JsonObject)

@kotlinx.serialization.Serializable
data class EnrolementFinishBody(
    val token: String,
    val credential: JsonObject,
    val name: String? = null,
    @kotlinx.serialization.SerialName("current_password") val currentPassword: String = "",
) {
    override fun toString() = "EnrolementFinishBody(name=$name, current_password=<masqué>)"
}

/** Transport réel (OkHttp). Pas de journalisation des corps ni des en-têtes. */
class TransportOkHttp(
    private val http: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(20, TimeUnit.SECONDS)
        .followRedirects(false)
        .build(),
    private val userAgent: String = "ScolIA-Android",
) : Transport {
    override suspend fun executer(requete: RequeteHttp): ReponseHttp = withContext(Dispatchers.IO) {
        val corps = requete.corps?.toRequestBody("application/json; charset=utf-8".toMediaType())
        val b = Request.Builder().url(requete.url).method(requete.methode, corps ?: if (requete.methode == "POST") ByteArray(0).toRequestBody() else null)
        requete.entetes.forEach { (k, v) -> b.header(k, v) }
        b.header("User-Agent", userAgent)
        http.newCall(b.build()).execute().use { r -> ReponseHttp(r.code, r.body?.string() ?: "") }
    }
}

/** Liste typée réutilisable par les écrans (lecture via cache). */
object Chemins {
    fun mesCreneaux(jour: String) = "/edt/mes-creneaux?jour=${ApiClient.q(jour)}"
    fun edtEleve(id: String, jour: String) = "/eleves/${ApiClient.seg(id)}/edt?jour=${ApiClient.q(jour)}"
    fun cahier(eleveId: String?) = "/cahier-texte" + (eleveId?.let { "?eleve_id=${ApiClient.q(it)}" } ?: "")
    fun notes(eleveId: String) = "/eleves/${ApiClient.seg(eleveId)}/notes"
    const val FILS = "/messages/fils"
    const val ENFANTS = "/famille/enfants"
    val listeCreneaux = ListSerializer(EdtCreneau.serializer())
    val listeCahier = ListSerializer(CahierTexte.serializer())
    val listeNotes = ListSerializer(Note.serializer())
    val listeFils = ListSerializer(MsgFil.serializer())
    val listeEnfants = ListSerializer(EnfantFamille.serializer())
}
