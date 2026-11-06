import Foundation

/// Stockage sécurisé du jeton (iOS : Keychain, `WhenUnlockedThisDeviceOnly`).
public protocol CoffreJeton: AnyObject, Sendable {
    func lire() -> String?
    func ecrire(_ jeton: String)
    func effacer()
}

public final class CoffreMemoire: CoffreJeton, @unchecked Sendable {
    private var j: String?
    private let verrou = NSLock()
    public init(_ j: String? = nil) { self.j = j }
    public func lire() -> String? { verrou.lock(); defer { verrou.unlock() }; return j }
    public func ecrire(_ jeton: String) { verrou.lock(); j = jeton; verrou.unlock() }
    public func effacer() { verrou.lock(); j = nil; verrou.unlock() }
}

/// Authentificateur natif (AuthenticationServices sur iOS).
public protocol AuthentificateurPasskey: Sendable {
    func signer(_ demande: PasskeyPont.DemandeAssertion) async throws -> PasskeyPont.Assertion
    func creer(_ demande: PasskeyPont.DemandeCreation) async throws -> (credentialId: Data, attestationObject: Data, clientDataJSON: Data)
}

/// Push : interface prête, fournisseur non choisi (APNs direct, relais…). Par défaut : rien.
public protocol FournisseurPush: Sendable {
    var plateforme: String { get }
    func jetonAppareil() async -> String?
}

public struct PushInactif: FournisseurPush {
    public init() {}
    public let plateforme = "ios"
    public func jetonAppareil() async -> String? { nil }
}

public enum EtatConnexion: Equatable, Sendable {
    case deconnecte
    /// Mot de passe accepté, passkey exigée en second facteur.
    case secondFacteur(PasskeyChallenge)
    case connecte(Me, Espace)
}

/// Parcours de connexion : passkey d'abord (D-009), mot de passe en secours,
/// session `client: "mobile"` (D-011), jeton au coffre.
public final class ServiceConnexion: @unchecked Sendable {
    let client: ApiClient
    let coffre: CoffreJeton
    let cache: Cache
    let passkey: AuthentificateurPasskey
    let push: FournisseurPush
    let rpId: String
    let appId: String

    public init(client: ApiClient, coffre: CoffreJeton, cache: Cache, passkey: AuthentificateurPasskey,
                push: FournisseurPush = PushInactif(), rpId: String = LecteurRattachement.domaineScolia,
                appId: String = "fr.scolia.mobile") {
        self.client = client; self.coffre = coffre; self.cache = cache; self.passkey = passkey
        self.push = push; self.rpId = rpId; self.appId = appId
    }

    public func connexionPasskey(email: String) async throws -> EtatConnexion {
        try await terminerPasskey(try await client.passkeyLoginStart(email: email))
    }

    public func connexionMotDePasse(email: String, motDePasse: String) async throws -> EtatConnexion {
        let r = try await client.login(email: email, motDePasse: motDePasse)
        if r.mfaRequired {
            guard let defi = r.passkey else { throw ErreurApi.format }
            return .secondFacteur(defi)
        }
        guard let s = r.session else { throw ErreurApi.format }
        return try await ouvrir(s.token)
    }

    public func terminerPasskey(_ defi: PasskeyChallenge) async throws -> EtatConnexion {
        let demande = try PasskeyPont.demandeConnexion(defi.options, rpIdAttendu: rpId)
        let assertion = try await passkey.signer(demande)
        let s = try await client.passkeyLoginFinish(token: defi.token, credential: PasskeyPont.assertionPourServeur(assertion))
        return try await ouvrir(s.token)
    }

    public func reprendre() async throws -> EtatConnexion {
        guard coffre.lire() != nil else { return .deconnecte }
        do {
            return try await charger()
        } catch ErreurApi.nonAuthentifie {
            oublier()
            return .deconnecte
        }
    }

    public func enrolerPasskey(motDePasse: String, nom: String) async throws {
        let defi = try await client.passkeyRegisterStart()
        let demande = try PasskeyPont.demandeEnrolement(defi.options, rpIdAttendu: rpId)
        let r = try await passkey.creer(demande)
        let credential = PasskeyPont.attestationPourServeur(credentialId: r.credentialId, attestationObject: r.attestationObject, clientDataJSON: r.clientDataJSON)
        try await client.passkeyRegisterFinish(token: defi.token, credential: credential, nom: nom, motDePasse: motDePasse)
    }

    public func deconnexion() async {
        // Hors ligne : on oublie quand même le jeton localement.
        try? await client.logout()
        oublier()
    }

    public func oublier() {
        coffre.effacer()
        cache.toutEffacer()
    }

    private func ouvrir(_ jeton: String) async throws -> EtatConnexion {
        coffre.ecrire(jeton)
        let etat = try await charger()
        if let j = await push.jetonAppareil() {
            try? await client.enregistrerPush(plateforme: push.plateforme, jeton: j, appId: appId)
        }
        return etat
    }

    private func charger() async throws -> EtatConnexion {
        let me = try await client.me()
        var droits: [Droit] = []
        if me.portee != "enrolement" && !["parent", "eleve"].contains(me.role) {
            do { droits = try await client.mesDroits() } catch ErreurApi.interdit { droits = [] }
        }
        return .connecte(me, Capacites.espace(me: me, droits: droits))
    }
}
