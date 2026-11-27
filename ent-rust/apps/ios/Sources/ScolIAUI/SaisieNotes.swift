#if os(iOS)
import ScolIACore
import SwiftUI

// Saisie des notes sur la grille d'évaluation `/eva/*` (palier « notes eva » du socle 11), miroir de l'écran Android.
// Parcours court du quotidien : cours → période (celle du jour par défaut) → évaluations → saisie d'une évaluation.
// Le web garde la grille complète (modification, annulation, import, historique, corrections motivées).

private func jourIso() -> String {
    let f = DateFormatter()
    f.calendar = Calendar(identifier: .iso8601)
    f.locale = Locale(identifier: "en_US_POSIX")
    f.dateFormat = "yyyy-MM-dd"
    return f.string(from: Date())
}

private func direct<T>(_ v: T) -> Lecture<T> { Lecture(valeur: v, horsLigne: false, horodatage: Date()) }

private func etatEvaluation(_ e: EvaluationEva) -> String {
    if e.etat == "annulee" { return " · annulée" }
    if e.publiee { return " · publiée" }
    if e.publicationPrevueLe != nil { return " · publication programmée" }
    return " · non publiée"
}

struct EcranSaisieNotes: View {
    let modele: AppModele
    @State private var cours: EnseignementEva?
    @State private var periode: PeriodeEva?
    @State private var evaluationId: String?
    /// La période du jour s'ouvre d'elle-même une fois par cours ; « Autre période » rend ensuite le choix.
    @State private var auto = true

    var body: some View {
        if let c = cours, let p = periode {
            GrilleCours(modele: modele, cours: c, periode: p, evaluationId: $evaluationId) {
                if evaluationId != nil { evaluationId = nil } else { periode = nil }
            }
        } else if let c = cours {
            List {
                Button("Autre cours") { cours = nil }
                Chargeur(modele: modele, cle: "eva-periodes-\(c.coursId)", charger: { direct(try await modele.client!.evaPeriodes(coursId: c.coursId)) }) { liste, _ in
                    let jour = jourIso()
                    let duJour = liste.first { $0.contient(jour) }
                    if liste.isEmpty { Text("Aucune période de notes pour ce cours.") }
                    ForEach(liste) { p in
                        Button { periode = p; auto = false } label: {
                            Ligne(titre: p.libelle, detail: "\(dateFr(p.dateDebut)) au \(dateFr(p.dateFin))", droite: p.id == duJour?.id ? "en cours" : nil)
                        }
                    }
                    Color.clear.frame(height: 0).task(id: duJour?.id) {
                        if auto, let d = duJour { periode = d; auto = false }
                    }
                }
            }
            .navigationTitle(c.libelle)
        } else {
            List {
                Chargeur(modele: modele, cle: "eva-cours", charger: { direct(try await modele.client!.evaEnseignements()) }) { liste, _ in
                    let enSaisie = liste.filter(\.enSaisie)
                    if enSaisie.isEmpty { Text("Aucun cours où vous saisissez des notes. Les grilles en lecture restent sur le web.") }
                    ForEach(enSaisie) { e in
                        Button { cours = e; periode = nil; auto = true } label: { Ligne(titre: e.libelle, detail: e.cibleCode) }
                    }
                }
            }
            .navigationTitle("Saisir des notes")
        }
    }
}

private struct GrilleCours: View {
    let modele: AppModele
    let cours: EnseignementEva
    let periode: PeriodeEva
    @Binding var evaluationId: String?
    let retour: () -> Void
    @State private var essai = 0
    @State private var creer = false

    var body: some View {
        Chargeur(modele: modele, cle: "eva-grille-\(cours.coursId)-\(periode.id)-\(essai)", charger: {
            direct(try await modele.client!.evaGrille(coursId: cours.coursId, periodeId: periode.id))
        }) { g, _ in
            if let id = evaluationId, let ev = g.evaluations.first(where: { $0.id == id }), let b = BrouillonSaisie(grille: g, evaluationId: id) {
                SaisieEvaluation(modele: modele, brouillon: b, evaluation: ev) { essai += 1; retour() }
            } else {
                liste(g)
            }
        }
    }

    @ViewBuilder private func liste(_ g: GrilleEva) -> some View {
        List {
            Section {
                Button("Autre période", action: retour)
                if g.peutSaisir { Button("Nouvelle évaluation") { creer = true } } else {
                    Text("Grille en lecture : la saisie n'est pas ouverte pour vous sur ce cours.")
                }
                ForEach(g.avertissements, id: \.self) { Text($0).foregroundStyle(.secondary) }
                if let m = g.moyenneClasse {
                    Text("Moyenne de la classe : \(SaisieNotes.formater(m.moyenneM)) / \(SaisieNotes.formater(g.echelleM))")
                }
            }
            let visibles = g.evaluations.filter { $0.etat != "brouillon" }.sorted { $0.dateEval > $1.dateEval }
            if visibles.isEmpty { Text("Aucune évaluation sur cette période.") }
            ForEach(visibles) { e in
                Button { evaluationId = e.id } label: {
                    Ligne(titre: e.titre,
                          detail: "\(dateFr(e.dateEval)) · sur \(SaisieNotes.formater(e.baremeM)) · coef \(SaisieNotes.formater(e.coefM))" + etatEvaluation(e),
                          droite: e.stats.map { SaisieNotes.formater($0.moyenneM) })
                }
            }
        }
        .navigationTitle("\(g.enseignement.libelle) · \(g.periode.libelle)")
        .sheet(isPresented: $creer) {
            NouvelleEvaluation(modele: modele, grille: g) { id in
                creer = false
                essai += 1
                evaluationId = id
            }
        }
    }
}

private struct NouvelleEvaluation: View {
    let modele: AppModele
    let grille: GrilleEva
    let creee: (String) -> Void
    @Environment(\.dismiss) private var fermer
    @State private var titre = ""
    @State private var date = Date()
    @State private var bareme = "20"
    @State private var coef = "1"
    @State private var erreur: String?
    @State private var occupe = false

    private var dateIso: String {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .iso8601)
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "yyyy-MM-dd"
        return f.string(from: date)
    }

    var body: some View {
        let baremeM = SaisieNotes.milliemesDepuisTexte(bareme).flatMap { $0 > 0 ? $0 : nil }
        let coefM = SaisieNotes.milliemesDepuisTexte(coef).flatMap { $0 > 0 ? $0 : nil }
        let t = titre.trimmingCharacters(in: .whitespacesAndNewlines)
        NavigationStack {
            Form {
                TextField("Titre", text: $titre)
                DatePicker("Date", selection: $date, displayedComponents: .date)
                TextField("Sur", text: $bareme).keyboardType(.decimalPad)
                TextField("Coefficient", text: $coef).keyboardType(.decimalPad)
                Text("Non publiée : les familles ne la verront qu'après « Publier ».").font(.footnote)
                if let e = erreur { MessageErreur(texte: e) }
            }
            .navigationTitle("Nouvelle évaluation")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Annuler") { fermer() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Créer") {
                        guard let b = baremeM, let c = coefM else { return }
                        let corps = CreerEvaluationEvaBody(coursId: grille.enseignement.coursId, titre: t, dateEval: dateIso, baremeM: b, coefM: c)
                        occupe = true
                        Task {
                            defer { occupe = false }
                            erreur = nil
                            do {
                                let e = try await modele.cles.executer("eva-creer:\(corps.coursId):\(corps.titre)|\(corps.dateEval)|\(corps.baremeM)|\(corps.coefM)") { k in
                                    try await modele.client!.creerEvaluation(corps, cle: k)
                                }
                                creee(e.id)
                            } catch ErreurApi.nonAuthentifie { modele.sessionPerdue() } catch ErreurApi.appliTropAncienne(let m) {
                                modele.miseAJourRequise = m
                            } catch { erreur = error.localizedDescription }
                        }
                    }
                    .disabled(t.isEmpty || t.count > 200 || baremeM == nil || coefM == nil || occupe)
                }
            }
        }
        .onAppear {
            // Date du jour si elle tombe dans la période, sinon son premier jour.
            if !grille.periode.contient(jourIso()) {
                let f = DateFormatter()
                f.calendar = Calendar(identifier: .iso8601)
                f.locale = Locale(identifier: "en_US_POSIX")
                f.dateFormat = "yyyy-MM-dd"
                if let d = f.date(from: grille.periode.dateDebut) { date = d }
            }
        }
    }
}

private struct SaisieEvaluation: View {
    let modele: AppModele
    @State var brouillon: BrouillonSaisie
    @State var evaluation: EvaluationEva
    let retour: () -> Void
    @State private var erreur: String?
    @State private var info: String?
    @State private var occupe = false
    @State private var publier = false
    @State private var motif = ""

    init(modele: AppModele, brouillon: BrouillonSaisie, evaluation: EvaluationEva, retour: @escaping () -> Void) {
        self.modele = modele
        _brouillon = State(initialValue: brouillon)
        _evaluation = State(initialValue: evaluation)
        self.retour = retour
    }

    private func lancer(_ bloc: @escaping () async throws -> Void) {
        occupe = true
        Task {
            defer { occupe = false }
            erreur = nil
            do { try await bloc() } catch ErreurApi.nonAuthentifie { modele.sessionPerdue() } catch ErreurApi.appliTropAncienne(let m) {
                modele.miseAJourRequise = m
            } catch ErreurApi.horsLigne {
                erreur = "Hors ligne : rien n'est perdu, réessayez « Enregistrer » une fois le réseau revenu."
            } catch { erreur = error.localizedDescription }
        }
    }

    var body: some View {
        let lot = brouillon.lot()
        let erreurs = brouillon.enErreur().count
        let peutPublier = brouillon.grille.peutSaisir && evaluation.ouverte && (!evaluation.publiee || evaluation.retraitPossible)
        List {
            Section {
                Text("\(dateFr(evaluation.dateEval)) · sur \(SaisieNotes.formater(evaluation.baremeM)) · coef \(SaisieNotes.formater(evaluation.coefM))" + etatEvaluation(evaluation))
                HStack {
                    Button("Retour", action: retour).buttonStyle(.bordered)
                    Button(lot.cellules.isEmpty ? "Enregistrer" : "Enregistrer (\(lot.cellules.count))") {
                        lancer {
                            let r = try await modele.cles.executer(brouillon.intention(lot), famille: brouillon.famille) { k in
                                try await modele.client!.saisirNotes(lot, cle: k)
                            }
                            let n = brouillon.appliquer(r, envoye: lot)
                            info = "\(n) case(s) enregistrée(s)." + (brouillon.nbConflits > 0 ? " \(brouillon.nbConflits) conflit(s) à trancher." : "")
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(lot.cellules.isEmpty || occupe)
                    if peutPublier {
                        Button(evaluation.publiee ? "Retirer" : "Publier") { motif = ""; publier = true }
                            .buttonStyle(.bordered)
                            .disabled(occupe || !lot.cellules.isEmpty)
                    }
                }
                if erreurs > 0 { MessageErreur(texte: "\(erreurs) case(s) non comprise(s) : corrigez-les avant d'enregistrer.") }
                if let e = erreur { MessageErreur(texte: e) }
                if let i = info { Text(i) }
                if !evaluation.ouverte { Text("Évaluation \(evaluation.etat == "annulee" ? "annulée" : "non ouverte") : lecture seule.") }
                Text("Codes : " + SaisieNotes.codesRapides.map { "\($0.code) \($0.libelle.lowercased())" }.joined(separator: ", ")).font(.footnote)
            }
            ForEach(brouillon.eleves) { l in
                caseEleve(l)
            }
        }
        .navigationTitle(evaluation.titre)
        .alert(evaluation.publiee ? "Retirer la publication ?" : "Publier les notes ?", isPresented: $publier) {
            if evaluation.publiee { TextField("Motif", text: $motif) }
            Button("Annuler", role: .cancel) {}
            Button(evaluation.publiee ? "Retirer" : "Publier") {
                let retirer = evaluation.publiee
                let m = motif.trimmingCharacters(in: .whitespacesAndNewlines)
                if retirer && !(3...500).contains(m.count) { erreur = "Motif écrit obligatoire pour retirer (3 à 500 caractères)."; return }
                let action = retirer ? "retirer" : "maintenant"
                let id = evaluation.id
                lancer {
                    evaluation = try await modele.cles.executer("eva-pub:\(id):\(action):\(retirer ? m : "")", famille: "eva-pub:\(id):") { k in
                        try await modele.client!.publierEvaluation(id, action: action, motif: retirer ? m : nil, cle: k)
                    }
                    info = evaluation.publiee ? "Publiée : les élèves et les familles voient les notes." : "Publication retirée."
                }
            }
        } message: {
            Text(evaluation.publiee ? "Les élèves et les familles ne verront plus ces notes. Un motif écrit est exigé."
                 : "Les élèves et les familles verront aussitôt les notes de « \(evaluation.titre) ».")
        }
    }

    @ViewBuilder private func caseEleve(_ l: LigneEleveEva) -> some View {
        let id = l.eleveId
        let nom = "\(l.nom) \(l.prenom)"
        let a = brouillon.analyse(id)
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                VStack(alignment: .leading) {
                    Text(nom)
                    if let m = l.mouvement { Text(m).font(.footnote) }
                    if brouillon.caseDe(id)?.saisieOuverte == false {
                        Text("Saisie fermée (période arrêtée) : sur le web, avec un motif.").font(.footnote)
                    }
                }
                Spacer()
                TextField("—", text: Binding(get: { brouillon.texte(id) }, set: { brouillon.taper(id, $0) }))
                    .multilineTextAlignment(.trailing)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .frame(maxWidth: 110)
                    .textFieldStyle(.roundedBorder)
                    .disabled(!brouillon.modifiable(id))
                    .accessibilityLabel("Note de \(nom)")
            }
            if brouillon.modifiee(id), case .erreur(let m) = a { MessageErreur(texte: m) }
            else if brouillon.videeSansEffet(id) {
                Text("Une note ne s'efface pas : saisissez un code (abs, nn…) ou corrigez sur le web.").font(.footnote)
            } else if brouillon.modifiee(id), case .statut(let s) = a { Text(SaisieNotes.libelleStatut(s)).font(.footnote) }
            if let r = brouillon.refus(id) { MessageErreur(texte: r) }
            if let k = brouillon.conflit(id) {
                VStack(alignment: .leading) {
                    Text("Modifiée entre-temps" + (k.parNom.map { " par \($0)" } ?? "") + " : "
                         + SaisieNotes.resume(statut: k.statut, valeurM: k.valeurM, baremeM: evaluation.baremeM))
                    HStack {
                        Button("Garder la sienne") { brouillon.reprendreServeur(id) }.buttonStyle(.bordered)
                        Button("Garder la mienne") { brouillon.garderLaMienne(id) }.buttonStyle(.bordered)
                    }
                }
                .padding(8)
                .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 8))
            }
        }
    }
}
#endif
