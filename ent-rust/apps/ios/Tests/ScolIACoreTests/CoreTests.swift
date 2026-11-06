import XCTest
@testable import ScolIACore

/// Faux serveur : répond selon « MÉTHODE chemin », garde les requêtes reçues.
final class FauxServeur: Transport, @unchecked Sendable {
    static let base = URL(string: "https://lpnb.scolia.fr")!
    var routes: [String: (Int, String)]
    var recues: [RequeteHttp] = []
    var horsLigne = false
    init(_ routes: [String: (Int, String)]) { self.routes = routes }

    func executer(_ r: RequeteHttp) async throws -> ReponseHttp {
        if horsLigne { throw URLError(.notConnectedToInternet) }
        recues.append(r)
        let chemin = String(r.url.absoluteString.dropFirst(Self.base.absoluteString.count))
        let (s, c) = routes["\(r.methode) \(chemin)"] ?? (404, "introuvable")
        return ReponseHttp(statut: s, corps: Data(c.utf8))
    }
}

struct FauxPasskey: AuthentificateurPasskey {
    func signer(_ d: PasskeyPont.DemandeAssertion) async throws -> PasskeyPont.Assertion {
        PasskeyPont.Assertion(credentialId: Data("id1".utf8), authenticatorData: Data("auth".utf8),
                              clientDataJSON: Data("client".utf8), signature: Data("sig".utf8), userHandle: Data("user".utf8))
    }
    func creer(_ d: PasskeyPont.DemandeCreation) async throws -> (credentialId: Data, attestationObject: Data, clientDataJSON: Data) {
        (Data("id1".utf8), Data("att".utf8), Data("cd".utf8))
    }
}

let optionsConnexion = #"{"publicKey":{"challenge":"Y2hhbGxlbmdl","timeout":60000,"rpId":"scolia.fr","allowCredentials":[{"type":"public-key","id":"aWQx"}],"userVerification":"required"}}"#
let meProf = #"{"user_id":"u1","email":"prof@lpnb.fr","etablissement_id":"e1","role":"prof","passkey_count":1,"portee":"complete","assurance":"passkey","client":"mobile","passkey_obligatoire":true}"#

final class ModelesTests: XCTestCase {
    func testLoginAPlat() throws {
        let r: LoginResponse = try JSONDecoder().decode(LoginResponse.self, from: Data(#"{"mfa_required":false,"token":"t","user_id":"u","email":"a@b.fr","etablissement_id":"e","role":"parent","expires_at":"2027-01-01T00:00:00Z"}"#.utf8))
        XCTAssertEqual(r.session?.role, "parent")
        XCTAssertFalse(String(describing: r.session!).contains("\"t\""))
    }

    func testLoginMfa() throws {
        let r = try JSONDecoder().decode(LoginResponse.self, from: Data(#"{"mfa_required":true,"passkey":{"token":"x","options":\#(optionsConnexion),"expires_at":"z"}}"#.utf8))
        XCTAssertNil(r.session)
        XCTAssertEqual(r.passkey?.token, "x")
    }

    func testMeChampsAbsents() throws {
        let me = try ApiClient.decodeur.decode(Me.self, from: Data(#"{"user_id":"u","email":"e","etablissement_id":"e","role":"eleve","eleve_id":"el1","inconnu":1}"#.utf8))
        XCTAssertEqual(me.portee, "complete")
        XCTAssertEqual(me.eleveId, "el1")
    }

    func testAppel() throws {
        let a = try ApiClient.decodeur.decode(Appel.self, from: Data(#"{"id":"a","classe":"3A","jour":"2026-11-06","statut":"clos","creneau_id":null,"matiere_libelle":"Maths","lignes":[{"eleve_id":"e","eleve_label":"Lou","etat":"retard"}]}"#.utf8))
        XCTAssertTrue(a.estClos)
        XCTAssertEqual(EtatAppel(rawValue: a.lignes[0].etat), .retard)
    }
}

final class CapacitesTests: XCTestCase {
    func me(_ role: String, eleve: String? = nil, portee: String = "complete") -> Me {
        Me(userId: "u", email: "x", etablissementId: "e", role: role, eleveId: eleve, passkeyCount: 1, portee: portee)
    }

    func testProfesseur() {
        let e = Capacites.espace(me: me("prof"), droits: [Droit(action: "notes.saisir", portee: "mes_classes")])
        XCTAssertEqual(e.profil, .professeur)
        XCTAssertEqual(Array(e.modules.prefix(2)), [.accueil, .appel])
        XCTAssertTrue(e.contient(.saisieNotes))
        XCTAssertEqual(e.modules.last, .profil)
    }

    func testFamille() {
        let e = Capacites.espace(me: me("parent"), droits: [])
        XCTAssertEqual(e.profil, .famille)
        XCTAssertTrue(e.contient(.absences))
        XCTAssertFalse(e.contient(.appel))
    }

    func testEleve() {
        let e = Capacites.espace(me: me("eleve", eleve: "el1"), droits: [])
        XCTAssertEqual(e.profil, .eleve)
        XCTAssertFalse(e.contient(.absences))
    }

    func testRoleLibreVieScolaire() {
        let e = Capacites.espace(me: me("administratif"), droits: [Droit(action: "absences.lire", portee: "etablissement")])
        XCTAssertEqual(e.profil, .vieScolaire)
        XCTAssertTrue(e.contient(.vieScolaire))
    }

    func testEnrolement() {
        let e = Capacites.espace(me: me("prof", portee: "enrolement"), droits: [])
        XCTAssertTrue(e.enrolementRequis)
        XCTAssertEqual(e.modules, [.profil])
    }
}

final class RattachementTests: XCTestCase {
    let l = LecteurRattachement()

    func testInvitation() {
        guard case .ok(let r) = l.lire("https://lpnb.scolia.fr/rejoindre?invitation=AbCdEf123456") else { return XCTFail() }
        XCTAssertEqual(r.baseURL.absoluteString, "https://lpnb.scolia.fr")
        XCTAssertEqual(r.slug, "lpnb")
        XCTAssertEqual(r.invitation, "AbCdEf123456")
    }

    func testSaisieCourte() {
        guard case .ok(let r) = l.lire(" LPNB ") else { return XCTFail() }
        XCTAssertEqual(r.baseURL.absoluteString, "https://lpnb.scolia.fr")
    }

    func testRefus() {
        for s in ["http://lpnb.scolia.fr", "https://scolia.fr.evil.com", "https://evilscolia.fr", "https://lpnb.scolia.fr:8443",
                  "https://u:p@lpnb.scolia.fr", "scolia://x?h=lpnb", "https://www.scolia.fr", "", "javascript:alert(1)"] {
            if case .ok = l.lire(s) { XCTFail(s) }
        }
    }

    func testInvitationMalformeeIgnoree() {
        guard case .ok(let r) = l.lire("https://lpnb.scolia.fr/rejoindre?invitation=abc") else { return XCTFail() }
        XCTAssertNil(r.invitation)
    }

    func testLabo() {
        if case .ok = l.lire("http://127.0.0.1:3001") { XCTFail() }
        guard case .ok(let r) = LecteurRattachement(modeLabo: true).lire("http://127.0.0.1:3001?etablissement_id=abc") else { return XCTFail() }
        XCTAssertEqual(r.baseURL.absoluteString, "http://127.0.0.1:3001")
        XCTAssertEqual(r.etablissementIdLabo, "abc")
    }
}

final class PasskeyTests: XCTestCase {
    func options(_ s: String = optionsConnexion) throws -> JSONValue { try JSONDecoder().decode(JSONValue.self, from: Data(s.utf8)) }

    func testDemandeConnexion() throws {
        let d = try PasskeyPont.demandeConnexion(try options(), rpIdAttendu: "scolia.fr")
        XCTAssertEqual(d.challenge, Data("challenge".utf8))
        XCTAssertEqual(d.credentialIds, [Data("id1".utf8)])
        XCTAssertTrue(d.verificationRequise)
    }

    func testRpIdEtranger() throws {
        XCTAssertThrowsError(try PasskeyPont.demandeConnexion(try options(optionsConnexion.replacingOccurrences(of: "scolia.fr", with: "evil.fr")), rpIdAttendu: "scolia.fr"))
    }

    func testAssertionFormatServeur() throws {
        let a = PasskeyPont.Assertion(credentialId: Data([0xfb, 0xff]), authenticatorData: Data("a".utf8), clientDataJSON: Data("c".utf8), signature: Data("s".utf8), userHandle: nil)
        let c = PasskeyPont.assertionPourServeur(a)
        XCTAssertEqual(c["rawId"] as? String, "-_8") // base64url sans remplissage
        let r = c["response"] as! [String: Any]
        XCTAssertEqual(Set(r.keys), ["authenticatorData", "clientDataJSON", "signature"])
        XCTAssertNotNil(c["extensions"])
        XCTAssertTrue(JSONSerialization.isValidJSONObject(c))
    }

    func testBase64URL() {
        XCTAssertNil(PasskeyPont.base64URLDecoder("ab+/"))
        XCTAssertEqual(PasskeyPont.base64URLDecoder("aWQx"), Data("id1".utf8))
    }

    func testDemandeEnrolement() throws {
        let o = try options(#"{"publicKey":{"rp":{"id":"scolia.fr","name":"ScolIA"},"user":{"id":"dXNlcg","name":"a@b.fr","displayName":"A"},"challenge":"Y2g","pubKeyCredParams":[]}}"#)
        let d = try PasskeyPont.demandeEnrolement(o, rpIdAttendu: "scolia.fr")
        XCTAssertEqual(d.userId, Data("user".utf8))
        XCTAssertEqual(d.nom, "a@b.fr")
    }
}

final class ConnexionTests: XCTestCase {
    func client(_ s: FauxServeur, _ coffre: CoffreJeton) -> ApiClient {
        ApiClient(baseURL: FauxServeur.base, transport: s, jeton: { coffre.lire() })
    }

    func testConnexionPasskey() async throws {
        let s = FauxServeur([
            "POST /auth/passkey/login/start": (200, #"{"token":"defi1","options":\#(optionsConnexion),"expires_at":"z"}"#),
            "POST /auth/passkey/login/finish": (200, #"{"token":"jeton-secret","user_id":"u1","email":"p","etablissement_id":"e1","role":"prof","expires_at":"z"}"#),
            "GET /auth/me": (200, meProf),
            "GET /roles/mes-droits": (200, #"[{"action":"notes.saisir","portee":"mes_classes","cibles":[]}]"#),
        ])
        let coffre = CoffreMemoire()
        let svc = ServiceConnexion(client: client(s, coffre), coffre: coffre, cache: CacheMemoire(), passkey: FauxPasskey())
        guard case .connecte(_, let espace) = try await svc.connexionPasskey(email: "prof@lpnb.fr") else { return XCTFail() }
        XCTAssertEqual(espace.profil, .professeur)
        XCTAssertEqual(coffre.lire(), "jeton-secret")
        let start = try JSONSerialization.jsonObject(with: s.recues[0].corps!) as! [String: Any]
        XCTAssertEqual(start["client"] as? String, "mobile")
        XCTAssertNil(s.recues[0].entetes["Authorization"])
        let finish = try JSONSerialization.jsonObject(with: s.recues[1].corps!) as! [String: Any]
        XCTAssertEqual(finish["token"] as? String, "defi1")
        XCTAssertNotNil((finish["credential"] as? [String: Any])?["response"])
        XCTAssertEqual(s.recues[2].entetes["Authorization"], "Bearer jeton-secret")
    }

    func testSecondFacteur() async throws {
        let s = FauxServeur(["POST /auth/login": (200, #"{"mfa_required":true,"passkey":{"token":"d2","options":\#(optionsConnexion),"expires_at":"z"}}"#)])
        let coffre = CoffreMemoire()
        let svc = ServiceConnexion(client: client(s, coffre), coffre: coffre, cache: CacheMemoire(), passkey: FauxPasskey())
        guard case .secondFacteur(let d) = try await svc.connexionMotDePasse(email: "p", motDePasse: "pw") else { return XCTFail() }
        XCTAssertEqual(d.token, "d2")
        XCTAssertNil(coffre.lire())
        let corps = try JSONSerialization.jsonObject(with: s.recues[0].corps!) as! [String: Any]
        XCTAssertEqual(corps["client"] as? String, "mobile")
    }

    func testMauvaisMotDePasseNEstPasUneSessionPerdue() async throws {
        let s = FauxServeur(["POST /auth/login": (401, "identifiants invalides")])
        let coffre = CoffreMemoire("autre-session")
        let svc = ServiceConnexion(client: client(s, coffre), coffre: coffre, cache: CacheMemoire(), passkey: FauxPasskey())
        do { _ = try await svc.connexionMotDePasse(email: "p", motDePasse: "faux"); XCTFail() } catch {
            XCTAssertEqual(error as? ErreurApi, .refus(401, "identifiants invalides"))
        }
        XCTAssertEqual(coffre.lire(), "autre-session")
    }

    func testRequeteSansJetonDansDescription() {
        let r = RequeteHttp(methode: "GET", url: URL(string: "https://x")!, entetes: ["Authorization": "Bearer secret"], corps: nil)
        XCTAssertFalse(String(describing: r).contains("secret"))
    }

    func testRevocationEfface() async throws {
        let s = FauxServeur(["GET /auth/me": (401, "")])
        let coffre = CoffreMemoire("ancien")
        let cache = CacheMemoire()
        cache.ecrire("k", EntreeCache(json: Data("[]".utf8), horodatage: Date()))
        let svc = ServiceConnexion(client: client(s, coffre), coffre: coffre, cache: cache, passkey: FauxPasskey())
        let etat = try await svc.reprendre()
        XCTAssertEqual(etat, .deconnecte)
        XCTAssertNil(coffre.lire())
        XCTAssertNil(cache.lire("k"))
    }

    func testErreurs() async throws {
        let s = FauxServeur([
            "GET /famille/enfants": (403, "accès refusé"),
            "GET /annonces": (500, "<html>"),
            "POST /absences/a1/justifier": (422, "motif trop court"),
        ])
        let c = client(s, CoffreMemoire("j"))
        do { _ = try await c.mesEnfants(); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .interdit("accès refusé")) }
        do { _ = try await c.annonces(); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .serveur(500)) }
        do { _ = try await c.justifierAbsence("a1", motif: "x"); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .refus(422, "motif trop court")) }
        s.horsLigne = true
        do { _ = try await c.annonces(); XCTFail() } catch { XCTAssertEqual(error as? ErreurApi, .horsLigne) }
    }

    func testCheminsEncodes() async throws {
        let s = FauxServeur(["GET /classes/3e%20A/effectif?jour=2026-11-06": (200, #"{"classe":"3e A","jour":"2026-11-06","eleves":[]}"#)])
        let e = try await client(s, CoffreMemoire("j")).effectif(classe: "3e A", jour: "2026-11-06")
        XCTAssertEqual(e.classe, "3e A")
    }

    func testAppelMarquerClore() async throws {
        let s = FauxServeur([
            "POST /appels/a1/lignes": (204, ""),
            "POST /appels/a1/clore": (200, #"{"id":"a1","classe":"3A","jour":"2026-11-06","statut":"clos","lignes":[]}"#),
        ])
        let c = client(s, CoffreMemoire("j"))
        try await c.marquer(appelId: "a1", eleveId: "el1", etat: .absent)
        let corps = try JSONSerialization.jsonObject(with: s.recues[0].corps!) as! [String: String]
        XCTAssertEqual(corps, ["eleve_id": "el1", "etat": "absent"])
        let a = try await c.clore(appelId: "a1")
        XCTAssertTrue(a.estClos)
    }

    func testHorsLigne() async throws {
        let s = FauxServeur(["GET /messages/fils": (200, "[]")])
        let cache = CacheMemoire()
        let c = client(s, CoffreMemoire("j"))
        let l = LecteurHorsLigne(client: c, cache: cache, prefixe: { "u1|e1" })
        let l1: Lecture<[MsgFil]> = try await l.lire(Chemins.fils)
        XCTAssertFalse(l1.horsLigne)
        s.horsLigne = true
        let l2: Lecture<[MsgFil]> = try await l.lire(Chemins.fils)
        XCTAssertTrue(l2.horsLigne)
        let autre = LecteurHorsLigne(client: c, cache: cache, prefixe: { "u2|e1" })
        do { let _: Lecture<[MsgFil]> = try await autre.lire(Chemins.fils); XCTFail() } catch {}
    }
}
