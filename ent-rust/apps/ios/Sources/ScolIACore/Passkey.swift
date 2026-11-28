import Foundation

/// Passerelle entre le JSON WebAuthn du serveur (`webauthn-rs`) et AuthenticationServices.
///
/// Entrée : `{"publicKey": {challenge, rpId, allowCredentials, ...}}` (base64url).
/// Sortie : même forme que le client web Leptos (`ent-web-app/src/noyau/webauthn.rs`).
/// Les vérifications de sécurité restent côté serveur (rp_id `scolia.fr`, D-013).
public enum PasskeyPont {
    public enum Erreur: Error, Equatable, LocalizedError {
        case invalide(String)
        public var errorDescription: String? {
            if case .invalide(let m) = self { return m }
            return nil
        }
    }

    /// Ce qu'il faut à `ASAuthorizationPlatformPublicKeyCredentialProvider` pour signer.
    public struct DemandeAssertion: Equatable, Sendable {
        public let rpId: String
        public let challenge: Data
        public let credentialIds: [Data]
        public let verificationRequise: Bool
    }

    public struct DemandeCreation: Equatable, Sendable {
        public let rpId: String
        public let challenge: Data
        public let userId: Data
        public let nom: String
    }

    /// Assertion brute rendue par le système.
    public struct Assertion: Sendable {
        public let credentialId: Data
        public let authenticatorData: Data
        public let clientDataJSON: Data
        public let signature: Data
        public let userHandle: Data?
        public init(credentialId: Data, authenticatorData: Data, clientDataJSON: Data, signature: Data, userHandle: Data?) {
            self.credentialId = credentialId; self.authenticatorData = authenticatorData
            self.clientDataJSON = clientDataJSON; self.signature = signature; self.userHandle = userHandle
        }
    }

    public static func demandeConnexion(_ options: JSONValue, rpIdAttendu: String) throws -> DemandeAssertion {
        guard let pk = options["publicKey"] else { throw Erreur.invalide("Défi passkey invalide.") }
        let rp = pk["rpId"]?.texte ?? rpIdAttendu
        guard rp == rpIdAttendu else { throw Erreur.invalide("Passkey : domaine inattendu.") }
        guard let c = pk["challenge"]?.texte, let defi = base64URLDecoder(c), !defi.isEmpty else {
            throw Erreur.invalide("Défi passkey sans challenge.")
        }
        let ids = (pk["allowCredentials"]?.tableau ?? []).compactMap { $0["id"]?.texte.flatMap(base64URLDecoder) }
        return DemandeAssertion(rpId: rp, challenge: defi, credentialIds: ids, verificationRequise: pk["userVerification"]?.texte == "required")
    }

    public static func demandeEnrolement(_ options: JSONValue, rpIdAttendu: String) throws -> DemandeCreation {
        guard let pk = options["publicKey"] else { throw Erreur.invalide("Défi passkey invalide.") }
        let rp = pk["rp"]?["id"]?.texte ?? rpIdAttendu
        guard rp == rpIdAttendu else { throw Erreur.invalide("Passkey : domaine inattendu.") }
        guard let c = pk["challenge"]?.texte, let defi = base64URLDecoder(c),
              let u = pk["user"]?["id"]?.texte, let uid = base64URLDecoder(u) else {
            throw Erreur.invalide("Défi d'enrôlement incomplet.")
        }
        return DemandeCreation(rpId: rp, challenge: defi, userId: uid, nom: pk["user"]?["name"]?.texte ?? "")
    }

    /// `credential` pour `/auth/passkey/login/finish`.
    public static func assertionPourServeur(_ a: Assertion) -> [String: Any] {
        let id = base64URLEncoder(a.credentialId)
        var reponse: [String: Any] = [
            "authenticatorData": base64URLEncoder(a.authenticatorData),
            "clientDataJSON": base64URLEncoder(a.clientDataJSON),
            "signature": base64URLEncoder(a.signature),
        ]
        if let u = a.userHandle { reponse["userHandle"] = base64URLEncoder(u) }
        return ["id": id, "rawId": id, "type": "public-key", "extensions": [String: Any](), "response": reponse]
    }

    /// `credential` pour `/auth/passkey/register/finish`.
    public static func attestationPourServeur(credentialId: Data, attestationObject: Data, clientDataJSON: Data) -> [String: Any] {
        let id = base64URLEncoder(credentialId)
        return [
            "id": id, "rawId": id, "type": "public-key", "extensions": [String: Any](),
            "response": [
                "attestationObject": base64URLEncoder(attestationObject),
                "clientDataJSON": base64URLEncoder(clientDataJSON),
                "transports": ["internal", "hybrid"],
            ],
        ]
    }

    public static func base64URLEncoder(_ d: Data) -> String {
        d.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }

    public static func base64URLDecoder(_ s: String) -> Data? {
        guard s.allSatisfy({ $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "-" || $0 == "_") }) else { return nil }
        var b = s.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
        while b.count % 4 != 0 { b += "=" }
        return Data(base64Encoded: b)
    }
}
