package fr.scolia.mobile.core

import kotlinx.serialization.EncodeDefault
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject

// Modèles JSON de l'API ScolIA (`ent-rust/crates/ent-api`). Copie des DTO serveur :
// mêmes noms de champs (snake_case via @SerialName). Les dates restent des chaînes
// ISO 8601 (`2026-11-06`, `2026-11-06T08:00:00Z`) ; les états restent des chaînes
// comme dans `ent-types`. Les UUID sont des chaînes.
//
// Source : `ent-types/src/{auth,appel,edt}.rs`, `ent-api/src/dto.rs`, `routes.rs`, `roles.rs`.

// ---------- Session et identité ----------

/** Corps de `POST /auth/login`. `client = "mobile"` : session longue (D-011). */
@Serializable
data class LoginBody(
    val email: String,
    val password: String,
    @SerialName("etablissement_id") val etablissementId: String? = null,
    @OptIn(ExperimentalSerializationApi::class)
    @EncodeDefault val client: String = CLIENT_MOBILE,
) {
    override fun toString() = "LoginBody(email=$email, password=<masqué>)"
}

/** Corps de `POST /auth/passkey/login/start`. */
@Serializable
data class PasskeyLoginStartBody(
    val email: String,
    @SerialName("etablissement_id") val etablissementId: String? = null,
    @OptIn(ExperimentalSerializationApi::class)
    @EncodeDefault val client: String = CLIENT_MOBILE,
)

/** Corps de `POST /auth/passkey/login/finish`. `credential` : JSON WebAuthn standard. */
@Serializable
data class PasskeyLoginFinishBody(val token: String, val credential: JsonObject)

/** Session ouverte : jeton opaque à envoyer en `Authorization: Bearer`. */
@Serializable
data class AuthSession(
    val token: String,
    @SerialName("user_id") val userId: String,
    val email: String,
    @SerialName("etablissement_id") val etablissementId: String,
    val role: String,
    @SerialName("expires_at") val expiresAt: String,
) {
    override fun toString() = "AuthSession(token=<masqué>, userId=$userId, role=$role)"
}

/** Défi passkey (options WebAuthn produites par `webauthn-rs` + jeton opaque). */
@Serializable
data class PasskeyChallenge(
    val token: String,
    val options: JsonObject,
    @SerialName("expires_at") val expiresAt: String? = null,
)

/** Réponse de `POST /auth/login` : session à plat, ou second facteur passkey exigé. */
@Serializable
data class LoginResponse(
    @SerialName("mfa_required") val mfaRequired: Boolean,
    val token: String? = null,
    @SerialName("user_id") val userId: String? = null,
    val email: String? = null,
    @SerialName("etablissement_id") val etablissementId: String? = null,
    val role: String? = null,
    @SerialName("expires_at") val expiresAt: String? = null,
    val passkey: PasskeyChallenge? = null,
) {
    /** Session à plat si la connexion est complète. */
    fun session(): AuthSession? {
        if (mfaRequired) return null
        return AuthSession(
            token ?: return null, userId ?: return null, email ?: return null,
            etablissementId ?: return null, role ?: return null, expiresAt ?: return null,
        )
    }

    override fun toString() = "LoginResponse(mfaRequired=$mfaRequired, role=$role)"
}

/** Réponse de `GET /auth/me`. */
@Serializable
data class Me(
    @SerialName("user_id") val userId: String,
    val email: String,
    @SerialName("etablissement_id") val etablissementId: String,
    val role: String,
    @SerialName("eleve_id") val eleveId: String? = null,
    @SerialName("passkey_count") val passkeyCount: Long = 0,
    /** `complete` ou `enrolement` (passkey à enrôler avant tout, D-009). */
    val portee: String = "complete",
    /** `mot_de_passe`, `passkey` ou `sso`. */
    val assurance: String = "",
    /** `web` ou `mobile`. */
    val client: String = "",
    @SerialName("passkey_obligatoire") val passkeyObligatoire: Boolean = false,
)

/** Droit effectif (`GET /roles/mes-droits`, D-017). */
@Serializable
data class Droit(
    val action: String,
    /** `etablissement`, `mes_classes`, `mes_groupes`, `designes`. */
    val portee: String,
    val cibles: List<String> = emptyList(),
)

/** Établissement servi par l'adresse (`GET /auth/etablissement`, D-012). */
@Serializable
data class EtablissementPublic(val id: String, val slug: String, val nom: String)

/** Session d'un appareil (`GET /auth/sessions`). */
@Serializable
data class SessionAppareil(
    val id: String,
    val client: String,
    val assurance: String,
    val appareil: String? = null,
    @SerialName("created_at") val createdAt: String,
    @SerialName("last_seen_at") val lastSeenAt: String,
    @SerialName("expires_at") val expiresAt: String,
    val courante: Boolean,
)

/** Corps de `POST /devices/push`. */
@Serializable
data class PushDeviceBody(
    val platform: String,
    val token: String,
    @SerialName("app_id") val appId: String? = null,
)

// ---------- Emploi du temps et appel ----------

/** Créneau d'EDT. `jour_semaine` : 1 = lundi … 7 = dimanche (ISO). */
@Serializable
data class EdtCreneau(
    val id: String,
    val classe: String,
    @SerialName("matiere_libelle") val matiereLibelle: String,
    @SerialName("prof_user_id") val profUserId: String,
    @SerialName("prof_label") val profLabel: String? = null,
    @SerialName("jour_semaine") val jourSemaine: Int,
    @SerialName("heure_debut") val heureDebut: String,
    @SerialName("heure_fin") val heureFin: String,
    val salle: String? = null,
    @SerialName("semaine_ab") val semaineAb: String = "",
)

@Serializable
data class OuvrirAppelBody(
    @SerialName("creneau_id") val creneauId: String,
    val jour: String,
)

/** `etat` : `present` | `absent` | `retard`. */
@Serializable
data class MarquerAppelLigneBody(
    @SerialName("eleve_id") val eleveId: String,
    val etat: String,
)

@Serializable
data class AppelLigne(
    @SerialName("eleve_id") val eleveId: String,
    @SerialName("eleve_label") val eleveLabel: String? = null,
    val etat: String,
)

/** Appel. `statut` : `ouvert` | `clos`. */
@Serializable
data class Appel(
    val id: String,
    val classe: String,
    val jour: String,
    val statut: String,
    @SerialName("creneau_id") val creneauId: String? = null,
    @SerialName("matiere_libelle") val matiereLibelle: String? = null,
    val lignes: List<AppelLigne> = emptyList(),
) {
    val estClos: Boolean get() = statut == "clos"
}

/** États possibles d'une ligne d'appel, dans l'ordre d'affichage. */
enum class EtatAppel(val code: String, val libelle: String) {
    PRESENT("present", "Présent"),
    ABSENT("absent", "Absent"),
    RETARD("retard", "En retard");

    companion object {
        fun depuis(code: String): EtatAppel? = entries.firstOrNull { it.code == code }
    }
}

// ---------- Famille, absences ----------

@Serializable
data class EnfantFamille(
    val id: String,
    val nom: String,
    val prenom: String,
    val classe: String? = null,
    val status: String = "",
    val lien: String = "",
    @SerialName("peut_voir_notes") val peutVoirNotes: Boolean = false,
    @SerialName("peut_voir_vs") val peutVoirVs: Boolean = false,
    @SerialName("peut_justifier_absence") val peutJustifierAbsence: Boolean = false,
    @SerialName("peut_autoriser_sorties") val peutAutoriserSorties: Boolean = false,
) {
    val nomComplet: String get() = "$prenom $nom"
}

@Serializable
data class Absence(
    val id: String,
    @SerialName("etablissement_id") val etablissementId: String = "",
    @SerialName("eleve_id") val eleveId: String,
    @SerialName("eleve_label") val eleveLabel: String? = null,
    val debut: String,
    val fin: String,
    val motif: String = "",
    val statut: String,
    @SerialName("motif_justif") val motifJustif: String? = null,
    @SerialName("decision_note") val decisionNote: String? = null,
)

@Serializable
data class JustifierAbsenceBody(@SerialName("motif_justif") val motifJustif: String)

@Serializable
data class CarnetEntree(
    val id: String,
    @SerialName("eleve_id") val eleveId: String,
    @SerialName("date_entree") val dateEntree: String,
    val categorie: String = "",
    val titre: String,
    val corps: String = "",
    @SerialName("visible_famille") val visibleFamille: Boolean = true,
    @SerialName("signe_at") val signeAt: String? = null,
)

/** Résumé famille d'un enfant (`GET /famille/enfants/{id}/resume`). */
@Serializable
data class FamilleResume(
    @SerialName("eleve_id") val eleveId: String,
    @SerialName("absences_a_justifier") val absencesAJustifier: List<Absence> = emptyList(),
    @SerialName("carnet_a_signer") val carnetASigner: List<CarnetEntree> = emptyList(),
    val notes: List<Note> = emptyList(),
)

// ---------- Notes, devoirs, cahier de textes ----------

@Serializable
data class Note(
    val id: String,
    @SerialName("eleve_id") val eleveId: String,
    @SerialName("matiere_id") val matiereId: String,
    @SerialName("matiere_libelle") val matiereLibelle: String? = null,
    val valeur: Double,
    val sur: Double,
    @SerialName("date_note") val dateNote: String,
    val libelle: String? = null,
    @SerialName("devoir_id") val devoirId: String? = null,
    @SerialName("periode_id") val periodeId: String? = null,
)

@Serializable
data class NotePeriode(
    val id: String,
    val code: String,
    val libelle: String,
    @SerialName("date_debut") val dateDebut: String,
    @SerialName("date_fin") val dateFin: String,
    val statut: String,
)

@Serializable
data class MoyenneMatiere(
    @SerialName("matiere_id") val matiereId: String,
    @SerialName("matiere_libelle") val matiereLibelle: String? = null,
    @SerialName("periode_id") val periodeId: String,
    @SerialName("moyenne_sur_20") val moyenneSur20: Double,
)

@Serializable
data class Devoir(
    val id: String,
    @SerialName("periode_id") val periodeId: String,
    @SerialName("matiere_id") val matiereId: String,
    @SerialName("matiere_libelle") val matiereLibelle: String? = null,
    val classe: String,
    val titre: String,
    @SerialName("date_devoir") val dateDevoir: String,
    val coef: Double = 1.0,
    val sur: Double = 20.0,
)

/** Entrée du cahier de textes (`GET /cahier-texte`). */
@Serializable
data class CahierTexte(
    val id: String,
    val classe: String,
    @SerialName("matiere_libelle") val matiereLibelle: String,
    val titre: String,
    val consigne: String,
    @SerialName("pour_le") val pourLe: String,
    @SerialName("cree_le") val creeLe: String,
    @SerialName("created_at") val createdAt: String = "",
)

// ---------- Messagerie ----------

@Serializable
data class MsgFil(
    val id: String,
    @SerialName("eleve_id") val eleveId: String,
    @SerialName("eleve_label") val eleveLabel: String? = null,
    val sujet: String,
    @SerialName("created_by_user_id") val createdByUserId: String,
    @SerialName("created_at") val createdAt: String,
    @SerialName("last_message_at") val lastMessageAt: String,
    val archive: Boolean = false,
    @SerialName("modele_id") val modeleId: String? = null,
    val priorite: String = "normale",
    val dossier: String? = null,
    @SerialName("nb_non_lus") val nbNonLus: Long = 0,
)

@Serializable
data class MsgPiece(
    val id: String,
    @SerialName("message_id") val messageId: String,
    val nom: String,
    @SerialName("mime_type") val mimeType: String,
    @SerialName("taille_octets") val tailleOctets: Int,
    @SerialName("created_at") val createdAt: String,
)

@Serializable
data class Message(
    val id: String,
    @SerialName("fil_id") val filId: String,
    @SerialName("auteur_user_id") val auteurUserId: String,
    @SerialName("auteur_label") val auteurLabel: String? = null,
    val corps: String,
    @SerialName("created_at") val createdAt: String,
    @SerialName("lu_par_moi") val luParMoi: Boolean = false,
    @SerialName("nb_accuses") val nbAccuses: Long = 0,
    val pieces: List<MsgPiece> = emptyList(),
)

@Serializable
data class FilDetail(val fil: MsgFil, val messages: List<Message> = emptyList())

@Serializable
data class RepondreFilBody(val corps: String)

// ---------- Annonces et notifications ----------

@Serializable
data class Annonce(
    val id: String,
    val titre: String,
    val corps: String,
    val audience: String = "",
    @SerialName("cible_classe") val cibleClasse: String? = null,
    @SerialName("auteur_user_id") val auteurUserId: String = "",
    @SerialName("created_at") val createdAt: String,
)

@Serializable
data class Notification(
    val id: String,
    @SerialName("eleve_id") val eleveId: String? = null,
    val kind: String,
    val titre: String,
    val corps: String,
    val lu: Boolean,
    @SerialName("created_at") val createdAt: String,
)

// ---------- Saisie de notes (professeur) ----------

/** Élève tel que renvoyé dans un effectif de classe (champs utiles au mobile). */
@Serializable
data class EleveEffectif(
    val id: String,
    val nom: String,
    val prenom: String,
    val classe: String? = null,
    val status: String = "",
)

/** `GET /classes/{classe}/effectif?jour=`. */
@Serializable
data class Effectif(val classe: String, val jour: String, val eleves: List<EleveEffectif> = emptyList())

/** Corps de `POST /eleves/{id}/notes`. */
@Serializable
data class SaisirNoteBody(
    @SerialName("matiere_id") val matiereId: String,
    val valeur: Double,
    val sur: Double? = null,
    @SerialName("date_note") val dateNote: String,
    val libelle: String? = null,
    @SerialName("devoir_id") val devoirId: String? = null,
    @SerialName("periode_id") val periodeId: String? = null,
)

const val CLIENT_MOBILE = "mobile"
