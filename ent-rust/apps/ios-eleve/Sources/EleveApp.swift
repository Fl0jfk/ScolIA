import SwiftUI
import UIKit

/// Point d’entrée app élève — notes, cahier, messagerie, EDT, absences.
@main
struct EleveApp: App {
    @State private var model = EleveSessionModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(model)
        }
    }
}

@Observable
final class EleveSessionModel {
    var client = EntApiClient()
    var notes: [EntApiClient.Note] = []
    var devoirs: [EntApiClient.Devoir] = []
    var cahier: [EntApiClient.CahierTexte] = []
    var fils: [EntApiClient.MsgFil] = []
    var filOuvert: EntApiClient.FilDetail?
    var edt: [EntApiClient.EdtCreneau] = []
    var absences: [EntApiClient.Absence] = []
    var journal: [EntApiClient.JournalEntree] = []
    var periode: EntApiClient.NotePeriode?
    var moyennes: [EntApiClient.MoyenneMatiere] = []
    var appreciations: [EntApiClient.Appreciation] = []
    var photoData: Data?
    var email = ""
    var error: String?
    var busy = false
    var connected = false

    private static func ymd(_ date: Date = Date()) -> String {
        let fmt = DateFormatter()
        fmt.calendar = Calendar(identifier: .gregorian)
        fmt.locale = Locale(identifier: "en_US_POSIX")
        fmt.dateFormat = "yyyy-MM-dd"
        return fmt.string(from: date)
    }

    func login(email: String, password: String, etablissementId: UUID) async {
        busy = true
        error = nil
        defer { busy = false }
        do {
            let s = try await client.login(email: email, password: password, etablissementId: etablissementId)
            guard s.role == "eleve", let eid = client.eleveId else {
                self.error = "Ce compte n’est pas une face élève"
                return
            }
            self.email = s.email
            try? await client.registerPush(platform: "ios", token: "ios-eleve-\(UUID().uuidString)", appId: "scola.eleve.ios")
            notes = (try? await client.notes(eleveId: eid)) ?? []
            if let (p, d) = try? await client.devoirsCourants(eleveId: eid) {
                if periode == nil { periode = p }
                devoirs = d
            }
            cahier = (try? await client.cahierTexte()) ?? []
            fils = (try? await client.messages()) ?? []
            edt = (try? await client.edt(eleveId: eid, jour: Self.ymd())) ?? []
            absences = (try? await client.absences(eleveId: eid)) ?? []
            journal = (try? await client.journal(eleveId: eid)) ?? []
            if let (p, m) = try? await client.moyennesCourantes(eleveId: eid) {
                periode = p
                moyennes = m
            }
            if let (_, apps) = try? await client.appreciationsCourantes(eleveId: eid) {
                appreciations = apps
            }
            photoData = try? await client.photoEleve(eleveId: eid)
            connected = true
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
}

struct ContentView: View {
    @Environment(EleveSessionModel.self) private var model
    @State private var email = "harry@ent.lab"
    @State private var password = "DevLocalPass1!"
    @State private var etab = ""

    var body: some View {
        NavigationStack {
            if !model.connected {
                Form {
                    Section("Connexion élève") {
                        TextField("E-mail", text: $email)
                        SecureField("Mot de passe", text: $password)
                        TextField("Établissement UUID", text: $etab)
                        Button("Entrer") {
                            Task {
                                guard let id = UUID(uuidString: etab) else {
                                    model.error = "UUID établissement invalide"
                                    return
                                }
                                await model.login(email: email, password: password, etablissementId: id)
                            }
                        }
                        .disabled(model.busy)
                    }
                    if let err = model.error {
                        Text(err).foregroundStyle(.red)
                    }
                }
                .navigationTitle("ScolIA Élève")
            } else {
                List {
                    Section("Notes") {
                        if model.notes.isEmpty {
                            Text("Aucune note").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.notes) { n in
                                Text("\(n.matiere_libelle ?? "Matière") · \(n.libelle ?? "Note") — \(n.valeur.map { String(format: "%.1f" , $0) } ?? "—")/\(n.sur.map { String(format: "%.0f", $0) } ?? "20")")
                            }
                        }
                    }
                    if !model.devoirs.isEmpty {
                        Section("Devoirs\(model.periode.map { " — \($0.libelle)" } ?? "")") {
                            ForEach(model.devoirs) { d in
                                Text("\(d.matiere_libelle.map { "\($0) · " } ?? "")\(d.titre) · \(d.date_devoir) · coef \(String(format: "%.1f", d.coef)) / \(String(format: "%.0f", d.sur))")
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
                    if !model.moyennes.isEmpty {
                        Section("Moyennes\(model.periode.map { " — \($0.libelle)" } ?? "")") {
                            ForEach(Array(model.moyennes.enumerated()), id: \.offset) { _, m in
                                Text("\(m.matiere_libelle ?? "Matière") — " + String(format: "%.1f / 20", m.moyenne_sur_20))
                            }
                        }
                    }
                    if !model.appreciations.isEmpty {
                        Section("Appréciations\(model.periode.map { " — \($0.libelle)" } ?? "")") {
                            ForEach(model.appreciations) { a in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(a.matiere_libelle ?? (a.matiere_id == nil ? "Générale" : "Matière"))
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                    Text(a.texte)
                                }
                            }
                        }
                    }
                    Section("Cahier de texte") {
                        if model.cahier.isEmpty {
                            Text("Aucune consigne").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.cahier) { c in
                                VStack(alignment: .leading) {
                                    Text("\(c.matiere_libelle) — \(c.titre)")
                                    Text("Pour le \(c.pour_le)").font(.caption).foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    Section("Emploi du temps") {
                        if model.edt.isEmpty {
                            Text("Aucun créneau").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.edt) { c in
                                Text("\(c.heure_debut)–\(c.heure_fin) · \(c.matiere_libelle)\(c.prof_label.map { " · \($0)" } ?? "")")
                            }
                        }
                    }
                    Section("Absences (\(model.absences.count))") {
                        if model.absences.isEmpty {
                            Text("Aucune absence").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.absences) { a in
                                VStack(alignment: .leading) {
                                    Text("\(a.debut) → \(a.fin) · \(a.motif)")
                                    Text(a.statut + (a.motif_justif.map { " — \($0)" } ?? ""))
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    Section("Journal (\(model.journal.count))") {
                        if model.journal.isEmpty {
                            Text("Aucune entrée").foregroundStyle(.secondary)
                        } else {
                            ForEach(model.journal) { j in
                                VStack(alignment: .leading) {
                                    Text(j.libelle)
                                    Text("\(j.domaine) · \(j.created_at?.prefix(16) ?? "—")")
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
                                Text(f.sujet)
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
                .navigationTitle(model.email)
            }
        }
    }
}
