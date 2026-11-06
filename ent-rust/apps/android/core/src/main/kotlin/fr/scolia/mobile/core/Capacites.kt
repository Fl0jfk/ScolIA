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
    SAISIE_NOTES("Saisir des notes"),
    AGENDA("Cahier de textes"),
    EDT("Emploi du temps"),
    MESSAGERIE("Messages"),
    PROFIL("Mon compte"),
}

/** Famille de profil : pilote l'accueil et la façon de choisir l'élève concerné. */
enum class Profil { ELEVE, FAMILLE, PROFESSEUR, VIE_SCOLAIRE, DIRECTION, PERSONNEL }

data class Espace(
    val profil: Profil,
    val modules: List<Module>,
    /** Passkey à enrôler avant tout (D-009) : seul l'écran d'enrôlement est accessible. */
    val enrolementRequis: Boolean,
) {
    /** Onglets de la barre du bas (4 au plus + « Plus »). */
    val onglets: List<Module> get() = modules.take(4)
    val autres: List<Module> get() = modules.drop(4)
    fun contient(m: Module) = m in modules
}

/**
 * Déduit les écrans d'après `GET /auth/me` et `GET /roles/mes-droits`.
 *
 * Ce n'est **que de l'affichage** : chaque appel reste contrôlé par le serveur
 * (D-003, D-017). Un module affiché à tort renverra 403, géré proprement.
 */
object Capacites {
    private val ROLES_DIRECTION = setOf("admin", "direction")
    private val ROLES_VIE_SCOLAIRE = setOf("cpe")

    fun espacePour(me: Me, droits: List<Droit>): Espace {
        val enrolement = me.portee == "enrolement"
        if (enrolement) return Espace(profilDe(me, droits), listOf(Module.PROFIL), true)

        val actions = droits.map { it.action }.toSet()
        val profil = profilDe(me, droits)
        val m = linkedSetOf(Module.ACCUEIL)
        when (profil) {
            Profil.ELEVE -> m += listOf(Module.EDT, Module.AGENDA, Module.NOTES, Module.MESSAGERIE)
            Profil.FAMILLE -> m += listOf(Module.ABSENCES, Module.NOTES, Module.AGENDA, Module.MESSAGERIE, Module.EDT)
            else -> {
                if (profil == Profil.PROFESSEUR) m += Module.APPEL
                if ("absences.lire" in actions || "appel.verifier" in actions ||
                    profil == Profil.VIE_SCOLAIRE || profil == Profil.DIRECTION
                ) {
                    m += Module.VIE_SCOLAIRE
                }
                m += Module.MESSAGERIE
                if ("notes.saisir" in actions || profil == Profil.PROFESSEUR) m += Module.SAISIE_NOTES
                m += Module.EDT
                if (profil == Profil.PROFESSEUR) m += Module.AGENDA
            }
        }
        m += Module.PROFIL
        return Espace(profil, m.sortedBy { it.ordinal }.let(::ordreNavigation), false)
    }

    fun profilDe(me: Me, droits: List<Droit>): Profil {
        val role = me.role.lowercase()
        return when {
            role == "eleve" || me.eleveId != null -> Profil.ELEVE
            role == "parent" -> Profil.FAMILLE
            role == "prof" -> Profil.PROFESSEUR
            role in ROLES_DIRECTION -> Profil.DIRECTION
            role in ROLES_VIE_SCOLAIRE -> Profil.VIE_SCOLAIRE
            // Rôles libres de l'établissement (D-017) : on regarde les droits.
            droits.any { it.action == "appel.verifier" || it.action == "absences.lire" && it.portee == "etablissement" } ->
                Profil.VIE_SCOLAIRE
            else -> Profil.PERSONNEL
        }
    }

    /** Les actions du quotidien d'abord (appel pour un professeur, absences pour une famille). */
    private fun ordreNavigation(modules: List<Module>): List<Module> {
        val prioritaires = listOf(
            Module.ACCUEIL, Module.APPEL, Module.VIE_SCOLAIRE, Module.ABSENCES,
            Module.MESSAGERIE, Module.NOTES, Module.EDT, Module.AGENDA, Module.SAISIE_NOTES,
        )
        val tri = modules.sortedBy { mod -> prioritaires.indexOf(mod).let { if (it < 0) Int.MAX_VALUE else it } }
        // « Mon compte » toujours en dernier.
        return tri.filter { it != Module.PROFIL } + modules.filter { it == Module.PROFIL }
    }
}
