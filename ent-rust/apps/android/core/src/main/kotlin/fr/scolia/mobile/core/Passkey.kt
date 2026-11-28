package fr.scolia.mobile.core

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put

/**
 * Passerelle entre le JSON WebAuthn du serveur (`webauthn-rs`) et l'API native
 * (Credential Manager sur Android, qui prend et rend du JSON WebAuthn standard).
 *
 * - Entrée serveur : `{"publicKey": {challenge, rpId, allowCredentials, ...}, "mediation": ...}`.
 * - Sortie serveur attendue : même forme que le client web Leptos
 *   (`ent-web-app/src/noyau/webauthn.rs`) : `id`, `rawId`, `type`, `extensions`, `response{...}`
 *   en base64url sans remplissage.
 *
 * Le contrôle de sécurité reste côté serveur (origine `android:apk-key-hash:…`
 * déclarée dans `ENT_RP_ORIGINS`, `rp_id = scolia.fr`, D-013).
 */
object PasskeyPont {
    private val json = Json { ignoreUnknownKeys = true }

    class ErreurPasskey(message: String) : Exception(message)

    /**
     * Options de connexion à donner à `GetPublicKeyCredentialOption(requestJson)`.
     * Refuse un `rpId` différent de celui attendu (défense en profondeur ; le système
     * refuse de toute façon un domaine non associé à l'appli).
     */
    fun optionsConnexion(options: JsonObject, rpIdAttendu: String): String {
        val pk = publicKey(options)
        val rp = pk["rpId"]?.jsonPrimitive?.contentOrNull
        if (rp != null && rp != rpIdAttendu) throw ErreurPasskey("Passkey : domaine inattendu.")
        if (pk["challenge"]?.jsonPrimitive?.contentOrNull.isNullOrEmpty()) {
            throw ErreurPasskey("Défi passkey sans challenge.")
        }
        return pk.toString()
    }

    /** Options d'enrôlement pour `CreatePublicKeyCredentialRequest(requestJson)`. */
    fun optionsEnrolement(options: JsonObject, rpIdAttendu: String): String {
        val pk = publicKey(options)
        val rp = pk["rp"]?.jsonObject?.get("id")?.jsonPrimitive?.contentOrNull
        if (rp != null && rp != rpIdAttendu) throw ErreurPasskey("Passkey : domaine inattendu.")
        return pk.toString()
    }

    /** Assertion native (JSON Credential Manager) → `credential` pour `/auth/passkey/login/finish`. */
    fun assertionPourServeur(reponseNative: String): JsonObject {
        val r = parse(reponseNative)
        val resp = r["response"]?.jsonObject ?: throw ErreurPasskey("Réponse passkey incomplète.")
        return buildJsonObject {
            put("id", exiger(r, "id"))
            put("rawId", r["rawId"]?.jsonPrimitive?.contentOrNull ?: exiger(r, "id"))
            put("type", r["type"]?.jsonPrimitive?.contentOrNull ?: "public-key")
            put("extensions", JsonObject(emptyMap()))
            put(
                "response",
                buildJsonObject {
                    put("authenticatorData", exiger(resp, "authenticatorData"))
                    put("clientDataJSON", exiger(resp, "clientDataJSON"))
                    put("signature", exiger(resp, "signature"))
                    resp["userHandle"]?.jsonPrimitive?.contentOrNull?.let { put("userHandle", it) }
                },
            )
        }
    }

    /** Attestation native → `credential` pour `/auth/passkey/register/finish`. */
    fun attestationPourServeur(reponseNative: String): JsonObject {
        val r = parse(reponseNative)
        val resp = r["response"]?.jsonObject ?: throw ErreurPasskey("Réponse passkey incomplète.")
        return buildJsonObject {
            put("id", exiger(r, "id"))
            put("rawId", r["rawId"]?.jsonPrimitive?.contentOrNull ?: exiger(r, "id"))
            put("type", r["type"]?.jsonPrimitive?.contentOrNull ?: "public-key")
            put("extensions", JsonObject(emptyMap()))
            put(
                "response",
                buildJsonObject {
                    put("attestationObject", exiger(resp, "attestationObject"))
                    put("clientDataJSON", exiger(resp, "clientDataJSON"))
                    (resp["transports"] as? JsonArray)?.let { put("transports", it) }
                },
            )
        }
    }

    private fun publicKey(options: JsonObject): JsonObject =
        (options["publicKey"] as? JsonObject) ?: throw ErreurPasskey("Défi passkey invalide.")

    private fun parse(s: String): JsonObject =
        try {
            json.parseToJsonElement(s).jsonObject
        } catch (e: Exception) {
            throw ErreurPasskey("Réponse passkey illisible.")
        }

    private fun exiger(o: JsonObject, cle: String): String {
        val v = (o[cle] as? JsonPrimitive)?.contentOrNull
        if (v.isNullOrEmpty()) throw ErreurPasskey("Réponse passkey incomplète ($cle).")
        if (!estBase64Url(v) && cle != "type") throw ErreurPasskey("Réponse passkey mal encodée ($cle).")
        return v
    }

    /** base64url sans remplissage (format attendu par `webauthn-rs`). */
    fun estBase64Url(s: String): Boolean = s.all { it.isLetterOrDigit() && it.code < 128 || it == '-' || it == '_' }
}
