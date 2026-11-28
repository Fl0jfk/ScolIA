import Foundation

// « Mes autres comptes » (transverse individu, migration 279, `docs/socles/TRANSVERSE_INDIVIDU.md` §10 et §11).
// Miroir de `Individu.kt` : modèles (copie d'`ent-types/src/individu.rs`), trois appels, textes de l'écran web
// (`ent-web-app/src/modules/individu/modele.rs`), à garder identiques. Les routes de la direction (relier, lever,
// propositions) restent sur le web (D-003). Fichier à part pour ne pas toucher `ApiClient`/`Modeles`, modifiés en
// parallèle.

// MARK: - Modèles (décodés avec `.convertFromSnakeCase`)

/// Une demande reçue d'un autre compte, à confirmer avant `expireLe`.
public struct DeclarationRecue: Decodable, Equatable, Identifiable, Sendable {
    public let id: String
    /// Nom affiché du compte déclarant.
    public let declarant: String
    public let declareLe: String
    public let expireLe: String
    public init(id: String, declarant: String, declareLe: String, expireLe: String) {
        self.id = id; self.declarant = declarant; self.declareLe = declareLe; self.expireLe = expireLe
    }
}

/// Un lien actif de l'individu : quand et comment, **jamais** l'autre fiche.
public struct MonLienIndividu: Decodable, Equatable, Sendable {
    public let origine: String
    public let motifCode: String
    public let poseLe: String
    public init(origine: String, motifCode: String, poseLe: String) {
        self.origine = origine; self.motifCode = motifCode; self.poseLe = poseLe
    }
}

/// `GET /dossier/individu/moi`. `liens` est absent d'un serveur antérieur aux écrans.
public struct MonIndividu: Decodable, Equatable, Sendable {
    public let autresComptes: Int64
    public let fichesReliees: Int64
    public let declarationsRecues: [DeclarationRecue]
    public let liens: [MonLienIndividu]

    enum CodingKeys: String, CodingKey { case autresComptes, fichesReliees, declarationsRecues, liens }

    public init(autresComptes: Int64, fichesReliees: Int64, declarationsRecues: [DeclarationRecue] = [], liens: [MonLienIndividu] = []) {
        self.autresComptes = autresComptes; self.fichesReliees = fichesReliees
        self.declarationsRecues = declarationsRecues; self.liens = liens
    }

    public init(from d: Decoder) throws {
        let c = try d.container(keyedBy: CodingKeys.self)
        autresComptes = try c.decode(Int64.self, forKey: .autresComptes)
        fichesReliees = try c.decode(Int64.self, forKey: .fichesReliees)
        declarationsRecues = try c.decodeIfPresent([DeclarationRecue].self, forKey: .declarationsRecues) ?? []
        liens = try c.decodeIfPresent([MonLienIndividu].self, forKey: .liens) ?? []
    }
}

private struct DeclarationIndividuBody: Encodable { let email: String }
private struct ReponseDeclarationBody: Encodable { let confirmer: Bool }

// MARK: - Appels

public enum CheminsIndividu {
    public static let moi = "/dossier/individu/moi"
    public static let declarations = "/dossier/individu/declarations"
    public static func reponse(_ id: String) -> String { "/dossier/individu/declarations/\(seg(id))/reponse" }
}

public extension ApiClient {
    /// Ce que l'établissement reconnaît, et les demandes reçues. Jamais mis en cache.
    func individuMoi() async throws -> MonIndividu { try await get(CheminsIndividu.moi) }

    /// « Ce compte est aussi moi » : 202 **identique** que l'adresse soit connue ou non ; afficher toujours
    /// `Individu.messageDeclaration`. Corps vide : rien à lire.
    func declarerAutreCompte(email: String, cle: String? = nil) async throws {
        _ = try await envoyer("POST", CheminsIndividu.declarations, corps: try Self.encodeur.encode(DeclarationIndividuBody(email: email)), cle: cle)
    }

    /// Confirmer ou refuser une demande reçue : l'identifiant du lien posé (confirmation), sinon `nil`.
    func repondreDeclaration(_ id: String, confirmer: Bool, cle: String? = nil) async throws -> String? {
        let data = try await envoyer("POST", CheminsIndividu.reponse(id), corps: try Self.encodeur.encode(ReponseDeclarationBody(confirmer: confirmer)), cle: cle)
        return try decoder(data) as String?
    }
}

// MARK: - Logique d'écran (miroir de `modules/individu/modele.rs`)

public enum Individu {
    public static let effetLien = "Les deux fiches seront traitées comme une seule personne : cela ajoute des refus "
        + "(une personne ne décide pas pour elle-même ni pour son enfant, même depuis un autre compte) et n'ouvre aucun "
        + "accès. Les comptes concernés sont prévenus. Seule la direction peut lever ce lien, jamais la personne concernée."

    public static let messageDeclaration = "Demande enregistrée. Si cette adresse est celle d'un compte de "
        + "l'établissement, ce compte reçoit une demande à confirmer dans les 24 heures. Connectez-vous avec lui pour "
        + "répondre."

    public static let pourquoi = "Certaines décisions ne se prennent jamais pour soi-même ni pour son enfant : une aide "
        + "accordée à son propre enfant, une seconde validation de sa propre demande, une absence que l'on validerait "
        + "soi-même. Reconnaître vos comptes comme une seule personne empêche qu'un second compte contourne ces règles. "
        + "Cela n'ouvre aucun accès supplémentaire, et vous êtes prévenu(e) de chaque lien posé ou levé."

    public static let introDemandes = "Un autre compte de l'établissement déclare être aussi vous. Ne confirmez que si ce "
        + "compte est bien le vôtre."

    public static let effetRefus = "Rien ne sera relié. Ce compte ne pourra plus vous le redemander pendant 30 jours ; "
        + "vous pourrez toujours le déclarer vous-même."

    public static let aideDeclarer = "Saisissez l'adresse e-mail de votre autre compte dans l'établissement. La demande se "
        + "confirme ensuite depuis cet autre compte, dans les 24 heures : sans confirmation, elle ne vaut rien. Cinq "
        + "demandes par jour au plus."

    public static let lienFaux = "Un lien vous semble faux ? Signalez-le au secrétariat : seule la direction peut le lever."

    public static let aucuneDemande = "Quand un autre compte déclare être aussi vous, la demande apparaît ici pendant 24 heures."

    public static func libelleOrigine(_ code: String) -> String {
        switch code {
        case "declaration_direction": return "Relié par l'établissement"
        case "declaration_confirmee": return "Déclaré puis confirmé depuis les deux comptes"
        case "proposition_acceptee": return "Ressemblance confirmée par l'établissement"
        default: return "Lien enregistré"
        }
    }

    public static func libelleMotifLien(_ code: String) -> String {
        switch code {
        case "deux_comptes": return "Deux comptes de la même personne"
        case "fiche_sans_compte": return "Fiche sans compte de la même personne"
        case "doublon_agent": return "Deux fiches de personnel de la même personne"
        case "autre": return "Autre motif"
        default: return "Motif non listé"
        }
    }

    public static func resumeMoi(_ m: MonIndividu) -> String {
        let autres: String
        switch m.autresComptes {
        case ...0: return "Aucun autre compte n'est reconnu comme le vôtre dans cet établissement."
        case 1: autres = "1 autre compte"
        default: autres = "\(m.autresComptes) autres comptes"
        }
        return "\(autres) de l'établissement vous appartiennent aussi : l'établissement vous reconnaît comme une seule personne."
    }

    public static func resumeFiches(_ m: MonIndividu) -> String? {
        m.fichesReliees > 1
            ? "\(m.fichesReliees) fiches de l'établissement vous désignent (fiches de personnel, de responsable ou d'élève)."
            : nil
    }

    public enum AdresseInvalide: Error, Equatable, LocalizedError {
        case vide, invalide
        public var errorDescription: String? {
            switch self {
            case .vide: return "Saisissez l'adresse e-mail de votre autre compte."
            case .invalide: return "Cette adresse e-mail n'est pas valide."
            }
        }
    }

    /// Contrôle local d'une adresse (le serveur refait le même) : l'adresse normalisée, ou l'erreur à afficher.
    public static func adresseAEnvoyer(_ saisie: String) -> Result<String, AdresseInvalide> {
        let e = saisie.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if e.isEmpty { return .failure(.vide) }
        // Longueur en octets, comme `str::len` côté serveur.
        if e.utf8.count > 254 || !e.contains("@") || e.hasPrefix("@") || e.hasSuffix("@")
            || e.unicodeScalars.contains(where: { CharacterSet.whitespacesAndNewlines.contains($0) }) {
            return .failure(.invalide)
        }
        return .success(e)
    }

    public static func messageReponse(confirmer: Bool) -> String {
        confirmer ? "Confirmé : les deux comptes sont reconnus comme une seule personne." : "Demande refusée : rien n'a été relié."
    }

    /// Liens les plus récents d'abord (ordre stable).
    public static func trierLiens(_ v: [MonLienIndividu]) -> [MonLienIndividu] {
        v.sorted { a, b in
            let (da, db) = (instant(a.poseLe) ?? .distantPast, instant(b.poseLe) ?? .distantPast)
            return da != db ? da > db : a.origine < b.origine
        }
    }

    /// Demandes qui expirent le plus tôt d'abord.
    public static func trierDemandes(_ v: [DeclarationRecue]) -> [DeclarationRecue] {
        v.sorted { a, b in
            let (da, db) = (instant(a.expireLe) ?? .distantFuture, instant(b.expireLe) ?? .distantFuture)
            return da != db ? da < db : a.id < b.id
        }
    }

    public static func detailLien(_ l: MonLienIndividu, fuseau: TimeZone = .current) -> String {
        "\(libelleMotifLien(l.motifCode)) · depuis le \(dateHeure(l.poseLe, fuseau: fuseau))"
    }

    public static func detailDemande(_ d: DeclarationRecue, fuseau: TimeZone = .current) -> String {
        "Demandé le \(dateHeure(d.declareLe, fuseau: fuseau)) · à confirmer avant le \(dateHeure(d.expireLe, fuseau: fuseau))"
    }

    /// Clés d'intention (idempotence) : même saisie ou même réponse, même clé tant que l'issue est incertaine.
    public static func intentionDeclarer(_ email: String) -> String { "individu-declarer:\(email)" }
    public static func intentionRepondre(_ id: String, confirmer: Bool) -> String { "individu-repondre:\(id):\(confirmer)" }

    /// Horodatage ISO du serveur → `28/11/2026 à 05h50` dans le fuseau de l'appareil ; brut si illisible.
    public static func dateHeure(_ iso: String, fuseau: TimeZone = .current) -> String {
        guard let d = instant(iso) else { return iso }
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.calendar = Calendar(identifier: .gregorian)
        f.timeZone = fuseau
        f.dateFormat = "dd/MM/yyyy 'à' HH'h'mm"
        return f.string(from: d)
    }

    /// RFC 3339, fraction de seconde de longueur quelconque (chrono en écrit 0, 3, 6 ou 9 chiffres) : la fraction est
    /// retirée avant la lecture (une minute près suffit à l'affichage).
    static func instant(_ iso: String) -> Date? {
        var s = iso
        if let point = s.firstIndex(of: "."), let fin = s[point...].firstIndex(where: { !$0.isNumber && $0 != "." }) {
            s.removeSubrange(point..<fin)
        }
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime]
        return f.date(from: s)
    }
}
