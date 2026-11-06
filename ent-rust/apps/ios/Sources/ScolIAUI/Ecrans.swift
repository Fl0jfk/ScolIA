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
    /// `.task` posé sur un `Group` s'exécute pour chaque ligne : on ne charge qu'une fois par clé.
    @State private var derniere: String?

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
        .task(id: "\(cle)#\(essai)") { @MainActor in
            let k = "\(cle)#\(essai)"
            guard derniere != k else { return }
            derniere = k
            erreur = nil
            do {
                lecture = try await charger()
            } catch ErreurApi.nonAuthentifie {
                modele.sessionPerdue()
            } catch {
                if Task.isCancelled || error is CancellationError { derniere = nil; return }
                erreur = error.localizedDescription
            }
        }
        // Changement d'enfant / de clé : ne pas afficher les données précédentes.
        .onChange(of: cle) { _, _ in lecture = nil }
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
            if espace.enseigne {
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
                    Picker(l.eleveLabel ?? "Élève", selection: Binding<EtatAppel?>(
                        get: { EtatAppel(rawValue: l.etat) },
                        set: { nouvel in
                            guard let e = nouvel else { return }
                            lancer {
                                try await modele.client!.marquer(appelId: a.id, eleveId: l.eleveId, etat: e)
                                guard var copie = appel else { return }
                                if let i = copie.lignes.firstIndex(where: { $0.eleveId == l.eleveId }) { copie.lignes[i].etat = e.rawValue }
                                appel = copie
                            }
                        })) {
                        ForEach(EtatAppel.allCases, id: \.self) { Text($0.libelle).tag(Optional($0)) }
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
                    contenu(e).id(e.id)
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
    @State private var envoi = false
    @State private var rechargement = 0

    var body: some View {
        List {
            Chargeur(modele: modele, cle: "\(enfant.id)#\(rechargement)", charger: { direct(try await modele.client!.resumeEnfant(enfant.id)) }) { r, _ in
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
            }
        }
        .sheet(item: $aJustifier) { a in
                    NavigationStack {
                        Form {
                            TextField("Motif", text: $motif, axis: .vertical).lineLimit(2...6)
                        }
                        .navigationTitle("Absence du \(dateFr(a.debut))")
                        .toolbar {
                            ToolbarItem(placement: .cancellationAction) { Button("Annuler") { aJustifier = nil } }
                            ToolbarItem(placement: .confirmationAction) {
                                Button("Envoyer") {
                                    envoi = true
                                    Task { @MainActor in
                                        defer { envoi = false }
                                        do {
                                            _ = try await modele.client!.justifierAbsence(a.id, motif: String(motif.prefix(500)))
                                            aJustifier = nil
                                            rechargement += 1
                                        } catch ErreurApi.nonAuthentifie {
                                            modele.sessionPerdue()
                                        } catch {
                                            erreur = error.localizedDescription
                                            aJustifier = nil
                                        }
                                    }
                                }
                                .disabled(motif.trimmingCharacters(in: .whitespaces).isEmpty || envoi)
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
    @State private var enCours = false
    @State private var confirmer = false

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
                    Button("Enregistrer les notes") { confirmer = true }
                        .disabled(enCours || aEnvoyer.isEmpty)
                        .confirmationDialog("Enregistrer \(aEnvoyer.count) note(s) ?", isPresented: $confirmer, titleVisibility: .visible) {
                            Button("Enregistrer") { enregistrer(d) }
                        } message: {
                            Text("« \(d.titre) », \(k), notes sur \(formatNote(d.sur)).")
                        }
                }
            }
        }
        // Saisies propres au devoir choisi : changer de devoir ou de classe repart de zéro.
        .onChange(of: devoir?.id) { _, _ in saisies = [:]; enregistrees = []; erreur = nil }
        .onChange(of: classe) { _, _ in saisies = [:]; enregistrees = []; erreur = nil }
    }

    private var aEnvoyer: [String: String] {
        saisies.filter { !enregistrees.contains($0.key) && !$0.value.trimmingCharacters(in: .whitespaces).isEmpty }
    }

    private func enregistrer(_ d: Devoir) {
        enCours = true
        let lot = aEnvoyer
        Task { @MainActor in
            defer { enCours = false }
            erreur = nil
            for (eleve, texte) in lot {
                guard let v = Double(texte), v.isFinite, v >= 0, v <= d.sur else { erreur = "Note invalide : « \(texte) »."; continue }
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
    @State private var envoi = false
    @State private var rechargement = 0
    @State private var erreur: String?

    var body: some View {
        List {
            Chargeur(modele: modele, cle: "\(filId)#\(rechargement)", charger: { direct(try await modele.client!.fil(filId)) }) { d, _ in
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
                    Button("Envoyer") { confirmer = true }
                        .disabled(reponse.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || envoi)
                }
            }
        }
        // Envoi de message : confirmation humaine (posée sur la liste, pas sur une section).
        .confirmationDialog("Envoyer ce message ?", isPresented: $confirmer, titleVisibility: .visible) {
            Button("Envoyer") {
                envoi = true
                Task { @MainActor in
                    defer { envoi = false }
                    do {
                        _ = try await modele.client!.repondre(filId: filId, corps: String(reponse.prefix(5000)).trimmingCharacters(in: .whitespacesAndNewlines))
                        reponse = ""
                        rechargement += 1
                    } catch ErreurApi.nonAuthentifie {
                        modele.sessionPerdue()
                    } catch {
                        erreur = error.localizedDescription
                    }
                }
            }
        } message: {
            Text(String(reponse.prefix(300)))
        }
        .navigationTitle("Message")
    }
}

// MARK: - Mon compte

struct EcranProfil: View {
    let modele: AppModele
    let me: Me
    @State private var erreur: String?
    @State private var aRevoquer: SessionAppareil?
    @State private var rechargement = 0

    var body: some View {
        List {
            Section {
                Ligne(titre: me.email, detail: "\(modele.actif?.nom ?? "") · rôle : \(me.role)")
                Ligne(titre: "Passkeys : \(me.passkeyCount)",
                      detail: me.passkeyCount == 0 ? "Ajoutez une passkey : elle sera bientôt obligatoire." : "Connexion sans mot de passe active.")
            }
            Section("Appareils connectés") {
                Chargeur(modele: modele, cle: "sessions#\(rechargement)", charger: { direct(try await modele.client!.sessions()) }) { sessions, _ in
                    ForEach(sessions) { s in
                        HStack {
                            // Libellé issu du User-Agent : texte brut uniquement (E8).
                            Ligne(titre: String((s.appareil ?? "Appareil inconnu").prefix(60)) + (s.courante ? " (cet appareil)" : ""),
                                  detail: "\(s.client == "mobile" ? "Appli" : "Web") · vu le \(dateFr(s.lastSeenAt))")
                            if !s.courante {
                                Button("Déconnecter", role: .destructive) { aRevoquer = s }
                                    .buttonStyle(.borderless) // seul le bouton est actif, pas toute la ligne
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
        .confirmationDialog("Déconnecter cet appareil ?", isPresented: Binding(get: { aRevoquer != nil }, set: { if !$0 { aRevoquer = nil } }),
                            titleVisibility: .visible, presenting: aRevoquer) { s in
            Button("Déconnecter", role: .destructive) {
                Task { @MainActor in
                    do {
                        try await modele.client!.revoquerSession(s.id)
                        rechargement += 1
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
#endif
