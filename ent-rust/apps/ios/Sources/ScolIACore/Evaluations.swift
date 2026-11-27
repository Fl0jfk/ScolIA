import Foundation

// Saisie des notes sur la grille d'évaluation `/eva/*` (socle 11, palier « notes eva » ; socle 21, lot V3).
// Copie des DTO de `ent-types/src/evaluations_v3.rs` (décodage `.convertFromSnakeCase`). **Aucun calcul de
// moyenne ici** : moyennes et statistiques viennent du moteur unique du serveur, en millièmes. Le seul calcul local
// est l'analyse du texte d'une case, copie exacte de `analyser_cellule` (vecteurs partagés
// `contrat/mobile/regles/analyse_cellule.json`) ; le serveur reste juge (droit, période, version).

/// Un cours auquel le compte a accès (`GET /eva/enseignements`). `droit` : `saisie` ou `lecture`.
public struct EnseignementEva: Codable, Equatable, Sendable, Identifiable {
    public let coursId: String
    public let libelle: String
    public let cibleCode: String
    public let matiereId: String?
    public let sansMatiere: Bool
    public let droit: String
    public var id: String { coursId }
    public var enSaisie: Bool { droit == "saisie" }
    public init(coursId: String, libelle: String, cibleCode: String, matiereId: String? = nil, sansMatiere: Bool = false, droit: String) {
        self.coursId = coursId; self.libelle = libelle; self.cibleCode = cibleCode; self.matiereId = matiereId
        self.sansMatiere = sansMatiere; self.droit = droit
    }
}

public struct PeriodeEva: Codable, Equatable, Sendable, Identifiable {
    public let id: String
    public let code: String
    public let libelle: String
    public let dateDebut: String
    public let dateFin: String
    public init(id: String, code: String, libelle: String, dateDebut: String, dateFin: String) {
        self.id = id; self.code = code; self.libelle = libelle; self.dateDebut = dateDebut; self.dateFin = dateFin
    }
    /// Dates ISO : la comparaison de chaînes suffit.
    public func contient(_ jour: String) -> Bool { dateDebut <= jour && jour <= dateFin }
}

public struct StatsEva: Codable, Equatable, Sendable {
    public let effectif: Int
    public let moyenneM: Int64
    public let minM: Int64
    public let maxM: Int64
    public let medianeM: Int64
}

/// Une colonne de la grille : une évaluation.
public struct EvaluationEva: Codable, Equatable, Sendable, Identifiable {
    public let id: String
    public let titre: String
    public let dateEval: String
    public let baremeM: Int64
    public let coefM: Int64
    public let comptage: String
    public let horsBaremeAutorise: Bool
    /// `brouillon`, `ouverte`, `annulee`.
    public let etat: String
    public let motifAnnulation: String?
    public let version: Int
    public let publication: String
    public let publicationPrevueLe: String?
    public let publieeLe: String?
    public let retireeLe: String?
    public let enCoursDeCorrection: Bool
    public let sousMatiereId: String?
    public let restreinte: Bool
    public let commune: Bool
    public let annoncee: Bool
    public let aDesResultats: Bool
    public let peutModifier: Bool
    public let retraitPossible: Bool
    public let stats: StatsEva?
    public var ouverte: Bool { etat == "ouverte" }
    public var publiee: Bool { publieeLe != nil }

    public init(id: String, titre: String, dateEval: String, baremeM: Int64 = 20_000, coefM: Int64 = 1_000, etat: String = "ouverte",
                horsBaremeAutorise: Bool = false, publieeLe: String? = nil, retraitPossible: Bool = false) {
        self.id = id; self.titre = titre; self.dateEval = dateEval; self.baremeM = baremeM; self.coefM = coefM
        comptage = "normal"; self.horsBaremeAutorise = horsBaremeAutorise; self.etat = etat; motifAnnulation = nil; version = 1
        publication = "explicite"; publicationPrevueLe = nil; self.publieeLe = publieeLe; retireeLe = nil
        enCoursDeCorrection = false; sousMatiereId = nil; restreinte = false; commune = false; annoncee = false
        aDesResultats = false; peutModifier = true; self.retraitPossible = retraitPossible; stats = nil
    }
}

/// Une case : le résultat d'un élève à une évaluation.
public struct CelluleEva: Codable, Equatable, Sendable {
    public let concerne: Bool
    public var statut: String?
    public var valeurM: Int64?
    public var version: Int?
    public let resultatId: String?
    public let saisieOuverte: Bool
    public let rattrapageAutorise: Bool
    public let horsPeriode: Bool
    public let corrige: Bool
    public let commentaire: Bool
    public init(concerne: Bool = true, statut: String? = nil, valeurM: Int64? = nil, version: Int? = nil, saisieOuverte: Bool = true) {
        self.concerne = concerne; self.statut = statut; self.valeurM = valeurM; self.version = version; resultatId = nil
        self.saisieOuverte = saisieOuverte; rattrapageAutorise = false; horsPeriode = false; corrige = false; commentaire = false
    }
}

public struct LigneEleveEva: Codable, Equatable, Sendable, Identifiable {
    public let eleveId: String
    public let nom: String
    public let prenom: String
    public let mouvement: String?
    /// Même ordre que `GrilleEva.evaluations`.
    public let cellules: [CelluleEva]
    public let moyenneM: Int64?
    public let nbComptees: Int
    public var id: String { eleveId }
    public init(eleveId: String, nom: String, prenom: String, cellules: [CelluleEva]) {
        self.eleveId = eleveId; self.nom = nom; self.prenom = prenom; mouvement = nil; self.cellules = cellules
        moyenneM = nil; nbComptees = 0
    }
}

/// `GET /eva/grille?cours_id=…&periode_id=…`.
public struct GrilleEva: Codable, Equatable, Sendable {
    public let enseignement: EnseignementEva
    public let periode: PeriodeEva
    public let jour: String
    public let peutSaisir: Bool
    public let peutCorriger: Bool
    public let echelleM: Int64
    public let pasM: Int64
    public let delaiPublicationMaxJours: Int?
    public let evaluations: [EvaluationEva]
    public let eleves: [LigneEleveEva]
    public let moyenneClasse: StatsEva?
    public let avertissements: [String]
    public init(enseignement: EnseignementEva, periode: PeriodeEva, jour: String, peutSaisir: Bool = true, pasM: Int64 = 10,
                evaluations: [EvaluationEva], eleves: [LigneEleveEva]) {
        self.enseignement = enseignement; self.periode = periode; self.jour = jour; self.peutSaisir = peutSaisir
        peutCorriger = false; echelleM = 20_000; self.pasM = pasM; delaiPublicationMaxJours = nil
        self.evaluations = evaluations; self.eleves = eleves; moyenneClasse = nil; avertissements = []
    }
}

/// Une case à écrire : le **texte tapé** (le serveur l'analyse) et la version que l'écran avait lue.
public struct CelluleSaisieEva: Codable, Equatable, Sendable {
    public let evaluationId: String
    public let eleveId: String
    public let texte: String
    /// `nil` : l'écran croyait la case vide ; sinon la version lue (une autre version en base = conflit).
    public let versionAttendue: Int?
    public init(evaluationId: String, eleveId: String, texte: String, versionAttendue: Int? = nil) {
        self.evaluationId = evaluationId; self.eleveId = eleveId; self.texte = texte; self.versionAttendue = versionAttendue
    }
}

/// Corps de `POST /eva/saisie`.
public struct SaisieEvaBody: Codable, Equatable, Sendable {
    public let coursId: String
    public let cellules: [CelluleSaisieEva]
    public init(coursId: String, cellules: [CelluleSaisieEva]) { self.coursId = coursId; self.cellules = cellules }
}

public struct ConflitEva: Codable, Equatable, Sendable {
    public let statut: String
    public let valeurM: Int64?
    public let version: Int
    public let parNom: String?
    public let le: String?
    public init(statut: String, valeurM: Int64?, version: Int, parNom: String? = nil, le: String? = nil) {
        self.statut = statut; self.valeurM = valeurM; self.version = version; self.parNom = parNom; self.le = le
    }
}

/// Issue d'une case : `ecrit`, `inchange`, `vide`, `conflit`, `refuse`.
public struct CelluleRetourEva: Codable, Equatable, Sendable {
    public let evaluationId: String
    public let eleveId: String
    public let issue: String
    public let version: Int?
    public let statut: String?
    public let valeurM: Int64?
    public let message: String?
    public let conflit: ConflitEva?
    public init(evaluationId: String, eleveId: String, issue: String, version: Int? = nil, statut: String? = nil, valeurM: Int64? = nil,
                message: String? = nil, conflit: ConflitEva? = nil) {
        self.evaluationId = evaluationId; self.eleveId = eleveId; self.issue = issue; self.version = version; self.statut = statut
        self.valeurM = valeurM; self.message = message; self.conflit = conflit
    }
}

public struct SaisieEvaRetour: Codable, Equatable, Sendable {
    public let cellules: [CelluleRetourEva]
    public init(cellules: [CelluleRetourEva]) { self.cellules = cellules }
}

/// Corps de `POST /eva/evaluations` : ce que l'appli règle (le reste se règle sur le web).
public struct CreerEvaluationEvaBody: Codable, Equatable, Sendable {
    public let coursId: String
    public let titre: String
    public let dateEval: String
    public let baremeM: Int64
    public let coefM: Int64
    public init(coursId: String, titre: String, dateEval: String, baremeM: Int64, coefM: Int64) {
        self.coursId = coursId; self.titre = titre; self.dateEval = dateEval; self.baremeM = baremeM; self.coefM = coefM
    }
}

/// Corps de `POST /eva/evaluations/{id}/publication` : `maintenant` ou `retirer` (motif écrit).
public struct PublierEvaBody: Codable, Equatable, Sendable {
    public let action: String
    public let motif: String?
}

/// Résultat de l'analyse d'un texte de case (copie de `ent_types::evaluations_v3::Analyse`).
public enum AnalyseCellule: Equatable, Sendable {
    /// Case vide : rien à écrire (un résultat ne se supprime jamais, R-V6).
    case vide
    case statut(String)
    case note(Int64)
    case erreur(String)
}

/// Règles de saisie partagées avec le serveur (`ent-types/src/evaluations_v3.rs`), au caractère près.
public enum SaisieNotes {
    /// Codes rapides au clavier : (code, statut, libellé).
    public static let codesRapides: [(code: String, statut: String, libelle: String)] = [
        ("abs", "absent", "Absent"), ("ar", "a_rattraper", "À rattraper"), ("disp", "dispense", "Dispensé"),
        ("nr", "non_rendu", "Non rendu"), ("nn", "non_note", "Non noté"),
    ]

    public static func libelleStatut(_ s: String) -> String {
        switch s {
        case "note": return "Note"
        case "absent": return "Absent"
        case "a_rattraper": return "À rattraper"
        case "dispense": return "Dispensé"
        case "non_rendu": return "Non rendu"
        case "non_note": return "Non noté"
        case "annule": return "Annulé"
        default: return "Statut inconnu"
        }
    }

    public static func codeStatut(_ s: String) -> String {
        switch s {
        case "absent": return "abs"
        case "a_rattraper": return "ar"
        case "dispense": return "disp"
        case "non_rendu": return "nr"
        case "non_note": return "nn"
        case "annule": return "ann"
        default: return ""
        }
    }

    static func nettoye(_ t: String) -> String { t.trimmingCharacters(in: .whitespacesAndNewlines) }

    private static func sansAccentsMinuscules(_ t: String) -> String {
        String(nettoye(t).lowercased().map { c -> Character in
            switch c {
            case "à", "â", "ä": return "a"
            case "é", "è", "ê", "ë": return "e"
            case "î", "ï": return "i"
            case "ô", "ö": return "o"
            case "ù", "û", "ü": return "u"
            case "ç": return "c"
            default: return c
            }
        })
    }

    public static func statutDuCode(_ texte: String) -> String? {
        var t = sansAccentsMinuscules(texte)
        while t.hasSuffix(".") { t.removeLast() }
        switch t {
        case "abs", "absent", "absente": return "absent"
        case "ar", "a rattraper", "arattraper", "rattrapage": return "a_rattraper"
        case "disp", "dispense", "dispensee": return "dispense"
        case "nr", "non rendu", "nonrendu", "n.r": return "non_rendu"
        case "nn", "non note", "nonnote", "n.n": return "non_note"
        default: return nil
        }
    }

    private static func chiffres(_ s: Substring) -> Bool { s.unicodeScalars.allSatisfy { $0.value >= 48 && $0.value <= 57 } }

    /// Nombre décimal en millièmes (« 12 », « 12,5 », « 12.25 »), au plus trois décimales.
    public static func milliemesDepuisTexte(_ texte: String) -> Int64? {
        let t = nettoye(texte).replacingOccurrences(of: ",", with: ".")
        if t.isEmpty || t.hasPrefix("+") { return nil }
        let entier: Substring
        let frac: Substring
        if let i = t.firstIndex(of: ".") {
            entier = t[t.startIndex..<i]
            frac = t[t.index(after: i)...]
        } else {
            entier = Substring(t)
            frac = ""
        }
        if entier.isEmpty && frac.isEmpty { return nil }
        if !chiffres(entier) || !chiffres(frac) { return nil }
        if entier.utf8.count > 6 || frac.utf8.count > 3 { return nil }
        let e = entier.isEmpty ? 0 : Int64(entier) ?? 0
        let f = Int64(frac.padding(toLength: 3, withPad: "0", startingAt: 0)) ?? 0
        return e * 1000 + f
    }

    /// Millièmes à la française (« 12,5 », « 8 », « 13,125 »).
    public static func formater(_ m: Int64) -> String {
        let a = abs(m)
        var s = m < 0 ? "-" : ""
        s += String(a / 1000)
        let f = a % 1000
        if f != 0 {
            var d = String(format: "%03lld", f)
            while d.hasSuffix("0") { d.removeLast() }
            s += "," + d
        }
        return s
    }

    /// Analyse le texte d'une case. **Le serveur reste juge** : il rejoue cette même règle puis contrôle le reste.
    public static func analyser(_ texte: String, baremeM: Int64, horsBaremeAutorise: Bool, pasM: Int64) -> AnalyseCellule {
        let t = nettoye(texte)
        if t.isEmpty { return .vide }
        if let s = statutDuCode(t) { return .statut(s) }
        if t.hasPrefix("-") { return .erreur("une note ne peut pas être négative") }
        guard let m = milliemesDepuisTexte(t) else {
            let debut = String(String.UnicodeScalarView(t.unicodeScalars.prefix(20)))
            return .erreur("« \(debut) » n'est ni une note ni un code (abs, ar, disp, nr, nn)")
        }
        if m > baremeM && !horsBaremeAutorise {
            return .erreur("\(formater(m)) dépasse le barème (\(formater(baremeM))) : vérifiez le barème de l'évaluation")
        }
        if pasM > 1 && m % pasM != 0 { return .erreur("la note doit être un multiple de \(formater(pasM))") }
        return .note(m)
    }

    /// Ce que la case affiche avant toute frappe : la note, le code du statut, ou rien.
    public static func texteDe(_ c: CelluleEva) -> String {
        guard let s = c.statut else { return "" }
        if s == "note", let v = c.valeurM { return formater(v) }
        return codeStatut(s)
    }

    /// Phrase courte d'une case lue (liste des élèves, conflit).
    public static func resume(statut: String?, valeurM: Int64?, baremeM: Int64) -> String {
        guard let s = statut else { return "vide" }
        if s == "note", let v = valeurM { return "\(formater(v)) / \(formater(baremeM))" }
        return libelleStatut(s)
    }
}

/// Brouillon de saisie d'**une** évaluation (même règles que `BrouillonSaisie` côté Kotlin) : seules les cases
/// modifiées, comprises et ouvertes partent, avec la version lue ; un conflit attend un choix explicite.
public struct BrouillonSaisie: Sendable {
    public let grille: GrilleEva
    public let evaluationId: String
    public let evaluation: EvaluationEva
    private let rang: Int
    private var cases: [String: CelluleEva]
    private var textes: [String: String] = [:]
    private var conflits: [String: ConflitEva] = [:]
    private var refus: [String: String] = [:]
    /// Élèves concernés, dans l'ordre de la grille (les non concernés ne sont jamais saisis).
    public let eleves: [LigneEleveEva]

    public init?(grille: GrilleEva, evaluationId: String) {
        guard let r = grille.evaluations.firstIndex(where: { $0.id == evaluationId }) else { return nil }
        self.grille = grille
        self.evaluationId = evaluationId
        evaluation = grille.evaluations[r]
        rang = r
        cases = Dictionary(uniqueKeysWithValues: grille.eleves.map { ($0.eleveId, $0.cellules[r]) })
        eleves = grille.eleves.filter { $0.cellules[r].concerne }
    }

    public func caseDe(_ id: String) -> CelluleEva? { cases[id] }
    public func texte(_ id: String) -> String { textes[id] ?? cases[id].map(SaisieNotes.texteDe) ?? "" }
    public func conflit(_ id: String) -> ConflitEva? { conflits[id] }
    public func refus(_ id: String) -> String? { refus[id] }
    public var nbConflits: Int { conflits.count }

    /// La case peut-elle être écrite d'ici ? (Une correction motivée après l'arrêt reste sur le web.)
    public func modifiable(_ id: String) -> Bool {
        guard grille.peutSaisir, evaluation.ouverte, let c = cases[id] else { return false }
        return c.concerne && c.saisieOuverte
    }

    public mutating func taper(_ id: String, _ texte: String) {
        guard modifiable(id) else { return }
        textes[id] = texte
        refus[id] = nil
    }

    public func analyse(_ id: String) -> AnalyseCellule {
        SaisieNotes.analyser(texte(id), baremeM: evaluation.baremeM, horsBaremeAutorise: evaluation.horsBaremeAutorise, pasM: grille.pasM)
    }

    public func modifiee(_ id: String) -> Bool {
        guard let t = textes[id] else { return false }
        return SaisieNotes.nettoye(t) != (cases[id].map(SaisieNotes.texteDe) ?? "")
    }

    /// Une case modifiée qu'on vide : rien ne s'efface (R-V6) ; l'écran le signale.
    public func videeSansEffet(_ id: String) -> Bool {
        modifiee(id) && analyse(id) == .vide && cases[id]?.statut != nil
    }

    public func aEnvoyer() -> [CelluleSaisieEva] {
        eleves.compactMap { l in
            let id = l.eleveId
            guard modifiable(id), modifiee(id), conflits[id] == nil else { return nil }
            switch analyse(id) {
            case .note, .statut:
                return CelluleSaisieEva(evaluationId: evaluationId, eleveId: id, texte: SaisieNotes.nettoye(texte(id)), versionAttendue: cases[id]?.version)
            default: return nil
            }
        }
    }

    public func enErreur() -> [String] {
        eleves.map(\.eleveId).filter { id in
            guard modifiee(id), case .erreur = analyse(id) else { return false }
            return true
        }
    }

    public func lot() -> SaisieEvaBody { SaisieEvaBody(coursId: grille.enseignement.coursId, cellules: aEnvoyer()) }

    public var famille: String { "eva-saisie:\(evaluationId):" }

    /// Intention d'idempotence du lot : le contenu exact (élève, texte, version), pas un hachage (D-069).
    public func intention(_ l: SaisieEvaBody? = nil) -> String {
        let lot = l ?? self.lot()
        return famille + lot.cellules.sorted { $0.eleveId < $1.eleveId }
            .map { "\($0.eleveId)=\($0.texte)@\($0.versionAttendue.map(String.init) ?? "-")" }.joined(separator: ";")
    }

    /// Applique les issues du serveur, case par case. Rend le nombre de cases écrites.
    @discardableResult
    public mutating func appliquer(_ retour: SaisieEvaRetour) -> Int {
        var ecrites = 0
        for r in retour.cellules where r.evaluationId == evaluationId {
            let id = r.eleveId
            guard var c = cases[id] else { continue }
            switch r.issue {
            case "ecrit", "inchange":
                c.statut = r.statut ?? c.statut
                c.valeurM = r.valeurM
                c.version = r.version ?? c.version
                cases[id] = c
                textes[id] = nil; conflits[id] = nil; refus[id] = nil
                if r.issue == "ecrit" { ecrites += 1 }
            case "vide":
                textes[id] = nil; conflits[id] = nil
            case "conflit":
                if let k = r.conflit { conflits[id] = k }
                refus[id] = nil
            default:
                refus[id] = r.message ?? "Case refusée par le serveur."
            }
        }
        return ecrites
    }

    private mutating func prendre(_ k: ConflitEva, _ id: String) {
        guard var c = cases[id] else { return }
        c.statut = k.statut; c.valeurM = k.valeurM; c.version = k.version
        cases[id] = c
    }

    /// « Reprendre celle du serveur » : la case prend la valeur du conflit, la frappe est abandonnée.
    public mutating func reprendreServeur(_ id: String) {
        guard let k = conflits.removeValue(forKey: id) else { return }
        prendre(k, id)
        textes[id] = nil
    }

    /// « Garder la mienne » : la frappe repart au prochain envoi, sur la version que le serveur a montrée.
    public mutating func garderLaMienne(_ id: String) {
        guard let k = conflits.removeValue(forKey: id) else { return }
        prendre(k, id)
    }
}
