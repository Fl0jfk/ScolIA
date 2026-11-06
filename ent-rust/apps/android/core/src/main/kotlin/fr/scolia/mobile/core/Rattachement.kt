package fr.scolia.mobile.core

import java.net.URI
import java.net.URLDecoder

/**
 * Établissement mémorisé par l'appli (D-012 : un sous-domaine par établissement).
 * L'appli peut en mémoriser plusieurs (parent d'enfants dans deux établissements).
 */
data class Rattachement(
    /** Adresse de l'établissement, ex. `https://lpnb.scolia.fr`. */
    val baseUrl: String,
    /** Sous-domaine (`lpnb`), affiché en attendant le nom officiel. */
    val slug: String,
    /** Nom renvoyé par `GET /auth/etablissement`, une fois vérifié. */
    val nom: String? = null,
    /** Code d'invitation présent dans le lien ou le QR code (consommé à la connexion). */
    val invitation: String? = null,
    /** Identifiant d'établissement : labo local uniquement (pas de sous-domaine). */
    val etablissementIdLabo: String? = null,
)

/**
 * Lecture d'un lien d'invitation, d'un QR code ou d'une adresse saisie.
 *
 * Formats acceptés :
 * - `https://<etab>.scolia.fr` (adresse simple) ;
 * - `https://<etab>.scolia.fr/rejoindre?invitation=<code>` (lien d'invitation, App Links / Universal Links) ;
 * - saisie libre `lpnb` ou `lpnb.scolia.fr`.
 *
 * Refusés : `http`, autre domaine, identifiants dans l'URL, port explicite, schéma personnalisé
 * (un schéma `scolia://` pourrait être détourné par une autre appli).
 */
class LecteurRattachement(
    private val domaine: String = DOMAINE_SCOLIA,
    /** Labo local (build de développement uniquement) : autorise `http://10.0.2.2:3001`, etc. */
    private val modeLabo: Boolean = false,
) {
    sealed class Resultat {
        data class Ok(val rattachement: Rattachement) : Resultat()
        data class Refuse(val raison: String) : Resultat()
    }

    fun lire(saisie: String): Resultat {
        val t = saisie.trim()
        if (t.isEmpty()) return Resultat.Refuse("Adresse vide.")
        if (t.length > 2048) return Resultat.Refuse("Lien trop long.")

        // Saisie courte : « lpnb » ou « lpnb.scolia.fr ».
        if (!t.contains("://")) {
            val hote = t.lowercase().removeSuffix("/")
            val complet = if (hote.contains('.')) hote else "$hote.$domaine"
            return lire("https://$complet")
        }

        val uri = try {
            URI(t)
        } catch (e: Exception) {
            return Resultat.Refuse("Lien illisible.")
        }
        val schema = uri.scheme?.lowercase()
        val hote = uri.host?.lowercase() ?: return Resultat.Refuse("Lien sans adresse.")

        if (modeLabo && schema == "http" && hote in HOTES_LABO) {
            val port = if (uri.port > 0) ":${uri.port}" else ""
            val etab = parametre(uri, "etablissement_id")
            return Resultat.Ok(Rattachement("http://$hote$port", "labo", "Labo local", parametre(uri, "invitation"), etab))
        }
        if (schema != "https") return Resultat.Refuse("Seules les adresses sécurisées (https) sont acceptées.")
        if (uri.userInfo != null) return Resultat.Refuse("Lien refusé.")
        if (uri.port != -1 && uri.port != 443) return Resultat.Refuse("Lien refusé.")
        if (!hote.endsWith(".$domaine")) return Resultat.Refuse("Ce lien n'est pas une adresse ScolIA.")
        val slug = hote.removeSuffix(".$domaine")
        if (!SLUG.matches(slug)) return Resultat.Refuse("Adresse d'établissement invalide.")
        if (slug in SOUS_DOMAINES_RESERVES) return Resultat.Refuse("Ce lien n'est pas celui d'un établissement.")

        val invitation = parametre(uri, "invitation")?.takeIf { INVITATION.matches(it) }
        return Resultat.Ok(Rattachement("https://$hote", slug, null, invitation))
    }

    private fun parametre(uri: URI, nom: String): String? =
        uri.rawQuery?.split('&')?.firstNotNullOfOrNull {
            val (k, v) = it.split('=', limit = 2).let { p -> p[0] to p.getOrElse(1) { "" } }
            if (k == nom) URLDecoder.decode(v, Charsets.UTF_8) else null
        }

    companion object {
        const val DOMAINE_SCOLIA = "scolia.fr"
        private val SLUG = Regex("^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$")
        private val INVITATION = Regex("^[A-Za-z0-9_-]{8,128}$")
        private val HOTES_LABO = setOf("10.0.2.2", "127.0.0.1", "localhost")
        /** Sous-domaines techniques qui ne désignent pas un établissement. */
        private val SOUS_DOMAINES_RESERVES = setOf("www", "api", "app", "admin", "static", "cdn", "mail")
    }
}
