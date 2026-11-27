import Foundation

/// Cache de lecture hors ligne (lecture seule). Clé = utilisateur + établissement + chemin.
/// Effacé à la déconnexion et à la révocation. iOS : fichiers en protection `complete`.
public protocol Cache: AnyObject, Sendable {
    func lire(_ cle: String) -> EntreeCache?
    func ecrire(_ cle: String, _ entree: EntreeCache)
    func toutEffacer()
}

public struct EntreeCache: Codable, Equatable, Sendable {
    public let json: Data
    public let horodatage: Date
    public init(json: Data, horodatage: Date) { self.json = json; self.horodatage = horodatage }
}

public final class CacheMemoire: Cache, @unchecked Sendable {
    private var m: [String: EntreeCache] = [:]
    private let verrou = NSLock()
    public init() {}
    public func lire(_ cle: String) -> EntreeCache? { verrou.lock(); defer { verrou.unlock() }; return m[cle] }
    public func ecrire(_ cle: String, _ entree: EntreeCache) { verrou.lock(); m[cle] = entree; verrou.unlock() }
    public func toutEffacer() { verrou.lock(); m.removeAll(); verrou.unlock() }
}

public struct Lecture<T> {
    public let valeur: T
    public let horsLigne: Bool
    public let horodatage: Date
    public init(valeur: T, horsLigne: Bool, horodatage: Date) {
        self.valeur = valeur; self.horsLigne = horsLigne; self.horodatage = horodatage
    }
}

/// Réseau d'abord, cache en secours (réseau absent ou serveur en panne uniquement).
/// Un 401 vide le cache : une session révoquée ne laisse rien de lisible.
public final class LecteurHorsLigne: @unchecked Sendable {
    let client: ApiClient
    let cache: Cache
    let prefixe: @Sendable () -> String
    let maintenant: @Sendable () -> Date

    public init(client: ApiClient, cache: Cache, prefixe: @escaping @Sendable () -> String, maintenant: @escaping @Sendable () -> Date = { Date() }) {
        self.client = client; self.cache = cache; self.prefixe = prefixe; self.maintenant = maintenant
    }

    public func lire<T: Decodable>(_ chemin: String, _ type: T.Type = T.self) async throws -> Lecture<T> {
        let cle = prefixe() + "|" + chemin
        do {
            let brut = try await client.getBrut(chemin)
            let v: T = try client.decoder(brut)
            let t = maintenant()
            cache.ecrire(cle, EntreeCache(json: brut, horodatage: t))
            return Lecture(valeur: v, horsLigne: false, horodatage: t)
        } catch let e as ErreurApi {
            switch e {
            case .horsLigne, .serveur:
                guard let c = cache.lire(cle) else { throw e }
                return Lecture(valeur: try client.decoder(c.json), horsLigne: true, horodatage: c.horodatage)
            case .nonAuthentifie:
                cache.toutEffacer()
                throw e
            default:
                throw e
            }
        }
    }
}
