import XCTest
@testable import ScolIACore

/// Signaler un incident (registre des violations, socle 27, complément G5) : miroir de `SignalementTest.kt`.
final class SignalementTests: XCTestCase {
    func me(_ role: String) -> Me {
        Me(userId: "u", email: "x", etablissementId: "e", role: role, eleveId: nil, passkeyCount: 1, portee: "complete")
    }

    func testModulePourLePersonnelQuiADroitDeDeclarer() {
        let prof = Capacites.espace(me: me("prof"), droits: [
            Droit(action: "absences.lire", portee: "mes_classes"), Droit(action: "violations.declarer", portee: "etablissement"),
        ])
        XCTAssertTrue(prof.contient(.signaler))
        XCTAssertEqual(prof.modules.last, .profil)
        XCTAssertFalse(prof.onglets.contains(.signaler))
        XCTAssertFalse(Capacites.espace(me: me("prof"), droits: [Droit(action: "absences.lire", portee: "mes_classes")]).contient(.signaler))
        XCTAssertFalse(Capacites.espace(me: me("parent"), droits: []).contient(.signaler))
        XCTAssertFalse(Capacites.espace(me: me("eleve"), droits: []).contient(.signaler))
    }

    func testMemesBornesQueLeServeur() {
        XCTAssertNotNil(Signalement.erreurIntitule("  "))
        XCTAssertNotNil(Signalement.erreurIntitule("ab"))
        XCTAssertNil(Signalement.erreurIntitule("Clé USB perdue"))
        XCTAssertNotNil(Signalement.erreurIntitule(String(repeating: "x", count: 201)))
        XCTAssertNil(Signalement.erreurDescription(""))
        XCTAssertNotNil(Signalement.erreurDescription("ab"))
        XCTAssertNotNil(Signalement.erreurDescription(String(repeating: "x", count: 4001)))
        // Le serveur compte des scalaires Unicode : une famille (5 scalaires, 1 graphème) en compte 5.
        XCTAssertNil(Signalement.erreurIntitule(String(repeating: "👨‍👩‍👧", count: 40)))
        XCTAssertNotNil(Signalement.erreurIntitule(String(repeating: "👨‍👩‍👧", count: 41)))
    }

    func testCaracteresDeControleChangesCommeLeServeurLesAttend() {
        XCTAssertEqual(Signalement.net(" a\tb\r\nc\rd "), "a b\nc\nd")
        XCTAssertEqual(Signalement.corps(intitule: "Clé\u{07}b", description: "", sansDonneePersonnelle: false).intitule, "Clé b")
    }

    func testCorpsSansPrecisionVideEtIntentionStable() throws {
        let c = Signalement.corps(intitule: "  Clé USB perdue ", description: "  ", sansDonneePersonnelle: true)
        XCTAssertEqual(c, DeclarationIncidentBody(intitule: "Clé USB perdue", description: nil, incidentSecurite: true))
        let o = try JSONSerialization.jsonObject(with: ApiClient.encodeur.encode(c)) as? [String: Any]
        XCTAssertEqual(Set(o?.keys.map { $0 } ?? []), ["intitule", "incident_securite"], "précisions vides : non envoyées")
        XCTAssertEqual(Signalement.intention(c), Signalement.intention(Signalement.corps(intitule: "Clé USB perdue", description: "", sansDonneePersonnelle: true)))
        XCTAssertNotEqual(Signalement.intention(c), Signalement.intention(Signalement.corps(intitule: "Clé USB perdue", description: "", sansDonneePersonnelle: false)))
        XCTAssertEqual(Signalement.libelleAvancement("close"), "Traitée")
    }

    func testDeclarerPorteUneCleEtLitLaReference() async throws {
        let s = FauxServeur(["POST /rgpd/violations/declarations": (201, #"{"id":"v1","reference":"V-2027-003","intitule":"Clé USB perdue","connue_le":"2027-03-01T08:00:00Z","avancement":"recue","message":"Merci"}"#)])
        let c = ApiClient(baseURL: FauxServeur.base, transport: s, jeton: { "jeton" }, appli: "ios/1.0.0 (build 1)", nouvelleCle: { "cle-1" })
        let d = try await c.declarerIncident(Signalement.corps(intitule: "Clé USB perdue", description: "", sansDonneePersonnelle: false))
        XCTAssertEqual(d.reference, "V-2027-003")
        XCTAssertEqual(s.recues.first?.entetes["Idempotency-Key"], "cle-1")
    }
}
