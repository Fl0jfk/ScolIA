package fr.scolia.mobile.core

/**
 * Écrans de l'appli quotidienne (D-003). Les tâches complexes restent sur le web.
 * L'ordre de l'énumération est l'ordre d'affichage dans la navigation.
 */
enum class Module(val titre: String) {
    ACCUEIL("Accueil"),
    APPEL("Appel"),
    VIE_SCOLAIRE("Vie scolaire"),
    ABSENCES("Absences"),
    NOTES("Notes"),
    AGENDA("Cahier de textes"),
    EDT("Emploi du temps"),
    MESSAGERIE("Messages"),
    PROFIL("Mon compte"),
}

/**
 * Famille de profil : sert uniquement à choisir la **source des données** (l'élève lui-même,
 * l'enfant d'une famille, ou les classes du personnel), jamais à décider des écrans du
 * personnel : ceux-ci découlent des droits renvoyés par le serveur.
 */
enum class Profil { ELEVE, FAMILLE, PERSONNEL }

data class Espace(
    val profil: Profil,
    val modules: List<Module>,
    /** Passkey à enrôler avant tout (D-009) : seul l'écran d'enrôlement est accessible. */
    val enrolementRequis: Boolean,
    /** Le personnel a une portée « mes classes / mes groupes » (cours, appel, cahier de textes). */
    val enseigne: Boolean = false,
) {
    /** Onglets de la barre du bas (4 au plus + « Plus »). */
    val onglets: List<Module> get() = modules.take(4)
    val autres: List<Module> get() = modules.drop(4)
    fun contient(m: Module) = m in modules
}

/**
 * Déduit les écrans d'après `GET /auth/me` et `GET /roles/mes-droits`.
 *
 * - **Personnel** (tout rôle sauf élève et parent) : écrans calculés **uniquement** à partir des
 *   actions et portées renvoyées par le serveur (D-017) ; aucun nom de rôle n'est codé en dur,
 *   un rôle libre créé par l'établissement obtient donc les bons écrans.
 * - **Élève et famille** : `droits_effectifs` ne renvoie aucun droit pour eux (garde-fou « hors
 *   rôles libres », lien légal) ; seul le type de compte (`/auth/me`) distingue ces deux cas.
 *
 * Ce n'est **que de l'affichage** : chaque appel reste contrôlé par le serveur
 * (D-003, D-017). Un module affiché à tort renverra 403, géré proprement.
 */
object Capacites {
    private const val PORTEE_ETABLISSEMENT = "etablissement"
    private val PORTEES_ENSEIGNANT = setOf("mes_classes", "mes_groupes")

    fun espacePour(me: Me, droits: List<Droit>): Espace {
        val profil = profilDe(me)
        if (me.portee == "enrolement") return Espace(profil, listOf(Module.PROFIL), true)

        val m = linkedSetOf(Module.ACCUEIL)
        var enseigne = false
        when (profil) {
            Profil.ELEVE -> m += listOf(Module.EDT, Module.AGENDA, Module.NOTES, Module.MESSAGERIE)
            Profil.FAMILLE -> m += listOf(Module.ABSENCES, Module.NOTES, Module.AGENDA, Module.MESSAGERIE, Module.EDT)
            Profil.PERSONNEL -> {
                fun a(action: String, portee: (String) -> Boolean = { true }) =
                    droits.any { it.action == action && portee(it.portee) }
                enseigne = droits.any { it.portee in PORTEES_ENSEIGNANT }
                // Appel : porté par les cours du professeur (mes classes / groupes) ou par la
                // déclaration d'absences. La liste de ses créneaux reste filtrée côté serveur.
                if ((a("absences.lire") && enseigne) || a("absences.declarer")) m += Module.APPEL
                if (a("absences.lire") { it == PORTEE_ETABLISSEMENT } || a("appel.verifier") ||
                    a("absences.justifier") || a("assiduite.signaler")
                ) {
                    m += Module.VIE_SCOLAIRE
                }
                // Saisie de notes : retirée du mobile (MB2). L'ancienne route `POST /eleves/{id}/notes` est fermée
                // (410, V0 D1) et la saisie passe par la grille d'évaluation `/eva/*`, que l'appli ne porte pas encore.
                if (enseigne) m += Module.AGENDA
                m += listOf(Module.MESSAGERIE, Module.EDT)
            }
        }
        m += Module.PROFIL
        return Espace(profil, ordreNavigation(m.sortedBy { it.ordinal }), false, enseigne)
    }

    /** Élève (compte lié à un élève) ou parent ; tout autre compte est du personnel. */
    fun profilDe(me: Me): Profil {
        val role = me.role.lowercase()
        return when {
            role == "eleve" || me.eleveId != null -> Profil.ELEVE
            role == "parent" -> Profil.FAMILLE
            else -> Profil.PERSONNEL
        }
    }

    /** Les actions du quotidien d'abord (appel pour un professeur, absences pour une famille). */
    private fun ordreNavigation(modules: List<Module>): List<Module> {
        val prioritaires = listOf(
            Module.ACCUEIL, Module.APPEL, Module.VIE_SCOLAIRE, Module.ABSENCES,
            Module.MESSAGERIE, Module.NOTES, Module.EDT, Module.AGENDA,
        )
        val tri = modules.sortedBy { mod -> prioritaires.indexOf(mod).let { if (it < 0) Int.MAX_VALUE else it } }
        // « Mon compte » toujours en dernier.
        return tri.filter { it != Module.PROFIL } + modules.filter { it == Module.PROFIL }
    }
}

/**
 * Bandeaux de sécurité de la coquille (M3 et M5 de MB1), calculés ici pour être testés hors Android.
 * Ils informent sans bloquer : le serveur reste seul juge de la durée de session (MB0, DS2).
 */
enum class Avertissement(val titre: String, val texte: String) {
    /** Session ouverte sans passkey : le serveur lui donne la durée du web (quelques heures), pas celle du mobile. */
    SESSION_COURTE(
        "Session courte",
        "Vous êtes connecté sans passkey : la session expirera dans quelques heures. Connectez-vous avec une passkey (à créer dans « Mon compte » si besoin) pour rester connecté sur ce téléphone.",
    ),

    /** Aucun code ni biométrie sur l'appareil : ScolIA ne peut pas se verrouiller (D-011). */
    APPAREIL_SANS_VERROU(
        "Téléphone sans verrouillage",
        "Ce téléphone n'a ni code ni empreinte : ScolIA ne peut pas se verrouiller. Activez un verrouillage d'écran dans les réglages pour protéger les données des élèves.",
    );

    companion object {
        fun pour(me: Me, appareilSansVerrou: Boolean): List<Avertissement> = buildList {
            if (me.portee == "complete" && me.assurance != "passkey") add(SESSION_COURTE)
            if (appareilSansVerrou) add(APPAREIL_SANS_VERROU)
        }
    }
}
