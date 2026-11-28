import Foundation

/// Écrans de l'appli quotidienne (D-003), dans l'ordre d'affichage par défaut.
public enum Module: String, CaseIterable, Identifiable, Sendable {
    case accueil, appel, vieScolaire, absences, notes, saisieNotes, agenda, edt, messagerie, signaler, profil
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
        case .signaler: return "Signaler un incident"
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
        case .signaler: return "lock.shield"
        case .profil: return "person.crop.circle"
        }
    }
}

/// Famille de profil : sert uniquement à choisir la **source des données** (l'élève lui-même,
/// l'enfant d'une famille, ou les classes du personnel), jamais à décider des écrans du personnel.
public enum Profil: Sendable { case eleve, famille, personnel }

public struct Espace: Equatable, Sendable {
    public let profil: Profil
    public let modules: [Module]
    /// Passkey à enrôler avant tout (D-009).
    public let enrolementRequis: Bool
    /// Le personnel a une portée « mes classes / mes groupes » (cours, appel, cahier de textes).
    public let enseigne: Bool
    /// Onglets visibles (TabView : 4 + « Plus » géré par iOS au-delà de 5).
    public var onglets: [Module] { Array(modules.prefix(4)) }
    public var autres: [Module] { Array(modules.dropFirst(4)) }
    public func contient(_ m: Module) -> Bool { modules.contains(m) }

    public init(profil: Profil, modules: [Module], enrolementRequis: Bool, enseigne: Bool = false) {
        self.profil = profil
        self.modules = modules
        self.enrolementRequis = enrolementRequis
        self.enseigne = enseigne
    }
}

/// Déduit les écrans de `/auth/me` et `/roles/mes-droits`.
///
/// - **Personnel** (tout rôle sauf élève et parent) : écrans calculés **uniquement** à partir des
///   actions et portées renvoyées par le serveur (D-017) ; aucun nom de rôle codé en dur, donc un
///   rôle libre créé par l'établissement obtient les bons écrans.
/// - **Élève et famille** : `droits_effectifs` ne renvoie aucun droit pour eux (lien légal, hors
///   rôles libres) ; seul le type de compte (`/auth/me`) les distingue.
///
/// Affichage seulement : chaque appel reste contrôlé par le serveur (D-003, D-017).
public enum Capacites {
    private static let portéesEnseignant: Set<String> = ["mes_classes", "mes_groupes"]

    public static func espace(me: Me, droits: [Droit]) -> Espace {
        let profil = profilDe(me: me)
        if me.portee == "enrolement" { return Espace(profil: profil, modules: [.profil], enrolementRequis: true) }
        var m: [Module] = [.accueil]
        var enseigne = false
        switch profil {
        case .eleve: m += [.edt, .agenda, .notes, .messagerie]
        case .famille: m += [.absences, .notes, .agenda, .messagerie, .edt]
        case .personnel:
            func a(_ action: String, _ portee: (String) -> Bool = { _ in true }) -> Bool {
                droits.contains { $0.action == action && portee($0.portee) }
            }
            enseigne = droits.contains { portéesEnseignant.contains($0.portee) }
            // Appel : cours du professeur (mes classes / groupes) ou déclaration d'absences ;
            // la liste de ses créneaux reste filtrée côté serveur.
            if (a("absences.lire") && enseigne) || a("absences.declarer") { m.append(.appel) }
            if a("absences.lire", { $0 == "etablissement" }) || a("appel.verifier")
                || a("absences.justifier") || a("assiduite.signaler") {
                m.append(.vieScolaire)
            }
            // Saisie de notes sur la grille d'évaluation `/eva/*` (palier « notes eva ») ; l'ancienne route
            // `POST /eleves/{id}/notes` reste fermée (410, V0 D1). Le serveur ne rend que les cours du compte.
            if a("notes.saisir") { m.append(.saisieNotes) }
            if enseigne { m.append(.agenda) }
            m += [.messagerie, .edt]
            // Registre des violations (socle 27) : tout personnel déclare en trente secondes ; le registre reste
            // sur le web. Élèves et familles passent par le guichet des droits.
            if a("violations.declarer") { m.append(.signaler) }
        }
        let ordre: [Module] = [.accueil, .appel, .vieScolaire, .absences, .messagerie, .notes, .edt, .agenda, .saisieNotes, .signaler]
        let tries = m.sorted { (ordre.firstIndex(of: $0) ?? 99) < (ordre.firstIndex(of: $1) ?? 99) }
        return Espace(profil: profil, modules: tries + [.profil], enrolementRequis: false, enseigne: enseigne)
    }

    /// Élève (compte lié à un élève) ou parent ; tout autre compte est du personnel.
    public static func profilDe(me: Me) -> Profil {
        let role = me.role.lowercased()
        if role == "eleve" || me.eleveId != nil { return .eleve }
        if role == "parent" { return .famille }
        return .personnel
    }
}

/// Bandeaux de sécurité de la coquille (M3 et M5 de MB1), miroir de `Avertissement` (Kotlin).
/// Ils informent sans bloquer : le serveur reste seul juge de la durée de session (MB0, DS2).
public enum Avertissement: String, CaseIterable, Sendable {
    /// Session ouverte sans passkey : le serveur lui donne la durée du web (quelques heures).
    case sessionCourte
    /// Aucun code sur l'appareil : ScolIA ne peut pas se verrouiller (D-011).
    case appareilSansVerrou

    public var titre: String {
        switch self {
        case .sessionCourte: return "Session courte"
        case .appareilSansVerrou: return "iPhone sans code"
        }
    }

    public var texte: String {
        switch self {
        case .sessionCourte:
            return "Vous êtes connecté sans passkey : la session expirera dans quelques heures. Connectez-vous avec une passkey (à créer dans « Mon compte » si besoin) pour rester connecté sur cet iPhone."
        case .appareilSansVerrou:
            return "Cet iPhone n'a pas de code : ScolIA ne peut pas se verrouiller. Activez un code dans Réglages pour protéger les données des élèves."
        }
    }

    public static func pour(_ me: Me, appareilSansVerrou: Bool) -> [Avertissement] {
        var a: [Avertissement] = []
        if me.portee == "complete" && me.assurance != "passkey" { a.append(.sessionCourte) }
        if appareilSansVerrou { a.append(.appareilSansVerrou) }
        return a
    }
}
