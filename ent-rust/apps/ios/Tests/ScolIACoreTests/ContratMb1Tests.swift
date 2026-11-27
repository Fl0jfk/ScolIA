import XCTest
@testable import ScolIACore

/// Contrat mobile MB1 côté appli (§5.2, §5.3) et correctifs d'appli de MB0 (DA1, DS4) :
/// miroir de `ContratMb1Test.kt` (mêmes cas, cœurs miroirs, §10).
private let appli = "ios/1.4.2 (build 142)"

private func enveloppe(_ code: String, _ message: String = "msg", reessayer: String = "null") -> String {
    #"{"erreur":{"code":"\#(code)","message":"\#(message)","champs":[],"reessayer_apres_s":\#(reessayer),"trace":"tr-1"}}"#
}

final class ContratMb1Tests: XCTestCase {
    let paris = TimeZone(identifier: "Europe/Paris")!

    func client(_ s: FauxServeur, _ coffre: CoffreJeton = CoffreMemoire("j")) -> ApiClient {
        ApiClient(baseURL: FauxServeur.base, transport: s, jeton: { coffre.lire() }, appli: appli)
    }

    // MARK: En-têtes

    func testTouteRequeteDeclareLAppli() async throws {
        let s = FauxServeur(["GET /auth/me": (200, meProf), "GET /auth/etablissement": (200, #"{"id":"e1","slug":"lpnb","nom":"Lycée"}"#)])
        let c = client(s)
        _ = try await c.me()
        _ = try? await c.etablissementDeLAdresse()
        XCTAssertEqual(s.recues.count, 2)
        for r in s.recues { XCTAssertEqual(r.entetes["X-ScolIA-Appli"], appli) }
        XCTAssertEqual(ApiClient.valeurAppli(plateforme: "ios", version: "0.1.0", build: "1"), "ios/0.1.0 (build 1)")
    }

    func testEcritureAuthentifieePorteUneCleUUIDEtLectureNon() async throws {
        let s = FauxServeur(["POST /appels/a1/lignes": (204, ""), "GET /auth/me": (200, meProf), "DELETE /devices/push/p1": (204, "")])
        let c = client(s)
        try await c.marquer(appelId: "a1", eleveId: "el1", etat: .present)
        _ = try await c.me()
        try await c.desinscrirePush("p1")
        let cle = try XCTUnwrap(s.recues[0].entetes["Idempotency-Key"])
        XCTAssertNotNil(UUID(uuidString: cle))
        XCTAssertEqual(cle.count, 36)
        XCTAssertNil(s.recues[1].entetes["Idempotency-Key"])
        XCTAssertNotNil(s.recues[2].entetes["Idempotency-Key"])
    }

    func testDeuxIntentionsDeuxClesEtUnNouvelEssaiGardeLaSienne() async throws {
        let s = FauxServeur(["POST /appels/a1/lignes": (204, "")])
        let c = client(s)
        try await c.marquer(appelId: "a1", eleveId: "el1", etat: .absent)
        try await c.marquer(appelId: "a1", eleveId: "el2", etat: .absent)
        XCTAssertNotEqual(s.recues[0].entetes["Idempotency-Key"], s.recues[1].entetes["Idempotency-Key"])
        let k = "6f1c1a8e-0e8b-4a43-9c55-3a9f2c7a2b10"
        try await c.marquer(appelId: "a1", eleveId: "el1", etat: .absent, cle: k)
        try await c.marquer(appelId: "a1", eleveId: "el1", etat: .absent, cle: k)
        XCTAssertEqual(s.recues[2].entetes["Idempotency-Key"], k)
        XCTAssertEqual(s.recues[3].entetes["Idempotency-Key"], k)
    }

    func testAuthentificationHorsRegistreSansCle() async throws {
        let s = FauxServeur([
            "POST /auth/login": (401, enveloppe("auth.refusee")),
            "POST /auth/logout": (204, ""),
            "DELETE /auth/sessions/s1": (204, ""),
        ])
        let c = client(s)
        _ = try? await c.login(email: "a@b.fr", motDePasse: "x")
        try await c.logout()
        try await c.revoquerSession("s1")
        for r in s.recues { XCTAssertNil(r.entetes["Idempotency-Key"], r.url.absoluteString) }
    }

    // MARK: Enveloppe d'erreur codée

    func testSessionExpireeEstUneSessionPerdue() async {
        let s = FauxServeur(["GET /auth/me": (401, enveloppe("session.expiree", "Session expirée."))])
        do { _ = try await client(s).me(); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .nonAuthentifie) }
    }

    func testAuthRefuseeNEstPasUneSessionPerdueMemeAvecJeton() async {
        let s = FauxServeur(["POST /auth/passkey/register/start": (401, enveloppe("auth.refusee", "Identifiants refusés."))])
        do {
            _ = try await client(s).passkeyRegisterStart(); XCTFail()
        } catch let e as ErreurApi {
            XCTAssertEqual(e.code, "auth.refusee")
            XCTAssertEqual(e.errorDescription, "Identifiants refusés.")
            XCTAssertNotEqual(e, .nonAuthentifie)
        } catch { XCTFail() }
    }

    func testConnexionRefuseeAvecEnveloppe() async {
        let s = FauxServeur(["POST /auth/login": (401, enveloppe("auth.refusee", "Identifiants refusés."))])
        do { _ = try await client(s, CoffreMemoire()).login(email: "a@b.fr", motDePasse: "x"); XCTFail() } catch let e as ErreurApi {
            XCTAssertEqual(e.code, "auth.refusee")
        } catch { XCTFail() }
    }

    func testMessageNeutreJamaisLeJsonBrut() async {
        let s = FauxServeur([
            "GET /eleves/x/notes": (404, enveloppe("objet.introuvable", "Élément introuvable.")),
            "GET /annonces": (403, #"{"autre":"json"}"#),
        ])
        let c = client(s)
        do { _ = try await c.notes("x"); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .introuvable("Élément introuvable.")) }
        do { _ = try await c.annonces(); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .interdit("Action non autorisée.")) }
    }

    func testChampsEtRegleMetier() async {
        let s = FauxServeur([
            "POST /absences/ab1/justifier": (400, #"{"erreur":{"code":"requete.invalide","message":"Motif manquant.","champs":[{"champ":"motif","code":"champ.requis"}],"reessayer_apres_s":null,"trace":"t"}}"#),
            "POST /eleves/el1/notes": (422, enveloppe("periode.arretee", "Période arrêtée.")),
        ])
        let c = client(s)
        do { _ = try await c.justifierAbsence("ab1", motif: ""); XCTFail() } catch let ErreurApi.codee(statut, e) {
            XCTAssertEqual(statut, 400)
            XCTAssertEqual(e.code, "requete.invalide")
            XCTAssertEqual(e.champs, [ErreurCodee.Champ(champ: "motif", code: "champ.requis")])
        } catch { XCTFail("\(error)") }
        let corps = SaisirNoteBody(matiereId: "m1", valeur: 12, sur: nil, dateNote: "2027-03-14", libelle: nil, devoirId: nil, periodeId: nil)
        do { _ = try await c.saisirNote(eleveId: "el1", corps); XCTFail() } catch let e as ErreurApi {
            XCTAssertEqual(e.code, "periode.arretee")
            XCTAssertEqual(e.errorDescription, "Période arrêtée.")
        } catch { XCTFail() }
    }

    func testIdempotenceEnCoursDonneLeDelai() async {
        let s = FauxServeur(["POST /appels/a1/clore": (409, enveloppe("idempotence.en_cours", "En cours.", reessayer: "2"))])
        do { _ = try await client(s).clore(appelId: "a1"); XCTFail() } catch let ErreurApi.codee(statut, e) {
            XCTAssertEqual(statut, 409)
            XCTAssertEqual(e.code, "idempotence.en_cours")
            XCTAssertEqual(e.reessayerApresS, 2)
        } catch { XCTFail() }
    }

    func testAppliTropAncienneEtDebit() async {
        let s = FauxServeur([
            "GET /auth/me": (426, enveloppe("appli.trop_ancienne", "Mettez l'application à jour pour continuer.")),
            "GET /annonces": (429, enveloppe("debit.depasse", "Trop de demandes.", reessayer: "30")),
        ])
        let c = client(s)
        do { _ = try await c.me(); XCTFail() } catch { XCTAssertEqual((error as? ErreurApi)?.code, "appli.trop_ancienne") }
        do { _ = try await c.annonces(); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .debitDepasse("Trop de demandes.", 30)) }
    }

    func testAncienTexteCourtToujoursLu() async {
        let s = FauxServeur(["POST /absences/ab1/justifier": (422, "Délai dépassé.")])
        do { _ = try await client(s).justifierAbsence("ab1", motif: "malade"); XCTFail() } catch {
            XCTAssertEqual(error as? ErreurApi, .refus(422, "Délai dépassé."))
        }
    }

    // MARK: DA1 : expiration du cache

    func date(_ t: String) -> Date {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.timeZone = paris
        f.dateFormat = "yyyy-MM-dd'T'HH:mm:ss"
        return f.date(from: t)!
    }

    func testEdtEtAppelExpirentAMinuitLeResteA24h() {
        let ecrit = date("2027-03-14T22:30:00")
        let minuit = date("2027-03-15T00:00:00")
        XCTAssertEqual(DureeCache.expiration(Chemins.mesCreneaux("2027-03-14"), ecriture: ecrit, fuseau: paris), minuit)
        XCTAssertEqual(DureeCache.expiration(Chemins.edtEleve("el1", "2027-03-14"), ecriture: ecrit, fuseau: paris), minuit)
        XCTAssertEqual(DureeCache.expiration("/appels/a1", ecriture: ecrit, fuseau: paris), minuit)
        XCTAssertEqual(DureeCache.expiration(Chemins.fils, ecriture: ecrit, fuseau: paris), ecrit.addingTimeInterval(24 * 3600))
        XCTAssertEqual(DureeCache.expiration(Chemins.notes("el1"), ecriture: ecrit, fuseau: paris), ecrit.addingTimeInterval(24 * 3600))
        XCTAssertFalse(DureeCache.duJour("/eleves/el1/notes"))
    }

    final class Horloge: @unchecked Sendable { var t: Date; init(_ t: Date) { self.t = t } }

    func testCacheDe25hNonServiEtEfface() async throws {
        let s = FauxServeur(["GET /messages/fils": (200, "[]")])
        let cache = CacheMemoire()
        let h = Horloge(date("2027-03-14T08:00:00"))
        let tz = paris
        let l = LecteurHorsLigne(client: client(s), cache: cache, prefixe: { "u1|e1" }, maintenant: { h.t }, fuseau: { tz })
        let _: Lecture<[MsgFil]> = try await l.lire(Chemins.fils)
        s.horsLigne = true
        h.t = h.t.addingTimeInterval(23 * 3600)
        let hl: Lecture<[MsgFil]> = try await l.lire(Chemins.fils)
        XCTAssertTrue(hl.horsLigne)
        h.t = h.t.addingTimeInterval(2 * 3600)
        do { let _: Lecture<[MsgFil]> = try await l.lire(Chemins.fils); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .horsLigne) }
        XCTAssertEqual(cache.taille, 0)
    }

    func testEdtDeLaVeilleNonServiApresMinuit() async throws {
        let chemin = Chemins.mesCreneaux("2027-03-14")
        let s = FauxServeur(["GET \(chemin)": (200, "[]")])
        let h = Horloge(date("2027-03-14T23:50:00"))
        let tz = paris
        let l = LecteurHorsLigne(client: client(s), cache: CacheMemoire(), prefixe: { "u1|e1" }, maintenant: { h.t }, fuseau: { tz })
        let _: Lecture<[EdtCreneau]> = try await l.lire(chemin)
        s.horsLigne = true
        h.t = date("2027-03-15T00:00:01")
        do { let _: Lecture<[EdtCreneau]> = try await l.lire(chemin); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .horsLigne) }
    }

    func testPurgeAuDemarrageNeGardeQueLeValide() {
        let cache = CacheMemoire()
        let t0 = Date(timeIntervalSince1970: 0)
        cache.ecrire("a", EntreeCache(json: Data("[]".utf8), horodatage: t0, expire: Date(timeIntervalSince1970: 100)))
        cache.ecrire("b", EntreeCache(json: Data("[]".utf8), horodatage: t0, expire: Date(timeIntervalSince1970: 1000)))
        cache.purgerEchues(Date(timeIntervalSince1970: 500))
        XCTAssertNil(cache.lire("a"))
        XCTAssertNotNil(cache.lire("b"))
        let l = LecteurHorsLigne(client: client(FauxServeur([:])), cache: cache, prefixe: { "x" }, maintenant: { Date(timeIntervalSince1970: 2000) })
        l.purger()
        XCTAssertEqual(cache.taille, 0)
    }

    func testAncienFormatDeCacheIllisible() {
        // Entrée écrite avant DA1 (sans date de fin) : ne se décode plus, donc n'est jamais servie.
        let ancien = Data(#"{"json":"W10=","horodatage":0}"#.utf8)
        XCTAssertNil(try? JSONDecoder().decode(EntreeCache.self, from: ancien))
    }

    // MARK: DS4 : désinscription des notifications

    struct PushFaux: FournisseurPush {
        let plateforme = "ios"
        func jetonAppareil() async -> String? { "jeton-apns" }
    }

    func testPushInscritGardeLIdentifiantEtDesinscritALaDeconnexion() async throws {
        let s = FauxServeur([
            "POST /auth/login": (200, #"{"mfa_required":false,"token":"t","user_id":"u1","email":"a@b.fr","etablissement_id":"e1","role":"prof","expires_at":"z"}"#),
            "GET /auth/me": (200, meProf),
            "GET /roles/mes-droits": (200, "[]"),
            "POST /devices/push": (201, #"{"id":"p1","platform":"ios"}"#),
            "DELETE /devices/push/p1": (204, ""),
            "POST /auth/logout": (204, ""),
        ])
        let coffre = CoffreMemoire()
        let memoire = MemoirePushMemoire()
        let svc = ServiceConnexion(client: client(s, coffre), coffre: coffre, cache: CacheMemoire(), passkey: FauxPasskey(),
                                   push: PushFaux(), memoirePush: memoire)
        _ = try await svc.connexionMotDePasse(email: "a@b.fr", motDePasse: "x")
        XCTAssertEqual(memoire.lire(), "p1")
        let inscription = try XCTUnwrap(s.recues.first { $0.methode == "POST" && $0.url.path == "/devices/push" })
        XCTAssertNotNil(inscription.entetes["Idempotency-Key"])
        await svc.deconnexion()
        let ordre = s.recues.map { "\($0.methode) \($0.url.path)" }
        let d = try XCTUnwrap(ordre.firstIndex(of: "DELETE /devices/push/p1"))
        let l = try XCTUnwrap(ordre.firstIndex(of: "POST /auth/logout"))
        XCTAssertLessThan(d, l)
        XCTAssertNil(memoire.lire())
        XCTAssertNil(coffre.lire())
    }
}
