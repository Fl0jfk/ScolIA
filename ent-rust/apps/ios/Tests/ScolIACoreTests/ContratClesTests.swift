import XCTest
@testable import ScolIACore

/// Socle 11, complément du palier MB2 : **clés des fixtures que le modèle Swift ne lit pas**. Miroir de
/// `ContratClesTest.kt`.
///
/// `JSONDecoder` ignore toujours les clés inconnues : un champ que le serveur **renomme** passe inaperçu de
/// `ContratMb2Tests` si le modèle le déclare facultatif ou lui donne une valeur par défaut. Ce test décode chaque fixture
/// par la vraie méthode du client, réencode la valeur obtenue et liste chaque clé non nulle de la fixture absente du
/// réencodage : c'est une clé que le modèle n'a pas gardée. Une clé ignorée volontairement est déclarée dans
/// `contrat/mobile/cles_ignorees.txt` (plateformes `tous` ou `ios`) ; toute autre clé rend le test rouge.
///
/// Limite : un modèle dont l'encodage différerait de son décodage (encodage écrit à la main) fausserait la mesure ;
/// aucun modèle du cœur n'en a aujourd'hui.
private final class ServeurCles: Transport, @unchecked Sendable {
    let statut: Int
    let corps: Data
    init(statut: Int, corps: Data) { self.statut = statut; self.corps = corps }
    func executer(_ r: RequeteHttp) async throws -> ReponseHttp { ReponseHttp(statut: statut, corps: corps) }
}

final class ContratClesTests: XCTestCase {
    typealias Appel = @Sendable (ApiClient) async throws -> any Encodable

    /// L'appel du client qui consomme chaque fixture (mêmes cas que `ContratMb2Tests`), sa valeur de retour.
    let cas: [String: Appel] = [
        "auth_etablissement": { try await $0.etablissementDeLAdresse() },
        "auth_login": { try await $0.login(email: "persona@ent.test", motDePasse: "x") },
        "auth_passkey_login_start": { try await $0.passkeyLoginStart(email: "persona@ent.test") },
        "auth_passkey_register_start": { try await $0.passkeyRegisterStart() },
        "auth_me": { try await $0.me() },
        "roles_mes_droits": { try await $0.mesDroits() },
        "auth_sessions": { try await $0.sessions() },
        "devices_push": { try await $0.enregistrerPush(plateforme: "ios", jeton: "apns", appId: "fr.scolia.mobile") },
        "edt_mes_creneaux": { try await $0.mesCreneaux(jour: "2027-03-08") },
        "eleves_edt": { try await $0.edtEleve("e1", jour: "2027-03-08") },
        "appels_ouvrir": { try await $0.ouvrirAppel(creneauId: "c1", jour: "2027-03-08") },
        "appel": { try await $0.appel("a1") },
        "appels_clore": { try await $0.clore(appelId: "a1") },
        "famille_enfants": { try await $0.mesEnfants() },
        "famille_resume": { try await $0.resumeEnfant("e1") },
        "absences_justifier": { try await $0.justifierAbsence("ab1", motif: "motif") },
        "vs_absences": { try await $0.absencesATraiter(jour: "2027-03-08") },
        "note_periodes": { try await $0.periodesNotes() },
        "eleves_notes": { try await $0.notes("e1") },
        "eleves_moyennes": { try await $0.moyennes("e1", periodeId: "p1") },
        "devoirs": { try await $0.devoirsClasse(periodeId: "p1", classe: "4B") },
        "eleves_devoirs": { try await $0.devoirsEleve("e1", periodeId: "p1") },
        "classes_effectif": { try await $0.effectif(classe: "4B", jour: "2027-03-08") },
        "cahier_texte": { try await $0.cahierTexte(classe: "4B") },
        "cahier_texte_eleve": { try await $0.cahierTexte(eleveId: "e1") },
        "messages_fils": { try await $0.fils() },
        "messages_fil": { try await $0.fil("f1") },
        "messages_repondre": { try await $0.repondre(filId: "f1", corps: "Merci") },
        "annonces": { try await $0.annonces() },
        "eleves_annonces": { try await $0.annoncesEleve("e1") },
        "notifications": { try await $0.notifications() },
        "eva_enseignements": { try await $0.evaEnseignements() },
        "eva_cours_periodes": { try await $0.evaPeriodes(coursId: "c1") },
        "eva_grille": { try await $0.evaGrille(coursId: "c1", periodeId: "p1") },
        "eva_grille_apres_saisie": { try await $0.evaGrille(coursId: "c1", periodeId: "p1") },
        "eva_evaluations": {
            try await $0.creerEvaluation(CreerEvaluationEvaBody(coursId: "c1", titre: "Interro", dateEval: "2027-03-08", baremeM: 20_000, coefM: 1_000))
        },
        "eva_evaluations_publication": { try await $0.publierEvaluation("ev1", action: "maintenant") },
        "eva_saisie": {
            try await $0.saisirNotes(SaisieEvaBody(coursId: "c1", cellules: [CelluleSaisieEva(evaluationId: "ev1", eleveId: "e1", texte: "15,5")]))
        },
        "eva_saisie_conflit": {
            try await $0.saisirNotes(SaisieEvaBody(coursId: "c1", cellules: [CelluleSaisieEva(evaluationId: "ev1", eleveId: "e1", texte: "abs")]))
        },
        "eva_saisie_lot": {
            try await $0.saisirNotes(SaisieEvaBody(coursId: "c1", cellules: [CelluleSaisieEva(evaluationId: "ev1", eleveId: "e1", texte: "15,5")]))
        },
    ]

    /// Réponses sans corps (204) : rien à lire.
    let sansCorps: Set<String> = ["auth_sessions_revoquer", "auth_logout", "devices_push_supprimer", "appels_lignes"]

    static func ignorees() throws -> [String: String] {
        let texte = try String(contentsOf: ContratMb2Tests.dossier.appendingPathComponent("cles_ignorees.txt"), encoding: .utf8)
        var sortie: [String: String] = [:]
        for brut in texte.split(separator: "\n") {
            let ligne = brut.trimmingCharacters(in: .whitespaces)
            if ligne.isEmpty || ligne.hasPrefix("#") { continue }
            let parties = ligne.components(separatedBy: " — ")
            let tete = parties[0].split(separator: " ", maxSplits: 1).map(String.init)
            let raison = parties.count > 1 ? parties[1].trimmingCharacters(in: .whitespaces) : ""
            guard tete.count == 2, ["tous", "android", "ios"].contains(tete[0]), tete[1].contains(":"), !raison.isEmpty else {
                XCTFail("« \(ligne) » : attendu « plateformes fixture:chemin — raison »")
                continue
            }
            let cle = tete[1].trimmingCharacters(in: .whitespaces)
            XCTAssertNil(sortie[cle], "« \(cle) » déclarée deux fois")
            if tete[0] != "android" { sortie[cle] = raison }
        }
        return sortie
    }

    /// Clés non nulles de `fixture` absentes de `lu` (même chemin) ; `[]` pour un élément de liste.
    static func clesNonLues(_ fixture: Any, _ lu: Any?, _ chemin: String, _ sortie: inout [String]) {
        if let o = fixture as? [String: Any] {
            let l = lu as? [String: Any]
            for (k, x) in o where !(x is NSNull) {
                let ici = chemin.isEmpty ? k : "\(chemin).\(k)"
                if let l, let y = l[k] {
                    clesNonLues(x, y, ici, &sortie)
                } else if l != nil {
                    sortie.append(ici)
                }
            }
        } else if let a = fixture as? [Any], let b = lu as? [Any] {
            for (i, x) in a.enumerated() where i < b.count { clesNonLues(x, b[i], "\(chemin)[]", &sortie) }
        }
    }

    /// Réencodage d'une valeur décodée : les deux conventions de clés (modèles à `CodingKeys` explicites et modèles
    /// décodés par `convertFromSnakeCase`), réunies.
    static func reencoder(_ v: any Encodable) throws -> Any {
        func encoder(_ snake: Bool) throws -> Any {
            let e = JSONEncoder()
            if snake { e.keyEncodingStrategy = .convertToSnakeCase }
            return try JSONSerialization.jsonObject(with: e.encode(v), options: [.fragmentsAllowed])
        }
        return reunir(try encoder(false), try encoder(true))
    }

    static func reunir(_ a: Any, _ b: Any) -> Any {
        if let x = a as? [String: Any], let y = b as? [String: Any] {
            return x.merging(y) { p, q in reunir(p, q) }
        }
        if let x = a as? [Any], let y = b as? [Any], x.count == y.count {
            return zip(x, y).map { reunir($0, $1) }
        }
        return a
    }

    func testChaqueCleNonNulleDesFixturesEstLueParLeModeleOuDeclareeIgnoree() async throws {
        let ignorees = try Self.ignorees()
        let fichiers = try FileManager.default.contentsOfDirectory(at: ContratMb2Tests.dossier, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "json" }.sorted { $0.lastPathComponent < $1.lastPathComponent }
        var vues = Set<String>()
        var echecs: [String] = []
        for f in fichiers {
            let nom = f.deletingPathExtension().lastPathComponent
            let objet = try XCTUnwrap(try JSONSerialization.jsonObject(with: Data(contentsOf: f)) as? [String: Any])
            let corps = objet["corps"] ?? NSNull()
            if sansCorps.contains(nom) {
                if !(corps is NSNull) { echecs.append("\(nom) : déclarée sans corps, mais la fixture en a un") }
                continue
            }
            guard let appel = cas[nom] else {
                echecs.append("\(nom) : fixture sans cas dans ContratClesTests")
                continue
            }
            let statut = try XCTUnwrap(objet["statut"] as? Int)
            let octets = try JSONSerialization.data(withJSONObject: corps, options: [.fragmentsAllowed])
            let client = ApiClient(baseURL: FauxServeur.base, transport: ServeurCles(statut: statut, corps: octets),
                                   jeton: { "jeton" }, appli: "ios/1.4.2 (build 142)")
            let lu: Any
            do {
                lu = try Self.reencoder(try await appel(client))
            } catch {
                echecs.append("\(nom) : réponse non décodée ou non réencodée (\(error))")
                continue
            }
            var nonLues: [String] = []
            Self.clesNonLues(corps, lu, "", &nonLues)
            for chemin in Set(nonLues).sorted() {
                let cle = "\(nom):\(chemin)"
                vues.insert(cle)
                if ignorees[cle] == nil { echecs.append("\(cle) : clé servie par le serveur, ignorée par le modèle Swift") }
            }
        }
        for cle in Set(ignorees.keys).subtracting(vues).sorted() {
            echecs.append("\(cle) : déclarée ignorée, mais absente des fixtures ou lue par le modèle (retirer la ligne)")
        }
        XCTAssertTrue(echecs.isEmpty, "clés de contrat non lues côté Swift :\n  " + echecs.joined(separator: "\n  "))
    }

    /// Le parcours lui-même : une clé renommée par le serveur est signalée, même si le modèle la tolère.
    func testUneCleRenommeeParLeServeurEstSignalee() async throws {
        let f = ContratMb2Tests.dossier.appendingPathComponent("auth_me.json")
        let objet = try XCTUnwrap(try JSONSerialization.jsonObject(with: Data(contentsOf: f)) as? [String: Any])
        var me = try XCTUnwrap(objet["corps"] as? [String: Any])
        me["portee_session"] = me.removeValue(forKey: "portee")
        let octets = try JSONSerialization.data(withJSONObject: me)
        let client = ApiClient(baseURL: FauxServeur.base, transport: ServeurCles(statut: 200, corps: octets),
                               jeton: { "jeton" }, appli: "ios/1.4.2 (build 142)")
        // Le décodage tolérant, lui, ne voit rien : c'est la limite que ce test comble.
        let decode = try await client.me()
        var sortie: [String] = []
        Self.clesNonLues(me, try Self.reencoder(decode), "", &sortie)
        XCTAssertTrue(sortie.contains("portee_session") && !sortie.contains("portee"), "\(sortie)")
    }
}
