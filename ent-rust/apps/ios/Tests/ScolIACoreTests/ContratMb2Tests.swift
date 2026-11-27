import XCTest
@testable import ScolIACore

/// Socle 11, palier MB2 : le cœur décode **toutes** les fixtures dorées produites par le serveur
/// (`ent-rust/contrat/mobile/*.json`, banc `ent-api/src/tests_contrat_mobile.rs`) avec ses vraies méthodes, et
/// n'appelle aucune route hors de la liste blanche `routes.txt`. Miroir de `ContratMb2Test.kt`.
private final class ServeurFixture: Transport, @unchecked Sendable {
    let statut: Int
    let corps: Data
    var recues: [RequeteHttp] = []
    init(statut: Int, corps: Data) { self.statut = statut; self.corps = corps }
    func executer(_ r: RequeteHttp) async throws -> ReponseHttp {
        recues.append(r)
        return ReponseHttp(statut: statut, corps: corps)
    }
}

final class ContratMb2Tests: XCTestCase {
    /// `SCOLIA_CONTRAT` sinon `ent-rust/contrat/mobile` à côté de ce fichier (Tests/ScolIACoreTests → ent-rust).
    static let dossier: URL = {
        if let d = ProcessInfo.processInfo.environment["SCOLIA_CONTRAT"], !d.isEmpty { return URL(fileURLWithPath: d) }
        var u = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { u.deleteLastPathComponent() }
        return u.appendingPathComponent("contrat/mobile")
    }()

    func client(_ t: Transport) -> ApiClient {
        ApiClient(baseURL: FauxServeur.base, transport: t, jeton: { "jeton" }, appli: "ios/1.4.2 (build 142)")
    }

    /// Un cas par fixture : l'appel réel du client qui consomme cette réponse.
    let cas: [String: @Sendable (ApiClient) async throws -> Void] = [
        "auth_etablissement": { _ = try await $0.etablissementDeLAdresse() },
        "auth_login": { _ = try await $0.login(email: "persona@ent.test", motDePasse: "x") },
        "auth_passkey_login_start": { _ = try await $0.passkeyLoginStart(email: "persona@ent.test") },
        "auth_passkey_register_start": { _ = try await $0.passkeyRegisterStart() },
        "auth_me": { _ = try await $0.me() },
        "roles_mes_droits": { _ = try await $0.mesDroits() },
        "auth_sessions": { _ = try await $0.sessions() },
        "auth_sessions_revoquer": { try await $0.revoquerSession("s1") },
        "auth_logout": { try await $0.logout() },
        "devices_push": { _ = try await $0.enregistrerPush(plateforme: "ios", jeton: "apns", appId: "fr.scolia.mobile") },
        "devices_push_supprimer": { try await $0.desinscrirePush("p1") },
        "edt_mes_creneaux": { _ = try await $0.mesCreneaux(jour: "2027-03-08") },
        "eleves_edt": { _ = try await $0.edtEleve("e1", jour: "2027-03-08") },
        "appels_ouvrir": { _ = try await $0.ouvrirAppel(creneauId: "c1", jour: "2027-03-08") },
        "appel": { _ = try await $0.appel("a1") },
        "appels_lignes": { try await $0.marquer(appelId: "a1", eleveId: "e1", etat: .absent) },
        "appels_clore": { _ = try await $0.clore(appelId: "a1") },
        "famille_enfants": { _ = try await $0.mesEnfants() },
        "famille_resume": { _ = try await $0.resumeEnfant("e1") },
        "absences_justifier": { _ = try await $0.justifierAbsence("ab1", motif: "motif") },
        "vs_absences": { _ = try await $0.absencesATraiter(jour: "2027-03-08") },
        "note_periodes": { _ = try await $0.periodesNotes() },
        "eleves_notes": { _ = try await $0.notes("e1") },
        "eleves_moyennes": { _ = try await $0.moyennes("e1", periodeId: "p1") },
        "devoirs": { _ = try await $0.devoirsClasse(periodeId: "p1", classe: "4B") },
        "eleves_devoirs": { _ = try await $0.devoirsEleve("e1", periodeId: "p1") },
        "classes_effectif": { _ = try await $0.effectif(classe: "4B", jour: "2027-03-08") },
        "cahier_texte": { _ = try await $0.cahierTexte(classe: "4B") },
        "cahier_texte_eleve": { _ = try await $0.cahierTexte(eleveId: "e1") },
        "messages_fils": { _ = try await $0.fils() },
        "messages_fil": { _ = try await $0.fil("f1") },
        "messages_repondre": { _ = try await $0.repondre(filId: "f1", corps: "Merci") },
        "annonces": { _ = try await $0.annonces() },
        "eleves_annonces": { _ = try await $0.annoncesEleve("e1") },
        "notifications": { _ = try await $0.notifications() },
    ]

    static func gabaritVersRegex(_ route: String) -> NSRegularExpression {
        let parties = route.split(separator: " ", maxSplits: 1).map(String.init)
        let chemin = parties[1].split(separator: "/", omittingEmptySubsequences: false).map { seg -> String in
            seg.hasPrefix("{") && seg.hasSuffix("}") ? "[^/?]+" : NSRegularExpression.escapedPattern(for: String(seg))
        }.joined(separator: "/")
        return try! NSRegularExpression(pattern: "^\(parties[0]) \(chemin)(\\?.*)?$")
    }

    static func routes() throws -> [NSRegularExpression] {
        let texte = try String(contentsOf: dossier.appendingPathComponent("routes.txt"), encoding: .utf8)
        return texte.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty && !$0.hasPrefix("#") }.map(gabaritVersRegex)
    }

    static func ligne(_ r: RequeteHttp) -> String {
        "\(r.methode) \(String(r.url.absoluteString.dropFirst(FauxServeur.base.absoluteString.count)))"
    }

    static func correspond(_ re: NSRegularExpression, _ s: String) -> Bool {
        re.firstMatch(in: s, range: NSRange(s.startIndex..., in: s)) != nil
    }

    func testChaqueFixtureEstDecodeeParLeVraiClientSurLaBonneRoute() async throws {
        let liste = try Self.routes()
        let fichiers = try FileManager.default.contentsOfDirectory(at: Self.dossier, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "json" }.sorted { $0.lastPathComponent < $1.lastPathComponent }
        var echecs: [String] = []
        var vus = Set<String>()
        for f in fichiers {
            let nom = f.deletingPathExtension().lastPathComponent
            vus.insert(nom)
            guard let appel = cas[nom] else {
                echecs.append("\(nom) : fixture sans cas dans ContratMb2Tests (nouvelle route servie aux applis ?)")
                continue
            }
            let objet = try XCTUnwrap(try JSONSerialization.jsonObject(with: Data(contentsOf: f)) as? [String: Any])
            let statut = try XCTUnwrap(objet["statut"] as? Int)
            let route = try XCTUnwrap(objet["route"] as? String)
            let corps: Data
            if let c = objet["corps"], !(c is NSNull) {
                corps = try JSONSerialization.data(withJSONObject: c, options: [.fragmentsAllowed])
            } else {
                corps = Data()
            }
            let serveur = ServeurFixture(statut: statut, corps: corps)
            do {
                try await appel(client(serveur))
            } catch {
                echecs.append("\(nom) : réponse non décodée (\(error))")
                continue
            }
            guard serveur.recues.count == 1, let r = serveur.recues.first else {
                echecs.append("\(nom) : \(serveur.recues.count) requêtes au lieu d'une")
                continue
            }
            let l = Self.ligne(r)
            if !Self.correspond(Self.gabaritVersRegex(route), l) {
                echecs.append("\(nom) : le client appelle \(l) alors que la fixture vient de \(route)")
            }
            if !liste.contains(where: { Self.correspond($0, l) }) { echecs.append("\(nom) : \(l) hors de routes.txt") }
        }
        for nom in Set(cas.keys).subtracting(vus).sorted() { echecs.append("\(nom) : cas sans fixture") }
        XCTAssertGreaterThanOrEqual(fichiers.count, cas.count)
        XCTAssertTrue(echecs.isEmpty, "contrat mobile rompu côté Swift :\n  " + echecs.joined(separator: "\n  "))
    }

    func testLesCheminementsHorsFixtureRestentDansLaListeBlanche() async throws {
        let liste = try Self.routes()
        let s = ServeurFixture(statut: 200, corps: Data("{}".utf8))
        let c = client(s)
        _ = try? await c.passkeyLoginStart(email: "persona@ent.test")
        _ = try? await c.passkeyLoginFinish(token: "t", credential: [:])
        _ = try? await c.passkeyRegisterStart()
        try? await c.passkeyRegisterFinish(token: "t", credential: [:], nom: "iPhone", motDePasse: "x")
        for chemin in [Chemins.mesCreneaux("2027-03-08"), Chemins.edtEleve("e1", "2027-03-08"), Chemins.cahierClasse("4B"),
                       Chemins.cahier("e1"), Chemins.notes("e1"), Chemins.fils, Chemins.enfants] {
            _ = try? await c.getBrut(chemin)
        }
        XCTAssertEqual(s.recues.count, 11)
        let hors = s.recues.map(Self.ligne).filter { l in !liste.contains(where: { Self.correspond($0, l) }) }
        XCTAssertTrue(hors.isEmpty, "requêtes hors de routes.txt : \(hors)")
    }

    func testBandeauxSessionCourteEtAppareilSansVerrou() {
        func me(_ assurance: String, portee: String = "complete") -> Me {
            Me(userId: "u1", email: "p@ent.test", etablissementId: "e1", role: "prof", portee: portee, assurance: assurance)
        }
        XCTAssertEqual(Avertissement.pour(me("passkey"), appareilSansVerrou: false), [])
        XCTAssertEqual(Avertissement.pour(me("mot_de_passe"), appareilSansVerrou: false), [.sessionCourte])
        XCTAssertEqual(Avertissement.pour(me("sso"), appareilSansVerrou: false), [.sessionCourte])
        XCTAssertEqual(Avertissement.pour(me("mot_de_passe", portee: "enrolement"), appareilSansVerrou: false), [])
        XCTAssertEqual(Avertissement.pour(me("mot_de_passe"), appareilSansVerrou: true), [.sessionCourte, .appareilSansVerrou])
    }

    func testCleGardeeApresUnEchecIncertainEtOublieeApresSuccesOuRefus() async throws {
        final class Compteur: @unchecked Sendable { var n = 0 }
        let k = Compteur()
        let cles = ClesIntention { k.n += 1; return "cle-\(k.n)" }
        var vues: [String] = []
        _ = try? await cles.executer("marquer:a1:e1:absent") { c -> Void in vues.append(c); throw ErreurApi.horsLigne }
        _ = try? await cles.executer("marquer:a1:e1:absent") { c -> Void in
            vues.append(c)
            throw ErreurApi.codee(409, ErreurCodee(code: "idempotence.en_cours", message: "en cours", champs: [], reessayerApresS: 1, trace: nil))
        }
        try await cles.executer("marquer:a1:e1:absent") { c in vues.append(c) }
        XCTAssertEqual(vues, ["cle-1", "cle-1", "cle-1"])
        let apres = try await cles.executer("marquer:a1:e1:absent") { $0 }
        XCTAssertEqual(apres, "cle-2")
        _ = try? await cles.executer("clore:a1") { _ -> Void in throw ErreurApi.interdit("non") }
        XCTAssertEqual(cles.cle("clore:a1"), "cle-4")
        XCTAssertNotEqual(cles.cle("marquer:a1:e1:present"), cles.cle("marquer:a1:e1:retard"))
        XCTAssertTrue(ClesIntention.incertaine(.serveur(503)))
        XCTAssertTrue(ClesIntention.incertaine(.debitDepasse("lent", 5)))
        XCTAssertFalse(ClesIntention.incertaine(.refus(422, "invalide")))
        XCTAssertFalse(ClesIntention.incertaine(.appliTropAncienne("vieille")))
        XCTAssertFalse(ClesIntention.incertaine(.codee(409, ErreurCodee(code: "appel.clos", message: "clos", champs: [], reessayerApresS: nil, trace: nil))))
    }

    func testUnSuccesOublieLesIntentionsSoeurs() async throws {
        final class Compteur: @unchecked Sendable { var n = 0 }
        let k = Compteur()
        let cles = ClesIntention { k.n += 1; return "cle-\(k.n)" }
        let f = "marquer:a1:e1:"
        _ = try? await cles.executer("\(f)absent", famille: f) { _ -> Void in throw ErreurApi.horsLigne }
        try await cles.executer("\(f)retard", famille: f) { _ in }
        let neuve = try await cles.executer("\(f)absent", famille: f) { $0 }
        XCTAssertEqual(neuve, "cle-3")
    }
}
