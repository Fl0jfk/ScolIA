package fr.scolia.mobile.core

import kotlinx.serialization.KSerializer
import java.util.concurrent.ConcurrentHashMap

/**
 * Cache de lecture hors ligne (mode minimal, lecture seule).
 *
 * Règles :
 * - on ne met en cache que des lectures (EDT, cahier de textes, notes, fils de messages) ;
 * - clé = utilisateur + établissement + chemin : aucune fuite entre comptes ;
 * - tout est effacé à la déconnexion ou à la révocation de la session ;
 * - l'implémentation Android chiffre les fichiers (clé Keystore) ; iOS : protection
 *   de fichier `complete`.
 */
interface Cache {
    fun lire(cle: String): EntreeCache?
    fun ecrire(cle: String, entree: EntreeCache)
    fun toutEffacer()
}

data class EntreeCache(val json: String, val horodatageMs: Long)

class CacheMemoire : Cache {
    private val m = ConcurrentHashMap<String, EntreeCache>()
    override fun lire(cle: String) = m[cle]
    override fun ecrire(cle: String, entree: EntreeCache) {
        m[cle] = entree
    }
    override fun toutEffacer() = m.clear()
}

/** Donnée lue, éventuellement depuis le cache (affichage « hors ligne, mis à jour le … »). */
data class Lecture<T>(val valeur: T, val horsLigne: Boolean, val horodatageMs: Long)

/**
 * Lecture « réseau d'abord, cache en secours » : en cas d'absence de réseau uniquement.
 * Une erreur 401/403 n'est jamais masquée par le cache (session révoquée ⇒ plus rien).
 */
class LecteurHorsLigne(
    private val client: ApiClient,
    private val cache: Cache,
    private val prefixe: () -> String,
    private val maintenant: () -> Long = System::currentTimeMillis,
) {
    suspend fun <T> lire(chemin: String, ser: KSerializer<T>): Lecture<T> {
        val cle = prefixe() + "|" + chemin
        return try {
            val brut = client.getBrut(chemin)
            val valeur = client.decoder(brut, ser)
            val t = maintenant()
            cache.ecrire(cle, EntreeCache(brut, t))
            Lecture(valeur, false, t)
        } catch (e: ErreurApi.HorsLigne) {
            val e2 = cache.lire(cle) ?: throw e
            Lecture(client.decoder(e2.json, ser), true, e2.horodatageMs)
        } catch (e: ErreurApi.Serveur) {
            val e2 = cache.lire(cle) ?: throw e
            Lecture(client.decoder(e2.json, ser), true, e2.horodatageMs)
        } catch (e: ErreurApi.NonAuthentifie) {
            cache.toutEffacer()
            throw e
        }
    }
}
