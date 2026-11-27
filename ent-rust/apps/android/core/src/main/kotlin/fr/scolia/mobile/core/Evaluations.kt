package fr.scolia.mobile.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

// Saisie des notes sur la grille d'évaluation `/eva/*` (socle 11, palier « notes eva » ; socle 21, lot V3).
// Copie des DTO de `ent-types/src/evaluations_v3.rs`. **Aucun calcul de moyenne ici** : moyennes et statistiques
// viennent du moteur unique du serveur, en millièmes. Le seul calcul local est l'analyse du texte d'une case, copie
// exacte de `analyser_cellule` (vecteurs partagés `contrat/mobile/regles/analyse_cellule.json`) : elle montre tout
// de suite ce qui sera compris ; le serveur reste juge (droit, période, version).

/** Un cours auquel le compte a accès (`GET /eva/enseignements`). `droit` : `saisie` ou `lecture`. */
@Serializable
data class EnseignementEva(
    @SerialName("cours_id") val coursId: String,
    val libelle: String,
    @SerialName("cible_code") val cibleCode: String,
    @SerialName("matiere_id") val matiereId: String? = null,
    @SerialName("sans_matiere") val sansMatiere: Boolean,
    val droit: String,
) {
    val enSaisie: Boolean get() = droit == "saisie"
}

/** Une période de notes du cours (`GET /eva/cours/{id}/periodes`). */
@Serializable
data class PeriodeEva(
    val id: String,
    val code: String,
    val libelle: String,
    @SerialName("date_debut") val dateDebut: String,
    @SerialName("date_fin") val dateFin: String,
) {
    /** Dates ISO : la comparaison de chaînes suffit. */
    fun contient(jour: String): Boolean = dateDebut <= jour && jour <= dateFin
}

@Serializable
data class StatsEva(
    val effectif: Int,
    @SerialName("moyenne_m") val moyenneM: Long,
    @SerialName("min_m") val minM: Long,
    @SerialName("max_m") val maxM: Long,
    @SerialName("mediane_m") val medianeM: Long,
)

/** Une colonne de la grille : une évaluation. */
@Serializable
data class EvaluationEva(
    val id: String,
    val titre: String,
    @SerialName("date_eval") val dateEval: String,
    @SerialName("bareme_m") val baremeM: Long,
    @SerialName("coef_m") val coefM: Long,
    val comptage: String,
    @SerialName("hors_bareme_autorise") val horsBaremeAutorise: Boolean,
    /** `brouillon`, `ouverte`, `annulee`. */
    val etat: String,
    @SerialName("motif_annulation") val motifAnnulation: String? = null,
    val version: Int,
    val publication: String,
    @SerialName("publication_prevue_le") val publicationPrevueLe: String? = null,
    @SerialName("publiee_le") val publieeLe: String? = null,
    @SerialName("retiree_le") val retireeLe: String? = null,
    @SerialName("en_cours_de_correction") val enCoursDeCorrection: Boolean,
    @SerialName("sous_matiere_id") val sousMatiereId: String? = null,
    val restreinte: Boolean,
    val commune: Boolean,
    val annoncee: Boolean,
    @SerialName("a_des_resultats") val aDesResultats: Boolean,
    @SerialName("peut_modifier") val peutModifier: Boolean,
    @SerialName("retrait_possible") val retraitPossible: Boolean,
    val stats: StatsEva? = null,
) {
    val ouverte: Boolean get() = etat == "ouverte"
    val publiee: Boolean get() = publieeLe != null
}

/** Une case : le résultat d'un élève à une évaluation. */
@Serializable
data class CelluleEva(
    val concerne: Boolean,
    val statut: String? = null,
    @SerialName("valeur_m") val valeurM: Long? = null,
    val version: Int? = null,
    @SerialName("resultat_id") val resultatId: String? = null,
    @SerialName("saisie_ouverte") val saisieOuverte: Boolean,
    @SerialName("rattrapage_autorise") val rattrapageAutorise: Boolean,
    @SerialName("hors_periode") val horsPeriode: Boolean,
    val corrige: Boolean,
    val commentaire: Boolean,
)

@Serializable
data class LigneEleveEva(
    @SerialName("eleve_id") val eleveId: String,
    val nom: String,
    val prenom: String,
    val mouvement: String? = null,
    /** Même ordre que [GrilleEva.evaluations]. */
    val cellules: List<CelluleEva>,
    @SerialName("moyenne_m") val moyenneM: Long? = null,
    @SerialName("nb_comptees") val nbComptees: Int,
)

/** `GET /eva/grille?cours_id=…&periode_id=…`. */
@Serializable
data class GrilleEva(
    val enseignement: EnseignementEva,
    val periode: PeriodeEva,
    val jour: String,
    @SerialName("peut_saisir") val peutSaisir: Boolean,
    @SerialName("peut_corriger") val peutCorriger: Boolean,
    @SerialName("echelle_m") val echelleM: Long,
    @SerialName("pas_m") val pasM: Long,
    @SerialName("delai_publication_max_jours") val delaiPublicationMaxJours: Int? = null,
    val evaluations: List<EvaluationEva>,
    val eleves: List<LigneEleveEva>,
    @SerialName("moyenne_classe") val moyenneClasse: StatsEva? = null,
    val avertissements: List<String>,
)

/** Une case à écrire : le **texte tapé** (le serveur l'analyse) et la version que l'écran avait lue. */
@Serializable
data class CelluleSaisieEva(
    @SerialName("evaluation_id") val evaluationId: String,
    @SerialName("eleve_id") val eleveId: String,
    val texte: String,
    /** `null` : l'écran croyait la case vide ; sinon la version lue (une autre version en base = conflit). */
    @SerialName("version_attendue") val versionAttendue: Int? = null,
)

/** Corps de `POST /eva/saisie`. */
@Serializable
data class SaisieEvaBody(
    @SerialName("cours_id") val coursId: String,
    val cellules: List<CelluleSaisieEva>,
)

/** Ce que contient la case au moment d'un conflit. */
@Serializable
data class ConflitEva(
    val statut: String,
    @SerialName("valeur_m") val valeurM: Long? = null,
    val version: Int,
    @SerialName("par_nom") val parNom: String? = null,
    val le: String? = null,
)

/** Issue d'une case : `ecrit`, `inchange`, `vide`, `conflit`, `refuse`. */
@Serializable
data class CelluleRetourEva(
    @SerialName("evaluation_id") val evaluationId: String,
    @SerialName("eleve_id") val eleveId: String,
    val issue: String,
    val version: Int? = null,
    val statut: String? = null,
    @SerialName("valeur_m") val valeurM: Long? = null,
    val message: String? = null,
    val conflit: ConflitEva? = null,
)

@Serializable
data class SaisieEvaRetour(val cellules: List<CelluleRetourEva>)

/** Corps de `POST /eva/evaluations` : ce que l'appli règle (le reste se règle sur le web). */
@Serializable
data class CreerEvaluationEvaBody(
    @SerialName("cours_id") val coursId: String,
    val titre: String,
    @SerialName("date_eval") val dateEval: String,
    @SerialName("bareme_m") val baremeM: Long,
    @SerialName("coef_m") val coefM: Long,
)

/** Corps de `POST /eva/evaluations/{id}/publication` : `maintenant` ou `retirer` (motif écrit). */
@Serializable
data class PublierEvaBody(val action: String, val motif: String? = null)

/** Résultat de l'analyse d'un texte de case (copie de `ent_types::evaluations_v3::Analyse`). */
sealed class AnalyseCellule {
    /** Case vide : rien à écrire (un résultat ne se supprime jamais, R-V6). */
    data object Vide : AnalyseCellule()
    data class Statut(val statut: String) : AnalyseCellule()
    data class Note(val valeurM: Long) : AnalyseCellule()
    data class Erreur(val message: String) : AnalyseCellule()
}

/** Règles de saisie partagées avec le serveur (`ent-types/src/evaluations_v3.rs`), au caractère près. */
object SaisieNotes {
    /** Codes rapides au clavier : (code, statut, libellé). */
    val CODES_RAPIDES = listOf(
        Triple("abs", "absent", "Absent"),
        Triple("ar", "a_rattraper", "À rattraper"),
        Triple("disp", "dispense", "Dispensé"),
        Triple("nr", "non_rendu", "Non rendu"),
        Triple("nn", "non_note", "Non noté"),
    )

    fun libelleStatut(statut: String): String = when (statut) {
        "note" -> "Note"
        "absent" -> "Absent"
        "a_rattraper" -> "À rattraper"
        "dispense" -> "Dispensé"
        "non_rendu" -> "Non rendu"
        "non_note" -> "Non noté"
        "annule" -> "Annulé"
        else -> "Statut inconnu"
    }

    fun codeStatut(statut: String): String = when (statut) {
        "absent" -> "abs"
        "a_rattraper" -> "ar"
        "dispense" -> "disp"
        "non_rendu" -> "nr"
        "non_note" -> "nn"
        "annule" -> "ann"
        else -> ""
    }

    private fun sansAccentsMinuscules(t: String): String = buildString {
        for (c in t.trim().lowercase()) {
            append(
                when (c) {
                    'à', 'â', 'ä' -> 'a'
                    'é', 'è', 'ê', 'ë' -> 'e'
                    'î', 'ï' -> 'i'
                    'ô', 'ö' -> 'o'
                    'ù', 'û', 'ü' -> 'u'
                    'ç' -> 'c'
                    else -> c
                },
            )
        }
    }

    fun statutDuCode(texte: String): String? = when (sansAccentsMinuscules(texte).trimEnd('.')) {
        "abs", "absent", "absente" -> "absent"
        "ar", "a rattraper", "arattraper", "rattrapage" -> "a_rattraper"
        "disp", "dispense", "dispensee" -> "dispense"
        "nr", "non rendu", "nonrendu", "n.r" -> "non_rendu"
        "nn", "non note", "nonnote", "n.n" -> "non_note"
        else -> null
    }

    private fun chiffres(s: String) = s.all { it in '0'..'9' }

    /** Nombre décimal en millièmes (« 12 », « 12,5 », « 12.25 »), au plus trois décimales. */
    fun milliemesDepuisTexte(texte: String): Long? {
        val t = texte.trim().replace(',', '.')
        if (t.isEmpty() || t.startsWith('+')) return null
        val i = t.indexOf('.')
        val entier = if (i < 0) t else t.substring(0, i)
        val frac = if (i < 0) "" else t.substring(i + 1)
        if (entier.isEmpty() && frac.isEmpty()) return null
        if (!chiffres(entier) || !chiffres(frac)) return null
        if (entier.length > 6 || frac.length > 3) return null
        val e = if (entier.isEmpty()) 0L else entier.toLong()
        return e * 1000 + frac.padEnd(3, '0').toLong()
    }

    /** Millièmes à la française (« 12,5 », « 8 », « 13,125 »). */
    fun formater(m: Long): String {
        val a = kotlin.math.abs(m)
        val e = a / 1000
        val f = a % 1000
        val s = StringBuilder()
        if (m < 0) s.append('-')
        s.append(e)
        if (f != 0L) s.append(',').append(f.toString().padStart(3, '0').trimEnd('0'))
        return s.toString()
    }

    /** Analyse le texte d'une case. **Le serveur reste juge** : il rejoue cette même règle puis contrôle le reste. */
    fun analyser(texte: String, baremeM: Long, horsBaremeAutorise: Boolean, pasM: Long): AnalyseCellule {
        val t = texte.trim()
        if (t.isEmpty()) return AnalyseCellule.Vide
        statutDuCode(t)?.let { return AnalyseCellule.Statut(it) }
        if (t.startsWith('-')) return AnalyseCellule.Erreur("une note ne peut pas être négative")
        val m = milliemesDepuisTexte(t)
            ?: return AnalyseCellule.Erreur("« ${premiersCaracteres(t, 20)} » n'est ni une note ni un code (abs, ar, disp, nr, nn)")
        if (m > baremeM && !horsBaremeAutorise) {
            return AnalyseCellule.Erreur("${formater(m)} dépasse le barème (${formater(baremeM)}) : vérifiez le barème de l'évaluation")
        }
        if (pasM > 1 && m % pasM != 0L) return AnalyseCellule.Erreur("la note doit être un multiple de ${formater(pasM)}")
        return AnalyseCellule.Note(m)
    }

    /** Les `n` premiers caractères Unicode (comme `chars().take(n)` en Rust, pas des unités UTF-16). */
    private fun premiersCaracteres(t: String, n: Int): String {
        val fin = if (t.codePointCount(0, t.length) <= n) t.length else t.offsetByCodePoints(0, n)
        return t.substring(0, fin)
    }

    /** Ce que la case affiche avant toute frappe : la note, le code du statut, ou rien. */
    fun texteDe(c: CelluleEva): String = when {
        c.statut == null -> ""
        c.statut == "note" && c.valeurM != null -> formater(c.valeurM)
        else -> codeStatut(c.statut)
    }

    /** Phrase courte d'une case lue (liste des élèves, conflit). */
    fun resume(statut: String?, valeurM: Long?, baremeM: Long): String = when {
        statut == null -> "vide"
        statut == "note" && valeurM != null -> "${formater(valeurM)} / ${formater(baremeM)}"
        else -> libelleStatut(statut)
    }
}

/**
 * Brouillon de saisie d'**une** évaluation sur l'écran mobile : textes tapés, lot à envoyer, issues reçues.
 * Tenu hors Android pour être testé ; l'écran n'en est que l'affichage.
 *
 * - Seules les cases **modifiées**, comprises (note ou statut) et ouvertes partent ; une case vidée n'efface rien
 *   (R-V6) et le dit.
 * - Chaque case part avec la version lue : un autre auteur entre-temps donne un **conflit**, jamais un écrasement.
 *   L'écran propose alors « garder la mienne » (renvoi sur la version actuelle) ou « reprendre celle du serveur ».
 */
class BrouillonSaisie(val grille: GrilleEva, val evaluationId: String) {
    private val rang = grille.evaluations.indexOfFirst { it.id == evaluationId }.also {
        require(it >= 0) { "évaluation absente de la grille" }
    }
    val evaluation: EvaluationEva = grille.evaluations[rang]

    /**
     * Case actuelle de chaque élève (mise à jour par les retours du serveur). Une ligne sans case pour cette colonne
     * (réponse dégénérée, cache ancien) est ignorée plutôt que de faire planter l'écran (contre-revue M8).
     */
    private val cases = grille.eleves.mapNotNull { l -> l.cellules.getOrNull(rang)?.let { l.eleveId to it } }.toMap().toMutableMap()
    private val textes = mutableMapOf<String, String>()
    private val conflits = mutableMapOf<String, ConflitEva>()
    private val refus = mutableMapOf<String, String>()
    /** Cases à relire avant tout nouvel envoi (conflit sans état actuel). */
    private val bloquees = mutableSetOf<String>()

    /** Élèves concernés par l'évaluation, dans l'ordre de la grille (les non concernés ne sont jamais saisis). */
    val eleves: List<LigneEleveEva> = grille.eleves.filter { it.cellules.getOrNull(rang)?.concerne == true }.distinctBy { it.eleveId }

    fun caseDe(eleveId: String): CelluleEva? = cases[eleveId]
    fun texte(eleveId: String): String = textes[eleveId] ?: cases[eleveId]?.let(SaisieNotes::texteDe).orEmpty()
    fun conflit(eleveId: String): ConflitEva? = conflits[eleveId]
    fun refus(eleveId: String): String? = refus[eleveId]

    /** La case peut-elle être écrite d'ici ? (Une correction motivée après l'arrêt reste sur le web.) */
    fun modifiable(eleveId: String): Boolean =
        grille.peutSaisir && evaluation.ouverte && cases[eleveId]?.let { it.concerne && it.saisieOuverte } == true

    fun taper(eleveId: String, texte: String) {
        if (!modifiable(eleveId)) return
        textes[eleveId] = texte
        refus.remove(eleveId)
    }

    fun analyse(eleveId: String): AnalyseCellule =
        SaisieNotes.analyser(texte(eleveId), evaluation.baremeM, evaluation.horsBaremeAutorise, grille.pasM)

    /** Texte changé par rapport à la case lue. */
    fun modifiee(eleveId: String): Boolean {
        val t = textes[eleveId] ?: return false
        return t.trim() != cases[eleveId]?.let(SaisieNotes::texteDe).orEmpty()
    }

    /** Une case modifiée qu'on vide : rien ne s'efface (R-V6) ; l'écran le signale. */
    fun videeSansEffet(eleveId: String): Boolean =
        modifiee(eleveId) && analyse(eleveId) is AnalyseCellule.Vide && cases[eleveId]?.statut != null

    /** Cases prêtes à partir : modifiées, comprises, ouvertes, sans conflit en attente d'un choix. */
    fun aEnvoyer(): List<CelluleSaisieEva> = eleves.mapNotNull { l ->
        val id = l.eleveId
        if (!modifiable(id) || !modifiee(id) || conflits.containsKey(id) || id in bloquees) return@mapNotNull null
        when (analyse(id)) {
            is AnalyseCellule.Note, is AnalyseCellule.Statut -> CelluleSaisieEva(
                evaluationId,
                id,
                texte(id).trim(),
                cases[id]?.version,
            )
            else -> null
        }
    }

    /** Cases tapées mais refusées par l'analyse locale (non envoyées). */
    fun enErreur(): List<String> = eleves.map { it.eleveId }.filter { modifiee(it) && analyse(it) is AnalyseCellule.Erreur }

    fun lot(): SaisieEvaBody = SaisieEvaBody(grille.enseignement.coursId, aEnvoyer())

    /** Intention d'idempotence du lot : le contenu exact (élève, texte, version), pas un hachage (D-069). */
    fun intention(lot: SaisieEvaBody): String =
        famille() + lot.cellules.sortedBy { it.eleveId }.joinToString(";") { "${it.eleveId}=${it.texte}@${it.versionAttendue ?: "-"}" }

    fun famille(): String = "eva-saisie:$evaluationId:"

    /**
     * Applique les issues du serveur, case par case. Rend le nombre de cases écrites. `envoye` : le lot parti ; une
     * case retapée pendant l'envoi garde sa nouvelle frappe (contre-revue M3), qui repartira sur la nouvelle version.
     */
    fun appliquer(retour: SaisieEvaRetour, envoye: SaisieEvaBody): Int {
        val textesEnvoyes = envoye.cellules.filter { it.evaluationId == evaluationId }.associate { it.eleveId to it.texte }
        var ecrites = 0
        for (r in retour.cellules) {
            if (r.evaluationId != evaluationId) continue
            val id = r.eleveId
            val avant = cases[id] ?: continue
            when (r.issue) {
                "ecrit", "inchange" -> {
                    cases[id] = avant.copy(statut = r.statut ?: avant.statut, valeurM = r.valeurM, version = r.version ?: avant.version)
                    if (textes[id]?.trim() == textesEnvoyes[id]) textes.remove(id)
                    conflits.remove(id)
                    refus.remove(id)
                    if (r.issue == "ecrit") ecrites++
                }
                "vide" -> {
                    textes.remove(id)
                    conflits.remove(id)
                }
                "conflit" -> {
                    val c = r.conflit
                    if (c != null) {
                        conflits[id] = c
                        refus.remove(id)
                    } else {
                        // Conflit sans l'état actuel (case disparue) : ne pas renvoyer en boucle, relire la grille.
                        refus[id] = r.message ?: "La case a changé : revenez à la liste pour relire la grille."
                        bloquees += id
                    }
                }
                else -> refus[id] = r.message ?: "Case refusée par le serveur."
            }
        }
        return ecrites
    }

    /** « Reprendre celle du serveur » : la case prend la valeur du conflit, la frappe est abandonnée. */
    fun reprendreServeur(eleveId: String) {
        val c = conflits.remove(eleveId) ?: return
        cases[eleveId]?.let { cases[eleveId] = it.copy(statut = c.statut, valeurM = c.valeurM, version = c.version) }
        textes.remove(eleveId)
    }

    /**
     * « Garder la mienne » : la frappe repart au prochain envoi, sur la version que le serveur a montrée (choix
     * explicite : sans lui, une case en conflit ne repart jamais).
     */
    fun garderLaMienne(eleveId: String) {
        val c = conflits.remove(eleveId) ?: return
        cases[eleveId]?.let { cases[eleveId] = it.copy(statut = c.statut, valeurM = c.valeurM, version = c.version) }
    }

    val nbConflits: Int get() = conflits.size
}
