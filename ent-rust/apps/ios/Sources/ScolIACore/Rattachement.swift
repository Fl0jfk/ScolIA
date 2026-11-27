import Foundation

/// Établissement mémorisé (D-012 : un sous-domaine par établissement).
public struct Rattachement: Codable, Equatable, Hashable, Identifiable, Sendable {
    public let baseURL: URL
    public let slug: String
    public var nom: String?
    /// Code d'invitation lu dans le lien ou le QR code (non mémorisé).
    public var invitation: String?
    /// Labo local seulement (pas de sous-domaine).
    public var etablissementIdLabo: String?
    public var id: String { baseURL.absoluteString }

    public init(baseURL: URL, slug: String, nom: String? = nil, invitation: String? = nil, etablissementIdLabo: String? = nil) {
        self.baseURL = baseURL; self.slug = slug; self.nom = nom
        self.invitation = invitation; self.etablissementIdLabo = etablissementIdLabo
    }
}

/// Lecture d'un lien d'invitation (Universal Link), d'un QR code ou d'une saisie.
/// Accepte uniquement `https://<etab>.scolia.fr[/rejoindre?invitation=…]` ou `lpnb` / `lpnb.scolia.fr`.
public struct LecteurRattachement: Sendable {
    public enum Resultat: Equatable, Sendable {
        case ok(Rattachement)
        case refuse(String)
    }

    public static let domaineScolia = "scolia.fr"
    let domaine: String
    let modeLabo: Bool

    public init(domaine: String = LecteurRattachement.domaineScolia, modeLabo: Bool = false) {
        self.domaine = domaine
        self.modeLabo = modeLabo
    }

    private static let reserves: Set<String> = ["www", "api", "app", "admin", "static", "cdn", "mail"]
    private static let hotesLabo: Set<String> = ["127.0.0.1", "localhost", "10.0.2.2"]

    public func lire(_ saisie: String) -> Resultat {
        let t = saisie.trimmingCharacters(in: .whitespacesAndNewlines)
        if t.isEmpty { return .refuse("Adresse vide.") }
        if t.count > 2048 { return .refuse("Lien trop long.") }
        if !t.contains("://") {
            let hote = t.lowercased().trimmingSuffix("/")
            return lire("https://" + (hote.contains(".") ? hote : "\(hote).\(domaine)"))
        }
        guard let c = URLComponents(string: t), let hote = c.host?.lowercased(), !hote.isEmpty else {
            return .refuse("Lien illisible.")
        }
        let schema = c.scheme?.lowercased()
        let param: (String) -> String? = { nom in c.queryItems?.first(where: { $0.name == nom })?.value }

        if modeLabo, schema == "http", Self.hotesLabo.contains(hote) {
            let port = c.port.map { ":\($0)" } ?? ""
            guard let url = URL(string: "http://\(hote)\(port)") else { return .refuse("Lien illisible.") }
            return .ok(Rattachement(baseURL: url, slug: "labo", nom: "Labo local", invitation: param("invitation"), etablissementIdLabo: param("etablissement_id")))
        }
        guard schema == "https" else { return .refuse("Seules les adresses sécurisées (https) sont acceptées.") }
        if c.user != nil || c.password != nil { return .refuse("Lien refusé.") }
        if let p = c.port, p != 443 { return .refuse("Lien refusé.") }
        guard hote.hasSuffix(".\(domaine)") else { return .refuse("Ce lien n'est pas une adresse ScolIA.") }
        let slug = String(hote.dropLast(domaine.count + 1))
        guard Self.slugValide(slug) else { return .refuse("Adresse d'établissement invalide.") }
        if Self.reserves.contains(slug) { return .refuse("Ce lien n'est pas celui d'un établissement.") }
        guard let url = URL(string: "https://\(hote)") else { return .refuse("Lien illisible.") }
        let invitation = param("invitation").flatMap { Self.invitationValide($0) ? $0 : nil }
        return .ok(Rattachement(baseURL: url, slug: slug, invitation: invitation))
    }

    static func slugValide(_ s: String) -> Bool {
        guard (1...63).contains(s.count), let premier = s.first, let dernier = s.last else { return false }
        let permis = Set("abcdefghijklmnopqrstuvwxyz0123456789-")
        return s.allSatisfy(permis.contains) && premier != "-" && dernier != "-"
    }

    static func invitationValide(_ s: String) -> Bool {
        let permis = Set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_")
        return (8...128).contains(s.count) && s.allSatisfy(permis.contains)
    }
}
