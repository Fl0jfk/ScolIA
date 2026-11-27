package fr.scolia.mobile.app.securite

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricManager.Authenticators.BIOMETRIC_STRONG
import androidx.biometric.BiometricManager.Authenticators.DEVICE_CREDENTIAL
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import fr.scolia.mobile.core.Cache
import fr.scolia.mobile.core.CoffreJeton
import fr.scolia.mobile.core.EntreeCache
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/**
 * Chiffrement AES-256-GCM par une clé non exportable de l'Android Keystore
 * (matériel sécurisé / StrongBox si disponible). La clé ne quitte jamais le Keystore.
 */
internal class ChiffreurKeystore(private val alias: String) {
    private fun cle(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getKey(alias, null) as? SecretKey)?.let { return it }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        gen.init(
            KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return gen.generateKey()
    }

    fun chiffrer(clair: ByteArray): ByteArray {
        val c = Cipher.getInstance("AES/GCM/NoPadding")
        c.init(Cipher.ENCRYPT_MODE, cle())
        return c.iv + c.doFinal(clair) // IV (12 octets) + texte chiffré + étiquette
    }

    fun dechiffrer(donnees: ByteArray): ByteArray {
        val c = Cipher.getInstance("AES/GCM/NoPadding")
        c.init(Cipher.DECRYPT_MODE, cle(), GCMParameterSpec(128, donnees, 0, 12))
        return c.doFinal(donnees, 12, donnees.size - 12)
    }

    fun supprimer() {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        if (ks.containsAlias(alias)) ks.deleteEntry(alias)
    }
}

/** Jeton de session chiffré (un par établissement rattaché). */
class CoffreKeystore(context: Context, private val etablissement: String) : CoffreJeton {
    private val prefs: SharedPreferences = context.getSharedPreferences("scolia_coffre", Context.MODE_PRIVATE)
    private val chiffreur = ChiffreurKeystore("scolia.jeton")
    private val cle get() = "jeton." + empreinte(etablissement)

    override fun lire(): String? {
        val b64 = prefs.getString(cle, null) ?: return null
        return try {
            String(chiffreur.dechiffrer(Base64.decode(b64, Base64.NO_WRAP)), Charsets.UTF_8)
        } catch (e: Exception) {
            // Clé perdue (restauration, changement d'appareil) : on repart de zéro.
            effacer()
            null
        }
    }

    override fun ecrire(jeton: String) {
        val chiffre = chiffreur.chiffrer(jeton.toByteArray(Charsets.UTF_8))
        prefs.edit().putString(cle, Base64.encodeToString(chiffre, Base64.NO_WRAP)).apply()
    }

    override fun effacer() {
        prefs.edit().remove(cle).apply()
    }
}

/** Cache hors ligne chiffré sur disque (répertoire privé, exclu des sauvegardes). */
class CacheChiffre(context: Context) : Cache {
    private val dossier = File(context.noBackupFilesDir, "cache_lecture").apply { mkdirs() }
    private val chiffreur = ChiffreurKeystore("scolia.cache")

    override fun lire(cle: String): EntreeCache? {
        val f = File(dossier, empreinte(cle))
        if (!f.exists()) return null
        return try {
            val clair = String(chiffreur.dechiffrer(f.readBytes()), Charsets.UTF_8)
            val sep = clair.indexOf('\n')
            EntreeCache(clair.substring(sep + 1), clair.substring(0, sep).toLong())
        } catch (e: Exception) {
            f.delete()
            null
        }
    }

    override fun ecrire(cle: String, entree: EntreeCache) {
        val clair = "${entree.horodatageMs}\n${entree.json}".toByteArray(Charsets.UTF_8)
        File(dossier, empreinte(cle)).writeBytes(chiffreur.chiffrer(clair))
    }

    override fun toutEffacer() {
        dossier.listFiles()?.forEach { it.delete() }
    }
}

/** Les noms de fichiers ne révèlent ni l'utilisateur ni le chemin. */
internal fun empreinte(s: String): String =
    MessageDigest.getInstance("SHA-256").digest(s.toByteArray()).joinToString("") { "%02x".format(it) }

/**
 * Déverrouillage par biométrie forte ou code de l'appareil (D-011) :
 * demandé à l'ouverture et après 5 minutes en arrière-plan.
 */
class Verrou(private val activite: FragmentActivity) {
    enum class Etat { DISPONIBLE, AUCUN_VERROU_CONFIGURE, INDISPONIBLE }

    /** Android 9 et 10 n'acceptent pas BIOMETRIC_STRONG combiné au code de l'appareil. */
    private val authentificateurs: Int =
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
            BIOMETRIC_STRONG or DEVICE_CREDENTIAL
        } else {
            BiometricManager.Authenticators.BIOMETRIC_WEAK or DEVICE_CREDENTIAL
        }

    fun etat(): Etat = when (BiometricManager.from(activite).canAuthenticate(authentificateurs)) {
        BiometricManager.BIOMETRIC_SUCCESS -> Etat.DISPONIBLE
        BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> Etat.AUCUN_VERROU_CONFIGURE
        else -> Etat.INDISPONIBLE
    }

    suspend fun deverrouiller(): Boolean = suspendCancellableCoroutine { cont ->
        val prompt = BiometricPrompt(
            activite,
            ContextCompat.getMainExecutor(activite),
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    if (cont.isActive) cont.resume(true)
                }
                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    if (cont.isActive) cont.resume(false)
                }
            },
        )
        prompt.authenticate(
            BiometricPrompt.PromptInfo.Builder()
                .setTitle("Déverrouiller ScolIA")
                .setSubtitle("Empreinte, visage ou code de l'appareil")
                .setAllowedAuthenticators(authentificateurs)
                .build(),
        )
    }

    companion object {
        const val DELAI_MS = 5 * 60 * 1000L
    }
}
