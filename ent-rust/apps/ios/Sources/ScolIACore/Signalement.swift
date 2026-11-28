import Foundation

/// Signaler un incident (registre des violations de données, socle 27, complément G5) : logique d'écran pure,
/// miroir de `Signalement.kt`. Le serveur valide, numérote, alerte la direction et ne rend au déclarant que la
/// référence et l'avancement (R-RG32).
public enum Signalement {
    public static let intituleMax = 200
    public static let descriptionMax = 4000

    /// Texte tel que le serveur le recevra : fins de ligne unifiées, autres caractères de contrôle (tabulation…)
    /// changés en espace, car le serveur les refuse.
    public static func net(_ s: String) -> String {
        var sortie = String.UnicodeScalarView()
        for u in s.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n").unicodeScalars {
            sortie.append(u != "\n" && u.properties.generalCategory == .control ? " " : u)
        }
        return String(sortie).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// Longueur comptée comme le serveur (scalaires Unicode, pas graphèmes : un emoji composé compte pour plusieurs).
    private static func longueur(_ s: String) -> Int { net(s).unicodeScalars.count }

    /// Même règle que le serveur : une phrase de 3 à 200 caractères. `nil` : recevable.
    public static func erreurIntitule(_ intitule: String) -> String? {
        let n = longueur(intitule)
        if n == 0 { return "Dites en une phrase ce qui s'est passé." }
        if n < 3 { return "Une phrase un peu plus longue, s'il vous plaît (3 caractères au moins)." }
        if n > intituleMax { return "Une phrase de \(intituleMax) caractères au plus ; le reste va dans les précisions." }
        return nil
    }

    /// Précisions facultatives : vides, ou de 3 à 4 000 caractères.
    public static func erreurDescription(_ description: String) -> String? {
        let n = longueur(description)
        if (1...2).contains(n) { return "Précisions : 3 caractères au moins, ou laissez vide." }
        if n > descriptionMax { return "Précisions : \(descriptionMax) caractères au plus." }
        return nil
    }

    public static func corps(intitule: String, description: String, sansDonneePersonnelle: Bool) -> DeclarationIncidentBody {
        let d = net(description)
        return DeclarationIncidentBody(intitule: net(intitule), description: d.isEmpty ? nil : d, incidentSecurite: sansDonneePersonnelle)
    }

    /// Intention d'écriture (clé d'idempotence gardée après un échec incertain) : le même contenu, la même intention.
    public static func intention(_ c: DeclarationIncidentBody) -> String {
        "signaler:\(c.intitule)\u{1F}\(c.description ?? "")\u{1F}\(c.incidentSecurite)"
    }

    public static func libelleAvancement(_ a: String) -> String {
        switch a {
        case "recue": return "Reçue, en attente de traitement"
        case "en_cours": return "En cours de traitement"
        case "close": return "Traitée"
        default: return "Avancement inconnu"
        }
    }
}
