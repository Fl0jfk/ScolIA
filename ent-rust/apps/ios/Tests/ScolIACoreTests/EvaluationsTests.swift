import XCTest
@testable import ScolIACore

/// Palier « notes eva » : analyse d'une case identique au serveur, brouillon de saisie, envoi par lot.
/// Miroir de `EvaluationsTest.kt`.
private final class Enregistreur: Transport, @unchecked Sendable {
    var recues: [RequeteHttp] = []
    func executer(_ r: RequeteHttp) async throws -> ReponseHttp {
        recues.append(r)
        return ReponseHttp(statut: 200, corps: Data(#"{"cellules":[]}"#.utf8))
    }
}

final class EvaluationsTests: XCTestCase {
    func testAnalyseIdentiqueAuxVecteursDuServeur() throws {
        let url = ContratMb2Tests.dossier.appendingPathComponent("regles/analyse_cellule.json")
        let doc = try XCTUnwrap(try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any])
        let cas = try XCTUnwrap(doc["analyse"] as? [[String: Any]])
        XCTAssertGreaterThan(cas.count, 100)
        var ecarts: [String] = []
        for o in cas {
            let texte = o["texte"] as! String
            let a = SaisieNotes.analyser(texte, baremeM: (o["bareme_m"] as! NSNumber).int64Value,
                                         horsBaremeAutorise: o["hors_bareme_autorise"] as! Bool, pasM: (o["pas_m"] as! NSNumber).int64Value)
            let att = o["attendu"] as! [String: Any]
            let attendu: AnalyseCellule
            switch att["type"] as! String {
            case "vide": attendu = .vide
            case "statut": attendu = .statut(att["statut"] as! String)
            case "note": attendu = .note((att["valeur_m"] as! NSNumber).int64Value)
            default: attendu = .erreur(att["message"] as! String)
            }
            if a != attendu { ecarts.append("« \(texte) » : \(a) au lieu de \(attendu)") }
        }
        for o in try XCTUnwrap(doc["format"] as? [[String: Any]]) {
            let m = (o["valeur_m"] as! NSNumber).int64Value
            if SaisieNotes.formater(m) != o["texte"] as! String { ecarts.append("format \(m) : \(SaisieNotes.formater(m))") }
        }
        XCTAssertTrue(ecarts.isEmpty, "analyse divergente du serveur :\n  " + ecarts.joined(separator: "\n  "))
    }

    private func grille(_ lignes: [(String, CelluleEva)], peutSaisir: Bool = true, etat: String = "ouverte") -> GrilleEva {
        GrilleEva(
            enseignement: EnseignementEva(coursId: "c1", libelle: "Maths 4B", cibleCode: "4B", droit: "saisie"),
            periode: PeriodeEva(id: "p1", code: "T1", libelle: "Trimestre 1", dateDebut: "2027-01-01", dateFin: "2027-03-31"),
            jour: "2027-03-08", peutSaisir: peutSaisir,
            evaluations: [EvaluationEva(id: "ev0", titre: "A", dateEval: "2027-03-01"), EvaluationEva(id: "ev1", titre: "B", dateEval: "2027-03-08", etat: etat)],
            eleves: lignes.map { LigneEleveEva(eleveId: $0.0, nom: "Nom\($0.0)", prenom: "P", cellules: [CelluleEva(), $0.1]) }
        )
    }

    func testSeulesLesCasesModifieesComprisesEtOuvertesPartent() throws {
        var b = try XCTUnwrap(BrouillonSaisie(grille: grille([
            ("e1", CelluleEva()),
            ("e2", CelluleEva(statut: "note", valeurM: 12_000, version: 3)),
            ("e3", CelluleEva(saisieOuverte: false)),
            ("e4", CelluleEva(concerne: false)),
            ("e5", CelluleEva(statut: "absent", version: 2)),
        ]), evaluationId: "ev1"))
        XCTAssertEqual(b.eleves.map(\.eleveId), ["e1", "e2", "e3", "e5"])
        XCTAssertEqual(b.texte("e2"), "12")
        XCTAssertEqual(b.texte("e5"), "abs")
        b.taper("e1", " 15,5 ")
        b.taper("e2", "12")
        b.taper("e3", "10")
        b.taper("e5", "21")
        XCTAssertFalse(b.modifiable("e3"))
        XCTAssertEqual(b.aEnvoyer(), [CelluleSaisieEva(evaluationId: "ev1", eleveId: "e1", texte: "15,5")])
        XCTAssertEqual(b.enErreur(), ["e5"])
        b.taper("e5", "")
        XCTAssertTrue(b.videeSansEffet("e5"))
        b.taper("e2", "nr")
        XCTAssertEqual(b.aEnvoyer().first { $0.eleveId == "e2" }, CelluleSaisieEva(evaluationId: "ev1", eleveId: "e2", texte: "nr", versionAttendue: 3))
    }

    func testEvaluationAnnuleeOuGrilleEnLecture() throws {
        XCTAssertFalse(try XCTUnwrap(BrouillonSaisie(grille: grille([("e1", CelluleEva())], etat: "annulee"), evaluationId: "ev1")).modifiable("e1"))
        var l = try XCTUnwrap(BrouillonSaisie(grille: grille([("e1", CelluleEva())], peutSaisir: false), evaluationId: "ev1"))
        l.taper("e1", "12")
        XCTAssertTrue(l.aEnvoyer().isEmpty)
        XCTAssertNil(BrouillonSaisie(grille: grille([]), evaluationId: "inconnue"))
    }

    func testIssuesEcritConflitEtRefus() throws {
        var b = try XCTUnwrap(BrouillonSaisie(grille: grille([
            ("e1", CelluleEva()), ("e2", CelluleEva(statut: "note", valeurM: 8_000, version: 1)), ("e3", CelluleEva()),
        ]), evaluationId: "ev1"))
        b.taper("e1", "15,5"); b.taper("e2", "9"); b.taper("e3", "abs")
        let envoye = b.lot()
        let n = b.appliquer(SaisieEvaRetour(cellules: [
            CelluleRetourEva(evaluationId: "ev1", eleveId: "e1", issue: "ecrit", version: 1, statut: "note", valeurM: 15_500),
            CelluleRetourEva(evaluationId: "ev1", eleveId: "e2", issue: "conflit", conflit: ConflitEva(statut: "note", valeurM: 11_000, version: 2, parNom: "Mme Autre")),
            CelluleRetourEva(evaluationId: "ev1", eleveId: "e3", issue: "refuse", message: "période arrêtée"),
        ]), envoye: envoye)
        XCTAssertEqual(n, 1)
        XCTAssertEqual(b.texte("e1"), "15,5")
        XCTAssertFalse(b.modifiee("e1"))
        XCTAssertEqual(b.conflit("e2")?.valeurM, 11_000)
        XCTAssertEqual(b.texte("e2"), "9")
        XCTAssertEqual(b.refus("e3"), "période arrêtée")
        XCTAssertEqual(b.aEnvoyer().map(\.eleveId), ["e3"])
        b.garderLaMienne("e2")
        XCTAssertEqual(b.aEnvoyer().first { $0.eleveId == "e2" }, CelluleSaisieEva(evaluationId: "ev1", eleveId: "e2", texte: "9", versionAttendue: 2))
        var c = try XCTUnwrap(BrouillonSaisie(grille: grille([("e2", CelluleEva(statut: "note", valeurM: 8_000, version: 1))]), evaluationId: "ev1"))
        c.taper("e2", "9")
        c.appliquer(SaisieEvaRetour(cellules: [CelluleRetourEva(evaluationId: "ev1", eleveId: "e2", issue: "conflit", conflit: ConflitEva(statut: "note", valeurM: 11_000, version: 2))]), envoye: c.lot())
        c.reprendreServeur("e2")
        XCTAssertEqual(c.texte("e2"), "11")
        XCTAssertNil(c.conflit("e2"))
        XCTAssertTrue(c.aEnvoyer().isEmpty)
    }

    func testIntentionDuLotSuitSonContenu() throws {
        var b = try XCTUnwrap(BrouillonSaisie(grille: grille([("e1", CelluleEva()), ("e2", CelluleEva(statut: "note", valeurM: 8_000, version: 4))]), evaluationId: "ev1"))
        b.taper("e2", "9"); b.taper("e1", "15")
        XCTAssertEqual(b.intention(b.lot()), "eva-saisie:ev1:e1=15@-;e2=9@4")
        XCTAssertTrue(b.intention(b.lot()).hasPrefix(b.famille))
    }

    func testFrappePendantLEnvoiGardeeEtConflitSansEtatBloque() throws {
        var b = try XCTUnwrap(BrouillonSaisie(grille: grille([("e1", CelluleEva()), ("e2", CelluleEva(statut: "note", valeurM: 8_000, version: 1))]), evaluationId: "ev1"))
        b.taper("e1", "12"); b.taper("e2", "9")
        let envoye = b.lot()
        b.taper("e1", "14")
        b.appliquer(SaisieEvaRetour(cellules: [
            CelluleRetourEva(evaluationId: "ev1", eleveId: "e1", issue: "ecrit", version: 1, statut: "note", valeurM: 12_000),
            CelluleRetourEva(evaluationId: "ev1", eleveId: "e2", issue: "conflit", message: "la case a changé depuis votre lecture"),
        ]), envoye: envoye)
        XCTAssertEqual(b.texte("e1"), "14")
        XCTAssertEqual(b.aEnvoyer(), [CelluleSaisieEva(evaluationId: "ev1", eleveId: "e1", texte: "14", versionAttendue: 1)])
        XCTAssertEqual(b.refus("e2"), "la case a changé depuis votre lecture")
        b.taper("e2", "10")
        XCTAssertTrue(b.aEnvoyer().allSatisfy { $0.eleveId != "e2" })
    }

    func testAttenteEtRemplies() throws {
        var b = try XCTUnwrap(BrouillonSaisie(grille: grille([("e1", CelluleEva()), ("e2", CelluleEva(statut: "note", valeurM: 8_000, version: 1))]), evaluationId: "ev1"))
        XCTAssertFalse(b.enAttente())
        XCTAssertEqual(b.nbRemplies(), 1)
        b.taper("e1", "25")
        XCTAssertTrue(b.enAttente())
    }

    func testGrilleDegenereeSansPlantage() throws {
        let g = grille([("e1", CelluleEva()), ("e1", CelluleEva())])
        let b = try XCTUnwrap(BrouillonSaisie(grille: g, evaluationId: "ev1"))
        XCTAssertEqual(b.eleves.map(\.eleveId), ["e1"])
    }

    func testLotEnvoyeAvecLaCleEtSansVersionNulle() async throws {
        let t = Enregistreur()
        let c = ApiClient(baseURL: FauxServeur.base, transport: t, jeton: { "jeton" }, appli: "ios/1 (build 1)")
        _ = try await c.saisirNotes(SaisieEvaBody(coursId: "c1", cellules: [
            CelluleSaisieEva(evaluationId: "ev1", eleveId: "e1", texte: "12"),
            CelluleSaisieEva(evaluationId: "ev1", eleveId: "e2", texte: "abs", versionAttendue: 3),
        ]), cle: "k1")
        let r = try XCTUnwrap(t.recues.first)
        XCTAssertEqual(r.entetes["Idempotency-Key"], "k1")
        let corps = String(decoding: try XCTUnwrap(r.corps), as: UTF8.self)
        XCTAssertEqual(corps, #"{"cellules":[{"eleve_id":"e1","evaluation_id":"ev1","texte":"12"},{"eleve_id":"e2","evaluation_id":"ev1","texte":"abs","version_attendue":3}],"cours_id":"c1"}"#)
    }

    func testPeriodeDuJourEtLibelles() {
        let p = PeriodeEva(id: "p", code: "T1", libelle: "T1", dateDebut: "2027-01-01", dateFin: "2027-03-31")
        XCTAssertTrue(p.contient("2027-03-31"))
        XCTAssertFalse(p.contient("2027-04-01"))
        XCTAssertEqual(SaisieNotes.resume(statut: "note", valeurM: 12_500, baremeM: 20_000), "12,5 / 20")
        XCTAssertEqual(SaisieNotes.resume(statut: "dispense", valeurM: nil, baremeM: 20_000), "Dispensé")
    }
}
