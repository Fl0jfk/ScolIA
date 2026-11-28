import XCTest
@testable import ScolIACore

/// « Mes autres comptes » (transverse individu), miroir d'`IndividuTest.kt` : réponses **réelles** du serveur
/// (`contrat/mobile/individu/`, produites par `ent-web-server/tests/individu_contrat_mobile.rs`), requêtes émises,
/// liste blanche, textes et contrôles locaux identiques au web.
final class IndividuTests: XCTestCase {
    static let dossier: URL = {
        if let d = ProcessInfo.processInfo.environment["SCOLIA_CONTRAT"], !d.isEmpty {
            return URL(fileURLWithPath: d).appendingPathComponent("individu")
        }
        var u = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { u.deleteLastPathComponent() }
        return u.appendingPathComponent("contrat/mobile/individu")
    }()

    func fixture(_ nom: String) throws -> String {
        try String(contentsOf: Self.dossier.appendingPathComponent("\(nom).json"), encoding: .utf8)
    }

    func client(_ s: FauxServeur) -> ApiClient {
        ApiClient(baseURL: FauxServeur.base, transport: s, jeton: { "jeton" }, nouvelleCle: { "cle-neuve" })
    }

    func corps(_ r: RequeteHttp) throws -> [String: Any] {
        try JSONSerialization.jsonObject(with: r.corps ?? Data()) as? [String: Any] ?? [:]
    }

    func testMoiDecodeLaReponseReelle() async throws {
        let s = FauxServeur(["GET /dossier/individu/moi": (200, try fixture("moi"))])
        let m = try await client(s).individuMoi()
        XCTAssertGreaterThanOrEqual(m.autresComptes, 1)
        XCTAssertFalse(m.liens.isEmpty)
        let d = try XCTUnwrap(m.declarationsRecues.first)
        XCTAssertFalse(d.declarant.isEmpty)
        XCTAssertTrue(Individu.dateHeure(d.expireLe).contains(" à "), d.expireLe)
        // Jamais l'autre fiche : trois champs par lien.
        let brut = try JSONSerialization.jsonObject(with: Data(try fixture("moi").utf8)) as! [String: Any]
        let lien = (brut["liens"] as! [[String: Any]])[0]
        XCTAssertEqual(Set(lien.keys), ["origine", "motif_code", "pose_le"])
    }

    func testMoiSansLiens() async throws {
        let s = FauxServeur(["GET /dossier/individu/moi": (200, #"{"autres_comptes":0,"fiches_reliees":1,"declarations_recues":[]}"#)])
        let m = try await client(s).individuMoi()
        XCTAssertEqual(m.liens, [])
        XCTAssertEqual(Individu.resumeMoi(m), "Aucun autre compte n'est reconnu comme le vôtre dans cet établissement.")
        XCTAssertNil(Individu.resumeFiches(m))
    }

    func testDeclarerAccepteLe202Vide() async throws {
        let s = FauxServeur(["POST /dossier/individu/declarations": (202, "")])
        try await client(s).declarerAutreCompte(email: "autre@ent.test", cle: "k1")
        let r = try XCTUnwrap(s.recues.first)
        XCTAssertEqual(r.entetes["Idempotency-Key"], "k1")
        XCTAssertEqual(try corps(r)["email"] as? String, "autre@ent.test")
    }

    func testRepondreRendLeLienOuRien() async throws {
        let s = FauxServeur([
            "POST /dossier/individu/declarations/d%201/reponse": (200, try fixture("reponse_confirmer")),
            "POST /dossier/individu/declarations/d2/reponse": (200, try fixture("reponse_refuser")),
        ])
        let c = client(s)
        let lien = try await c.repondreDeclaration("d 1", confirmer: true)
        XCTAssertNotNil(lien)
        let rien = try await c.repondreDeclaration("d2", confirmer: false)
        XCTAssertNil(rien)
        XCTAssertEqual(try corps(s.recues[0])["confirmer"] as? Bool, true)
        XCTAssertEqual(try corps(s.recues[1])["confirmer"] as? Bool, false)
        XCTAssertEqual(s.recues[1].entetes["Idempotency-Key"], "cle-neuve")
    }

    func testChaqueRequeteEstDansLaListeBlanche() async throws {
        let lignes = try String(contentsOf: Self.dossier.appendingPathComponent("routes.txt"), encoding: .utf8)
            .split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty && !$0.hasPrefix("#") }
        func correspond(_ ligne: String, _ gabarit: String) -> Bool {
            let (a, b) = (ligne.split(separator: "/", omittingEmptySubsequences: false), gabarit.split(separator: "/", omittingEmptySubsequences: false))
            guard a.count == b.count else { return false }
            return zip(a, b).allSatisfy { x, g in (g.hasPrefix("{") && g.hasSuffix("}") && !x.isEmpty) || x == g }
        }
        let s = FauxServeur([:])
        let c = client(s)
        _ = try? await c.individuMoi()
        _ = try? await c.declarerAutreCompte(email: "a@b.fr")
        _ = try? await c.repondreDeclaration("abc", confirmer: true)
        XCTAssertEqual(s.recues.count, 3)
        for r in s.recues {
            let chemin = String(r.url.absoluteString.dropFirst(FauxServeur.base.absoluteString.count)).split(separator: "?")[0]
            let ligne = "\(r.methode) \(chemin)"
            XCTAssertTrue(lignes.contains { correspond(ligne, $0) }, "\(ligne) hors de contrat/mobile/individu/routes.txt")
        }
    }

    func testAdresseControleeCommeSurLeWeb() {
        XCTAssertEqual(try Individu.adresseAEnvoyer("  A@B.fr ").get(), "a@b.fr")
        XCTAssertEqual(Individu.adresseAEnvoyer(" "), .failure(.vide))
        for faux in ["ab.fr", "@b.fr", "a@", "a b@c.fr", String(repeating: "x", count: 250) + "@b.fr"] {
            XCTAssertEqual(Individu.adresseAEnvoyer(faux), .failure(.invalide), faux)
        }
        XCTAssertEqual(Individu.AdresseInvalide.invalide.localizedDescription, "Cette adresse e-mail n'est pas valide.")
    }

    func testLibellesDeChaqueMotifEtOrigine() {
        for m in ["deux_comptes", "fiche_sans_compte", "doublon_agent", "autre"] {
            XCTAssertNotEqual(Individu.libelleMotifLien(m), "Motif non listé", m)
        }
        for o in ["declaration_direction", "declaration_confirmee", "proposition_acceptee"] {
            XCTAssertNotEqual(Individu.libelleOrigine(o), "Lien enregistré", o)
        }
    }

    func testResumesTrisEtDates() {
        let demandes = [
            DeclarationRecue(id: "b", declarant: "Paul", declareLe: "2026-11-27T08:00:00Z", expireLe: "2026-11-28T08:00:00Z"),
            DeclarationRecue(id: "a", declarant: "Anne", declareLe: "2026-11-27T09:00:00Z", expireLe: "2026-11-28T07:00:00Z"),
        ]
        let liens = [
            MonLienIndividu(origine: "declaration_direction", motifCode: "deux_comptes", poseLe: "2026-10-01T10:00:00Z"),
            MonLienIndividu(origine: "declaration_confirmee", motifCode: "deux_comptes", poseLe: "2026-11-01T10:00:00Z"),
        ]
        let m = MonIndividu(autresComptes: 2, fichesReliees: 3, declarationsRecues: demandes, liens: liens)
        XCTAssertTrue(Individu.resumeMoi(m).hasPrefix("2 autres comptes"))
        XCTAssertTrue(Individu.resumeMoi(MonIndividu(autresComptes: 1, fichesReliees: 2)).hasPrefix("1 autre compte "))
        XCTAssertTrue(Individu.resumeFiches(m)!.hasPrefix("3 fiches"))
        XCTAssertEqual(Individu.trierLiens(liens).first?.origine, "declaration_confirmee")
        XCTAssertEqual(Individu.trierDemandes(demandes).first?.id, "a")
        let paris = TimeZone(identifier: "Europe/Paris")!
        XCTAssertEqual(Individu.dateHeure("2026-11-28T07:00:00Z", fuseau: paris), "28/11/2026 à 08h00")
        XCTAssertEqual(Individu.dateHeure("2026-11-28T07:00:00.123456Z", fuseau: paris), "28/11/2026 à 08h00")
        XCTAssertEqual(Individu.dateHeure("2026-11-28T07:00:00.123+01:00", fuseau: paris), "28/11/2026 à 07h00")
        XCTAssertEqual(Individu.dateHeure("pas une date"), "pas une date")
        XCTAssertEqual(Individu.detailDemande(demandes[1], fuseau: paris),
                       "Demandé le 27/11/2026 à 10h00 · à confirmer avant le 28/11/2026 à 08h00")
    }

    func testTextesIdentiquesAuCoeurKotlin() {
        // Les deux cœurs et le web portent les mêmes phrases : un écart se voit ici (Kotlin : `IndividuTest`).
        XCTAssertTrue(Individu.messageDeclaration.hasPrefix("Demande enregistrée. Si cette adresse"))
        XCTAssertTrue(Individu.effetLien.hasSuffix("jamais la personne concernée."))
        XCTAssertNotEqual(Individu.intentionRepondre("d1", confirmer: true), Individu.intentionRepondre("d1", confirmer: false))
    }
}
