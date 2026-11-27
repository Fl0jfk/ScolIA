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
            } catch ErreurApi.appliTropAncienne(let m) {
                modele.miseAJourRequise = m
                erreur = m
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
        case .accueil: Accueil(modele: modele, me: me, espace: espace, ouvrir: ouvrir)
        case .appel: EcranAppel(modele: modele)
        case .vieScolaire: EcranVieScolaire(modele: modele)
        case .absences: AvecEnfants(modele: modele) { EcranAbsences(modele: modele, enfant: $0) }
        case .notes: EcranNotes(modele: modele, me: me, espace: espace)
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
    let me: Me
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
                // MB2 : `/annonces` est réservée au personnel ; l'élève et la famille lisent celles de l'élève.
                switch espace.profil {
                case .personnel: Annonces(modele: modele, cle: "annonces") { try await modele.client!.annonces() }
                case .eleve:
                    if let id = me.eleveId { Annonces(modele: modele, cle: "annonces-\(id)") { try await modele.client!.annoncesEleve(id) } }
                case .famille:
                    AvecEnfants(modele: modele) { e in Annonces(modele: modele, cle: "annonces-\(e.id)") { try await modele.client!.annoncesEleve(e.id) } }
                }
            }
        }
    }
}

struct Annonces: View {
    let modele: AppModele
    let cle: String
    let charger: () async throws -> [Annonce]
    var body: some View {
        Chargeur(modele: modele, cle: cle, charger: { direct(try await charger()) }) { a, _ in
            if a.isEmpty { Text("Aucune annonce.") }
            ForEach(a.prefix(5)) { Ligne(titre: $0.titre, detail: String($0.corps.prefix(140))) }
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
            do { try await bloc() } catch ErreurApi.nonAuthentifie { modele.sessionPerdue() } catch ErreurApi.appliTropAncienne(let m) {
                modele.miseAJourRequise = m
            } catch { erreur = error.localizedDescription }
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
                            lancer {
                                let j = aujourdHui()
                                appel = try await modele.cles.executer("ouvrir:\(s.id):\(j)") { k in try await modele.client!.ouvrirAppel(creneauId: s.id, jour: j, cle: k) }
                            }
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
                                try await modele.cles.executer("marquer:\(a.id):\(l.eleveId):\(e.rawValue)", famille: "marquer:\(a.id):\(l.eleveId):") { k in try await modele.client!.marquer(appelId: a.id, eleveId: l.eleveId, etat: e, cle: k) }
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
            Button("Valider l'appel") { lancer { appel = try await modele.cles.executer("clore:\(a.id)") { k in try await modele.client!.clore(appelId: a.id, cle: k) } } }
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
                                            let texte = String(motif.prefix(500))
                                            _ = try await modele.cles.executer("justifier:\(a.id):\(texte)", famille: "justifier:\(a.id):") { k in try await modele.client!.justifierAbsence(a.id, motif: texte, cle: k) }
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

// MARK: - Cahier de textes, EDT

struct ListeCahier: View {
    let modele: AppModele
    let chemin: String
    var body: some View {
        List {
            Chargeur(modele: modele, cle: chemin, charger: { try await modele.lecteur!.lire(chemin, [CahierTexte].self) }) { liste, _ in
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
        case .eleve: ListeCahier(modele: modele, chemin: Chemins.cahier(me.eleveId))
        case .famille: AvecEnfants(modele: modele) { ListeCahier(modele: modele, chemin: Chemins.cahier($0.id)) }
        default: AgendaPersonnel(modele: modele)
        }
    }
}

/// Personnel : cahier de textes d'une des classes de ses séances du jour (le serveur borne au périmètre).
struct AgendaPersonnel: View {
    let modele: AppModele
    @State private var choisie: String?

    var body: some View {
        Chargeur(modele: modele, cle: "agenda-classes", charger: { try await modele.lecteur!.lire(Chemins.mesCreneaux(aujourdHui()), [EdtCreneau].self) }) { cr, _ in
            let classes = Array(Set(cr.map(\.classe))).sorted()
            if let premiere = classes.first {
                let classe = choisie.flatMap { classes.contains($0) ? $0 : nil } ?? premiere
                VStack {
                    if classes.count > 1 {
                        Picker("Classe", selection: Binding(get: { classe }, set: { choisie = $0 })) {
                            ForEach(classes, id: \.self) { Text($0).tag($0) }
                        }
                        .pickerStyle(.segmented)
                        .padding(.horizontal)
                    }
                    ListeCahier(modele: modele, chemin: Chemins.cahierClasse(classe)).id(classe)
                }
            } else {
                Text("Aucune classe dans vos séances du jour. Le cahier de textes complet est sur le web.")
            }
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
                        let texte = String(reponse.prefix(5000)).trimmingCharacters(in: .whitespacesAndNewlines)
                        _ = try await modele.cles.executer("repondre:\(filId):\(texte)") { k in try await modele.client!.repondre(filId: filId, corps: texte, cle: k) }
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
    @State private var motDePasse = ""

    var body: some View {
        List {
            Section {
                Ligne(titre: me.email, detail: "\(modele.actif?.nom ?? "") · rôle : \(me.role)")
                Ligne(titre: "Passkeys : \(me.passkeyCount)",
                      detail: me.passkeyCount == 0 ? "Ajoutez une passkey : elle sera bientôt obligatoire." : "Connexion sans mot de passe active.")
                if me.passkeyCount == 0 {
                    // M3 (MB1) : la passkey se crée ici aussi, sans attendre qu'elle soit exigée.
                    SecureField("Mot de passe actuel", text: $motDePasse)
                        .textContentType(.password)
                    Button("Ajouter une passkey") {
                        modele.enroler(motDePasse)
                        motDePasse = ""
                    }
                    .disabled(motDePasse.isEmpty || modele.occupe)
                    if let e = modele.erreur { MessageErreur(texte: e) }
                }
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
