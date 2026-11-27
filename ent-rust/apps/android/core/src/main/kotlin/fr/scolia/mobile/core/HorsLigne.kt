package fr.scolia.mobile.core

import kotlinx.serialization.KSerializer
import java.time.Instant
import java.time.ZoneId
import java.util.concurrent.ConcurrentHashMap

/**
 * Cache de lecture hors ligne (mode minimal, lecture seule).
 *
 * Règles :
 * - on ne met en cache que des lectures (EDT, cahier de textes, notes, fils de messages) ;
 * - clé = utilisateur + établissement + chemin : aucune fuite entre comptes ;
 * - tout est effacé à la déconnexion ou à la révocation de la session ;
 * - **toute entrée a une date de fin** (DA1, en attendant les baux de MB7) : minuit pour l'EDT et l'appel
 *   (R-AS32, R-PO33 : jamais l'EDT au-delà du jour), 24 h au plus pour le reste. Une entrée échue n'est
 *   jamais servie ; elle est effacée à la lecture et à chaque purge (démarrage, reprise) ;
 * - l'implémentation Android chiffre les fichiers (clé Keystore) ; iOS : protection
 *   de fichier `complete`.
 */
interface Cache {
    fun lire(cle: String): EntreeCache?
    fun ecrire(cle: String, entree: EntreeCache)
    fun effacer(cle: String)

    /** Efface toutes les entrées échues à l'instant `maintenantMs` (purge au démarrage et à la reprise). */
    fun purgerEchues(maintenantMs: Long)
    fun toutEffacer()
}

/** `expireMs` : instant après lequel l'entrée ne doit plus être servie ni gardée. */
data class EntreeCache(val json: String, val horodatageMs: Long, val expireMs: Long) {
    fun echue(maintenantMs: Long) = maintenantMs >= expireMs
}

class CacheMemoire : Cache {
    private val m = ConcurrentHashMap<String, EntreeCache>()
    override fun lire(cle: String) = m[cle]
    override fun ecrire(cle: String, entree: EntreeCache) {
        m[cle] = entree
    }
    override fun effacer(cle: String) {
        m.remove(cle)
    }
    override fun purgerEchues(maintenantMs: Long) {
        m.entries.removeIf { it.value.echue(maintenantMs) }
    }
    override fun toutEffacer() = m.clear()

    /** Pour les tests : nombre d'entrées gardées. */
    val taille: Int get() = m.size
}

/**
 * Durée de vie provisoire d'une lecture en cache (DA1, MO0). Remplacée par les baux de MB7.
 * Le fuseau est celui de l'appareil : faute de connaître celui de l'établissement côté appli, c'est le choix
 * prudent pour un appareil en France (D-069 ; un appareil réglé sur un autre fuseau voit son cache EDT expirer
 * à son minuit local, jamais plus de 24 h après l'écriture).
 */
object DureeCache {
    const val MAX_MS: Long = 24L * 60 * 60 * 1000

    /** Lectures « du jour » : expirent à minuit (EDT, créneaux, appel). */
    fun duJour(chemin: String): Boolean {
        val p = chemin.substringBefore('?')
        return p.startsWith("/edt/") || p.startsWith("/appels") || EDT_ELEVE.matches(p)
    }

    fun expiration(chemin: String, ecritureMs: Long, fuseau: ZoneId): Long {
        val plafond = ecritureMs + MAX_MS
        if (!duJour(chemin)) return plafond
        val jour = Instant.ofEpochMilli(ecritureMs).atZone(fuseau).toLocalDate()
        val minuit = jour.plusDays(1).atStartOfDay(fuseau).toInstant().toEpochMilli()
        return minOf(minuit, plafond)
    }

    private val EDT_ELEVE = Regex("^/eleves/[^/]+/edt$")
}

/** Donnée lue, éventuellement depuis le cache (affichage « hors ligne, mis à jour le … »). */
data class Lecture<T>(val valeur: T, val horsLigne: Boolean, val horodatageMs: Long)

/**
 * Lecture « réseau d'abord, cache en secours » : en cas d'absence de réseau uniquement.
 * Une erreur 401/403 n'est jamais masquée par le cache (session révoquée ⇒ plus rien).
 * Une entrée échue n'est jamais servie (DA1) : l'erreur réseau remonte telle quelle.
 */
class LecteurHorsLigne(
    private val client: ApiClient,
    private val cache: Cache,
    private val prefixe: () -> String,
    private val maintenant: () -> Long = System::currentTimeMillis,
    private val fuseau: () -> ZoneId = { ZoneId.systemDefault() },
) {
    /** Purge des entrées échues : à appeler au démarrage et à chaque reprise (retour au premier plan). */
    fun purger() = cache.purgerEchues(maintenant())

    suspend fun <T> lire(chemin: String, ser: KSerializer<T>): Lecture<T> {
        val cle = prefixe() + "|" + chemin
        return try {
            val brut = client.getBrut(chemin)
            val valeur = client.decoder(brut, ser)
            val t = maintenant()
            cache.ecrire(cle, EntreeCache(brut, t, DureeCache.expiration(chemin, t, fuseau())))
            Lecture(valeur, false, t)
        } catch (e: ErreurApi.HorsLigne) {
            secours(cle, ser) ?: throw e
        } catch (e: ErreurApi.Serveur) {
            secours(cle, ser) ?: throw e
        } catch (e: ErreurApi.NonAuthentifie) {
            cache.toutEffacer()
            throw e
        }
    }

    private fun <T> secours(cle: String, ser: KSerializer<T>): Lecture<T>? {
        val e = cache.lire(cle) ?: return null
        if (e.echue(maintenant())) {
            cache.effacer(cle)
            return null
        }
        return Lecture(client.decoder(e.json, ser), true, e.horodatageMs)
    }
}
