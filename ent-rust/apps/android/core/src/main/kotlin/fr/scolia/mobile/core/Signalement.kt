package fr.scolia.mobile.core

/**
 * Signaler un incident (registre des violations de données, socle 27, complément G5) : logique d'écran pure, testée
 * hors Android. Aucune règle métier ici : le serveur valide, numérote, alerte la direction et ne rend au déclarant
 * que la référence et l'avancement (R-RG32).
 */
object Signalement {
    const val INTITULE_MAX = 200
    const val DESCRIPTION_MAX = 4000

    /**
     * Texte tel que le serveur le recevra : fins de ligne unifiées, autres caractères de contrôle (tabulation…)
     * changés en espace, car le serveur les refuse.
     */
    fun net(s: String): String = buildString {
        s.replace("\r\n", "\n").replace('\r', '\n').forEach { c -> append(if (c != '\n' && c.isISOControl()) ' ' else c) }
    }.trim()

    /** Longueur comptée comme le serveur (caractères Unicode, pas unités UTF-16 : un emoji compte pour un). */
    private fun longueur(s: String): Int = net(s).let { it.codePointCount(0, it.length) }

    /** Même règle que le serveur : une phrase de 3 à 200 caractères. `null` : recevable. */
    fun erreurIntitule(intitule: String): String? {
        val n = longueur(intitule)
        return when {
            n == 0 -> "Dites en une phrase ce qui s'est passé."
            n < 3 -> "Une phrase un peu plus longue, s'il vous plaît (3 caractères au moins)."
            n > INTITULE_MAX -> "Une phrase de $INTITULE_MAX caractères au plus ; le reste va dans les précisions."
            else -> null
        }
    }

    /** Précisions facultatives : vides, ou de 3 à 4 000 caractères. */
    fun erreurDescription(description: String): String? {
        val n = longueur(description)
        return if (n in 1..2) "Précisions : 3 caractères au moins, ou laissez vide." else if (n > DESCRIPTION_MAX) "Précisions : $DESCRIPTION_MAX caractères au plus." else null
    }

    fun corps(intitule: String, description: String, sansDonneePersonnelle: Boolean) = DeclarationIncidentBody(
        intitule = net(intitule),
        description = net(description).ifEmpty { null },
        incidentSecurite = sansDonneePersonnelle,
    )

    /** Intention d'écriture (clé d'idempotence gardée après un échec incertain) : le même contenu, la même intention. */
    fun intention(corps: DeclarationIncidentBody): String =
        "signaler:" + corps.intitule + "\u001F" + corps.description.orEmpty() + "\u001F" + corps.incidentSecurite

    fun libelleAvancement(avancement: String): String = when (avancement) {
        "recue" -> "Reçue, en attente de traitement"
        "en_cours" -> "En cours de traitement"
        "close" -> "Traitée"
        else -> "Avancement inconnu"
    }
}
