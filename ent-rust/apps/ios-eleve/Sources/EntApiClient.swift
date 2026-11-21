import Foundation

/// Client HTTP ENT — face élève (notes, cahier, messagerie, EDT, absences).
public struct EntApiClient: Sendable {
    public let baseURL: URL
    public private(set) var token: String?
    public private(set) var etablissementId: UUID?
    public private(set) var eleveId: UUID?

    public init(baseURL: URL = URL(string: "http://127.0.0.1:3001")!) {
        self.baseURL = baseURL
    }

    public struct Session: Decodable, Sendable {
        public let token: String
        public let user_id: UUID
        public let email: String
        public let etablissement_id: UUID
        public let role: String
    }

    public struct Me: Decodable, Sendable {
        public let user_id: UUID
        public let email: String
        public let role: String
        public let eleve_id: UUID?
    }

    public struct Note: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let valeur: Double?
        public let libelle: String?
        public let matiere_libelle: String?
        public let date_note: String?
        public let sur: Double?
    }

    public struct Devoir: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let titre: String
        public let matiere_libelle: String?
        public let classe: String
        public let date_devoir: String
        public let coef: Double
        public let sur: Double
    }

    public struct CahierTexte: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let matiere_libelle: String
        public let titre: String
        public let consigne: String
        public let pour_le: String
    }

    public struct MsgFil: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let eleve_id: UUID
        public let sujet: String
        public let last_message_at: String?
    }

    public struct MsgPiece: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let nom: String
        public let mime_type: String
        public let taille_octets: Int?
    }

    public struct Message: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let fil_id: UUID
        public let auteur_user_id: UUID
        public let auteur_label: String?
        public let corps: String
        public let created_at: String
        public let pieces: [MsgPiece]?
    }

    public struct FilDetail: Decodable, Sendable {
        public let fil: MsgFil
        public let messages: [Message]
    }

    public struct EdtCreneau: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let matiere_libelle: String
        public let prof_label: String?
        public let heure_debut: String
        public let heure_fin: String
        public let salle: String?
    }

    public struct MoyenneMatiere: Decodable, Sendable {
        public let matiere_id: UUID
        public let matiere_libelle: String?
        public let moyenne_sur_20: Double
    }

    public struct NotePeriode: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let libelle: String
        public let statut: String
    }

    public struct Appreciation: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let matiere_id: UUID?
        public let matiere_libelle: String?
        public let texte: String
        public let auteur_label: String?
    }

    public struct Absence: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let debut: String
        public let fin: String
        public let motif: String
        public let statut: String
        public let motif_justif: String?
    }

    public struct JournalEntree: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let domaine: String
        public let action: String
        public let libelle: String
        public let created_at: String?
    }

    public mutating func login(email: String, password: String, etablissementId: UUID) async throws -> Session {
        struct Body: Encodable {
            let email: String
            let password: String
            let etablissement_id: UUID
        }
        let session: Session = try await post(
            path: "/auth/login",
            body: Body(email: email, password: password, etablissement_id: etablissementId),
            authed: false
        )
        self.token = session.token
        self.etablissementId = session.etablissement_id
        let me: Me = try await get(path: "/auth/me")
        self.eleveId = me.eleve_id
        return session
    }

    public func registerPush(platform: String, token: String, appId: String?) async throws {
        struct Body: Encodable {
            let platform: String
            let token: String
            let app_id: String?
        }
        struct Ok: Decodable { let id: UUID? }
        let _: Ok = try await post(
            path: "/devices/push",
            body: Body(platform: platform, token: token, app_id: appId),
            authed: true
        )
    }

    public func notes(eleveId: UUID) async throws -> [Note] {
        try await get(path: "/eleves/\(eleveId.uuidString)/notes")
    }

    public func devoirsCourants(eleveId: UUID) async throws -> (NotePeriode?, [Devoir]) {
        let periodes: [NotePeriode] = try await get(path: "/note-periodes")
        guard let p = periodes.first(where: { $0.statut == "ouverte" }) ?? periodes.first else {
            return (nil, [])
        }
        let d: [Devoir] = try await get(
            path: "/eleves/\(eleveId.uuidString)/devoirs?periode_id=\(p.id.uuidString)"
        )
        return (p, d)
    }

    public func cahierTexte() async throws -> [CahierTexte] {
        try await get(path: "/cahier-texte")
    }

    public func messages() async throws -> [MsgFil] {
        try await get(path: "/messages/fils")
    }

    public func filDetail(filId: UUID) async throws -> FilDetail {
        try await get(path: "/messages/fils/\(filId.uuidString)")
    }

    public func repondre(filId: UUID, corps: String) async throws {
        struct Body: Encodable { let corps: String }
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/messages/fils/\(filId.uuidString)/repondre",
            body: Body(corps: corps),
            authed: true
        )
    }

    public func attacherPieceMessage(messageId: UUID, nom: String, mimeType: String, data: Data) async throws {
        struct Body: Encodable {
            let nom: String
            let mime_type: String
            let contenu_base64: String
        }
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/messages/\(messageId.uuidString)/pieces",
            body: Body(nom: nom, mime_type: mimeType, contenu_base64: data.base64EncodedString()),
            authed: true
        )
    }

    public func telechargerPieceMessage(pieceId: UUID) async throws -> Data {
        var req = URLRequest(url: URL(string: baseURL.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + "/messages/pieces/\(pieceId.uuidString)")!)
        req.httpMethod = "GET"
        if let token {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        let (data, resp) = try await URLSession.shared.data(for: req)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        return data
    }

    public func edt(eleveId: UUID, jour: String) async throws -> [EdtCreneau] {
        try await get(path: "/eleves/\(eleveId.uuidString)/edt?jour=\(jour)")
    }

    public func absences(eleveId: UUID) async throws -> [Absence] {
        try await get(path: "/eleves/\(eleveId.uuidString)/absences")
    }

    public func journal(eleveId: UUID, limit: Int = 50) async throws -> [JournalEntree] {
        try await get(path: "/eleves/\(eleveId.uuidString)/journal?limit=\(limit)")
    }

    public func moyennesCourantes(eleveId: UUID) async throws -> (NotePeriode?, [MoyenneMatiere]) {
        let periodes: [NotePeriode] = try await get(path: "/note-periodes")
        guard let p = periodes.first(where: { $0.statut == "ouverte" }) ?? periodes.first else {
            return (nil, [])
        }
        let m: [MoyenneMatiere] = try await get(
            path: "/eleves/\(eleveId.uuidString)/moyennes?periode_id=\(p.id.uuidString)"
        )
        return (p, m)
    }

    public func appreciationsCourantes(eleveId: UUID) async throws -> (NotePeriode?, [Appreciation]) {
        let periodes: [NotePeriode] = try await get(path: "/note-periodes")
        guard let p = periodes.first(where: { $0.statut == "ouverte" }) ?? periodes.first else {
            return (nil, [])
        }
        let a: [Appreciation] = try await get(
            path: "/eleves/\(eleveId.uuidString)/appreciations?periode_id=\(p.id.uuidString)"
        )
        return (p, a)
    }

    public func photoEleve(eleveId: UUID) async throws -> Data {
        var req = URLRequest(url: URL(string: baseURL.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + "/eleves/\(eleveId.uuidString)/photo")!)
        req.httpMethod = "GET"
        if let token {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        let (data, resp) = try await URLSession.shared.data(for: req)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        return data
    }

    private func get<T: Decodable>(path: String) async throws -> T {
        var req = URLRequest(url: URL(string: baseURL.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + path)!)
        req.httpMethod = "GET"
        if let token {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        let (data, resp) = try await URLSession.shared.data(for: req)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func post<B: Encodable, T: Decodable>(path: String, body: B, authed: Bool) async throws -> T {
        var req = URLRequest(url: URL(string: baseURL.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + path)!)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if authed, let token {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        req.httpBody = try JSONEncoder().encode(body)
        let (data, resp) = try await URLSession.shared.data(for: req)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        return try JSONDecoder().decode(T.self, from: data)
    }
}
