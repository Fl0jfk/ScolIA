package fr.scolia.mobile.core

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.nullable
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.descriptors.buildClassSerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

// « Mes autres comptes » (transverse individu, migration 279, `docs/socles/TRANSVERSE_INDIVIDU.md` §10 et §11).
//
// Ce que le compte voit de son individu, les demandes reçues d'un autre compte (confirmer ou refuser) et la
// déclaration « cet autre compte est aussi moi ». Le serveur décide de tout ; ce fichier copie les DTO
// (`ent-types/src/individu.rs`), appelle les trois routes et porte les textes de l'écran web
// (`ent-web-app/src/modules/individu/modele.rs`), à garder identiques.
//
// Les routes de la direction (propositions, relier, lever) ne sont pas portées : relier deux fiches demande la
// recherche de fiches et un motif, c'est une tâche de bureau qui reste sur le web (D-003).
//
// Fichier à part (et fonctions d'extension) pour ne pas toucher `ApiClient`/`Modeles`, modifiés en parallèle.

// ---------- Modèles (copie de `ent-types::individu`) ----------

/** Une demande reçue d'un autre compte, à confirmer avant `expireLe`. */
@Serializable
data class DeclarationRecue(
    val id: String,
    /** Nom affiché du compte déclarant (la personne qui déclare se fait connaître de son autre compte). */
    val declarant: String,
    @SerialName("declare_le") val declareLe: String,
    @SerialName("expire_le") val expireLe: String,
)

/** Un lien actif de l'individu : quand et comment, **jamais** l'autre fiche. */
@Serializable
data class MonLienIndividu(
    val origine: String,
    @SerialName("motif_code") val motifCode: String,
    @SerialName("pose_le") val poseLe: String,
)

/** `GET /dossier/individu/moi`. `liens` est absent d'un serveur antérieur aux écrans (PR 172). */
@Serializable
data class MonIndividu(
    @SerialName("autres_comptes") val autresComptes: Long,
    @SerialName("fiches_reliees") val fichesReliees: Long,
    @SerialName("declarations_recues") val declarationsRecues: List<DeclarationRecue> = emptyList(),
    val liens: List<MonLienIndividu> = emptyList(),
)

@Serializable
internal data class DeclarationIndividuBody(val email: String)

@Serializable
internal data class ReponseDeclarationBody(val confirmer: Boolean)

/** Corps vide (202) : rien à lire. Le décodeur vérifie seulement qu'il ne reste rien. */
internal object CorpsVide : KSerializer<Unit> {
    override val descriptor: SerialDescriptor = buildClassSerialDescriptor("CorpsVide")
    override fun deserialize(decoder: Decoder) = Unit
    override fun serialize(encoder: Encoder, value: Unit) = Unit
}

// ---------- Appels ----------

object CheminsIndividu {
    const val MOI = "/dossier/individu/moi"
    const val DECLARATIONS = "/dossier/individu/declarations"
    fun reponse(id: String) = "/dossier/individu/declarations/${ApiClient.seg(id)}/reponse"
}

/** Ce que l'établissement reconnaît, et les demandes reçues. Jamais mis en cache (demandes datées, réponses). */
suspend fun ApiClient.individuMoi(): MonIndividu = get(CheminsIndividu.MOI, MonIndividu.serializer())

/**
 * « Ce compte est aussi moi » : 202 **identique** que l'adresse soit connue ou non (anti-énumération) ; l'appelant
 * affiche toujours [Individu.MESSAGE_DECLARATION]. Passer l'adresse déjà contrôlée par [Individu.adresseAEnvoyer].
 */
suspend fun ApiClient.declarerAutreCompte(email: String, cle: String? = null) {
    post(
        CheminsIndividu.DECLARATIONS,
        json.encodeToString(DeclarationIndividuBody.serializer(), DeclarationIndividuBody(email)),
        CorpsVide,
        cle = cle,
    )
}

/** Confirmer ou refuser une demande reçue. Rend l'identifiant du lien posé (confirmation), sinon `null`. */
suspend fun ApiClient.repondreDeclaration(id: String, confirmer: Boolean, cle: String? = null): String? =
    post(
        CheminsIndividu.reponse(id),
        json.encodeToString(ReponseDeclarationBody.serializer(), ReponseDeclarationBody(confirmer)),
        String.serializer().nullable,
        cle = cle,
    )

// ---------- Logique d'écran (miroir de `modules/individu/modele.rs`) ----------

object Individu {
    /** Ce qu'un lien change, rappelé avant de confirmer (principe 1 de la conception). */
    const val EFFET_LIEN = "Les deux fiches seront traitées comme une seule personne : cela ajoute des refus " +
        "(une personne ne décide pas pour elle-même ni pour son enfant, même depuis un autre compte) et n'ouvre aucun " +
        "accès. Les comptes concernés sont prévenus. Seule la direction peut lever ce lien, jamais la personne concernée."

    /** Réponse affichée après une déclaration, **identique** que l'adresse soit connue ou non. */
    const val MESSAGE_DECLARATION = "Demande enregistrée. Si cette adresse est celle d'un compte de " +
        "l'établissement, ce compte reçoit une demande à confirmer dans les 24 heures. Connectez-vous avec lui pour " +
        "répondre."

    const val POURQUOI = "Certaines décisions ne se prennent jamais pour soi-même ni pour son enfant : une aide " +
        "accordée à son propre enfant, une seconde validation de sa propre demande, une absence que l'on validerait " +
        "soi-même. Reconnaître vos comptes comme une seule personne empêche qu'un second compte contourne ces règles. " +
        "Cela n'ouvre aucun accès supplémentaire, et vous êtes prévenu(e) de chaque lien posé ou levé."

    const val INTRO_DEMANDES = "Un autre compte de l'établissement déclare être aussi vous. Ne confirmez que si ce " +
        "compte est bien le vôtre."

    /** Rappel avant un refus : il est définitif pour cette demande et bloque ce compte 30 jours. */
    const val EFFET_REFUS = "Rien ne sera relié. Ce compte ne pourra plus vous le redemander pendant 30 jours ; " +
        "vous pourrez toujours le déclarer vous-même."

    const val AIDE_DECLARER = "Saisissez l'adresse e-mail de votre autre compte dans l'établissement. La demande se " +
        "confirme ensuite depuis cet autre compte, dans les 24 heures : sans confirmation, elle ne vaut rien. Cinq " +
        "demandes par jour au plus."

    const val LIEN_FAUX = "Un lien vous semble faux ? Signalez-le au secrétariat : seule la direction peut le lever."

    const val AUCUNE_DEMANDE = "Quand un autre compte déclare être aussi vous, la demande apparaît ici pendant 24 heures."

    fun libelleOrigine(code: String): String = when (code) {
        "declaration_direction" -> "Relié par l'établissement"
        "declaration_confirmee" -> "Déclaré puis confirmé depuis les deux comptes"
        "proposition_acceptee" -> "Ressemblance confirmée par l'établissement"
        else -> "Lien enregistré"
    }

    /** Motifs de `ent-types::individu::MOTIFS_LIEN` ; une valeur nouvelle a un libellé neutre. */
    fun libelleMotifLien(code: String): String = when (code) {
        "deux_comptes" -> "Deux comptes de la même personne"
        "fiche_sans_compte" -> "Fiche sans compte de la même personne"
        "doublon_agent" -> "Deux fiches de personnel de la même personne"
        "autre" -> "Autre motif"
        else -> "Motif non listé"
    }

    fun resumeMoi(m: MonIndividu): String {
        val autres = when {
            m.autresComptes <= 0 -> return "Aucun autre compte n'est reconnu comme le vôtre dans cet établissement."
            m.autresComptes == 1L -> "1 autre compte"
            else -> "${m.autresComptes} autres comptes"
        }
        return "$autres de l'établissement vous appartiennent aussi : l'établissement vous reconnaît comme une seule personne."
    }

    fun resumeFiches(m: MonIndividu): String? =
        if (m.fichesReliees > 1) {
            "${m.fichesReliees} fiches de l'établissement vous désignent (fiches de personnel, de responsable ou d'élève)."
        } else {
            null
        }

    const val MSG_SA_PROPRE_ADRESSE = "C'est l'adresse de ce compte : saisissez celle de votre autre compte."

    /**
     * Contrôle local d'une adresse (le serveur refait le même) : l'adresse normalisée, ou le message d'erreur.
     * `monEmail` : l'adresse du compte connecté. Le serveur ignorerait la demande en silence (réponse neutre) ; la
     * refuser ici ne révèle rien et évite d'afficher « Demande enregistrée » pour rien (proposition du clavier).
     */
    fun adresseAEnvoyer(saisie: String, monEmail: String? = null): Result<String> {
        val e = saisie.trim().lowercase()
        if (e.isEmpty()) return Result.failure(IllegalArgumentException("Saisissez l'adresse e-mail de votre autre compte."))
        // Longueur en octets, comme `str::len` côté serveur.
        if (e.toByteArray(Charsets.UTF_8).size > 254 || !e.contains('@') || e.startsWith('@') || e.endsWith('@') || e.any { it.isWhitespace() }) {
            return Result.failure(IllegalArgumentException("Cette adresse e-mail n'est pas valide."))
        }
        if (monEmail != null && e == monEmail.trim().lowercase()) return Result.failure(IllegalArgumentException(MSG_SA_PROPRE_ADRESSE))
        return Result.success(e)
    }

    fun messageReponse(confirmer: Boolean): String =
        if (confirmer) {
            "Confirmé : les deux comptes sont reconnus comme une seule personne."
        } else {
            "Demande refusée : rien n'a été relié."
        }

    /** Liens les plus récents d'abord (ordre stable). */
    fun trierLiens(v: List<MonLienIndividu>): List<MonLienIndividu> =
        v.sortedWith(compareByDescending<MonLienIndividu> { instant(it.poseLe) ?: Instant.EPOCH }.thenBy { it.origine })

    /** Demandes qui expirent le plus tôt d'abord. */
    fun trierDemandes(v: List<DeclarationRecue>): List<DeclarationRecue> =
        v.sortedWith(compareBy<DeclarationRecue> { instant(it.expireLe) ?: Instant.MAX }.thenBy { it.id })

    fun detailLien(l: MonLienIndividu, zone: ZoneId = ZoneId.systemDefault()): String =
        "${libelleMotifLien(l.motifCode)} · depuis le ${dateHeure(l.poseLe, zone)}"

    fun detailDemande(d: DeclarationRecue, zone: ZoneId = ZoneId.systemDefault()): String =
        "Demandé le ${dateHeure(d.declareLe, zone)} · à confirmer avant le ${dateHeure(d.expireLe, zone)}"

    /** Clés d'intention (idempotence) : même saisie, même demande, même clé tant que l'issue est incertaine. */
    fun intentionDeclarer(email: String) = "individu-declarer:$email"
    fun intentionRepondre(id: String, confirmer: Boolean) = "individu-repondre:$id:$confirmer"

    private val FORMAT = DateTimeFormatter.ofPattern("dd/MM/yyyy 'à' HH'h'mm")

    /** Horodatage ISO du serveur (UTC) → `28/11/2026 à 05h50` dans le fuseau de l'appareil ; brut si illisible. */
    fun dateHeure(iso: String, zone: ZoneId = ZoneId.systemDefault()): String =
        instant(iso)?.atZone(zone)?.format(FORMAT) ?: iso

    private fun instant(iso: String): Instant? =
        try {
            java.time.OffsetDateTime.parse(iso).toInstant()
        } catch (e: Exception) {
            null
        }
}
