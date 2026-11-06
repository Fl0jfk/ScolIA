#if os(iOS)
import AuthenticationServices
import CryptoKit
import Foundation
import LocalAuthentication
import ScolIACore
import Security
import UIKit

/// Jeton au Keychain : `WhenUnlockedThisDeviceOnly` (ni sauvegarde iCloud, ni migration
/// vers un autre appareil). Un élément par établissement rattaché.
public final class CoffreKeychain: CoffreJeton, @unchecked Sendable {
    private let compte: String
    private let service = "fr.scolia.mobile.jeton"

    public init(etablissement: String) { compte = etablissement }

    private var requete: [String: Any] {
        [kSecClass as String: kSecClassGenericPassword,
         kSecAttrService as String: service,
         kSecAttrAccount as String: compte]
    }

    public func lire() -> String? {
        var q = requete
        q[kSecReturnData as String] = true
        q[kSecMatchLimit as String] = kSecMatchLimitOne
        var res: AnyObject?
        guard SecItemCopyMatching(q as CFDictionary, &res) == errSecSuccess, let d = res as? Data else { return nil }
        return String(data: d, encoding: .utf8)
    }

    public func ecrire(_ jeton: String) {
        effacer()
        var q = requete
        q[kSecValueData as String] = Data(jeton.utf8)
        q[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        SecItemAdd(q as CFDictionary, nil)
    }

    public func effacer() { SecItemDelete(requete as CFDictionary) }
}

/// Cache hors ligne : fichiers en protection `complete` (illisibles appareil verrouillé),
/// exclus de la sauvegarde ; noms de fichiers hachés.
public final class CacheFichiers: Cache, @unchecked Sendable {
    private let dossier: URL

    public init() {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        var d = base.appendingPathComponent("cache_lecture", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true,
                                                 attributes: [.protectionKey: FileProtectionType.complete])
        var v = URLResourceValues()
        v.isExcludedFromBackup = true
        try? d.setResourceValues(v)
        dossier = d
    }

    private func fichier(_ cle: String) -> URL {
        let h = SHA256.hash(data: Data(cle.utf8)).map { String(format: "%02x", $0) }.joined()
        return dossier.appendingPathComponent(h)
    }

    public func lire(_ cle: String) -> EntreeCache? {
        guard let d = try? Data(contentsOf: fichier(cle)) else { return nil }
        return try? JSONDecoder().decode(EntreeCache.self, from: d)
    }

    public func ecrire(_ cle: String, _ entree: EntreeCache) {
        guard let d = try? JSONEncoder().encode(entree) else { return }
        try? d.write(to: fichier(cle), options: [.atomic, .completeFileProtection])
    }

    public func toutEffacer() {
        let fichiers = (try? FileManager.default.contentsOfDirectory(at: dossier, includingPropertiesForKeys: nil)) ?? []
        fichiers.forEach { try? FileManager.default.removeItem(at: $0) }
    }
}

/// Déverrouillage Face ID / Touch ID / code (D-011).
public enum Verrou {
    public static let delai: TimeInterval = 5 * 60

    public static func deverrouiller() async -> Bool {
        let ctx = LAContext()
        var err: NSError?
        guard ctx.canEvaluatePolicy(.deviceOwnerAuthentication, error: &err) else { return true }
        return (try? await ctx.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: "Déverrouiller ScolIA")) ?? false
    }
}

/// Passkeys natives. Le domaine `scolia.fr` doit être déclaré dans les entitlements
/// (`webcredentials:scolia.fr`) et servir `/.well-known/apple-app-site-association`.
public final class PasskeyApple: NSObject, AuthentificateurPasskey, ASAuthorizationControllerDelegate,
    ASAuthorizationControllerPresentationContextProviding, @unchecked Sendable {
    public enum Erreur: LocalizedError {
        case annulee, echec(String)
        public var errorDescription: String? {
            switch self {
            case .annulee: return "Connexion par passkey annulée."
            case .echec(let m): return m
            }
        }
    }

    private var suite: CheckedContinuation<ASAuthorization, Error>?

    public override init() {}

    @MainActor
    private func executer(_ requete: ASAuthorizationRequest) async throws -> ASAuthorization {
        try await withCheckedThrowingContinuation { cont in
            suite = cont
            let c = ASAuthorizationController(authorizationRequests: [requete])
            c.delegate = self
            c.presentationContextProvider = self
            c.performRequests()
        }
    }

    public func signer(_ d: PasskeyPont.DemandeAssertion) async throws -> PasskeyPont.Assertion {
        let fournisseur = ASAuthorizationPlatformPublicKeyCredentialProvider(relyingPartyIdentifier: d.rpId)
        let req = fournisseur.createCredentialAssertionRequest(challenge: d.challenge)
        req.allowedCredentials = d.credentialIds.map { ASAuthorizationPlatformPublicKeyCredentialDescriptor(credentialID: $0) }
        req.userVerificationPreference = d.verificationRequise ? .required : .preferred
        let auth = try await executer(req)
        guard let a = auth.credential as? ASAuthorizationPlatformPublicKeyCredentialAssertion else {
            throw Erreur.echec("Réponse passkey inattendue.")
        }
        return PasskeyPont.Assertion(credentialId: a.credentialID, authenticatorData: a.rawAuthenticatorData,
                                     clientDataJSON: a.rawClientDataJSON, signature: a.signature, userHandle: a.userID)
    }

    public func creer(_ d: PasskeyPont.DemandeCreation) async throws -> (credentialId: Data, attestationObject: Data, clientDataJSON: Data) {
        let fournisseur = ASAuthorizationPlatformPublicKeyCredentialProvider(relyingPartyIdentifier: d.rpId)
        let req = fournisseur.createCredentialRegistrationRequest(challenge: d.challenge, name: d.nom, userID: d.userId)
        let auth = try await executer(req)
        guard let r = auth.credential as? ASAuthorizationPlatformPublicKeyCredentialRegistration,
              let att = r.rawAttestationObject else {
            throw Erreur.echec("Création de passkey impossible.")
        }
        return (r.credentialID, att, r.rawClientDataJSON)
    }

    public func authorizationController(controller: ASAuthorizationController, didCompleteWithAuthorization authorization: ASAuthorization) {
        suite?.resume(returning: authorization)
        suite = nil
    }

    public func authorizationController(controller: ASAuthorizationController, didCompleteWithError error: Error) {
        if let e = error as? ASAuthorizationError, e.code == .canceled {
            suite?.resume(throwing: Erreur.annulee)
        } else {
            suite?.resume(throwing: Erreur.echec("Passkey indisponible : \(error.localizedDescription)"))
        }
        suite = nil
    }

    public func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        return scenes.flatMap(\.windows).first(where: \.isKeyWindow) ?? ASPresentationAnchor()
    }
}
#endif
