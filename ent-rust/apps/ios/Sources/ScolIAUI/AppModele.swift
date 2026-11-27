#if os(iOS)
import Foundation
import Observation
import ScolIACore

/// État global : établissement actif, session, verrou.
@MainActor
@Observable
public final class AppModele {
    public private(set) var rattachements: [Rattachement]
    public private(set) var actif: Rattachement?
    public private(set) var etat: EtatConnexion = .deconnecte {
        didSet {
            if case .connecte(let me, _) = etat { utilisateur.valeur = me.userId } else { utilisateur.valeur = "-" }
        }
    }
    public var verrouille = true
    public private(set) var occupe = false
    public var erreur: String?
    public private(set) var client: ApiClient?
    public private(set) var lecteur: LecteurHorsLigne?

    @ObservationIgnored private var service: ServiceConnexion?
    /// Utilisateur courant lisible hors du fil principal (clé du cache hors ligne).
    @ObservationIgnored private let utilisateur = Boite()
    @ObservationIgnored private let cache = CacheFichiers()
    @ObservationIgnored private let passkey = PasskeyApple()
    @ObservationIgnored public let lecteurLiens: LecteurRattachement
    @ObservationIgnored private let defauts = UserDefaults.standard

    public var dernierEmail: String { defauts.string(forKey: "email") ?? "" }

    public init(modeLabo: Bool = false) {
        lecteurLiens = LecteurRattachement(modeLabo: modeLabo)
        // Le Keychain survit à la désinstallation : au premier lancement, on repart de zéro.
        if !UserDefaults.standard.bool(forKey: "deja_lance") {
            CoffreKeychain.toutEffacer()
            UserDefaults.standard.set(true, forKey: "deja_lance")
        }
        let liste = (defauts.data(forKey: "rattachements")).flatMap { try? JSONDecoder().decode([Rattachement].self, from: $0) } ?? []
        rattachements = liste
        actif = liste.first { $0.id == defauts.string(forKey: "actif") } ?? liste.first
        if let a = actif {
            configurer(a)
            reprendre()
        }
    }

    private func configurer(_ r: Rattachement) {
        let coffre = CoffreKeychain(etablissement: r.id)
        let c = ApiClient(baseURL: r.baseURL, transport: TransportURLSession(), jeton: { coffre.lire() }, etablissementIdLabo: r.etablissementIdLabo)
        client = c
        service = ServiceConnexion(client: c, coffre: coffre, cache: cache, passkey: passkey)
        let id = r.id
        let u = utilisateur
        lecteur = LecteurHorsLigne(client: c, cache: cache, prefixe: { "\(id)|\(u.valeur)" })
    }

    private func sauver() {
        // Les codes d'invitation ne sont pas conservés.
        let propres = rattachements.map { var r = $0; r.invitation = nil; return r }
        defauts.set(try? JSONEncoder().encode(propres), forKey: "rattachements")
        defauts.set(actif?.id, forKey: "actif")
    }

    public func rattacher(_ saisie: String) {
        action {
            switch self.lecteurLiens.lire(saisie) {
            case .refuse(let raison): self.erreur = raison
            case .ok(var r):
                let sonde = ApiClient(baseURL: r.baseURL, transport: TransportURLSession(), jeton: { nil }, etablissementIdLabo: r.etablissementIdLabo)
                do {
                    r.nom = try await sonde.etablissementDeLAdresse().nom
                } catch ErreurApi.introuvable(_) where r.etablissementIdLabo != nil {
                    // Labo local sans sous-domaine : on garde le nom provisoire.
                }
                self.rattachements = self.rattachements.filter { $0.id != r.id } + [r]
                self.choisir(r)
            }
        }
    }

    public func choisir(_ r: Rattachement) {
        actif = r
        etat = .deconnecte
        sauver()
        configurer(r)
        reprendre()
    }

    /// Retire l'établissement : session révoquée au mieux, puis jeton, cache et e-mail effacés.
    public func oublier(_ r: Rattachement) {
        let s = actif?.id == r.id ? service : nil
        let coffre = CoffreKeychain(etablissement: r.id)
        let c = cache
        Task {
            await s?.deconnexion() // lit le jeton avant de l'effacer
            coffre.effacer()
            c.toutEffacer()
        }
        defauts.removeObject(forKey: "email")
        rattachements.removeAll { $0.id == r.id }
        if actif?.id == r.id {
            actif = rattachements.first
            etat = .deconnecte
            if let a = actif { configurer(a) }
        }
        sauver()
    }

    @ObservationIgnored private var reprise: Task<Void, Never>?

    /// Reprise de session ; annule une reprise en cours pour un autre établissement.
    public func reprendre() {
        reprise?.cancel()
        let s = service
        let cible = actif?.id
        reprise = Task { @MainActor [self] in
            do {
                let e = try await s?.reprendre() ?? .deconnecte
                if !Task.isCancelled, actif?.id == cible { etat = e }
            } catch ErreurApi.horsLigne {
                erreur = "Pas de connexion au serveur. Réessayez quand le réseau revient."
            } catch {
                if !Task.isCancelled { erreur = error.localizedDescription }
            }
        }
    }

    public func connexionPasskey(_ email: String) {
        defauts.set(email.trimmingCharacters(in: .whitespaces), forKey: "email")
        action { self.etat = try await self.service!.connexionPasskey(email: email); self.verrouille = false }
    }

    public func connexionMotDePasse(_ email: String, _ mdp: String) {
        defauts.set(email.trimmingCharacters(in: .whitespaces), forKey: "email")
        action { self.etat = try await self.service!.connexionMotDePasse(email: email, motDePasse: mdp); self.verrouille = false }
    }

    public func secondFacteur(_ defi: PasskeyChallenge) { action { self.etat = try await self.service!.terminerPasskey(defi); self.verrouille = false } }

    public func enroler(_ mdp: String) {
        action {
            try await self.service!.enrolerPasskey(motDePasse: mdp, nom: "iPhone")
            self.etat = try await self.service!.reprendre()
        }
    }

    public func deconnexion() {
        action {
            await self.service?.deconnexion()
            self.etat = .deconnecte
        }
    }

    /// Session refusée (révoquée à distance, expirée) : tout est oublié localement.
    public func sessionPerdue() {
        service?.oublier()
        etat = .deconnecte
        erreur = "Votre session a pris fin. Reconnectez-vous."
    }

    private func action(_ bloc: @escaping @MainActor () async throws -> Void) {
        Task { @MainActor [self] in
            occupe = true
            erreur = nil
            defer { occupe = false }
            do {
                try await bloc()
            } catch ErreurApi.nonAuthentifie {
                sessionPerdue()
            } catch {
                erreur = error.localizedDescription
            }
        }
    }
}
/// Texte partagé entre fils, protégé par un verrou.
final class Boite: @unchecked Sendable {
    private let verrou = NSLock()
    private var v = "-"
    var valeur: String {
        get { verrou.lock(); defer { verrou.unlock() }; return v }
        set { verrou.lock(); v = newValue; verrou.unlock() }
    }
}
#endif
