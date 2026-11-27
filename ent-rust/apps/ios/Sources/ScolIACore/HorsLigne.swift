import Foundation

/// Cache de lecture hors ligne (lecture seule). Clé = utilisateur + établissement + chemin.
/// Effacé à la déconnexion et à la révocation. iOS : fichiers en protection `complete`.
/// **Toute entrée a une date de fin** (DA1, en attendant les baux de MB7) : minuit pour l'EDT et l'appel
/// (R-AS32, R-PO33), 24 h au plus pour le reste. Une entrée échue n'est jamais servie ; elle est effacée
/// à la lecture et à chaque purge (démarrage, retour au premier plan).
public protocol Cache: AnyObject, Sendable {
    func lire(_ cle: String) -> EntreeCache?
    func ecrire(_ cle: String, _ entree: EntreeCache)
    func effacer(_ cle: String)
    /// Efface toutes les entrées échues à l'instant `maintenant`.
    func purgerEchues(_ maintenant: Date)
    func toutEffacer()
}

public struct EntreeCache: Codable, Equatable, Sendable {
    public let json: Data
    public let horodatage: Date
    /// Instant après lequel l'entrée n'est plus servie ni gardée. Une entrée d'un ancien format (sans date de fin)
    /// ne se décode pas : elle est effacée, jamais servie.
    public let expire: Date
    public init(json: Data, horodatage: Date, expire: Date) { self.json = json; self.horodatage = horodatage; self.expire = expire }
    /// Échue aussi si l'horloge a reculé avant l'écriture ou dépasse 24 h après (plafond revérifié à la lecture).
    public func echue(_ maintenant: Date) -> Bool {
        maintenant >= expire || maintenant < horodatage || maintenant.timeIntervalSince(horodatage) >= DureeCache.max
    }
}

public final class CacheMemoire: Cache, @unchecked Sendable {
    private var m: [String: EntreeCache] = [:]
    private let verrou = NSLock()
    public init() {}
    public func lire(_ cle: String) -> EntreeCache? { verrou.lock(); defer { verrou.unlock() }; return m[cle] }
    public func ecrire(_ cle: String, _ entree: EntreeCache) { verrou.lock(); m[cle] = entree; verrou.unlock() }
    public func effacer(_ cle: String) { verrou.lock(); m[cle] = nil; verrou.unlock() }
    public func purgerEchues(_ maintenant: Date) { verrou.lock(); m = m.filter { !$0.value.echue(maintenant) }; verrou.unlock() }
    public func toutEffacer() { verrou.lock(); m.removeAll(); verrou.unlock() }
    /// Pour les tests : nombre d'entrées gardées.
    public var taille: Int { verrou.lock(); defer { verrou.unlock() }; return m.count }
}

/// Durée de vie provisoire d'une lecture en cache (DA1, MO0), remplacée par les baux de MB7.
/// Fuseau : celui de l'appareil (défaut prudent, D-069) ; jamais plus de 24 h après l'écriture.
public enum DureeCache {
    public static let max: TimeInterval = 24 * 3600

    /// Lectures « du jour » : expirent à minuit (EDT, créneaux, appel).
    public static func duJour(_ chemin: String) -> Bool {
        let p = chemin.split(separator: "?", maxSplits: 1).first.map(String.init) ?? chemin
        if p.hasPrefix("/edt/") || p.hasPrefix("/appels") { return true }
        let s = p.split(separator: "/", omittingEmptySubsequences: false)
        return s.count == 4 && s[0].isEmpty && s[1] == "eleves" && !s[2].isEmpty && s[3] == "edt"
    }

    public static func expiration(_ chemin: String, ecriture: Date, fuseau: TimeZone) -> Date {
        let plafond = ecriture.addingTimeInterval(max)
        guard duJour(chemin) else { return plafond }
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = fuseau
        let debut = cal.startOfDay(for: ecriture)
        guard let minuit = cal.date(byAdding: .day, value: 1, to: debut) else { return plafond }
        return min(minuit, plafond)
    }
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
/// Un 401 vide le cache : une session révoquée ne laisse rien de lisible. Une entrée échue n'est jamais servie.
public final class LecteurHorsLigne: @unchecked Sendable {
    let client: ApiClient
    let cache: Cache
    let prefixe: @Sendable () -> String
    let maintenant: @Sendable () -> Date
    let fuseau: @Sendable () -> TimeZone

    public init(client: ApiClient, cache: Cache, prefixe: @escaping @Sendable () -> String,
                maintenant: @escaping @Sendable () -> Date = { Date() },
                fuseau: @escaping @Sendable () -> TimeZone = { TimeZone.current }) {
        self.client = client; self.cache = cache; self.prefixe = prefixe; self.maintenant = maintenant; self.fuseau = fuseau
    }

    /// Purge des entrées échues : au démarrage et à chaque retour au premier plan.
    public func purger() { cache.purgerEchues(maintenant()) }

    public func lire<T: Decodable>(_ chemin: String, _ type: T.Type = T.self) async throws -> Lecture<T> {
        let cle = prefixe() + "|" + chemin
        do {
            let brut = try await client.getBrut(chemin)
            let v: T = try client.decoder(brut)
            let t = maintenant()
            cache.ecrire(cle, EntreeCache(json: brut, horodatage: t, expire: DureeCache.expiration(chemin, ecriture: t, fuseau: fuseau())))
            return Lecture(valeur: v, horsLigne: false, horodatage: t)
        } catch let e as ErreurApi {
            switch e {
            case .horsLigne, .serveur:
                guard let c = cache.lire(cle) else { throw e }
                if c.echue(maintenant()) {
                    cache.effacer(cle)
                    throw e
                }
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

/// Clés d'idempotence **gardées par intention** (contrat §5.5, M3 du lot 2 de MB1), miroir de `ClesIntention`
/// (Kotlin) : après un échec incertain (réseau, 5xx, 429, 409 « en cours »), un nouvel essai de la même intention
/// renvoie la même clé et le serveur rejoue sa première réponse. Oubliée après un succès ou un refus définitif.
public final class ClesIntention: @unchecked Sendable {
    private let verrou = NSLock()
    private var cles: [String: String] = [:]
    private let nouvelleCle: @Sendable () -> String

    public init(nouvelleCle: @escaping @Sendable () -> String = { UUID().uuidString.lowercased() }) {
        self.nouvelleCle = nouvelleCle
    }

    public func cle(_ intention: String) -> String {
        verrou.lock(); defer { verrou.unlock() }
        if let c = cles[intention] { return c }
        let c = nouvelleCle()
        cles[intention] = c
        return c
    }

    public func oublier(_ intention: String) {
        verrou.lock(); defer { verrou.unlock() }
        cles[intention] = nil
    }

    public func oublierFamille(_ prefixe: String) {
        verrou.lock(); defer { verrou.unlock() }
        cles = cles.filter { !$0.key.hasPrefix(prefixe) }
    }

    /// Exécute `bloc` avec la clé de l'intention ; la garde seulement si l'issue est incertaine. `famille` : après un
    /// succès, oublie aussi les intentions sœurs (ex. `marquer:<appel>:<élève>:` pour tous les états).
    public func executer<T>(_ intention: String, famille: String? = nil, _ bloc: (String) async throws -> T) async throws -> T {
        let c = cle(intention)
        do {
            let r = try await bloc(c)
            oublier(intention)
            if let f = famille { oublierFamille(f) }
            return r
        } catch let e as ErreurApi {
            if !Self.incertaine(e) { oublier(intention) }
            throw e
        }
    }

    public static func incertaine(_ e: ErreurApi) -> Bool {
        switch e {
        case .horsLigne, .serveur, .debitDepasse, .format: return true
        // Seul le 409 « en cours » est incertain ; un conflit métier (appel déjà clos…) est définitif.
        case .codee(let s, let e): return s == 409 && e.code == "idempotence.en_cours"
        default: return false
        }
    }
}
