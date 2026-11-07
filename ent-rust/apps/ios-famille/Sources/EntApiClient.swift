import Foundation

/// Client HTTP pour l’API ENT Rust (Bearer) — famille.
/// Surfaces : login, push, enfants, résumé, messagerie, signature carnet, sorties.
public struct EntApiClient: Sendable {
    public let baseURL: URL
    public private(set) var token: String?
    public private(set) var etablissementId: UUID?

    public init(baseURL: URL = URL(string: "http://127.0.0.1:3001")!) {
        self.baseURL = baseURL
    }

    public struct Session: Decodable, Sendable {
        public let token: String
        public let user_id: UUID
        public let email: String
        public let etablissement_id: UUID
        public let role: String
    }

    public struct Enfant: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let nom: String
        public let prenom: String
        public let classe: String?
        public let lien: String?
        public let peut_justifier_absence: Bool?
        public let peut_voir_notes: Bool?
    }

    public struct Absence: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let debut: String
        public let fin: String
        public let motif: String?
        public let statut: String
    }

    public struct CarnetEntree: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let titre: String?
        public let corps: String?
        public let signe_at: String?
    }

    public struct Note: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let valeur: Double?
        public let libelle: String?
        public let matiere_libelle: String?
    }

    public struct Resume: Decodable, Sendable {
        public let eleve_id: UUID
        public let absences_a_justifier: [Absence]
        public let carnet_a_signer: [CarnetEntree]
        public let notes: [Note]
    }

    public struct MsgFil: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let eleve_id: UUID
        public let sujet: String
        public let archive: Bool?
        public let last_message_at: String?
    }

    public struct MsgPiece: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let nom: String
        public let mime_type: String
        public let taille_octets: Int?
    }

    public struct Message: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let fil_id: UUID
        public let auteur_user_id: UUID
        public let auteur_label: String?
        public let corps: String
        public let created_at: String
        public let pieces: [MsgPiece]?
    }

    public struct DispenseVs: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let motif: String
        public let debut: String
        public let fin: String
        public let statut: String
    }

    public struct FilDetail: Decodable, Sendable {
        public let fil: MsgFil
        public let messages: [Message]
    }

    public struct AutorisationSortie: Decodable, Sendable {
        public let id: UUID
        public let eleve_id: UUID
        public let matin: Bool
        public let midi: Bool
        public let soir: Bool
        public let interdit_etablissement: Bool
    }

    public struct DemandeSortie: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let eleve_id: UUID
        public let jour: String
        public let creneau: String
        public let motif: String
        public let statut: String
    }

    public struct RecapSortie: Decodable, Sendable {
        public let eleve_id: UUID
        public let autorisation: AutorisationSortie?
        public let demandes_ouvertes: [DemandeSortie]
    }

    public struct DocASigner: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let eleve_id: UUID
        public let code: String
        public let titre: String
        public let statut: String
    }

    public struct MenuCantine: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let jour: String
        public let entree: String
        public let plat: String
        public let dessert: String
        public let notes: String
        public let publie: Bool
    }

    public struct ConventionStage: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let eleve_id: UUID
        public let entreprise_id: UUID
        public let entreprise_label: String?
        public let debut: String
        public let fin: String
        public let tuteur_nom: String
        public let statut: String
    }

    public struct JournalEntree: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let domaine: String
        public let action: String
        public let libelle: String
        public let created_at: String?
    }

    public struct EdtCreneau: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let classe: String
        public let matiere_libelle: String
        public let prof_label: String?
        public let heure_debut: String
        public let heure_fin: String
        public let salle: String?
    }

    public struct PassageRepas: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let eleve_id: UUID
        public let jour: String
        public let badge_lu: String
        public let source: String
    }

    public struct Facture: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let numero: String
        public let libelle: String
        public let statut: String
        public let total_centimes: Int64
        public let nature: String?
    }

    public struct Quittance: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let facture_id: UUID
        public let numero: String
        public let montant_centimes: Int64
        public let moyen: String
        public let emise_at: String
    }

    public struct CahierTexte: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let classe: String
        public let matiere_libelle: String
        public let titre: String
        public let consigne: String
        public let pour_le: String
    }

    public struct AlerteCpe: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let titre: String
        public let detail: String
        public let severite: String
        public let statut: String
        public let jour: String
    }

    public struct RecapInternat: Decodable, Sendable {
        public let eleve_id: UUID
        public let chambre: ChambreInternat?
        public let presence_du_jour: PresenceInternat?
        public let suivis: [SuiviInternat]
    }

    public struct ChambreInternat: Decodable, Sendable {
        public let code: String
        public let batiment: String
        public let etage: Int
    }

    public struct PresenceInternat: Decodable, Sendable {
        public let jour: String
        public let statut: String
    }

    public struct SuiviInternat: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let jour: String
        public let texte: String
    }

    public struct ConseilDossier: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let appreciation: String
        public let mention: String?
        public let decision_texte: String
        public let visible_famille: Bool
    }

    public struct FicheSante: Decodable, Sendable {
        public let allergies: String
        public let traitements: String
        public let contre_indications: String
        public let pai_actif: Bool
        public let medecin_nom: String?
        public let urgence_tel: String?
    }

    public struct Pai: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let motif: String
        public let mesures_accueil: String
        public let protocole_urgence: String
        public let date_debut: String
        public let actif: Bool
        public let signe_famille: Bool
    }

    public struct Voeu: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let filiere_id: UUID
        public let filiere_label: String?
        public let rang: Int
        public let annee_scolaire: String
    }

    public struct InfoCompValeur: Decodable, Sendable {
        public let champ_id: UUID
        public let champ_label: String?
        public let valeur: String
    }

    public struct InfoCompPack: Decodable, Sendable {
        public let champs: [InfoCompChamp]?
        public let valeurs: [InfoCompValeur]
    }

    public struct InfoCompChamp: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let code: String
        public let libelle: String
    }

    public struct DispenseRepas: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let jour: String
        public let motif: String
        public let annulee: Bool
    }

    public struct SoldePoints: Decodable, Sendable {
        public let eleve_id: UUID
        public let solde: Int
        public let depart: Int
    }

    public struct CommissionDossier: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let motifs: String
        public let decision_libelle: String?
        public let points_retires: Int
        public let signale_famille: Bool
    }

    public struct DossierPsy: Decodable, Sendable {
        public let axes_suivi: String
        public let suivi_actif: Bool
        public let signale_famille: Bool
    }

    public struct RdvPsy: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let jour: String
        public let heure_debut: String
        public let heure_fin: String
        public let kind_libelle: String?
        public let kind: String
        public let statut: String
        public let signale_famille: Bool
    }

    public struct ReinscriptionOption: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let code: String
        public let libelle: String
        public let statut: String
    }

    public struct ReinscriptionEnfant: Decodable, Identifiable, Sendable {
        public var id: UUID { enfant.id }
        public let enfant: Enfant
        public let confirmee: Bool
        public let documents_manquants: [String]
        public let acompte_ok: Bool
        public let options: [ReinscriptionOption]
        public let projection_niveau: String?
        public let projection_annee: String?
    }

    public struct ReinscriptionFamille: Decodable, Sendable {
        public let annee_n1: String
        public let campagne_ouverte: Bool
        public let enfants: [ReinscriptionEnfant]
    }

    public struct PassageInfirmerie: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let jour: String
        public let motif: String
        public let motif_libelle: String
        public let note_soins: String
        public let visible_famille: Bool
    }

    public struct NotePeriode: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let code: String
        public let libelle: String
        public let statut: String
    }

    public struct MoyenneMatiere: Decodable, Sendable {
        public let matiere_id: UUID
        public let matiere_libelle: String?
        public let periode_id: UUID
        public let moyenne_sur_20: Double
    }

    public struct Appreciation: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let periode_id: UUID
        public let matiere_id: UUID?
        public let matiere_libelle: String?
        public let texte: String
        public let auteur_label: String?
    }

    public struct TrombiEntree: Decodable, Identifiable, Sendable {
        public let eleve_id: UUID
        public let nom: String
        public let prenom: String
        public let classe: String?
        public let photo_key: String?
        public var id: UUID { eleve_id }
    }

    private struct DossierLite: Decodable {
        let grille_repas: [GrilleRepas]?
    }

    private struct GrilleRepas: Decodable {
        let activite_id: UUID
    }

    public mutating func login(email: String, password: String, etablissementId: UUID) async throws -> Session {
        struct Body: Encodable {
            let email: String
            let password: String
            let etablissement_id: UUID
        }
        let session: Session = try await post(
            path: "/auth/login",
            body: Body(email: email, password: password, etablissement_id: etablissementId),
            authed: false
        )
        self.token = session.token
        self.etablissementId = session.etablissement_id
        return session
    }

    public func registerPush(platform: String, token: String, appId: String?) async throws {
        struct Body: Encodable {
            let platform: String
            let token: String
            let app_id: String?
        }
        struct Resp: Decodable {
            let id: UUID
            let platform: String
        }
        let _: Resp = try await post(
            path: "/devices/push",
            body: Body(platform: platform, token: token, app_id: appId),
            authed: true
        )
    }

    public func enfants() async throws -> [Enfant] {
        try await get(path: "/famille/enfants")
    }

    public func resume(eleveId: UUID) async throws -> Resume {
        try await get(path: "/famille/enfants/\(eleveId.uuidString)/resume")
    }

    public func justifierAbsence(absenceId: UUID, motif: String) async throws {
        struct Body: Encodable { let motif_justif: String }
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/absences/\(absenceId.uuidString)/justifier",
            body: Body(motif_justif: motif),
            authed: true
        )
    }

    public struct AbsenceMotif: Decodable, Identifiable, Sendable {
        public let code: String
        public let libelle: String
        public var id: String { code }
        public init(code: String, libelle: String) {
            self.code = code
            self.libelle = libelle
        }
    }

    public func motifsAbsence() async throws -> [AbsenceMotif] {
        try await get(path: "/motifs-absence")
    }

    public func declarerAbsence(eleveId: UUID, debut: String, fin: String, motif: String) async throws {
        struct Body: Encodable {
            let debut: String
            let fin: String
            let motif: String
        }
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/eleves/\(eleveId.uuidString)/absences",
            body: Body(debut: debut, fin: fin, motif: motif),
            authed: true
        )
    }

    public func signerCarnet(entreeId: UUID) async throws {
        struct Body: Encodable {}
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/carnet/\(entreeId.uuidString)/signer",
            body: Body(),
            authed: true
        )
    }

    public func messages() async throws -> [MsgFil] {
        try await get(path: "/messages/fils")
    }

    public func filDetail(filId: UUID) async throws -> FilDetail {
        try await get(path: "/messages/fils/\(filId.uuidString)")
    }

    public func repondre(filId: UUID, corps: String) async throws {
        struct Body: Encodable { let corps: String }
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/messages/fils/\(filId.uuidString)/repondre",
            body: Body(corps: corps),
            authed: true
        )
    }

    public func pdfFacture(factureId: UUID) async throws -> Data {
        try await getData(path: "/facturation/factures/\(factureId.uuidString)/pdf")
    }

    public func pdfQuittance(quittanceId: UUID) async throws -> Data {
        try await getData(path: "/facturation/quittances/\(quittanceId.uuidString)/pdf")
    }

    public func pdfFicheSante(eleveId: UUID) async throws -> Data {
        try await getData(path: "/eleves/\(eleveId.uuidString)/fiche-sante.pdf")
    }

    public func pdfCourrierPsy(eleveId: UUID) async throws -> Data {
        try await getData(path: "/eleves/\(eleveId.uuidString)/dossier-psy/courrier.pdf")
    }

    public struct PaiPiece: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let nom: String
        public let mime_type: String
    }

    public func paiPieces(eleveId: UUID) async throws -> [PaiPiece] {
        try await get(path: "/eleves/\(eleveId.uuidString)/pai/pieces")
    }

    public func pdfPaiPiece(pieceId: UUID) async throws -> Data {
        try await getData(path: "/pai/pieces/\(pieceId.uuidString)")
    }

    public func deposerPaiPiece(eleveId: UUID, nom: String, mimeType: String, data: Data) async throws {
        struct Body: Encodable {
            let nom: String
            let mime_type: String
            let contenu_base64: String
        }
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/eleves/\(eleveId.uuidString)/pai/pieces",
            body: Body(nom: nom, mime_type: mimeType, contenu_base64: data.base64EncodedString()),
            authed: true
        )
    }

    public func dispensesVs(eleveId: UUID) async throws -> [DispenseVs] {
        try await get(path: "/eleves/\(eleveId.uuidString)/dispenses-vs")
    }

    public func poserDispenseVs(eleveId: UUID, motif: String, debut: String, fin: String, commentaire: String) async throws {
        struct Body: Encodable {
            let motif: String
            let debut: String
            let fin: String
            let commentaire: String
        }
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/eleves/\(eleveId.uuidString)/dispenses-vs",
            body: Body(motif: motif, debut: debut, fin: fin, commentaire: commentaire),
            authed: true
        )
    }

    public func attacherPieceMessage(messageId: UUID, nom: String, mimeType: String, data: Data) async throws {
        struct Body: Encodable {
            let nom: String
            let mime_type: String
            let contenu_base64: String
        }
        struct Resp: Decodable { let id: UUID }
        let _: Resp = try await post(
            path: "/messages/\(messageId.uuidString)/pieces",
            body: Body(nom: nom, mime_type: mimeType, contenu_base64: data.base64EncodedString()),
            authed: true
        )
    }

    public func telechargerPieceMessage(pieceId: UUID) async throws -> Data {
        try await getData(path: "/messages/pieces/\(pieceId.uuidString)")
    }

    public func recapSortie(eleveId: UUID) async throws -> RecapSortie {
        try await get(path: "/eleves/\(eleveId.uuidString)/sorties")
    }

    public func upsertAutorisationSortie(
        eleveId: UUID,
        matin: Bool,
        midi: Bool,
        soir: Bool
    ) async throws -> AutorisationSortie {
        struct Body: Encodable {
            let matin: Bool
            let midi: Bool
            let soir: Bool
            let interdit_etablissement: Bool
        }
        return try await put(
            path: "/eleves/\(eleveId.uuidString)/sorties",
            body: Body(matin: matin, midi: midi, soir: soir, interdit_etablissement: false)
        )
    }

    public func demanderSortie(
        eleveId: UUID,
        jour: String,
        creneau: String,
        motif: String
    ) async throws -> DemandeSortie {
        struct Body: Encodable {
            let jour: String
            let creneau: String
            let motif: String
        }
        return try await post(
            path: "/eleves/\(eleveId.uuidString)/sorties/demandes",
            body: Body(jour: jour, creneau: creneau, motif: motif),
            authed: true
        )
    }

    public func docsASigner(eleveId: UUID) async throws -> [DocASigner] {
        try await get(path: "/eleves/\(eleveId.uuidString)/docs-a-signer?actives=true")
    }

    public func signerDoc(_ docId: UUID) async throws -> DocASigner {
        struct Empty: Encodable {}
        return try await post(path: "/docs-a-signer/\(docId.uuidString)/signer", body: Empty(), authed: true)
    }

    public func menusCantine(debut: String, fin: String) async throws -> [MenuCantine] {
        try await get(path: "/menus-cantine?debut=\(debut)&fin=\(fin)")
    }

    public func conventionsStage(eleveId: UUID) async throws -> [ConventionStage] {
        try await get(path: "/eleves/\(eleveId.uuidString)/conventions-stage")
    }

    public func journal(eleveId: UUID, limit: Int = 80) async throws -> [JournalEntree] {
        try await get(path: "/eleves/\(eleveId.uuidString)/journal?limit=\(limit)")
    }

    public func passagesInfirmerie(eleveId: UUID) async throws -> [PassageInfirmerie] {
        try await get(path: "/infirmerie/passages?eleve_id=\(eleveId.uuidString)")
    }

    public func notePeriodes() async throws -> [NotePeriode] {
        try await get(path: "/note-periodes")
    }

    public func moyennes(eleveId: UUID, periodeId: UUID) async throws -> [MoyenneMatiere] {
        try await get(path: "/eleves/\(eleveId.uuidString)/moyennes?periode_id=\(periodeId.uuidString)")
    }

    public func appreciations(eleveId: UUID, periodeId: UUID) async throws -> [Appreciation] {
        try await get(path: "/eleves/\(eleveId.uuidString)/appreciations?periode_id=\(periodeId.uuidString)")
    }

    public func photoEleve(eleveId: UUID) async throws -> Data {
        try await getData(path: "/eleves/\(eleveId.uuidString)/photo")
    }

    public func trombi(classe: String) async throws -> [TrombiEntree] {
        let enc = classe.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? classe
        try await get(path: "/classes/\(enc)/trombi")
    }

    public func edt(eleveId: UUID, jour: String) async throws -> [EdtCreneau] {
        try await get(path: "/eleves/\(eleveId.uuidString)/edt?jour=\(jour)")
    }

    public func passagesRepas(eleveId: UUID, debut: String, fin: String) async throws -> [PassageRepas] {
        try await get(path: "/eleves/\(eleveId.uuidString)/repas/passages?debut=\(debut)&fin=\(fin)")
    }

    public func facturesFamille() async throws -> [Facture] {
        try await get(path: "/facturation/famille")
    }

    public func quittancesFamille() async throws -> [Quittance] {
        try await get(path: "/facturation/famille/quittances")
    }

    public func cahierTexte(eleveId: UUID) async throws -> [CahierTexte] {
        try await get(path: "/cahier-texte?eleve_id=\(eleveId.uuidString)")
    }

    public func alertesCpe(eleveId: UUID) async throws -> [AlerteCpe] {
        try await get(path: "/eleves/\(eleveId.uuidString)/alertes-cpe")
    }

    public func internat(eleveId: UUID, jour: String) async throws -> RecapInternat {
        try await get(path: "/eleves/\(eleveId.uuidString)/internat?jour=\(jour)")
    }

    public func conseils(eleveId: UUID) async throws -> [ConseilDossier] {
        try await get(path: "/eleves/\(eleveId.uuidString)/conseils")
    }

    public func ficheSante(eleveId: UUID) async throws -> FicheSante? {
        try await get(path: "/eleves/\(eleveId.uuidString)/fiche-sante")
    }

    public func pai(eleveId: UUID) async throws -> Pai? {
        try await get(path: "/eleves/\(eleveId.uuidString)/pai")
    }

    public func signerPai(eleveId: UUID) async throws -> Pai {
        struct Empty: Encodable {}
        return try await post(
            path: "/eleves/\(eleveId.uuidString)/pai/signer",
            body: Empty(),
            authed: true
        )
    }

    public func voeux(eleveId: UUID) async throws -> [Voeu] {
        try await get(path: "/eleves/\(eleveId.uuidString)/voeux")
    }

    public func infoComp(eleveId: UUID) async throws -> [InfoCompValeur] {
        let pack: InfoCompPack = try await get(path: "/eleves/\(eleveId.uuidString)/info-comp")
        return pack.valeurs
    }

    public func dispensesRepas(eleveId: UUID) async throws -> [DispenseRepas] {
        try await get(path: "/eleves/\(eleveId.uuidString)/repas/dispenses")
    }

    public func activiteRepasMidi(eleveId: UUID) async throws -> UUID? {
        let d: DossierLite = try await get(path: "/eleves/\(eleveId.uuidString)/dossier")
        return d.grille_repas?.first?.activite_id
    }

    public func poserDispenseRepas(eleveId: UUID, activiteId: UUID, jour: String, motif: String) async throws -> DispenseRepas {
        struct Body: Encodable {
            let activite_id: UUID
            let jour: String
            let motif: String
        }
        return try await post(
            path: "/eleves/\(eleveId.uuidString)/repas/dispenses",
            body: Body(activite_id: activiteId, jour: jour, motif: motif),
            authed: true
        )
    }

    public func annulerDispenseRepas(eleveId: UUID, dispenseId: UUID) async throws -> DispenseRepas {
        struct Empty: Encodable {}
        return try await post(
            path: "/eleves/\(eleveId.uuidString)/repas/dispenses/\(dispenseId.uuidString)/annuler",
            body: Empty(),
            authed: true
        )
    }

    public struct Sanction: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let jour: String
        public let duree_minutes: Int?
        public let note: String?
        public let statut: String
    }

    public struct Incident: Decodable, Identifiable, Sendable {
        public let id: UUID
        public let jour: String
        public let description: String?
    }

    public func sanctions(eleveId: UUID) async throws -> [Sanction] {
        try await get(path: "/eleves/\(eleveId.uuidString)/sanctions")
    }

    public func incidents(eleveId: UUID) async throws -> [Incident] {
        try await get(path: "/eleves/\(eleveId.uuidString)/incidents")
    }

    public func pointsVs(eleveId: UUID) async throws -> SoldePoints {
        try await get(path: "/eleves/\(eleveId.uuidString)/points-vs")
    }

    public func commissions(eleveId: UUID) async throws -> [CommissionDossier] {
        try await get(path: "/eleves/\(eleveId.uuidString)/commissions")
    }

    public func dossierPsy(eleveId: UUID) async throws -> DossierPsy? {
        try await get(path: "/eleves/\(eleveId.uuidString)/dossier-psy")
    }

    public func rdvsPsy(eleveId: UUID) async throws -> [RdvPsy] {
        try await get(path: "/psy/rdvs?eleve_id=\(eleveId.uuidString)")
    }

    public func reinscriptionFamille() async throws -> ReinscriptionFamille {
        struct RawCampagne: Decodable { let ouverte: Bool? }
        struct RawProjection: Decodable {
            let annee_scolaire: String?
            let niveau: String?
        }
        struct RawEnfant: Decodable {
            let enfant: Enfant
            let confirmee: Bool
            let documents_manquants: [String]
            let acompte_ok: Bool
            let options: [ReinscriptionOption]
            let projection_n1: RawProjection?
        }
        struct Raw: Decodable {
            let annee_n1: String
            let campagne: RawCampagne?
            let enfants: [RawEnfant]
        }
        let raw: Raw = try await get(path: "/reinscription/famille")
        return ReinscriptionFamille(
            annee_n1: raw.annee_n1,
            campagne_ouverte: raw.campagne?.ouverte == true,
            enfants: raw.enfants.map {
                ReinscriptionEnfant(
                    enfant: $0.enfant,
                    confirmee: $0.confirmee,
                    documents_manquants: $0.documents_manquants,
                    acompte_ok: $0.acompte_ok,
                    options: $0.options,
                    projection_niveau: $0.projection_n1?.niveau,
                    projection_annee: $0.projection_n1?.annee_scolaire
                )
            }
        )
    }

    public func retenirOptionReinscription(eleveId: UUID, code: String, libelle: String) async throws {
        struct Body: Encodable {
            let code: String
            let libelle: String
            let nature: String
            let retenir: Bool
        }
        let _: ReinscriptionOption = try await post(
            path: "/reinscription/famille/\(eleveId.uuidString)/options",
            body: Body(code: code, libelle: libelle, nature: "academique", retenir: true),
            authed: true
        )
    }

    public func deposerDocReinscription(eleveId: UUID, code: String, titre: String) async throws {
        struct Body: Encodable {
            let code: String
            let titre: String
        }
        struct Doc: Decodable { let id: UUID? }
        let _: Doc = try await post(
            path: "/reinscription/famille/\(eleveId.uuidString)/documents",
            body: Body(code: code, titre: titre),
            authed: true
        )
    }

    public func confirmerReinscription(eleveId: UUID) async throws {
        struct EmptyBody: Encodable {}
        struct Ok: Decodable { let id: UUID? }
        let _: Ok = try await post(
            path: "/reinscription/famille/\(eleveId.uuidString)/confirmer",
            body: EmptyBody(),
            authed: true
        )
    }

    /// Charge la dernière période ouverte (ou la première) puis les moyennes.
    public func moyennesCourantes(eleveId: UUID) async throws -> (NotePeriode?, [MoyenneMatiere]) {
        let periodes = try await notePeriodes()
        guard let p = periodes.first(where: { $0.statut == "ouverte" || $0.statut == "ouvert" })
            ?? periodes.first
        else { return (nil, []) }
        let m = try await moyennes(eleveId: eleveId, periodeId: p.id)
        return (p, m)
    }

    public func appreciationsCourantes(eleveId: UUID) async throws -> (NotePeriode?, [Appreciation]) {
        let periodes = try await notePeriodes()
        guard let p = periodes.first(where: { $0.statut == "ouverte" || $0.statut == "ouvert" })
            ?? periodes.first
        else { return (nil, []) }
        let a = try await appreciations(eleveId: eleveId, periodeId: p.id)
        return (p, a)
    }

    private func getData(path: String) async throws -> Data {
        var req = URLRequest(url: URL(string: baseURL.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + path)!)
        req.httpMethod = "GET"
        if let token {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        let (data, resp) = try await URLSession.shared.data(for: req)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        return data
    }

    private func get<T: Decodable>(path: String) async throws -> T {
        var req = URLRequest(url: baseURL.appendingPathComponent(path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))))
        // appendingPathComponent casse les UUID path segments — reconstruit l’URL.
        req = URLRequest(url: URL(string: baseURL.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + path)!)
        req.httpMethod = "GET"
        if let token {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        let (data, resp) = try await URLSession.shared.data(for: req)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func post<B: Encodable, T: Decodable>(path: String, body: B, authed: Bool) async throws -> T {
        try await send(method: "POST", path: path, body: body, authed: authed)
    }

    private func put<B: Encodable, T: Decodable>(path: String, body: B) async throws -> T {
        try await send(method: "PUT", path: path, body: body, authed: true)
    }

    private func send<B: Encodable, T: Decodable>(
        method: String,
        path: String,
        body: B,
        authed: Bool
    ) async throws -> T {
        var req = URLRequest(url: URL(string: baseURL.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + path)!)
        req.httpMethod = method
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if authed, let token {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        req.httpBody = try JSONEncoder().encode(body)
        let (data, resp) = try await URLSession.shared.data(for: req)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        return try JSONDecoder().decode(T.self, from: data)
    }
}
