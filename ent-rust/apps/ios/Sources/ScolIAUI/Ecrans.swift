#if os(iOS)
import ScolIACore
import SwiftUI

private func aujourdHui() -> String {
    let f = DateFormatter()
    f.calendar = Calendar(identifier: .iso8601)
    f.locale = Locale(identifier: "en_US_POSIX")
    f.dateFormat = "yyyy-MM-dd"
    return f.string(from: Date())
}

/// `2026-11-06` → `06/11/2026`.
func dateFr(_ iso: String) -> String {
    let p = iso.prefix(10).split(separator: "-")
    return p.count == 3 ? "\(p[2])/\(p[1])/\(p[0])" : iso
}

func formatNote(_ v: Double) -> String {
    v.truncatingRemainder(dividingBy: 1) == 0 ? String(Int(v)) : String(format: "%.2f", v)
}

private func direct<T>(_ v: T) -> Lecture<T> { Lecture(valeur: v, horsLigne: false, horodatage: Date()) }

// MARK: - Chargement générique (attente, erreur, hors ligne, rafraîchissement)

struct Chargeur<T, Contenu: View>: View {
    let modele: AppModele
    let cle: String
    let charger: () async throws -> Lecture<T>
    @ViewBuilder let contenu: (T, @escaping () -> Void) -> Contenu
    @State private var lecture: Lecture<T>?
    @State private var erreur: String?
    @State private var essai = 0

    var body: some View {
        Group {
            if let e = erreur {
                VStack(spacing: 12) {
                    MessageErreur(texte: e)
                    Button("Réessayer") { essai += 1 }
                }
            } else if let l = lecture {
                if l.horsLigne {
                    Label("Hors ligne — données du \(l.horodatage.formatted(date: .abbreviated, time: .shortened)), en lecture seule.", systemImage: "wifi.slash")
                        .font(.footnote)
                }
                contenu(l.valeur) { essai += 1 }
            } else {
                ProgressView().accessibilityLabel("Chargement")
            }
        }
        .task(id: "\(cle)#\(essai)") {
            erreur = nil
            do {
                lecture = try await charger()
            } catch ErreurApi.nonAuthentifie {
                modele.sessionPerdue()
            } catch {
                erreur = error.localizedDescription
            }
        }
    }
}

struct Ligne: View {
    let titre: String
    var detail: String?
    var droite: String?
    var body: some View {
        HStack {
            VStack(alignment: .leading) {
                Text(titre)
                if let d = detail, !d.isEmpty { Text(d).font(.subheadline).foregroundStyle(.secondary) }
            }
            Spacer()
            if let d = droite { Text(d).font(.headline) }
        }
        .accessibilityElement(children: .combine)
    }
}

// MARK: - Aiguillage des modules

struct EcranModule: View {
    let module: Module
    let modele: AppModele
    let me: Me
    let espace: Espace
    let ouvrir: (Module) -> Void

    var body: some View {
        switch module {
        case .accueil: Accueil(modele: modele, espace: espace, ouvrir: ouvrir)
        case .appel: EcranAppel(modele: modele)
        case .vieScolaire: EcranVieScolaire(modele: modele)
        case .absences: AvecEnfants(modele: modele) { EcranAbsences(modele: modele, enfant: $0) }
        case .notes: EcranNotes(modele: modele, me: me, espace: espace)
        case .saisieNotes: EcranSaisieNotes(modele: modele)
        case .agenda: EcranAgenda(modele: modele, me: me, espace: espace)
        case .edt: EcranEdt(modele: modele, me: me, espace: espace)
        case .messagerie: EcranMessagerie(modele: modele)
        case .profil: EcranProfil(modele: modele, me: me)
        }
    }
}

// MARK: - Accueil

struct Accueil: View {
    let modele: AppModele
    let espace: Espace
    let ouvrir: (Module) -> Void

    var body: some View {
        List {
            Section {
                ForEach(espace.modules.filter { $0 != .accueil && $0 != .profil }.prefix(4)) { m in
                    Button { ouvrir(m) } label: { Label(m.titre, systemImage: m.symbole) }
                }
            }
            if espace.profil == .professeur {
                Section("Mes cours du jour") {
                    Chargeur(modele: modele, cle: "cours", charger: { try await modele.lecteur!.lire(Chemins.mesCreneaux(aujourdHui()), [EdtCreneau].self) }) { cr, _ in
                        if cr.isEmpty { Text("Aucun cours aujourd'hui.") }
                        ForEach(cr.sorted { $0.heureDebut < $1.heureDebut }) { s in
                            Ligne(titre: "\(s.heureDebut.prefix(5)) · \(s.classe)", detail: s.matiereLibelle, droite: s.salle)
                        }
                    }
                }
            }
            Section("Annonces") {
                Chargeur(modele: modele, cle: "annonces", charger: { direct(try await modele.client!.annonces()) }) { a, _ in
                    if a.isEmpty { Text("Aucune annonce.") }
                    ForEach(a.prefix(5)) { Ligne(titre: $0.titre, detail: String($0.corps.prefix(140))) }
                }
            }
        }
    }
}

// MARK: - Appel (professeur)

struct EcranAppel: View {
    let modele: AppModele
    @State private var appel: Appel?
    @State private var erreur: String?
    @State private var confirmer = false

    private func lancer(_ bloc: @escaping () async throws -> Void) {
        Task {
            erreur = nil
            do { try await bloc() } catch ErreurApi.nonAuthentifie { modele.sessionPerdue() } catch { erreur = error.localizedDescription }
        }
    }

    var body: some View {
        if let a = appel {
            feuille(a)
        } else {
            List {
                if let e = erreur { MessageErreur(texte: e) }
                Chargeur(modele: modele, cle: "seances", charger: { try await modele.lecteur!.lire(Chemins.mesCreneaux(aujourdHui()), [EdtCreneau].self) }) { cr, _ in
                    if cr.isEmpty { Text("Aucune séance aujourd'hui.") }
                    ForEach(cr.sorted { $0.heureDebut < $1.heureDebut }) { s in
                        Button {
                            lancer { appel = try await modele.client!.ouvrirAppel(creneauId: s.id, jour: aujourdHui()) }
                        } label: {
                            Ligne(titre: "\(s.heureDebut.prefix(5))–\(s.heureFin.prefix(5)) · \(s.classe)", detail: s.matiereLibelle, droite: "Faire l'appel")
                        }
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func feuille(_ a: Appel) -> some View {
        let absents = a.lignes.filter { $0.etat == "absent" }.count
        let retards = a.lignes.filter { $0.etat == "retard" }.count
        List {
            Section {
                Text("\(a.classe) · \(a.matiereLibelle ?? "")").font(.headline).accessibilityAddTraits(.isHeader)
                Text("\(a.lignes.count) élèves · \(absents) absent(s) · \(retards) retard(s)\(a.estClos ? " · appel clos" : "")")
                if let e = erreur { MessageErreur(texte: e) }
            }
            ForEach(a.lignes) { l in
                VStack(alignment: .leading) {
                    Text(l.eleveLabel ?? "Élève")
                    Picker(l.eleveLabel ?? "Élève", selection: Binding(
                        get: { EtatAppel(rawValue: l.etat) ?? .present },
                        set: { e in
                            lancer {
                                try await modele.client!.marquer(appelId: a.id, eleveId: l.eleveId, etat: e)
                                var copie = a
                                if let i = copie.lignes.firstIndex(where: { $0.eleveId == l.eleveId }) { copie.lignes[i].etat = e.rawValue }
                                appel = copie
                            }
                        })) {
                        ForEach(EtatAppel.allCases, id: \.self) { Text($0.libelle).tag($0) }
                    }
                    .pickerStyle(.segmented)
                    .disabled(a.estClos)
                }
            }
        }
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Retour") { appel = nil } }
            if !a.estClos { ToolbarItem(placement: .confirmationAction) { Button("Valider") { confirmer = true } } }
        }
        // Action sensible : confirmation explicite.
        .confirmationDialog("Valider l'appel ?", isPresented: $confirmer, titleVisibility: .visible) {
            Button("Valider l'appel") { lancer { appel = try await modele.client!.clore(appelId: a.id) } }
        } message: {
            Text("\(absents) absent(s), \(retards) retard(s). L'appel ne sera plus modifiable.")
        }
    }
}

// MARK: - Vie scolaire

struct EcranVieScolaire: View {
    let modele: AppModele
    var body: some View {
        List {
            Chargeur(modele: modele, cle: "vs", charger: { direct(try await modele.client!.absencesATraiter()) }) { liste, _ in
                if liste.isEmpty { Text("Rien à traiter.") }
                ForEach(liste) { a in
                    Ligne(titre: a.eleveLabel ?? "Élève", detail: "Du \(dateFr(a.debut)) au \(dateFr(a.fin)) · \(a.motif)", droite: a.statut)
                }
            }
            Text("Le traitement détaillé se fait sur le web.").font(.footnote)
        }
    }
}

// MARK: - Famille

struct AvecEnfants<Contenu: View>: View {
    let modele: AppModele
    @ViewBuilder let contenu: (EnfantFamille) -> Contenu
    @State private var choisi: String?

    var body: some View {
        Chargeur(modele: modele, cle: "enfants", charger: { try await modele.lecteur!.lire(Chemins.enfants, [EnfantFamille].self) }) { enfants, _ in
            if let premier = enfants.first {
                let e = enfants.first { $0.id == choisi } ?? premier
                VStack {
                    if enfants.count > 1 {
                        Picker("Enfant", selection: Binding(get: { e.id }, set: { choisi = $0 })) {
                            ForEach(enfants) { Text($0.prenom).tag($0.id) }
                        }
                        .pickerStyle(.segmented)
                        .padding(.horizontal)
                    }
                    contenu(e)
                }
            } else {
                Text("Aucun enfant rattaché à votre compte dans cet établissement.")
            }
        }
    }
}

struct EcranAbsences: View {
    let modele: AppModele
    let enfant: EnfantFamille
    @State private var aJustifier: Absence?
    @State private var motif = ""
    @State private var erreur: String?

    var body: some View {
        List {
            Chargeur(modele: modele, cle: enfant.id, charger: { direct(try await modele.client!.resumeEnfant(enfant.id)) }) { r, recharger in
                Section("Absences à justifier") {
                    if r.absencesAJustifier.isEmpty { Text("Aucune absence à justifier pour \(enfant.prenom).") }
                    ForEach(r.absencesAJustifier) { a in
                        Button { if enfant.peutJustifierAbsence { aJustifier = a; motif = "" } } label: {
                            Ligne(titre: a.debut == a.fin ? "Le \(dateFr(a.debut))" : "Du \(dateFr(a.debut)) au \(dateFr(a.fin))",
                                  detail: a.motif, droite: enfant.peutJustifierAbsence ? "Justifier" : nil)
                        }
                    }
                    if !enfant.peutJustifierAbsence { Text("Votre compte n'a pas le droit de justifier les absences de cet enfant.").font(.footnote) }
                }
                if !r.carnetASigner.isEmpty {
                    Section("Carnet à signer (sur le web)") {
                        ForEach(r.carnetASigner) { Ligne(titre: $0.titre, detail: dateFr($0.dateEntree)) }
                    }
                }
                if let e = erreur { MessageErreur(texte: e) }
                Color.clear.frame(height: 0).sheet(item: $aJustifier) { a in
                    NavigationStack {
                        Form {
                            TextField("Motif", text: $motif, axis: .vertical).lineLimit(2...6)
                        }
                        .navigationTitle("Absence du \(dateFr(a.debut))")
                        .toolbar {
                            ToolbarItem(placement: .cancellationAction) { Button("Annuler") { aJustifier = nil } }
                            ToolbarItem(placement: .confirmationAction) {
                                Button("Envoyer") {
                                    Task {
                                        do {
                                            _ = try await modele.client!.justifierAbsence(a.id, motif: String(motif.prefix(500)))
                                            aJustifier = nil
                                            recharger()
                                        } catch ErreurApi.nonAuthentifie {
                                            modele.sessionPerdue()
                                        } catch {
                                            erreur = error.localizedDescription
                                            aJustifier = nil
                                        }
                                    }
                                }
                                .disabled(motif.trimmingCharacters(in: .whitespaces).isEmpty)
                            }
                        }
                    }
                }
            }
        }
    }
}

// MARK: - Notes

struct ListeNotes: View {
    let modele: AppModele
    let eleveId: String
    var body: some View {
        List {
            Chargeur(modele: modele, cle: eleveId, charger: { try await modele.lecteur!.lire(Chemins.notes(eleveId), [Note].self) }) { notes, _ in
                if notes.isEmpty { Text("Aucune note pour le moment.") }
                ForEach(notes.sorted { $0.dateNote > $1.dateNote }) { n in
                    Ligne(titre: n.matiereLibelle ?? "Matière", detail: [n.libelle, dateFr(n.dateNote)].compactMap { $0 }.joined(separator: " · "),
                          droite: "\(formatNote(n.valeur))/\(formatNote(n.sur))")
                }
            }
        }
    }
}

struct EcranNotes: View {
    let modele: AppModele
    let me: Me
    let espace: Espace
    var body: some View {
        switch espace.profil {
        case .eleve:
            if let id = me.eleveId { ListeNotes(modele: modele, eleveId: id) } else { Text("Compte élève non lié.") }
        case .famille:
            AvecEnfants(modele: modele) { e in
                if e.peutVoirNotes { ListeNotes(modele: modele, eleveId: e.id) } else { Text("Votre compte n'a pas accès aux notes de cet enfant.") }
            }
        default:
            Text("Consultez les notes d'un élève depuis le web.")
        }
    }
}

// MARK: - Saisie de notes (professeur)

struct EcranSaisieNotes: View {
    let modele: AppModele
    @State private var classe: String?
    @State private var devoir: Devoir?
    @State private var saisies: [String: String] = [:]
    @State private var enregistrees: Set<String> = []
    @State private var erreur: String?

    var body: some View {
        List {
            Section("Classe") {
                Chargeur(modele: modele, cle: "classes", charger: { try await modele.lecteur!.lire(Chemins.mesCreneaux(aujourdHui()), [EdtCreneau].self) }) { cr, _ in
                    let classes = Array(Set(cr.map(\.classe))).sorted()
                    if classes.isEmpty { Text("Aucune classe aujourd'hui. La saisie complète est sur le web.") }
                    ForEach(classes, id: \.self) { k in
                        Button { classe = k; devoir = nil } label: { Ligne(titre: k, droite: k == classe ? "✓" : nil) }
                    }
                }
            }
            if let k = classe {
                Section("Devoir") {
                    Chargeur(modele: modele, cle: k, charger: {
                        let periodes = try await modele.client!.periodesNotes()
                        guard let p = periodes.first(where: { $0.statut == "ouverte" }) ?? periodes.first else { return direct([Devoir]()) }
                        return direct(try await modele.client!.devoirsClasse(periodeId: p.id, classe: k))
                    }) { devoirs, _ in
                        if devoirs.isEmpty { Text("Aucun devoir pour \(k) sur la période ouverte. Créez-le sur le web.") }
                        ForEach(devoirs) { d in
                            Button { devoir = d } label: {
                                Ligne(titre: d.titre, detail: "\(d.matiereLibelle ?? "") · \(dateFr(d.dateDevoir)) · sur \(formatNote(d.sur))", droite: d.id == devoir?.id ? "✓" : nil)
                            }
                        }
                    }
                }
            }
            if let k = classe, let d = devoir {
                Section("Notes sur \(formatNote(d.sur))") {
                    Chargeur(modele: modele, cle: d.id, charger: { direct(try await modele.client!.effectif(classe: k, jour: aujourdHui())) }) { eff, _ in
                        ForEach(eff.eleves) { el in
                            HStack {
                                Text("\(el.nom) \(el.prenom)")
                                Spacer()
                                TextField(enregistrees.contains(el.id) ? "Enregistrée" : "Note", text: Binding(
                                    get: { saisies[el.id] ?? "" },
                                    set: { saisies[el.id] = String($0.replacingOccurrences(of: ",", with: ".").prefix(5)) }))
                                    .keyboardType(.decimalPad)
                                    .multilineTextAlignment(.trailing)
                                    .frame(maxWidth: 90)
                                    .accessibilityLabel("Note de \(el.prenom) \(el.nom)")
                            }
                        }
                    }
                    if let e = erreur { MessageErreur(texte: e) }
                    Button("Enregistrer les notes") { enregistrer(d) }
                }
            }
        }
    }

    private func enregistrer(_ d: Devoir) {
        Task {
            erreur = nil
            for (eleve, texte) in saisies where !enregistrees.contains(eleve) {
                guard let v = Double(texte), v >= 0, v <= d.sur else { erreur = "Note invalide : « \(texte) »."; continue }
                do {
                    _ = try await modele.client!.saisirNote(eleveId: eleve, SaisirNoteBody(
                        matiereId: d.matiereId, valeur: v, sur: d.sur, dateNote: aujourdHui(), libelle: d.titre, devoirId: d.id, periodeId: d.periodeId))
                    enregistrees.insert(eleve)
                } catch ErreurApi.nonAuthentifie {
                    modele.sessionPerdue(); return
                } catch {
                    erreur = error.localizedDescription
                }
            }
        }
    }
}

// MARK: - Cahier de textes, EDT

struct ListeCahier: View {
    let modele: AppModele
    let eleveId: String?
    var body: some View {
        List {
            Chargeur(modele: modele, cle: eleveId ?? "moi", charger: { try await modele.lecteur!.lire(Chemins.cahier(eleveId), [CahierTexte].self) }) { liste, _ in
                let aVenir = liste.filter { $0.pourLe >= aujourdHui() }.sorted { $0.pourLe < $1.pourLe }
                if aVenir.isEmpty { Text("Rien à faire pour les prochains jours.") }
                ForEach(aVenir) { e in Ligne(titre: "\(e.matiereLibelle) · pour le \(dateFr(e.pourLe))", detail: "\(e.titre)\n\(e.consigne)") }
            }
        }
    }
}

struct EcranAgenda: View {
    let modele: AppModele
    let me: Me
    let espace: Espace
    var body: some View {
        switch espace.profil {
        case .eleve: ListeCahier(modele: modele, eleveId: me.eleveId)
        case .famille: AvecEnfants(modele: modele) { ListeCahier(modele: modele, eleveId: $0.id) }
        default: ListeCahier(modele: modele, eleveId: nil)
        }
    }
}

private let jours = ["", "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"]

struct ListeEdt: View {
    let modele: AppModele
    let chemin: String
    var body: some View {
        List {
            Chargeur(modele: modele, cle: chemin, charger: { try await modele.lecteur!.lire(chemin, [EdtCreneau].self) }) { cr, _ in
                if cr.isEmpty { Text("Aucun cours.") }
                ForEach(Array(Set(cr.map(\.jourSemaine))).sorted(), id: \.self) { j in
                    Section(j < jours.count ? jours[j] : "") {
                        ForEach(cr.filter { $0.jourSemaine == j }.sorted { $0.heureDebut < $1.heureDebut }) { s in
                            Ligne(titre: "\(s.heureDebut.prefix(5))–\(s.heureFin.prefix(5)) · \(s.matiereLibelle)",
                                  detail: [s.classe, s.profLabel, s.salle.map { "salle \($0)" }].compactMap { $0 }.joined(separator: " · "))
                        }
                    }
                }
            }
        }
    }
}

struct EcranEdt: View {
    let modele: AppModele
    let me: Me
    let espace: Espace
    var body: some View {
        switch espace.profil {
        case .eleve:
            if let id = me.eleveId { ListeEdt(modele: modele, chemin: Chemins.edtEleve(id, aujourdHui())) }
        case .famille:
            AvecEnfants(modele: modele) { ListeEdt(modele: modele, chemin: Chemins.edtEleve($0.id, aujourdHui())) }
        default:
            ListeEdt(modele: modele, chemin: Chemins.mesCreneaux(aujourdHui()))
        }
    }
}

// MARK: - Messagerie

struct EcranMessagerie: View {
    let modele: AppModele
    var body: some View {
        List {
            Chargeur(modele: modele, cle: "fils", charger: { try await modele.lecteur!.lire(Chemins.fils, [MsgFil].self) }) { fils, _ in
                if fils.isEmpty { Text("Aucun message.") }
                ForEach(fils.sorted { $0.lastMessageAt > $1.lastMessageAt }) { f in
                    NavigationLink(value: f.id) {
                        Ligne(titre: f.sujet, detail: [f.eleveLabel, dateFr(f.lastMessageAt)].compactMap { $0 }.joined(separator: " · "),
                              droite: f.nbNonLus > 0 ? "\(f.nbNonLus) non lu(s)" : nil)
                    }
                }
            }
            Text("Pour écrire un nouveau message, utilisez le web (choix des destinataires).").font(.footnote)
        }
        .navigationDestination(for: String.self) { EcranFil(modele: modele, filId: $0) }
    }
}

struct EcranFil: View {
    let modele: AppModele
    let filId: String
    @State private var reponse = ""
    @State private var confirmer = false
    @State private var erreur: String?

    var body: some View {
        List {
            Chargeur(modele: modele, cle: filId, charger: { direct(try await modele.client!.fil(filId)) }) { d, recharger in
                Section(d.fil.sujet) {
                    ForEach(d.messages) { m in
                        VStack(alignment: .leading, spacing: 4) {
                            Text("\(m.auteurLabel ?? "—") · \(dateFr(m.createdAt))").font(.caption)
                            Text(m.corps)
                            if !m.pieces.isEmpty { Text("\(m.pieces.count) pièce(s) jointe(s) : à ouvrir sur le web").font(.footnote) }
                        }
                        .accessibilityElement(children: .combine)
                    }
                }
                Section("Répondre") {
                    TextField("Votre réponse", text: $reponse, axis: .vertical).lineLimit(2...8)
                    if let e = erreur { MessageErreur(texte: e) }
                    Button("Envoyer") { confirmer = true }.disabled(reponse.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
                // Envoi de message : confirmation humaine.
                .confirmationDialog("Envoyer ce message ?", isPresented: $confirmer, titleVisibility: .visible) {
                    Button("Envoyer") {
                        Task {
                            do {
                                _ = try await modele.client!.repondre(filId: filId, corps: String(reponse.prefix(5000)).trimmingCharacters(in: .whitespacesAndNewlines))
                                reponse = ""
                                recharger()
                            } catch ErreurApi.nonAuthentifie {
                                modele.sessionPerdue()
                            } catch {
                                erreur = error.localizedDescription
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle("Message")
    }
}

// MARK: - Mon compte

struct EcranProfil: View {
    let modele: AppModele
    let me: Me
    @State private var erreur: String?

    var body: some View {
        List {
            Section {
                Ligne(titre: me.email, detail: "\(modele.actif?.nom ?? "") · rôle : \(me.role)")
                Ligne(titre: "Passkeys : \(me.passkeyCount)",
                      detail: me.passkeyCount == 0 ? "Ajoutez une passkey : elle sera bientôt obligatoire." : "Connexion sans mot de passe active.")
            }
            Section("Appareils connectés") {
                Chargeur(modele: modele, cle: "sessions", charger: { direct(try await modele.client!.sessions()) }) { sessions, recharger in
                    ForEach(sessions) { s in
                        HStack {
                            // Libellé issu du User-Agent : texte brut uniquement (E8).
                            Ligne(titre: String((s.appareil ?? "Appareil inconnu").prefix(60)) + (s.courante ? " (cet appareil)" : ""),
                                  detail: "\(s.client == "mobile" ? "Appli" : "Web") · vu le \(dateFr(s.lastSeenAt))")
                            if !s.courante {
                                Button("Déconnecter", role: .destructive) {
                                    Task {
                                        do { try await modele.client!.revoquerSession(s.id); recharger() } catch { erreur = error.localizedDescription }
                                    }
                                }
                            }
                        }
                    }
                }
                if let e = erreur { MessageErreur(texte: e) }
            }
            if modele.rattachements.count > 1 {
                Section("Changer d'établissement") {
                    ForEach(modele.rattachements.filter { $0.id != modele.actif?.id }) { r in Button(r.nom ?? r.slug) { modele.choisir(r) } }
                }
            }
            Section { Button("Se déconnecter", role: .destructive) { modele.deconnexion() } }
        }
    }
}
#endif
