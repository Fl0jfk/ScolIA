package fr.scolia.mobile.app.securite

import android.app.Activity
import androidx.credentials.CreatePublicKeyCredentialRequest
import androidx.credentials.CreatePublicKeyCredentialResponse
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetPublicKeyCredentialOption
import androidx.credentials.PublicKeyCredential
import androidx.credentials.exceptions.CreateCredentialCancellationException
import androidx.credentials.exceptions.CreateCredentialException
import androidx.credentials.exceptions.GetCredentialCancellationException
import androidx.credentials.exceptions.GetCredentialException
import androidx.credentials.exceptions.NoCredentialException
import fr.scolia.mobile.core.AuthentificateurPasskey

/**
 * Passkeys natives via Credential Manager. Le système n'accepte le `rpId` `scolia.fr`
 * que si `https://scolia.fr/.well-known/assetlinks.json` déclare cette appli
 * (empreinte du certificat de signature) — voir docs/MOBILE.md, « API manquantes ».
 */
class PasskeyAndroid(private val activite: () -> Activity) : AuthentificateurPasskey {
    class Annulee : Exception("Connexion par passkey annulée.")
    class Absente : Exception("Aucune passkey ScolIA sur cet appareil. Connectez-vous avec votre mot de passe puis ajoutez-en une.")
    class Echec(message: String) : Exception(message)

    override suspend fun signer(optionsJson: String): String {
        val a = activite()
        val req = GetCredentialRequest(listOf(GetPublicKeyCredentialOption(optionsJson)))
        return try {
            val r = CredentialManager.create(a).getCredential(a, req)
            (r.credential as? PublicKeyCredential)?.authenticationResponseJson
                ?: throw Echec("Réponse passkey inattendue.")
        } catch (e: GetCredentialCancellationException) {
            throw Annulee()
        } catch (e: NoCredentialException) {
            throw Absente()
        } catch (e: GetCredentialException) {
            throw Echec("Passkey indisponible : ${e.type}")
        }
    }

    override suspend fun creer(optionsJson: String): String {
        val a = activite()
        return try {
            val r = CredentialManager.create(a).createCredential(a, CreatePublicKeyCredentialRequest(optionsJson))
            (r as? CreatePublicKeyCredentialResponse)?.registrationResponseJson
                ?: throw Echec("Réponse passkey inattendue.")
        } catch (e: CreateCredentialCancellationException) {
            throw Annulee()
        } catch (e: CreateCredentialException) {
            throw Echec("Création de passkey impossible : ${e.type}")
        }
    }
}
