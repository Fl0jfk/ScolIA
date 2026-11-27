package fr.scolia.mobile.core

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import java.net.URLEncoder
import java.util.UUID

/** Requête HTTP minimale (le transport réel est `TransportHttps` ; les tests en injectent un faux). */
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

/**
 * Enveloppe d'erreur codée du serveur (contrat mobile §5.3, MB1) :
 * `{ "erreur": { code, message, champs, reessayer_apres_s, trace } }`.
 * Le `code` (`domaine.cause`) est la seule chose que l'appli interprète.
 */
@Serializable
data class ErreurCodee(
    val code: String,
    val message: String = "",
    val champs: List<ErreurChampCodee> = emptyList(),
    @SerialName("reessayer_apres_s") val reessayerApresS: Int? = null,
    val trace: String? = null,
)

@Serializable
data class ErreurChampCodee(val champ: String, val code: String)

@Serializable
internal data class EnveloppeErreur(val erreur: ErreurCodee)

/** Erreurs de l'API, traduites pour l'interface. Jamais de jeton dans les messages. */
sealed class ErreurApi(message: String) : Exception(message) {
    /** Code stable du serveur (`domaine.cause`), quand la réponse porte l'enveloppe codée. */
    open val code: String? get() = null

    /** Identifiant de requête pour le support (jamais une donnée personnelle). */
    open val trace: String? get() = null

    /** 401 `session.expiree` / `session.revoquee` : session perdue (téléphone perdu, déconnexion à distance). */
    class NonAuthentifie(message: String, override val code: String? = null, override val trace: String? = null) :
        ErreurApi(message)

    /** 403 : action non autorisée pour ce compte (le serveur décide, D-003). */
    class Interdit(message: String, override val code: String? = null, override val trace: String? = null) :
        ErreurApi(message)

    /** 404 : objet inexistant ou hors droits (pas d'oracle d'existence). */
    class Introuvable(message: String, override val code: String? = null, override val trace: String? = null) :
        ErreurApi(message)

    /**
     * 400 / 401 de connexion (`auth.refusee`) / 409 / 422 / 423… : donnée refusée par le serveur.
     * `champs` : champs en erreur (400) ; `reessayerApresS` : délai conseillé (409 `idempotence.en_cours`).
     */
    class Refus(
        val statut: Int,
        message: String,
        override val code: String? = null,
        val champs: List<ErreurChampCodee> = emptyList(),
        val reessayerApresS: Int? = null,
        override val trace: String? = null,
    ) : ErreurApi(message)

    /** 426 `appli.trop_ancienne` : version sous le minimum, l'interface affiche l'écran de mise à jour. */
    class AppliTropAncienne(message: String, override val trace: String? = null) : ErreurApi(message) {
        override val code: String get() = "appli.trop_ancienne"
    }

    /** 429 `debit.depasse` : attendre `reessayerApresS`, ne jamais rejouer en boucle. */
    class DebitDepasse(message: String, val reessayerApresS: Int?, override val trace: String? = null) :
        ErreurApi(message) {
        override val code: String get() = "debit.depasse"
    }

    class Serveur(val statut: Int, override val code: String? = null, override val trace: String? = null) :
        ErreurApi("Erreur du serveur ($statut). Réessayez plus tard.")

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
    /**
     * Valeur de `X-ScolIA-Appli` (contrat §5.2) : `android/<version> (build <n>)`. Envoyée sur toute requête :
     * le serveur répond alors par l'enveloppe d'erreur codée et exige `Idempotency-Key` sur les écritures.
     */
    val appli: String = APPLI_INCONNUE,
    /** Générateur de clés d'idempotence (UUID v4) ; injectable pour les tests. */
    private val nouvelleCle: () -> String = { UUID.randomUUID().toString() },
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

    /** Inscription du jeton de notification (MB0 : la réponse ne renvoie jamais le jeton, seulement l'identifiant). */
    suspend fun enregistrerPush(body: PushDeviceBody, cle: String? = null): PushDevice =
        post("/devices/push", json.encodeToString(PushDeviceBody.serializer(), body), PushDevice.serializer(), cle = cle)

    /** Désinscription à la déconnexion (DS4) : `DELETE /devices/push/{id}`. */
    suspend fun desinscrirePush(id: String, cle: String? = null) {
        envoyer("DELETE", "/devices/push/${seg(id)}", null, cle = cle)
    }

    // ---------- EDT et appel ----------

    suspend fun mesCreneaux(jour: String): List<EdtCreneau> =
        get("/edt/mes-creneaux?jour=${q(jour)}", ListSerializer(EdtCreneau.serializer()))

    suspend fun edtEleve(eleveId: String, jour: String): List<EdtCreneau> =
        get("/eleves/${seg(eleveId)}/edt?jour=${q(jour)}", ListSerializer(EdtCreneau.serializer()))

    suspend fun ouvrirAppel(creneauId: String, jour: String, cle: String? = null): Appel =
        post(
            "/appels",
            json.encodeToString(OuvrirAppelBody.serializer(), OuvrirAppelBody(creneauId, jour)),
            Appel.serializer(),
            cle = cle,
        )

    suspend fun appel(id: String): Appel = get("/appels/${seg(id)}", Appel.serializer())

    /** Réponse 204 sans corps : l'appelant met à jour sa copie locale. */
    suspend fun marquer(appelId: String, eleveId: String, etat: EtatAppel, cle: String? = null) {
        envoyer(
            "POST",
            "/appels/${seg(appelId)}/lignes",
            json.encodeToString(MarquerAppelLigneBody.serializer(), MarquerAppelLigneBody(eleveId, etat.code)),
            cle = cle,
        )
    }

    suspend fun clore(appelId: String, cle: String? = null): Appel =
        post("/appels/${seg(appelId)}/clore", "{}", Appel.serializer(), cle = cle)

    // ---------- Famille et absences ----------

    suspend fun mesEnfants(): List<EnfantFamille> =
        get("/famille/enfants", ListSerializer(EnfantFamille.serializer()))

    suspend fun resumeEnfant(eleveId: String): FamilleResume =
        get("/famille/enfants/${seg(eleveId)}/resume", FamilleResume.serializer())

    suspend fun justifierAbsence(absenceId: String, motif: String, cle: String? = null): Absence =
        post(
            "/absences/${seg(absenceId)}/justifier",
            json.encodeToString(JustifierAbsenceBody.serializer(), JustifierAbsenceBody(motif.trim())),
            Absence.serializer(),
            cle = cle,
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

    suspend fun saisirNote(eleveId: String, body: SaisirNoteBody, cle: String? = null): Note =
        post(
            "/eleves/${seg(eleveId)}/notes",
            json.encodeToString(SaisirNoteBody.serializer(), body),
            Note.serializer(),
            cle = cle,
        )

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

    suspend fun repondre(filId: String, corps: String, cle: String? = null): Message =
        post(
            "/messages/fils/${seg(filId)}/repondre",
            json.encodeToString(RepondreFilBody.serializer(), RepondreFilBody(corps)),
            Message.serializer(),
            cle = cle,
        )

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

    /**
     * `cle` : clé d'idempotence de l'intention d'écriture. Un appelant qui **réessaie** la même intention (après un
     * délai dépassé, par exemple) repasse la même clé : le serveur rejoue alors la première réponse au lieu
     * d'exécuter deux fois. Sans clé fournie, une clé neuve est tirée pour cet envoi.
     */
    suspend fun <T> post(
        chemin: String,
        corps: String,
        ser: KSerializer<T>,
        authentifie: Boolean = true,
        cle: String? = null,
    ): T = decoder(envoyer("POST", chemin, corps, authentifie, cle), ser)

    fun <T> decoder(texte: String, ser: KSerializer<T>): T =
        try {
            json.decodeFromString(ser, texte)
        } catch (e: Exception) {
            throw ErreurApi.Format("Réponse inattendue du serveur.")
        }

    private suspend fun envoyer(
        methode: String,
        chemin: String,
        corps: String?,
        authentifie: Boolean = true,
        cle: String? = null,
    ): String {
        val entetes = buildMap {
            put("Accept", "application/json")
            put(EN_TETE_APPLI, appli)
            if (corps != null) put("Content-Type", "application/json")
            if (authentifie) {
                val t = jeton() ?: throw ErreurApi.NonAuthentifie("Session absente.")
                put("Authorization", "Bearer $t")
                // Toute écriture authentifiée porte une clé d'idempotence (§5.2) ; l'authentification (`/auth/*`)
                // est hors registre côté serveur : pas de clé (aucun mot de passe ni jeton conservé).
                if (methode in ECRITURES && !chemin.startsWith("/auth")) {
                    put("Idempotency-Key", cle ?: nouvelleCle())
                }
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
        if (reponse.statut in 200..299) return reponse.corps
        throw erreurDe(reponse, authentifie, chemin)
    }

    /** Traduit une réponse d'erreur : enveloppe codée d'abord, texte court du serveur en repli (anciennes routes). */
    internal fun erreurDe(reponse: ReponseHttp, authentifie: Boolean, chemin: String = ""): ErreurApi {
        val e = lireEnveloppe(reponse.corps)
        val code = e?.code
        val trace = e?.trace
        fun msg(defaut: String) = e?.message?.takeIf { it.isNotBlank() && it.length <= 500 } ?: messageServeur(reponse.corps, defaut)
        return when (reponse.statut) {
            // 401 = identifiants refusés (pas de purge) seulement sur une route **sans jeton** (connexion) ou sur une route
            // qui redemande le mot de passe (`REFUS_401_ATTENDU`). Ailleurs, même avec `auth.refusee` (que le serveur pose
            // sur tout 401 de `/auth/*`, y compris un jeton mort sur `/auth/me`), c'est une session perdue.
            401 -> if (!authentifie || chemin.substringBefore('?') in REFUS_401_ATTENDU) {
                ErreurApi.Refus(401, msg("Identifiants incorrects."), code ?: CODE_AUTH_REFUSEE, trace = trace)
            } else {
                ErreurApi.NonAuthentifie("Session expirée : reconnectez-vous.", code, trace)
            }
            403 -> ErreurApi.Interdit(msg("Action non autorisée."), code, trace)
            404 -> ErreurApi.Introuvable(msg("Élément introuvable."), code, trace)
            426 -> ErreurApi.AppliTropAncienne(msg("Mettez l'application à jour pour continuer."), trace)
            429 -> ErreurApi.DebitDepasse(msg("Trop de demandes : réessayez dans un instant."), e?.reessayerApresS, trace)
            in 400..499 -> ErreurApi.Refus(
                reponse.statut,
                msg("Demande refusée."),
                code,
                e?.champs.orEmpty(),
                e?.reessayerApresS,
                trace,
            )
            else -> ErreurApi.Serveur(reponse.statut, code, trace)
        }
    }

    private fun lireEnveloppe(corps: String): ErreurCodee? {
        val t = corps.trim()
        if (!t.startsWith("{")) return null
        return try {
            json.decodeFromString(EnveloppeErreur.serializer(), t).erreur
        } catch (_: Exception) {
            null
        }
    }

    private fun messageServeur(corps: String, defaut: String): String {
        val t = corps.trim()
        // Le serveur renvoie un texte court ; on borne la longueur affichée ; jamais de JSON ni de HTML brut.
        return if (t.isEmpty() || t.length > 300 || t.startsWith("<") || t.startsWith("{")) defaut else t
    }

    companion object {
        const val EN_TETE_APPLI = "X-ScolIA-Appli"
        const val APPLI_INCONNUE = "android/0.0.0 (build 0)"
        const val CODE_AUTH_REFUSEE = "auth.refusee"
        private val ECRITURES = setOf("POST", "PUT", "PATCH", "DELETE")

        /** Routes authentifiées où un 401 signifie « mot de passe actuel refusé », jamais « session perdue ». */
        val REFUS_401_ATTENDU = setOf("/auth/passkey/register/finish", "/auth/password")

        /** Valeur de `X-ScolIA-Appli` : plateforme et version, aucun identifiant d'appareil. */
        fun valeurAppli(plateforme: String, version: String, build: Long): String = "$plateforme/$version (build $build)"

        /** Encode un segment de chemin (classe « 3e A » → `3e%20A`). */
        fun seg(s: String): String = URLEncoder.encode(s, Charsets.UTF_8).replace("+", "%20")

        fun q(s: String): String = URLEncoder.encode(s, Charsets.UTF_8).replace("+", "%20")
    }
}
