import SwiftUI
import UIKit

/// Point d’entrée app famille — marque, connexion, enfants, résumé (absences / carnet / notes).
@main
struct FamilleApp: App {
    @State private var model = FamilleSessionModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(model)
        }
    }
}

@Observable
final class FamilleSessionModel {
    var client = EntApiClient()
    var enfants: [EntApiClient.Enfant] = []
    var resume: EntApiClient.Resume?
    var sortie: EntApiClient.RecapSortie?
    var docsASigner: [EntApiClient.DocASigner] = []
    var menus: [EntApiClient.MenuCantine] = []
    var passagesRepas: [EntApiClient.PassageRepas] = []
    var factures: [EntApiClient.Facture] = []
    var quittances: [EntApiClient.Quittance] = []
    var cahier: [EntApiClient.CahierTexte] = []
    var alertes: [EntApiClient.AlerteCpe] = []
    var internat: EntApiClient.RecapInternat?
    var conseils: [EntApiClient.ConseilDossier] = []
    var ficheSante: EntApiClient.FicheSante?
    var pai: EntApiClient.Pai?
    var voeux: [EntApiClient.Voeu] = []
    var infoComp: [EntApiClient.InfoCompValeur] = []
    var dispensesRepas: [EntApiClient.DispenseRepas] = []
    var activiteRepasId: UUID?
    var pointsVs: EntApiClient.SoldePoints?
    var sanctions: [EntApiClient.Sanction] = []
    var incidents: [EntApiClient.Incident] = []
    var commissions: [EntApiClient.CommissionDossier] = []
    var dossierPsy: EntApiClient.DossierPsy?
    var rdvsPsy: [EntApiClient.RdvPsy] = []
    var reinscription: EntApiClient.ReinscriptionFamille?
    var passagesInfirmerie: [EntApiClient.PassageInfirmerie] = []
    var periodeNotes: EntApiClient.NotePeriode?
    var moyennes: [EntApiClient.MoyenneMatiere] = []
    var appreciations: [EntApiClient.Appreciation] = []
    var photoData: Data?
    var trombi: [EntApiClient.TrombiEntree] = []
    var conventions: [EntApiClient.ConventionStage] = []
    var journal: [EntApiClient.JournalEntree] = []
    var edt: [EntApiClient.EdtCreneau] = []
    var fils: [EntApiClient.MsgFil] = []
    var filOuvert: EntApiClient.FilDetail?
    var selectedEleveId: UUID?
    var error: String?
    var busy = false
    var pdfShareURL: URL?
    var paiPieces: [EntApiClient.PaiPiece] = []
    var dispensesVs: [EntApiClient.DispenseVs] = []
    var motifsAbsence: [EntApiClient.AbsenceMotif] = []

    private static func ymd(_ date: Date, offsetDays: Int = 0) -> String {
        let cal = Calendar(identifier: .gregorian)
        let d = cal.date(byAdding: .day, value: offsetDays, to: date) ?? date
        let fmt = DateFormatter()
        fmt.calendar = cal
        fmt.locale = Locale(identifier: "en_US_POSIX")
        fmt.dateFormat = "yyyy-MM-dd"
        return fmt.string(from: d)
    }

    private static func euros(_ centimes: Int64) -> String {
        let v = Double(centimes) / 100.0
        return String(format: "%.2f €", v)
    }

    func login(email: String, password: String, etablissementId: UUID) async {
        busy = true
        error = nil
        defer { busy = false }
        do {
            _ = try await client.login(email: email, password: password, etablissementId: etablissementId)
            try? await client.registerPush(
                platform: "ios",
                token: "ios-dev-\(UUID().uuidString)",
                appId: "scola.famille.ios"
            )
            enfants = try await client.enfants()
            fils = (try? await client.messages()) ?? []
            motifsAbsence = (try? await client.motifsAbsence()) ?? []
            factures = (try? await client.facturesFamille()) ?? []
            quittances = (try? await client.quittancesFamille()) ?? []
            reinscription = try? await client.reinscriptionFamille()
            menus = (try? await client.menusCantine(
                debut: Self.ymd(Date()),
                fin: Self.ymd(Date(), offsetDays: 6)
            )) ?? []
            if let first = enfants.first {
                selectedEleveId = first.id
                resume = try? await client.resume(eleveId: first.id)
                sortie = try? await client.recapSortie(eleveId: first.id)
                docsASigner = (try? await client.docsASigner(eleveId: first.id)) ?? []
                conventions = (try? await client.conventionsStage(eleveId: first.id)) ?? []
                journal = (try? await client.journal(eleveId: first.id)) ?? []
                edt = (try? await client.edt(eleveId: first.id, jour: Self.ymd(Date()))) ?? []
                passagesRepas = (try? await client.passagesRepas(
                    eleveId: first.id,
                    debut: Self.ymd(Date(), offsetDays: -14),
                    fin: Self.ymd(Date())
                )) ?? []
                cahier = (try? await client.cahierTexte(eleveId: first.id)) ?? []
                alertes = (try? await client.alertesCpe(eleveId: first.id)) ?? []
                internat = try? await client.internat(eleveId: first.id, jour: Self.ymd(Date()))
                conseils = (try? await client.conseils(eleveId: first.id)) ?? []
                ficheSante = try? await client.ficheSante(eleveId: first.id)
                paiPieces = (try? await client.paiPieces(eleveId: first.id)) ?? []
                dispensesVs = (try? await client.dispensesVs(eleveId: first.id)) ?? []
                pai = try? await client.pai(eleveId: first.id)
                voeux = (try? await client.voeux(eleveId: first.id)) ?? []
                infoComp = (try? await client.infoComp(eleveId: first.id)) ?? []
                dispensesRepas = (try? await client.dispensesRepas(eleveId: first.id)) ?? []
                activiteRepasId = try? await client.activiteRepasMidi(eleveId: first.id)
                pointsVs = try? await client.pointsVs(eleveId: first.id)
                sanctions = (try? await client.sanctions(eleveId: first.id)) ?? []
                incidents = (try? await client.incidents(eleveId: first.id)) ?? []
                commissions = ((try? await client.commissions(eleveId: first.id)) ?? []).filter(\.signale_famille)
                dossierPsy = try? await client.dossierPsy(eleveId: first.id)
                rdvsPsy = ((try? await client.rdvsPsy(eleveId: first.id)) ?? []).filter(\.signale_famille)
                passagesInfirmerie = (try? await client.passagesInfirmerie(eleveId: first.id)) ?? []
                if let (per, moy) = try? await client.moyennesCourantes(eleveId: first.id) {
                    periodeNotes = per
                    moyennes = moy
                }
            }
        } catch {
            self.error = error.localizedDescription
        }
    }

    func selectEleve(_ id: UUID) async {
        selectedEleveId = id
        resume = try? await client.resume(eleveId: id)
        sortie = try? await client.recapSortie(eleveId: id)
        docsASigner = (try? await client.docsASigner(eleveId: id)) ?? []
        conventions = (try? await client.conventionsStage(eleveId: id)) ?? []
        journal = (try? await client.journal(eleveId: id)) ?? []
        edt = (try? await client.edt(eleveId: id, jour: Self.ymd(Date()))) ?? []
        passagesRepas = (try? await client.passagesRepas(
            eleveId: id,
            debut: Self.ymd(Date(), offsetDays: -14),
            fin: Self.ymd(Date())
        )) ?? []
        cahier = (try? await client.cahierTexte(eleveId: id)) ?? []
        alertes = (try? await client.alertesCpe(eleveId: id)) ?? []
        internat = try? await client.internat(eleveId: id, jour: Self.ymd(Date()))
        conseils = (try? await client.conseils(eleveId: id)) ?? []
        ficheSante = try? await client.ficheSante(eleveId: id)
        pai = try? await client.pai(eleveId: id)
        paiPieces = (try? await client.paiPieces(eleveId: id)) ?? []
        dispensesVs = (try? await client.dispensesVs(eleveId: id)) ?? []
        voeux = (try? await client.voeux(eleveId: id)) ?? []
        infoComp = (try? await client.infoComp(eleveId: id)) ?? []
        dispensesRepas = (try? await client.dispensesRepas(eleveId: id)) ?? []
        activiteRepasId = try? await client.activiteRepasMidi(eleveId: id)
        pointsVs = try? await client.pointsVs(eleveId: id)
        sanctions = (try? await client.sanctions(eleveId: id)) ?? []
        incidents = (try? await client.incidents(eleveId: id)) ?? []
        commissions = ((try? await client.commissions(eleveId: id)) ?? []).filter(\.signale_famille)
        dossierPsy = try? await client.dossierPsy(eleveId: id)
        rdvsPsy = ((try? await client.rdvsPsy(eleveId: id)) ?? []).filter(\.signale_famille)
        passagesInfirmerie = (try? await client.passagesInfirmerie(eleveId: id)) ?? []
        if let (per, moy) = try? await client.moyennesCourantes(eleveId: id) {
            periodeNotes = per
            moyennes = moy
        } else {
            periodeNotes = nil
            moyennes = []
        }
        if let (_, apps) = try? await client.appreciationsCourantes(eleveId: id) {
            appreciations = apps
        } else {
            appreciations = []
        }
        photoData = try? await client.photoEleve(eleveId: id)
        if let classe = enfants.first(where: { $0.id == id })?.classe, !classe.isEmpty {
            trombi = (try? await client.trombi(classe: classe)) ?? []
        } else {
            trombi = []
        }
        menus = (try? await client.menusCantine(
            debut: Self.ymd(Date()),
            fin: Self.ymd(Date(), offsetDays: 6)
        )) ?? []
    }

    func signerDoc(_ docId: UUID) async {
        do {
            _ = try await client.signerDoc(docId)
            if let id = selectedEleveId {
                docsASigner = try await client.docsASigner(eleveId: id)
            }
        } catch {
            self.error = error.localizedDescription
        }
    }

    func signerPai() async {
        guard let id = selectedEleveId else { return }
        do {
            pai = try await client.signerPai(eleveId: id)
        } catch {
            self.error = error.localizedDescription
        }
    }

    func prevenirCantine(motif: String = "sortie scolaire") async {
        guard let id = selectedEleveId, let act = activiteRepasId else { return }
        do {
            _ = try await client.poserDispenseRepas(
                eleveId: id,
                activiteId: act,
                jour: Self.ymd(Date()),
                motif: motif
            )
            dispensesRepas = try await client.dispensesRepas(eleveId: id)
        } catch {
            self.error = error.localizedDescription
        }
    }

    func annulerDispense(_ dispenseId: UUID) async {
        guard let id = selectedEleveId else { return }
        do {
            _ = try await client.annulerDispenseRepas(eleveId: id, dispenseId: dispenseId)
            dispensesRepas = try await client.dispensesRepas(eleveId: id)
        } catch {
            self.error = error.localizedDescription
        }
    }

    func refreshReinscription() async {
        reinscription = try? await client.reinscriptionFamille()
    }

    func retenirOption(_ eleveId: UUID, code: String, libelle: String) async {
        do {
            try await client.retenirOptionReinscription(eleveId: eleveId, code: code, libelle: libelle)
            await refreshReinscription()
        } catch {
            self.error = error.localizedDescription
        }
    }

    func deposerDocReins(_ eleveId: UUID, code: String) async {
        do {
            try await client.deposerDocReinscription(eleveId: eleveId, code: code, titre: code)
            await refreshReinscription()
        } catch {
            self.error = error.localizedDescription
        }
    }

    func confirmerReins(_ eleveId: UUID) async {
        do {
            try await client.confirmerReinscription(eleveId: eleveId)
            await refreshReinscription()
        } catch {
            self.error = error.localizedDescription
        }
    }

    func ouvrirFil(_ id: UUID) async {
        do {
            filOuvert = try await client.filDetail(filId: id)
        } catch {
            self.error = error.localizedDescription
        }
    }

    func repondreFil(_ id: UUID, corps: String) async {
        do {
            try await client.repondre(filId: id, corps: corps)
            filOuvert = try await client.filDetail(filId: id)
            fils = (try? await client.messages()) ?? fils
        } catch {
            self.error = error.localizedDescription
        }
    }

    func ouvrirPdfFacture(_ id: UUID, numero: String) async {
        do {
            let data = try await client.pdfFacture(factureId: id)
            let url = FileManager.default.temporaryDirectory.appendingPathComponent("\(numero).pdf")
            try data.write(to: url, options: .atomic)
            pdfShareURL = url
        } catch {
            self.error = error.localizedDescription
        }
    }

    func ouvrirPdfQuittance(_ id: UUID, numero: String) async {
        do {
            let data = try await client.pdfQuittance(quittanceId: id)
            let url = FileManager.default.temporaryDirectory.appendingPathComponent("\(numero).pdf")
            try data.write(to: url, options: .atomic)
            pdfShareURL = url
        } catch {
            self.error = error.localizedDescription
        }
    }

    func ouvrirPdfFicheSante() async {
        guard let eid = selectedEleveId else { return }
        do {
            let data = try await client.pdfFicheSante(eleveId: eid)
            let url = FileManager.default.temporaryDirectory.appendingPathComponent("fiche-sante.pdf")
            try data.write(to: url, options: .atomic)
            pdfShareURL = url
            paiPieces = (try? await client.paiPieces(eleveId: eid)) ?? []
        } catch {
            self.error = error.localizedDescription
        }
    }

    func ouvrirPdfCourrierPsy() async {
        guard let eid = selectedEleveId else { return }
        do {
            let data = try await client.pdfCourrierPsy(eleveId: eid)
            let url = FileManager.default.temporaryDirectory.appendingPathComponent("courrier-psy.pdf")
            try data.write(to: url, options: .atomic)
            pdfShareURL = url
        } catch {
            self.error = error.localizedDescription
        }
    }

    func ouvrirPdfPaiPiece(_ id: UUID, nom: String) async {
        do {
            let data = try await client.pdfPaiPiece(pieceId: id)
            let safe = nom.replacingOccurrences(of: "/", with: "-")
            let url = FileManager.default.temporaryDirectory.appendingPathComponent(safe.isEmpty ? "ordonnance.pdf" : safe)
            try data.write(to: url, options: .atomic)
            pdfShareURL = url
        } catch {
            self.error = error.localizedDescription
        }
    }

    func deposerOrdonnanceDemo() async {
        guard let eid = selectedEleveId else { return }
        // Mini PDF « %PDF-1.1 » minimal pour geste démo.
        let pdf = Data("%PDF-1.1\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n".utf8)
        do {
            try await client.deposerPaiPiece(
                eleveId: eid,
                nom: "ordonnance-demo.pdf",
                mimeType: "application/pdf",
                data: pdf
            )
            paiPieces = (try? await client.paiPieces(eleveId: eid)) ?? []
        } catch {
            self.error = error.localizedDescription
        }
    }

    func poserDispenseEps() async {
        guard let eid = selectedEleveId else { return }
        let jour = Self.ymd(Date())
        do {
            try await client.poserDispenseVs(
                eleveId: eid,
                motif: "eps",
                debut: jour,
                fin: jour,
                commentaire: "blessure légère"
            )
            dispensesVs = (try? await client.dispensesVs(eleveId: eid)) ?? []
        } catch {
            self.error = error.localizedDescription
        }
    }

    func joindrePieceDemo(messageId: UUID) async {
        let data = Data("pièce jointe démo".utf8)
        do {
            try await client.attacherPieceMessage(
                messageId: messageId,
                nom: "note.txt",
                mimeType: "text/plain",
                data: data
            )
            if let fil = filOuvert {
                filOuvert = try await client.filDetail(filId: fil.fil.id)
            }
        } catch {
            self.error = error.localizedDescription
        }
    }

    func telechargerPiece(pieceId: UUID, nom: String) async {
        do {
            let data = try await client.telechargerPieceMessage(pieceId: pieceId)
            self.error = "PJ \(nom) : \(data.count) octets"
        } catch {
            self.error = error.localizedDescription
        }
    }

    func setSortie(midi: Bool, soir: Bool) async {
        guard let id = selectedEleveId else { return }
        do {
            _ = try await client.upsertAutorisationSortie(
                eleveId: id,
                matin: false,
                midi: midi,
                soir: soir
            )
            sortie = try await client.recapSortie(eleveId: id)
        } catch {
            self.error = error.localizedDescription
        }
    }

    func demanderSortieExceptionnelle(motif: String) async {
        guard let id = selectedEleveId else { return }
        let fmt = DateFormatter()
        fmt.calendar = Calendar(identifier: .gregorian)
        fmt.locale = Locale(identifier: "en_US_POSIX")
        fmt.dateFormat = "yyyy-MM-dd"
        let jour = fmt.string(from: Date())
        do {
            _ = try await client.demanderSortie(
                eleveId: id,
                jour: jour,
                creneau: "matin",
                motif: motif
            )
            sortie = try await client.recapSortie(eleveId: id)
        } catch {
            self.error = error.localizedDescription
        }
    }

    func declarerAbsenceAujourdhui(motif: String) async {
        let jour = Self.ymd(Date())
        await declarerAbsence(debut: jour, fin: jour, motif: motif)
    }

    func declarerAbsence(debut: String, fin: String, motif: String) async {
        guard let eid = selectedEleveId else { return }
        do {
            try await client.declarerAbsence(eleveId: eid, debut: debut, fin: fin, motif: motif)
            if let r = try? await client.resume(eleveId: eid) { resume = r }
        } catch {
            self.error = error.localizedDescription
        }
    }

    func justifier(_ absenceId: UUID, motif: String) async {
        do {
            try await client.justifierAbsence(absenceId: absenceId, motif: motif)
            if let id = selectedEleveId {
                resume = try await client.resume(eleveId: id)
            }
        } catch {
            self.error = error.localizedDescription
        }
    }

    func signer(_ entreeId: UUID) async {
        do {
            try await client.signerCarnet(entreeId: entreeId)
            if let id = selectedEleveId {
                resume = try await client.resume(eleveId: id)
            }
        } catch {
            self.error = error.localizedDescription
        }
    }
}

struct ContentView: View {
    @Environment(FamilleSessionModel.self) private var model
    @State private var email = ""
    @State private var password = ""
    @State private var etab = ""

    var body: some View {
        NavigationStack {
            if model.enfants.isEmpty {
                Form {
                    Section("Connexion famille") {
                        TextField("E-mail", text: $email)
                            .textContentType(.username)
                            .textInputAutocapitalization(.never)
                        SecureField("Mot de passe", text: $password)
                        TextField("Établissement (UUID)", text: $etab)
                            .textInputAutocapitalization(.never)
                    }
                    if let error = model.error {
                        Text(error).foregroundStyle(.red)
                    }
                    Button("Se connecter") {
                        guard let id = UUID(uuidString: etab) else {
                            model.error = "UUID établissement invalide"
                            return
                        }
                        Task { await model.login(email: email, password: password, etablissementId: id) }
                    }
                    .disabled(model.busy)
                }
                .navigationTitle("ScolIA")
            } else {
                List {
                    Section("Mes enfants") {
                        ForEach(model.enfants) { e in
                            Button {
                                Task { await model.selectEleve(e.id) }
                            } label: {
                                HStack {
                                    VStack(alignment: .leading, spacing: 4) {
                                        Text("\(e.prenom) \(e.nom)").font(.headline)
                                        Text(e.classe ?? "—").foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                    if model.selectedEleveId == e.id {
                                        Image(systemName: "checkmark.circle.fill")
                                    }
                                }
                            }
                        }
                    }
                    if let resume = model.resume {
                        Section("Prévenir une absence") {
                            let motifs = model.motifsAbsence.isEmpty
                                ? [EntApiClient.AbsenceMotif(code: "maladie", libelle: "Maladie")]
                                : model.motifsAbsence
                            ForEach(motifs) { m in
                                Button("Aujourd’hui — \(m.libelle)") {
                                    Task { await model.declarerAbsenceAujourdhui(motif: m.code) }
                                }
                            }
                        }
                        Section("Absences à justifier") {
                            if resume.absences_a_justifier.isEmpty {
                                Text("Aucune").foregroundStyle(.secondary)
                            } else {
                                let motifsJustif = model.motifsAbsence.isEmpty
                                    ? [EntApiClient.AbsenceMotif(code: "maladie", libelle: "Maladie")]
                                    : model.motifsAbsence
                                ForEach(resume.absences_a_justifier) { a in
                                    HStack {
                                        VStack(alignment: .leading) {
                                            Text("\(a.debut) → \(a.fin)")
                                            Text(a.motif ?? a.statut).font(.caption).foregroundStyle(.secondary)
                                        }
                                        Spacer()
                                        Menu("Justifier") {
                                            ForEach(motifsJustif) { m in
                                                Button(m.libelle) {
                                                    Task { await model.justifier(a.id, motif: m.code) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Section("Carnet à signer") {
                            if resume.carnet_a_signer.isEmpty {
                                Text("Rien à signer").foregroundStyle(.secondary)
                            } else {
                                ForEach(resume.carnet_a_signer) { c in
                                    HStack {
                                        Text(c.titre ?? c.corps ?? "Entrée")
                                        Spacer()
                                        Button("Signer") {
                                            Task { await model.signer(c.id) }
                                        }
                                    }
                                }
                            }
                        }
                        if !resume.notes.isEmpty {
                            Section("Notes") {
                                ForEach(resume.notes) { n in
                                    Text("\(n.matiere_libelle ?? "Matière") · \(n.libelle ?? "Note") — \(n.valeur.map { String($0) } ?? "—")")
                                }
                            }
                        }
                    }
                    Section("Documents à signer") {
                        if model.docsASigner.isEmpty {
                            Text("Aucun").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.docsASigner) { d in
                                HStack {
                                    Text(d.titre)
                                    Spacer()
                                    Button("Signer") {
                                        Task { await model.signerDoc(d.id) }
                                    }
                                }
                            }
                        }
                    }
                    Section("Menus cantine") {
                        if model.menus.isEmpty {
                            Text("Aucun menu publié").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.menus) { m in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(m.jour).font(.caption).foregroundStyle(.secondary)
                                    Text(m.plat).font(.headline)
                                    if !m.entree.isEmpty { Text("Entrée: \(m.entree)").font(.caption) }
                                    if !m.dessert.isEmpty { Text("Dessert: \(m.dessert)").font(.caption) }
                                }
                            }
                        }
                    }
                    Section("Passages badge") {
                        if model.passagesRepas.isEmpty {
                            Text("Aucun passage récent").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.passagesRepas) { p in
                                HStack {
                                    Text(p.jour)
                                    Spacer()
                                    Text(p.source).font(.caption).foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    if model.activiteRepasId != nil {
                        Section("Prévenance cantine") {
                            Button("Dispenser aujourd’hui") {
                                Task { await model.prevenirCantine() }
                            }
                            ForEach(model.dispensesRepas.filter { !$0.annulee }) { d in
                                HStack {
                                    Text("\(d.jour) — \(d.motif)")
                                    Spacer()
                                    Button("Annuler") {
                                        Task { await model.annulerDispense(d.id) }
                                    }
                                }
                            }
                        }
                    }
                    if let pts = model.pointsVs {
                        Section("Permis à points") {
                            Text("Solde \(pts.solde) / \(pts.depart)")
                        }
                    }
                    if !model.sanctions.isEmpty {
                        Section("Sanctions") {
                            ForEach(model.sanctions) { s in
                                Text("\(s.jour) · \(s.statut)\(s.duree_minutes.map { " · \($0) min" } ?? "")\(s.note.map { " — \($0)" } ?? "")")
                                    .font(.caption)
                            }
                        }
                    }
                    if !model.incidents.isEmpty {
                        Section("Incidents") {
                            ForEach(model.incidents) { i in
                                Text("\(i.jour)\(i.description.map { " — \($0)" } ?? "")")
                                    .font(.caption)
                            }
                        }
                    }
                    if !model.commissions.isEmpty {
                        Section("Commissions VS") {
                            ForEach(model.commissions) { c in
                                Text("\(c.motifs) — \(c.decision_libelle ?? "en attente")")
                                    .font(.caption)
                            }
                        }
                    }
                    if let psy = model.dossierPsy, psy.signale_famille {
                        Section("Suivi psy") {
                            Text(psy.suivi_actif ? "Suivi actif" : "Suivi inactif")
                            if !psy.axes_suivi.isEmpty {
                                Text(psy.axes_suivi).font(.caption)
                            }
                            Button("PDF courrier psy") { Task { await model.ouvrirPdfCourrierPsy() } }
                                .font(.caption)
                            ForEach(model.rdvsPsy) { r in
                                Text("\(r.jour) \(r.heure_debut)–\(r.heure_fin) · \(r.kind_libelle ?? r.kind)")
                                    .font(.caption)
                            }
                        }
                    } else if !model.rdvsPsy.isEmpty {
                        Section("RDV psy") {
                            ForEach(model.rdvsPsy) { r in
                                Text("\(r.jour) \(r.heure_debut)–\(r.heure_fin) · \(r.kind_libelle ?? r.kind)")
                                    .font(.caption)
                            }
                        }
                    }
                    if let reins = model.reinscription {
                        Section("Réinscription \(reins.annee_n1)") {
                            if !reins.campagne_ouverte {
                                Text("Campagne non ouverte").foregroundStyle(.secondary)
                            } else {
                                ForEach(reins.enfants) { row in
                                    VStack(alignment: .leading, spacing: 4) {
                                        Text("\(row.enfant.prenom) \(row.enfant.nom)")
                                            .font(.headline)
                                        if let annee = row.projection_annee {
                                            Text("Projection \(annee) · \(row.projection_niveau ?? "—")")
                                                .font(.caption)
                                        }
                                        if row.confirmee {
                                            Text("Confirmée").foregroundStyle(.secondary)
                                        } else {
                                            ForEach(row.options.filter { $0.statut != "retenue" }) { o in
                                                Button("Retenir \(o.libelle)") {
                                                    Task { await model.retenirOption(row.enfant.id, code: o.code, libelle: o.libelle) }
                                                }
                                            }
                                            ForEach(row.documents_manquants, id: \.self) { code in
                                                Button("Déposer \(code)") {
                                                    Task { await model.deposerDocReins(row.enfant.id, code: code) }
                                                }
                                            }
                                            if row.documents_manquants.isEmpty && row.acompte_ok {
                                                Button("Confirmer") {
                                                    Task { await model.confirmerReins(row.enfant.id) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Section("Factures") {
                        if model.factures.isEmpty {
                            Text("Aucune facture").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.factures) { f in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text("\(f.numero) — \(f.libelle)")
                                    Text("\(Self.euros(f.total_centimes)) · \(f.statut)")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                    Button("PDF") {
                                        Task { await model.ouvrirPdfFacture(f.id, numero: f.numero) }
                                    }
                                    .font(.caption)
                                }
                            }
                        }
                    }
                    Section("Quittances") {
                        if model.quittances.isEmpty {
                            Text("Aucune quittance").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.quittances) { q in
                                HStack {
                                    Text(q.numero)
                                    Spacer()
                                    Text(Self.euros(q.montant_centimes))
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                    Button("PDF") {
                                        Task { await model.ouvrirPdfQuittance(q.id, numero: q.numero) }
                                    }
                                    .font(.caption)
                                }
                            }
                        }
                    }
                    Section("Cahier de texte") {
                        if model.cahier.isEmpty {
                            Text("Aucune consigne").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.cahier) { c in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text("\(c.matiere_libelle) — \(c.titre)")
                                    Text("Pour le \(c.pour_le)").font(.caption).foregroundStyle(.secondary)
                                    if !c.consigne.isEmpty {
                                        Text(c.consigne).font(.caption)
                                    }
                                }
                            }
                        }
                    }
                    Section("Alertes CPE") {
                        if model.alertes.isEmpty {
                            Text("Aucune alerte").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.alertes) { a in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(a.titre)
                                    Text("\(a.jour) · \(a.severite) · \(a.statut)")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    if let internat = model.internat, internat.chambre != nil || !(internat.suivis.isEmpty) {
                        Section("Internat") {
                            if let ch = internat.chambre {
                                Text("Chambre \(ch.code) — bât. \(ch.batiment) ét. \(ch.etage)")
                            }
                            if let p = internat.presence_du_jour {
                                Text("Présence \(p.jour) : \(p.statut)").font(.caption)
                            }
                            ForEach(internat.suivis) { s in
                                Text("\(s.jour) — \(s.texte)").font(.caption)
                            }
                        }
                    }
                    Section("Conseil de classe") {
                        if model.conseils.isEmpty {
                            Text("Aucune décision").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.conseils) { c in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(c.mention ?? "Décision")
                                    Text(c.appreciation).font(.caption).foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    Section("Passages infirmière") {
                        if model.passagesInfirmerie.isEmpty {
                            Text("Aucun passage visible").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.passagesInfirmerie) { p in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text("\(p.jour) — \(p.motif_libelle)")
                                    if !p.note_soins.isEmpty {
                                        Text(p.note_soins).font(.caption).foregroundStyle(.secondary)
                                    }
                                }
                            }
                        }
                    }
                    if let data = model.photoData, let ui = UIImage(data: data) {
                        Section("Photo") {
                            Image(uiImage: ui)
                                .resizable()
                                .scaledToFill()
                                .frame(width: 96, height: 96)
                                .clipped()
                        }
                    }
                    if !model.trombi.isEmpty {
                        Section("Trombi classe") {
                            ForEach(model.trombi) { t in
                                Text("\(t.prenom) \(t.nom)")
                                    .font(.caption)
                            }
                        }
                    }
                    if !model.moyennes.isEmpty {
                        Section("Moyennes\(model.periodeNotes.map { " — \($0.libelle)" } ?? "")") {
                            ForEach(Array(model.moyennes.enumerated()), id: \.offset) { _, m in
                                Text("\(m.matiere_libelle ?? "Matière") — " + String(format: "%.1f / 20", m.moyenne_sur_20))
                            }
                        }
                    }
                    if !model.appreciations.isEmpty {
                        Section("Appréciations\(model.periodeNotes.map { " — \($0.libelle)" } ?? "")") {
                            ForEach(model.appreciations) { a in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(a.matiere_libelle ?? (a.matiere_id == nil ? "Générale" : "Matière"))
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                    if let auteur = a.auteur_label {
                                        Text(auteur).font(.caption2).foregroundStyle(.tertiary)
                                    }
                                    Text(a.texte)
                                }
                            }
                        }
                    }
                    if let fs = model.ficheSante {
                        Section("Fiche santé") {
                            Text("Allergies: \(fs.allergies.isEmpty ? "—" : fs.allergies)")
                            Text("Traitements: \(fs.traitements.isEmpty ? "—" : fs.traitements)")
                            if fs.pai_actif { Text("PAI actif").foregroundStyle(.orange) }
                            Button("PDF fiche santé") { Task { await model.ouvrirPdfFicheSante() } }
                                .font(.caption)
                        }
                    }
                    if let pai = model.pai {
                        Section("PAI") {
                            Text(pai.motif)
                            Text(pai.mesures_accueil).font(.caption)
                            if pai.signe_famille {
                                Text("Signé").foregroundStyle(.secondary)
                            } else {
                                Button("Signer le PAI") { Task { await model.signerPai() } }
                            }
                            Button("Joindre ordonnance démo") {
                                Task { await model.deposerOrdonnanceDemo() }
                            }
                            .font(.caption)
                            ForEach(model.paiPieces) { p in
                                Button("Ordonnance \(p.nom)") {
                                    Task { await model.ouvrirPdfPaiPiece(p.id, nom: p.nom) }
                                }
                                .font(.caption)
                            }
                        }
                    }
                    Section("Dispenses VS (EPS)") {
                        Button("Déclarer EPS aujourd’hui") {
                            Task { await model.poserDispenseEps() }
                        }
                        if model.dispensesVs.isEmpty {
                            Text("Aucune").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.dispensesVs) { d in
                                Text("\(d.motif) · \(d.debut) → \(d.fin) · \(d.statut)")
                                    .font(.caption)
                            }
                        }
                    }
                    Section("Vœux") {
                        if model.voeux.isEmpty {
                            Text("Aucun vœu").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.voeux) { v in
                                Text("Rang \(v.rang) — \(v.filiere_label ?? "Filière") · \(v.annee_scolaire)")
                            }
                        }
                    }
                    if !model.infoComp.isEmpty {
                        Section("Infos complémentaires") {
                            ForEach(Array(model.infoComp.enumerated()), id: \.offset) { _, v in
                                Text("\(v.champ_label ?? "Champ") · \(v.valeur)")
                            }
                        }
                    }
                    Section("Stages") {
                        if model.conventions.isEmpty {
                            Text("Aucune convention").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.conventions) { c in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(c.entreprise_label ?? "Stage")
                                    Text("\(c.debut) → \(c.fin)").font(.caption).foregroundStyle(.secondary)
                                    Text("Tuteur: \(c.tuteur_nom)")
                                    Text(c.statut).font(.caption)
                                }
                            }
                        }
                    }
                    Section("Journal") {
                        if model.journal.isEmpty {
                            Text("Aucune entrée visible").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.journal) { j in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(j.libelle)
                                    Text("\(j.domaine) · \(j.action)\(j.created_at.map { " · \($0.prefix(16))" } ?? "")")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    Section("Emploi du temps") {
                        if model.edt.isEmpty {
                            Text("Aucun créneau aujourd’hui").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.edt) { c in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text("\(c.heure_debut)–\(c.heure_fin) · \(c.matiere_libelle)\(c.prof_label.map { " · \($0)" } ?? "")")
                                    Text("\(c.classe) · \(c.salle ?? "—")")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    if let sortie = model.sortie {
                        Section("Autorisations de sortie") {
                            if sortie.autorisation?.interdit_etablissement == true {
                                Text("Sortie interdite par l’établissement")
                                    .foregroundStyle(.red)
                            } else {
                                let a = sortie.autorisation
                                Toggle(
                                    "Midi",
                                    isOn: Binding(
                                        get: { a?.midi ?? false },
                                        set: { v in Task { await model.setSortie(midi: v, soir: a?.soir ?? false) } }
                                    )
                                )
                                Toggle(
                                    "Soir",
                                    isOn: Binding(
                                        get: { a?.soir ?? false },
                                        set: { v in Task { await model.setSortie(midi: a?.midi ?? false, soir: v) } }
                                    )
                                )
                                Button("Demander sortie exceptionnelle (matin)") {
                                    Task { await model.demanderSortieExceptionnelle(motif: "RDV médical") }
                                }
                            }
                            if !sortie.demandes_ouvertes.isEmpty {
                                ForEach(sortie.demandes_ouvertes) { d in
                                    Text("\(d.jour) \(d.creneau) — \(d.statut)")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    Section("Messagerie (\(model.fils.count))") {
                        ForEach(model.fils) { f in
                            Button {
                                Task { await model.ouvrirFil(f.id) }
                            } label: {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(f.sujet).font(.headline)
                                    if let at = f.last_message_at {
                                        Text(at).font(.caption).foregroundStyle(.secondary)
                                    }
                                }
                            }
                        }
                        if let detail = model.filOuvert {
                            Text(detail.fil.sujet).font(.subheadline)
                            ForEach(detail.messages) { m in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(m.auteur_label ?? "Correspondant")
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                    Text(m.corps).font(.caption)
                                    if let pcs = m.pieces, !pcs.isEmpty {
                                        ForEach(pcs) { p in
                                            Button("📎 \(p.nom)") {
                                                Task { await model.telechargerPiece(pieceId: p.id, nom: p.nom) }
                                            }
                                            .font(.caption2)
                                        }
                                    }
                                    Button("Joindre note.txt") {
                                        Task { await model.joindrePieceDemo(messageId: m.id) }
                                    }
                                    .font(.caption2)
                                }
                            }
                            Button("Répondre « Bien reçu »") {
                                Task { await model.repondreFil(detail.fil.id, corps: "Bien reçu, merci.") }
                            }
                        }
                    }
                }
                .navigationTitle("ScolIA Famille")
                .sheet(item: Binding(
                    get: { model.pdfShareURL.map(IdentifiableURL.init) },
                    set: { model.pdfShareURL = $0?.url }
                )) { item in
                    ShareLink(item: item.url) {
                        Text("Partager le PDF")
                    }
                    .padding()
                }
            }
        }
    }
}

private struct IdentifiableURL: Identifiable {
    let url: URL
    var id: String { url.absoluteString }
}
