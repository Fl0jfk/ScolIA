import Foundation

// Modèles JSON de l'API ScolIA (`ent-rust/crates/ent-api`), copiés des DTO serveur.
// Décodage avec `.convertFromSnakeCase` (voir `ApiClient.decodeur`), sauf les types
// passkey qui transportent du JSON WebAuthn en camelCase (décodeur sans stratégie).
// Dates : chaînes ISO 8601 ; états : chaînes, comme dans `ent-types`.

public let clientMobile = "mobile"

// MARK: - JSON libre (options WebAuthn)

/// Valeur JSON quelconque, pour transporter les options WebAuthn sans les altérer.
public enum JSONValue: Codable, Equatable, Sendable {
    case objet([String: JSONValue])
    case tableau([JSONValue])
    case texte(String)
    case nombre(Double)
    case booleen(Bool)
    case nul

    public init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() { self = .nul }
        else if let b = try? c.decode(Bool.self) { self = .booleen(b) }
        else if let n = try? c.decode(Double.self) { self = .nombre(n) }
        else if let s = try? c.decode(String.self) { self = .texte(s) }
        else if let a = try? c.decode([JSONValue].self) { self = .tableau(a) }
        else { self = .objet(try c.decode([String: JSONValue].self)) }
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        switch self {
        case .objet(let o): try c.encode(o)
        case .tableau(let a): try c.encode(a)
        case .texte(let s): try c.encode(s)
        case .nombre(let n): try c.encode(n)
        case .booleen(let b): try c.encode(b)
        case .nul: try c.encodeNil()
        }
    }

    public subscript(cle: String) -> JSONValue? {
        if case .objet(let o) = self { return o[cle] }
        return nil
    }

    public var texte: String? {
        if case .texte(let s) = self { return s }
        return nil
    }

    public var tableau: [JSONValue]? {
        if case .tableau(let a) = self { return a }
        return nil
    }
}

// MARK: - Session et identité

/// Défi passkey : options WebAuthn (`webauthn-rs`) + jeton opaque.
public struct PasskeyChallenge: Codable, Equatable, Sendable {
    public let token: String
    public let options: JSONValue
    public let expiresAt: String?

    enum CodingKeys: String, CodingKey {
        case token, options
        case expiresAt = "expires_at"
    }

    public init(token: String, options: JSONValue, expiresAt: String? = nil) {
        self.token = token
        self.options = options
        self.expiresAt = expiresAt
    }
}

/// Session ouverte. Le jeton n'apparaît jamais dans `description`.
public struct AuthSession: Codable, Sendable, CustomStringConvertible {
    public let token: String
    public let userId: String
    public let email: String
    public let etablissementId: String
    public let role: String
    public let expiresAt: String

    enum CodingKeys: String, CodingKey {
        case token, email, role
        case userId = "user_id"
        case etablissementId = "etablissement_id"
        case expiresAt = "expires_at"
    }

    public var description: String { "AuthSession(token: <masqué>, role: \(role))" }
}

/// Réponse de `POST /auth/login` : session à plat, ou second facteur passkey.
public struct LoginResponse: Codable, Sendable, CustomStringConvertible {
    public var description: String { "LoginResponse(mfaRequired: \(mfaRequired), role: \(role ?? "-"))" }
    public let mfaRequired: Bool
    public let token: String?
    public let userId: String?
    public let email: String?
    public let etablissementId: String?
    public let role: String?
    public let expiresAt: String?
    public let passkey: PasskeyChallenge?

    enum CodingKeys: String, CodingKey {
        case token, email, role, passkey
        case mfaRequired = "mfa_required"
        case userId = "user_id"
        case etablissementId = "etablissement_id"
        case expiresAt = "expires_at"
    }

    public var session: AuthSession? {
        guard !mfaRequired, let token, let userId, let email, let etablissementId, let role, let expiresAt else { return nil }
        return AuthSession(token: token, userId: userId, email: email, etablissementId: etablissementId, role: role, expiresAt: expiresAt)
    }
}

/// `GET /auth/me`.
public struct Me: Codable, Equatable, Sendable {
    public let userId: String
    public let email: String
    public let etablissementId: String
    public let role: String
    public let eleveId: String?
    public let passkeyCount: Int
    /// `complete` ou `enrolement` (passkey à enrôler avant tout, D-009).
    public let portee: String
    public let assurance: String
    public let client: String
    public let passkeyObligatoire: Bool

    public init(userId: String, email: String, etablissementId: String, role: String, eleveId: String? = nil,
                passkeyCount: Int = 0, portee: String = "complete", assurance: String = "", client: String = "",
                passkeyObligatoire: Bool = false) {
        self.userId = userId; self.email = email; self.etablissementId = etablissementId; self.role = role
        self.eleveId = eleveId; self.passkeyCount = passkeyCount; self.portee = portee
        self.assurance = assurance; self.client = client; self.passkeyObligatoire = passkeyObligatoire
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        userId = try c.decode(String.self, forKey: .userId)
        email = try c.decode(String.self, forKey: .email)
        etablissementId = try c.decode(String.self, forKey: .etablissementId)
        role = try c.decode(String.self, forKey: .role)
        eleveId = try c.decodeIfPresent(String.self, forKey: .eleveId)
        passkeyCount = try c.decodeIfPresent(Int.self, forKey: .passkeyCount) ?? 0
        portee = try c.decodeIfPresent(String.self, forKey: .portee) ?? "complete"
        assurance = try c.decodeIfPresent(String.self, forKey: .assurance) ?? ""
        client = try c.decodeIfPresent(String.self, forKey: .client) ?? ""
        passkeyObligatoire = try c.decodeIfPresent(Bool.self, forKey: .passkeyObligatoire) ?? false
    }
}

/// Droit effectif (`GET /roles/mes-droits`, D-017).
public struct Droit: Codable, Equatable, Sendable {
    public let action: String
    public let portee: String
    public let cibles: [String]?
    public init(action: String, portee: String, cibles: [String]? = nil) {
        self.action = action; self.portee = portee; self.cibles = cibles
    }
}

public struct EtablissementPublic: Codable, Sendable {
    public let id: String
    public let slug: String
    public let nom: String
}

public struct SessionAppareil: Codable, Identifiable, Sendable {
    public let id: String
    public let client: String
    public let assurance: String
    public let appareil: String?
    public let createdAt: String
    public let lastSeenAt: String
    public let expiresAt: String
    public let courante: Bool
}

// MARK: - EDT et appel

public struct EdtCreneau: Codable, Identifiable, Sendable {
    public let id: String
    public let classe: String
    public let matiereLibelle: String
    public let profUserId: String
    public let profLabel: String?
    /// 1 = lundi … 7 = dimanche (ISO).
    public let jourSemaine: Int
    public let heureDebut: String
    public let heureFin: String
    public let salle: String?
}

public enum EtatAppel: String, CaseIterable, Codable, Sendable {
    case present, absent, retard
    public var libelle: String {
        switch self {
        case .present: return "Présent"
        case .absent: return "Absent"
        case .retard: return "En retard"
        }
    }
}

public struct AppelLigne: Codable, Identifiable, Equatable, Sendable {
    public let eleveId: String
    public let eleveLabel: String?
    public var etat: String
    public var id: String { eleveId }
}

public struct Appel: Codable, Identifiable, Equatable, Sendable {
    public let id: String
    public let classe: String
    public let jour: String
    public let statut: String
    public let creneauId: String?
    public let matiereLibelle: String?
    public var lignes: [AppelLigne]
    public var estClos: Bool { statut == "clos" }
}

// MARK: - Famille et absences

public struct EnfantFamille: Codable, Identifiable, Hashable, Sendable {
    public let id: String
    public let nom: String
    public let prenom: String
    public let classe: String?
    public let peutVoirNotes: Bool
    public let peutVoirVs: Bool
    public let peutJustifierAbsence: Bool
}

public struct Absence: Codable, Identifiable, Sendable {
    public let id: String
    public let eleveId: String
    public let eleveLabel: String?
    public let debut: String
    public let fin: String
    public let motif: String
    public let statut: String
    public let motifJustif: String?
}

public struct CarnetEntree: Codable, Identifiable, Sendable {
    public let id: String
    public let dateEntree: String
    public let titre: String
}

public struct FamilleResume: Codable, Sendable {
    public let eleveId: String
    public let absencesAJustifier: [Absence]
    public let carnetASigner: [CarnetEntree]
    public let notes: [Note]
}

// MARK: - Notes, devoirs, cahier de textes

public struct Note: Codable, Identifiable, Sendable {
    public let id: String
    public let eleveId: String
    public let matiereId: String
    public let matiereLibelle: String?
    public let valeur: Double
    public let sur: Double
    public let dateNote: String
    public let libelle: String?
}

public struct NotePeriode: Codable, Identifiable, Sendable {
    public let id: String
    public let code: String
    public let libelle: String
    public let statut: String
}

public struct Devoir: Codable, Identifiable, Sendable {
    public let id: String
    public let periodeId: String
    public let matiereId: String
    public let matiereLibelle: String?
    public let classe: String
    public let titre: String
    public let dateDevoir: String
    public let sur: Double
}

public struct EleveEffectif: Codable, Identifiable, Sendable {
    public let id: String
    public let nom: String
    public let prenom: String
}

public struct Effectif: Codable, Sendable {
    public let classe: String
    public let jour: String
    public let eleves: [EleveEffectif]
}

public struct SaisirNoteBody: Codable, Sendable {
    public let matiereId: String
    public let valeur: Double
    public let sur: Double?
    public let dateNote: String
    public let libelle: String?
    public let devoirId: String?
    public let periodeId: String?
    public init(matiereId: String, valeur: Double, sur: Double?, dateNote: String, libelle: String?, devoirId: String?, periodeId: String?) {
        self.matiereId = matiereId; self.valeur = valeur; self.sur = sur; self.dateNote = dateNote
        self.libelle = libelle; self.devoirId = devoirId; self.periodeId = periodeId
    }
}

public struct CahierTexte: Codable, Identifiable, Sendable {
    public let id: String
    public let classe: String
    public let matiereLibelle: String
    public let titre: String
    public let consigne: String
    public let pourLe: String
}

// MARK: - Messagerie, annonces

public struct MsgFil: Codable, Identifiable, Sendable {
    public let id: String
    public let eleveId: String
    public let eleveLabel: String?
    public let sujet: String
    public let lastMessageAt: String
    public let nbNonLus: Int
}

public struct MsgPiece: Codable, Identifiable, Sendable {
    public let id: String
    public let nom: String
}

public struct Message: Codable, Identifiable, Sendable {
    public let id: String
    public let filId: String
    public let auteurLabel: String?
    public let corps: String
    public let createdAt: String
    public let pieces: [MsgPiece]
}

public struct FilDetail: Codable, Sendable {
    public let fil: MsgFil
    public let messages: [Message]
}

public struct Annonce: Codable, Identifiable, Sendable {
    public let id: String
    public let titre: String
    public let corps: String
    public let createdAt: String
}
