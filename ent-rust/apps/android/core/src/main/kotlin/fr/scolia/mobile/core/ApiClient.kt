package fr.scolia.mobile.core

import kotlinx.serialization.KSerializer
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import java.net.URLEncoder

/** Requête HTTP minimale (le transport réel est OkHttp ; les tests en injectent un faux). */
data class RequeteHttp(
    val methode: String,
    val url: String,
    val entetes: Map<String, String>,
    val corps: String? = null,
) {
    // Jamais d'en-tête Authorization ni de corps (mot de passe) dans un journal.
    override fun toString() = "RequeteHttp($methode $url)"
}

data class ReponseHttp(val statut: Int, val corps: String)

/** Transport HTTP abstrait : permet de tester le client sans réseau. */
fun interface Transport {
    suspend fun executer(requete: RequeteHttp): ReponseHttp
}

/** Erreurs de l'API, traduites pour l'interface. Jamais de jeton dans les messages. */
sealed class ErreurApi(message: String) : Exception(message) {
    /** 401 : session expirée ou révoquée (téléphone perdu, déconnexion à distance). */
    class NonAuthentifie(message: String) : ErreurApi(message)

    /** 403 : action non autorisée pour ce compte (le serveur décide, D-003). */
    class Interdit(message: String) : ErreurApi(message)

    class Introuvable(message: String) : ErreurApi(message)

    /** 400 / 409 / 422 : donnée refusée par le serveur (message lisible du serveur). */
    class Refus(val statut: Int, message: String) : ErreurApi(message)

    class Serveur(val statut: Int) : ErreurApi("Erreur du serveur ($statut). Réessayez plus tard.")

    /** Pas de réseau : l'interface bascule sur le cache en lecture seule. */
    class HorsLigne(cause: Throwable?) : ErreurApi("Pas de connexion au serveur.") {
        init {
            cause?.let { initCause(it) }
        }
    }

    class Format(message: String) : ErreurApi(message)
}

/**
 * Client typé de l'API ScolIA.
 *
 * - `baseUrl` : adresse de l'établissement (D-012), ex. `https://lpnb.scolia.fr` ;
 * - `jeton` : fourni par le stockage sécurisé (Keystore), jamais journalisé ;
 * - toute autorisation est vérifiée côté serveur : le client n'en déduit rien de définitif.
 */
class ApiClient(
    val baseUrl: String,
    private val transport: Transport,
    private val jeton: () -> String? = { null },
    /** Établissement explicite : uniquement hors sous-domaine (labo local). */
    private val etablissementIdLabo: String? = null,
) {
    internal val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
        encodeDefaults = true
    }

    // ---------- Authentification ----------

    suspend fun etablissementDeLAdresse(): EtablissementPublic =
        get("/auth/etablissement", EtablissementPublic.serializer(), authentifie = false)

    suspend fun login(email: String, motDePasse: String): LoginResponse =
        post(
            "/auth/login",
            json.encodeToString(LoginBody.serializer(), LoginBody(email.trim(), motDePasse, etablissementIdLabo)),
            LoginResponse.serializer(),
            authentifie = false,
        )

    suspend fun passkeyLoginStart(email: String): PasskeyChallenge =
        post(
            "/auth/passkey/login/start",
            json.encodeToString(
                PasskeyLoginStartBody.serializer(),
                PasskeyLoginStartBody(email.trim(), etablissementIdLabo),
            ),
            PasskeyChallenge.serializer(),
            authentifie = false,
        )

    suspend fun passkeyLoginFinish(body: PasskeyLoginFinishBody): AuthSession =
        post(
            "/auth/passkey/login/finish",
            json.encodeToString(PasskeyLoginFinishBody.serializer(), body),
            AuthSession.serializer(),
            authentifie = false,
        )

    suspend fun me(): Me = get("/auth/me", Me.serializer())

    suspend fun mesDroits(): List<Droit> = get("/roles/mes-droits", ListSerializer(Droit.serializer()))

    suspend fun sessions(): List<SessionAppareil> =
        get("/auth/sessions", ListSerializer(SessionAppareil.serializer()))

    suspend fun revoquerSession(id: String) {
        envoyer("DELETE", "/auth/sessions/${seg(id)}", null)
    }

    suspend fun logout() {
        envoyer("POST", "/auth/logout", null)
    }

    suspend fun enregistrerPush(body: PushDeviceBody) {
        envoyer("POST", "/devices/push", json.encodeToString(PushDeviceBody.serializer(), body))
    }

    // ---------- EDT et appel ----------

    suspend fun mesCreneaux(jour: String): List<EdtCreneau> =
        get("/edt/mes-creneaux?jour=${q(jour)}", ListSerializer(EdtCreneau.serializer()))

    suspend fun edtEleve(eleveId: String, jour: String): List<EdtCreneau> =
        get("/eleves/${seg(eleveId)}/edt?jour=${q(jour)}", ListSerializer(EdtCreneau.serializer()))

    suspend fun ouvrirAppel(creneauId: String, jour: String): Appel =
        post("/appels", json.encodeToString(OuvrirAppelBody.serializer(), OuvrirAppelBody(creneauId, jour)), Appel.serializer())

    suspend fun appel(id: String): Appel = get("/appels/${seg(id)}", Appel.serializer())

    /** Réponse 204 sans corps : l'appelant met à jour sa copie locale. */
    suspend fun marquer(appelId: String, eleveId: String, etat: EtatAppel) {
        envoyer(
            "POST",
            "/appels/${seg(appelId)}/lignes",
            json.encodeToString(MarquerAppelLigneBody.serializer(), MarquerAppelLigneBody(eleveId, etat.code)),
        )
    }

    suspend fun clore(appelId: String): Appel = post("/appels/${seg(appelId)}/clore", "{}", Appel.serializer())

    // ---------- Famille et absences ----------

    suspend fun mesEnfants(): List<EnfantFamille> =
        get("/famille/enfants", ListSerializer(EnfantFamille.serializer()))

    suspend fun resumeEnfant(eleveId: String): FamilleResume =
        get("/famille/enfants/${seg(eleveId)}/resume", FamilleResume.serializer())

    suspend fun justifierAbsence(absenceId: String, motif: String): Absence =
        post(
            "/absences/${seg(absenceId)}/justifier",
            json.encodeToString(JustifierAbsenceBody.serializer(), JustifierAbsenceBody(motif.trim())),
            Absence.serializer(),
        )

    /** Vie scolaire : absences à traiter (`GET /vs/absences`). */
    suspend fun absencesATraiter(jour: String? = null): List<Absence> =
        get("/vs/absences" + (jour?.let { "?jour=${q(it)}" } ?: ""), ListSerializer(Absence.serializer()))

    // ---------- Notes ----------

    suspend fun periodesNotes(): List<NotePeriode> =
        get("/note-periodes", ListSerializer(NotePeriode.serializer()))

    suspend fun notes(eleveId: String): List<Note> =
        get("/eleves/${seg(eleveId)}/notes", ListSerializer(Note.serializer()))

    suspend fun moyennes(eleveId: String, periodeId: String): List<MoyenneMatiere> =
        get("/eleves/${seg(eleveId)}/moyennes?periode_id=${q(periodeId)}", ListSerializer(MoyenneMatiere.serializer()))

    suspend fun devoirsClasse(periodeId: String, classe: String): List<Devoir> =
        get("/devoirs?periode_id=${q(periodeId)}&classe=${q(classe)}", ListSerializer(Devoir.serializer()))

    suspend fun devoirsEleve(eleveId: String, periodeId: String): List<Devoir> =
        get("/eleves/${seg(eleveId)}/devoirs?periode_id=${q(periodeId)}", ListSerializer(Devoir.serializer()))

    suspend fun effectif(classe: String, jour: String): Effectif =
        get("/classes/${seg(classe)}/effectif?jour=${q(jour)}", Effectif.serializer())

    suspend fun saisirNote(eleveId: String, body: SaisirNoteBody): Note =
        post("/eleves/${seg(eleveId)}/notes", json.encodeToString(SaisirNoteBody.serializer(), body), Note.serializer())

    // ---------- Cahier de textes ----------

    suspend fun cahierTexte(eleveId: String? = null, classe: String? = null, du: String? = null, au: String? = null): List<CahierTexte> {
        val params = listOfNotNull(
            eleveId?.let { "eleve_id=${q(it)}" },
            classe?.let { "classe=${q(it)}" },
            du?.let { "from=${q(it)}" },
            au?.let { "to=${q(it)}" },
        )
        val suffixe = if (params.isEmpty()) "" else params.joinToString("&", prefix = "?")
        return get("/cahier-texte$suffixe", ListSerializer(CahierTexte.serializer()))
    }

    // ---------- Messagerie ----------

    suspend fun fils(eleveId: String? = null): List<MsgFil> =
        get("/messages/fils" + (eleveId?.let { "?eleve_id=${q(it)}" } ?: ""), ListSerializer(MsgFil.serializer()))

    suspend fun fil(id: String): FilDetail = get("/messages/fils/${seg(id)}", FilDetail.serializer())

    suspend fun repondre(filId: String, corps: String): Message =
        post("/messages/fils/${seg(filId)}/repondre", json.encodeToString(RepondreFilBody.serializer(), RepondreFilBody(corps)), Message.serializer())

    // ---------- Annonces, notifications ----------

    suspend fun annonces(): List<Annonce> = get("/annonces", ListSerializer(Annonce.serializer()))

    suspend fun notifications(): List<Notification> =
        get("/notifications", ListSerializer(Notification.serializer()))

    // ---------- Mécanique ----------

    /** GET brut (JSON texte), utilisé par le cache hors ligne. */
    suspend fun getBrut(chemin: String, authentifie: Boolean = true): String =
        envoyer("GET", chemin, null, authentifie)

    suspend fun <T> get(chemin: String, ser: KSerializer<T>, authentifie: Boolean = true): T =
        decoder(envoyer("GET", chemin, null, authentifie), ser)

    suspend fun <T> post(chemin: String, corps: String, ser: KSerializer<T>, authentifie: Boolean = true): T =
        decoder(envoyer("POST", chemin, corps, authentifie), ser)

    fun <T> decoder(texte: String, ser: KSerializer<T>): T =
        try {
            json.decodeFromString(ser, texte)
        } catch (e: Exception) {
            throw ErreurApi.Format("Réponse inattendue du serveur.")
        }

    private suspend fun envoyer(methode: String, chemin: String, corps: String?, authentifie: Boolean = true): String {
        val entetes = buildMap {
            put("Accept", "application/json")
            if (corps != null) put("Content-Type", "application/json")
            if (authentifie) {
                val t = jeton() ?: throw ErreurApi.NonAuthentifie("Session absente.")
                put("Authorization", "Bearer $t")
            }
        }
        val reponse = try {
            transport.executer(RequeteHttp(methode, baseUrl.trimEnd('/') + chemin, entetes, corps))
        } catch (e: ErreurApi) {
            throw e
        } catch (e: kotlinx.coroutines.CancellationException) {
            throw e
        } catch (e: Exception) {
            throw ErreurApi.HorsLigne(e)
        }
        return when (reponse.statut) {
            in 200..299 -> reponse.corps
            // 401 sans jeton (connexion) = identifiants refusés, pas une session perdue.
            401 -> if (authentifie) {
                throw ErreurApi.NonAuthentifie("Session expirée : reconnectez-vous.")
            } else {
                throw ErreurApi.Refus(401, messageServeur(reponse.corps, "Identifiants incorrects."))
            }
            403 -> throw ErreurApi.Interdit(messageServeur(reponse.corps, "Action non autorisée."))
            404 -> throw ErreurApi.Introuvable(messageServeur(reponse.corps, "Élément introuvable."))
            in 400..499 -> throw ErreurApi.Refus(reponse.statut, messageServeur(reponse.corps, "Demande refusée."))
            else -> throw ErreurApi.Serveur(reponse.statut)
        }
    }

    private fun messageServeur(corps: String, defaut: String): String {
        val t = corps.trim()
        // Le serveur renvoie un texte court ; on borne la longueur affichée.
        return if (t.isEmpty() || t.length > 300 || t.startsWith("<")) defaut else t
    }

    companion object {
        /** Encode un segment de chemin (classe « 3e A » → `3e%20A`). */
        fun seg(s: String): String = URLEncoder.encode(s, Charsets.UTF_8).replace("+", "%20")

        fun q(s: String): String = URLEncoder.encode(s, Charsets.UTF_8).replace("+", "%20")
    }
}
