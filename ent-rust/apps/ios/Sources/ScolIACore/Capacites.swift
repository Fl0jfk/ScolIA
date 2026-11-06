import Foundation

/// Écrans de l'appli quotidienne (D-003), dans l'ordre d'affichage par défaut.
public enum Module: String, CaseIterable, Identifiable, Sendable {
    case accueil, appel, vieScolaire, absences, notes, saisieNotes, agenda, edt, messagerie, profil
    public var id: String { rawValue }
    public var titre: String {
        switch self {
        case .accueil: return "Accueil"
        case .appel: return "Appel"
        case .vieScolaire: return "Vie scolaire"
        case .absences: return "Absences"
        case .notes: return "Notes"
        case .saisieNotes: return "Saisir des notes"
        case .agenda: return "Cahier de textes"
        case .edt: return "Emploi du temps"
        case .messagerie: return "Messages"
        case .profil: return "Mon compte"
        }
    }
    /// Symbole SF Symbols.
    public var symbole: String {
        switch self {
        case .accueil: return "house"
        case .appel: return "checklist"
        case .vieScolaire: return "exclamationmark.triangle"
        case .absences: return "calendar.badge.exclamationmark"
        case .notes: return "star"
        case .saisieNotes: return "square.and.pencil"
        case .agenda: return "book"
        case .edt: return "calendar"
        case .messagerie: return "envelope"
        case .profil: return "person.crop.circle"
        }
    }
}

public enum Profil: Sendable { case eleve, famille, professeur, vieScolaire, direction, personnel }

public struct Espace: Equatable, Sendable {
    public let profil: Profil
    public let modules: [Module]
    /// Passkey à enrôler avant tout (D-009).
    public let enrolementRequis: Bool
    /// Onglets visibles (TabView : 4 + « Plus » géré par iOS au-delà de 5).
    public var onglets: [Module] { Array(modules.prefix(4)) }
    public var autres: [Module] { Array(modules.dropFirst(4)) }
    public func contient(_ m: Module) -> Bool { modules.contains(m) }
}

/// Déduit les écrans de `/auth/me` et `/roles/mes-droits`. Affichage seulement :
/// chaque appel reste contrôlé par le serveur (D-003, D-017).
public enum Capacites {
    public static func espace(me: Me, droits: [Droit]) -> Espace {
        let profil = profilDe(me: me, droits: droits)
        if me.portee == "enrolement" { return Espace(profil: profil, modules: [.profil], enrolementRequis: true) }
        let actions = Set(droits.map(\.action))
        var m: [Module] = [.accueil]
        switch profil {
        case .eleve: m += [.edt, .agenda, .notes, .messagerie]
        case .famille: m += [.absences, .notes, .agenda, .messagerie, .edt]
        default:
            if profil == .professeur { m.append(.appel) }
            if actions.contains("absences.lire") || actions.contains("appel.verifier") || profil == .vieScolaire || profil == .direction {
                m.append(.vieScolaire)
            }
            m.append(.messagerie)
            if actions.contains("notes.saisir") || profil == .professeur { m.append(.saisieNotes) }
            m.append(.edt)
            if profil == .professeur { m.append(.agenda) }
        }
        let ordre: [Module] = [.accueil, .appel, .vieScolaire, .absences, .messagerie, .notes, .edt, .agenda, .saisieNotes]
        let tries = m.sorted { (ordre.firstIndex(of: $0) ?? 99) < (ordre.firstIndex(of: $1) ?? 99) }
        return Espace(profil: profil, modules: tries + [.profil], enrolementRequis: false)
    }

    public static func profilDe(me: Me, droits: [Droit]) -> Profil {
        let role = me.role.lowercased()
        if role == "eleve" || me.eleveId != nil { return .eleve }
        if role == "parent" { return .famille }
        if role == "prof" { return .professeur }
        if role == "admin" || role == "direction" { return .direction }
        if role == "cpe" { return .vieScolaire }
        // Rôles libres de l'établissement (D-017) : on regarde les droits.
        if droits.contains(where: { $0.action == "appel.verifier" || ($0.action == "absences.lire" && $0.portee == "etablissement") }) {
            return .vieScolaire
        }
        return .personnel
    }
}
