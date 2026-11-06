import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public struct RequeteHttp: Sendable, Equatable {
    public let methode: String
    public let url: URL
    public let entetes: [String: String]
    public let corps: Data?
}

public struct ReponseHttp: Sendable {
    public let statut: Int
    public let corps: Data
    public init(statut: Int, corps: Data) { self.statut = statut; self.corps = corps }
}

/// Transport HTTP abstrait (URLSession en vrai, faux serveur dans les tests).
public protocol Transport: Sendable {
    func executer(_ requete: RequeteHttp) async throws -> ReponseHttp
}

/// Erreurs traduites pour l'interface. Jamais de jeton dans les messages.
public enum ErreurApi: Error, Equatable, LocalizedError {
    /// 401 : session expirée ou révoquée à distance.
    case nonAuthentifie
    /// 403 : refusé par le serveur (les droits sont vérifiés côté serveur, D-003).
    case interdit(String)
    case introuvable(String)
    case refus(Int, String)
    case serveur(Int)
    case horsLigne
    case format

    public var errorDescription: String? {
        switch self {
        case .nonAuthentifie: return "Session expirée : reconnectez-vous."
        case .interdit(let m), .introuvable(let m), .refus(_, let m): return m
        case .serveur(let s): return "Erreur du serveur (\(s)). Réessayez plus tard."
        case .horsLigne: return "Pas de connexion au serveur."
        case .format: return "Réponse inattendue du serveur."
        }
    }
}

/// Client typé de l'API. `baseURL` = adresse de l'établissement (D-012).
public final class ApiClient: @unchecked Sendable {
    public let baseURL: URL
    private let transport: Transport
    private let jeton: @Sendable () -> String?
    private let etablissementIdLabo: String?

    static let decodeur: JSONDecoder = {
        let d = JSONDecoder()
        d.keyDecodingStrategy = .convertFromSnakeCase
        return d
    }()
    /// Sans stratégie : types aux clés explicites et JSON WebAuthn (camelCase).
    static let decodeurBrut = JSONDecoder()
    static let encodeur: JSONEncoder = {
        let e = JSONEncoder()
        e.keyEncodingStrategy = .convertToSnakeCase
        e.outputFormatting = [.sortedKeys]
        return e
    }()

    public init(baseURL: URL, transport: Transport, jeton: @escaping @Sendable () -> String?, etablissementIdLabo: String? = nil) {
        self.baseURL = baseURL
        self.transport = transport
        self.jeton = jeton
        self.etablissementIdLabo = etablissementIdLabo
    }

    // MARK: Authentification

    public func etablissementDeLAdresse() async throws -> EtablissementPublic {
        try await get("/auth/etablissement", authentifie: false)
    }

    public func login(email: String, motDePasse: String) async throws -> LoginResponse {
        var corps: [String: Any] = ["email": email.trimmingCharacters(in: .whitespaces), "password": motDePasse, "client": clientMobile]
        if let e = etablissementIdLabo { corps["etablissement_id"] = e }
        let data = try await envoyer("POST", "/auth/login", corps: try JSONSerialization.data(withJSONObject: corps), authentifie: false)
        return try decoder(data, brut: true)
    }

    public func passkeyLoginStart(email: String) async throws -> PasskeyChallenge {
        var corps: [String: Any] = ["email": email.trimmingCharacters(in: .whitespaces), "client": clientMobile]
        if let e = etablissementIdLabo { corps["etablissement_id"] = e }
        let data = try await envoyer("POST", "/auth/passkey/login/start", corps: try JSONSerialization.data(withJSONObject: corps), authentifie: false)
        return try decoder(data, brut: true)
    }

    /// `credential` : objet JSON WebAuthn (camelCase), construit par `PasskeyPont`.
    public func passkeyLoginFinish(token: String, credential: [String: Any]) async throws -> AuthSession {
        let corps = try JSONSerialization.data(withJSONObject: ["token": token, "credential": credential])
        return try decoder(try await envoyer("POST", "/auth/passkey/login/finish", corps: corps, authentifie: false), brut: true)
    }

    public func passkeyRegisterStart() async throws -> PasskeyChallenge {
        try decoder(try await envoyer("POST", "/auth/passkey/register/start", corps: Data("{}".utf8)), brut: true)
    }

    public func passkeyRegisterFinish(token: String, credential: [String: Any], nom: String, motDePasse: String) async throws {
        let corps = try JSONSerialization.data(withJSONObject: [
            "token": token, "credential": credential, "name": nom, "current_password": motDePasse,
        ])
        _ = try await envoyer("POST", "/auth/passkey/register/finish", corps: corps)
    }

    public func me() async throws -> Me { try await get("/auth/me") }
    public func mesDroits() async throws -> [Droit] { try await get("/roles/mes-droits") }
    public func sessions() async throws -> [SessionAppareil] { try await get("/auth/sessions") }
    public func revoquerSession(_ id: String) async throws { _ = try await envoyer("DELETE", "/auth/sessions/\(seg(id))", corps: nil) }
    public func logout() async throws { _ = try await envoyer("POST", "/auth/logout", corps: nil) }

    public func enregistrerPush(plateforme: String, jeton: String, appId: String) async throws {
        _ = try await envoyer("POST", "/devices/push", corps: try Self.encodeur.encode(["platform": plateforme, "token": jeton, "app_id": appId]))
    }

    // MARK: EDT et appel

    public func mesCreneaux(jour: String) async throws -> [EdtCreneau] { try await get(Chemins.mesCreneaux(jour)) }
    public func edtEleve(_ id: String, jour: String) async throws -> [EdtCreneau] { try await get(Chemins.edtEleve(id, jour)) }

    public func ouvrirAppel(creneauId: String, jour: String) async throws -> Appel {
        try await post("/appels", ["creneau_id": creneauId, "jour": jour])
    }

    public func appel(_ id: String) async throws -> Appel { try await get("/appels/\(seg(id))") }

    /// Réponse 204 sans corps.
    public func marquer(appelId: String, eleveId: String, etat: EtatAppel) async throws {
        _ = try await envoyer("POST", "/appels/\(seg(appelId))/lignes", corps: try Self.encodeur.encode(["eleve_id": eleveId, "etat": etat.rawValue]))
    }

    public func clore(appelId: String) async throws -> Appel {
        try decoder(try await envoyer("POST", "/appels/\(seg(appelId))/clore", corps: Data("{}".utf8)))
    }

    // MARK: Famille, absences, vie scolaire

    public func mesEnfants() async throws -> [EnfantFamille] { try await get(Chemins.enfants) }
    public func resumeEnfant(_ id: String) async throws -> FamilleResume { try await get("/famille/enfants/\(seg(id))/resume") }
    public func justifierAbsence(_ id: String, motif: String) async throws -> Absence {
        try await post("/absences/\(seg(id))/justifier", ["motif_justif": motif.trimmingCharacters(in: .whitespacesAndNewlines)])
    }
    public func absencesATraiter() async throws -> [Absence] { try await get("/vs/absences") }

    // MARK: Notes

    public func periodesNotes() async throws -> [NotePeriode] { try await get("/note-periodes") }
    public func notes(_ eleveId: String) async throws -> [Note] { try await get(Chemins.notes(eleveId)) }
    public func devoirsClasse(periodeId: String, classe: String) async throws -> [Devoir] {
        try await get("/devoirs?periode_id=\(q(periodeId))&classe=\(q(classe))")
    }
    public func effectif(classe: String, jour: String) async throws -> Effectif {
        try await get("/classes/\(seg(classe))/effectif?jour=\(q(jour))")
    }
    public func saisirNote(eleveId: String, _ corps: SaisirNoteBody) async throws -> Note {
        try decoder(try await envoyer("POST", "/eleves/\(seg(eleveId))/notes", corps: try Self.encodeur.encode(corps)))
    }

    // MARK: Cahier de textes, messagerie, annonces

    public func cahierTexte(eleveId: String?) async throws -> [CahierTexte] { try await get(Chemins.cahier(eleveId)) }
    public func fils() async throws -> [MsgFil] { try await get(Chemins.fils) }
    public func fil(_ id: String) async throws -> FilDetail { try await get("/messages/fils/\(seg(id))") }
    public func repondre(filId: String, corps: String) async throws -> Message {
        try await post("/messages/fils/\(seg(filId))/repondre", ["corps": corps])
    }
    public func annonces() async throws -> [Annonce] { try await get("/annonces") }

    // MARK: Mécanique

    public func getBrut(_ chemin: String) async throws -> Data { try await envoyer("GET", chemin, corps: nil) }

    public func get<T: Decodable>(_ chemin: String, authentifie: Bool = true) async throws -> T {
        try decoder(try await envoyer("GET", chemin, corps: nil, authentifie: authentifie))
    }

    func post<T: Decodable>(_ chemin: String, _ corps: [String: String]) async throws -> T {
        try decoder(try await envoyer("POST", chemin, corps: try Self.encodeur.encode(corps)))
    }

    public func decoder<T: Decodable>(_ data: Data, brut: Bool = false) throws -> T {
        do {
            return try (brut ? Self.decodeurBrut : Self.decodeur).decode(T.self, from: data)
        } catch {
            throw ErreurApi.format
        }
    }

    func envoyer(_ methode: String, _ chemin: String, corps: Data?, authentifie: Bool = true) async throws -> Data {
        guard let url = URL(string: baseURL.absoluteString.trimmingSuffix("/") + chemin) else { throw ErreurApi.format }
        var entetes = ["Accept": "application/json"]
        if corps != nil { entetes["Content-Type"] = "application/json" }
        if authentifie {
            guard let t = jeton() else { throw ErreurApi.nonAuthentifie }
            entetes["Authorization"] = "Bearer \(t)"
        }
        let reponse: ReponseHttp
        do {
            reponse = try await transport.executer(RequeteHttp(methode: methode, url: url, entetes: entetes, corps: corps))
        } catch let e as ErreurApi {
            throw e
        } catch {
            throw ErreurApi.horsLigne
        }
        let texte = String(decoding: reponse.corps, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
        func message(_ defaut: String) -> String {
            (texte.isEmpty || texte.count > 300 || texte.hasPrefix("<")) ? defaut : texte
        }
        switch reponse.statut {
        case 200..<300: return reponse.corps
        case 401: throw ErreurApi.nonAuthentifie
        case 403: throw ErreurApi.interdit(message("Action non autorisée."))
        case 404: throw ErreurApi.introuvable(message("Élément introuvable."))
        case 400..<500: throw ErreurApi.refus(reponse.statut, message("Demande refusée."))
        default: throw ErreurApi.serveur(reponse.statut)
        }
    }
}

/// Chemins de lecture partagés (cache hors ligne).
public enum Chemins {
    public static func mesCreneaux(_ jour: String) -> String { "/edt/mes-creneaux?jour=\(q(jour))" }
    public static func edtEleve(_ id: String, _ jour: String) -> String { "/eleves/\(seg(id))/edt?jour=\(q(jour))" }
    public static func cahier(_ eleveId: String?) -> String { "/cahier-texte" + (eleveId.map { "?eleve_id=\(q($0))" } ?? "") }
    public static func notes(_ eleveId: String) -> String { "/eleves/\(seg(eleveId))/notes" }
    public static let fils = "/messages/fils"
    public static let enfants = "/famille/enfants"
}

private let nonReserves: CharacterSet = {
    var c = CharacterSet.alphanumerics.intersection(CharacterSet(charactersIn: Unicode.Scalar(0)...Unicode.Scalar(127)))
    c.insert(charactersIn: "-._~")
    return c
}()

/// Encode un segment de chemin ou une valeur de paramètre (« 3e A » → `3e%20A`).
func seg(_ s: String) -> String { s.addingPercentEncoding(withAllowedCharacters: nonReserves) ?? s }
func q(_ s: String) -> String { seg(s) }

extension String {
    func trimmingSuffix(_ suffixe: String) -> String {
        var s = self
        while s.hasSuffix(suffixe) { s.removeLast(suffixe.count) }
        return s
    }
}

/// Transport réel. Pas de redirection suivie, pas de cache HTTP disque, pas de cookies.
public final class TransportURLSession: NSObject, Transport, URLSessionTaskDelegate, @unchecked Sendable {
    private lazy var session: URLSession = {
        let conf = URLSessionConfiguration.ephemeral
        conf.timeoutIntervalForRequest = 20
        conf.httpShouldSetCookies = false
        conf.urlCache = nil
        return URLSession(configuration: conf, delegate: self, delegateQueue: nil)
    }()
    private let userAgent: String

    public init(userAgent: String = "ScolIA-iOS") { self.userAgent = userAgent }

    public func executer(_ r: RequeteHttp) async throws -> ReponseHttp {
        var req = URLRequest(url: r.url)
        req.httpMethod = r.methode
        req.httpBody = r.corps
        for (k, v) in r.entetes { req.setValue(v, forHTTPHeaderField: k) }
        req.setValue(userAgent, forHTTPHeaderField: "User-Agent")
        let (data, rep) = try await session.data(for: req)
        return ReponseHttp(statut: (rep as? HTTPURLResponse)?.statusCode ?? 0, corps: data)
    }

    public func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                           newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) {
        completionHandler(nil) // Jamais de redirection : le jeton ne doit pas partir ailleurs.
    }
}
