#if os(iOS)
import ScolIACore
import SwiftUI

/// « Mes autres comptes », ouvert depuis « Mon compte » (transverse individu, `TRANSVERSE_INDIVIDU.md` §11) : ce que
/// l'établissement reconnaît, les demandes reçues (confirmer ou refuser, chacun après une confirmation qui rappelle
/// l'effet) et la déclaration d'un autre compte (réponse neutre). Mêmes textes que le web et Android. Jamais mis en
/// cache hors ligne.
struct EcranMesAutresComptes: View {
    let modele: AppModele
    @State private var rechargement = 0
    @State private var envoi = false
    @State private var annonce: String?
    @State private var erreur: String?
    @State private var aConfirmer: DeclarationRecue?
    @State private var aRefuser: DeclarationRecue?
    @State private var email = ""
    @State private var erreurEmail: String?
    @FocusState private var champEmail: Bool

    var body: some View {
        List {
            Chargeur(modele: modele, cle: "individu-moi#\(rechargement)", charger: {
                Lecture(valeur: try await modele.client!.individuMoi(), horsLigne: false, horodatage: Date())
            }) { moi, _ in
                Section("Ce que l'établissement reconnaît") {
                    Text(Individu.resumeMoi(moi))
                    if let f = Individu.resumeFiches(moi) { Text(f) }
                    ForEach(Array(Individu.trierLiens(moi.liens).enumerated()), id: \.offset) { _, l in
                        Ligne(titre: Individu.libelleOrigine(l.origine), detail: Individu.detailLien(l))
                    }
                    if !moi.liens.isEmpty { Text(Individu.lienFaux).font(.footnote) }
                }
                Section("Demandes à confirmer") {
                    let demandes = Individu.trierDemandes(moi.declarationsRecues)
                    if demandes.isEmpty {
                        Text(Individu.aucuneDemande).font(.footnote)
                    } else {
                        Text(Individu.introDemandes)
                        ForEach(demandes) { d in
                            VStack(alignment: .leading, spacing: 8) {
                                Ligne(titre: "Compte « \(d.declarant) »", detail: Individu.detailDemande(d))
                                HStack {
                                    Button("C'est moi") { aConfirmer = d }
                                        .buttonStyle(.borderedProminent)
                                        .accessibilityLabel("Confirmer : le compte \(d.declarant) est le mien")
                                    Button("Ce n'est pas moi") { aRefuser = d }
                                        .buttonStyle(.bordered)
                                        .accessibilityLabel("Refuser la demande du compte \(d.declarant)")
                                }
                                .disabled(envoi)
                            }
                        }
                    }
                }
            }
            if annonce != nil || erreur != nil {
                Section {
                    if let a = annonce { Text(a) }
                    if let e = erreur { MessageErreur(texte: e) }
                }
            }
            Section {
                Text(Individu.aideDeclarer).font(.footnote)
                TextField("Adresse e-mail de l'autre compte", text: $email)
                    .keyboardType(.emailAddress)
                    .textContentType(.emailAddress)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .submitLabel(.send)
                    .focused($champEmail)
                    .onSubmit(declarer)
                    .onChange(of: email) { _, _ in erreurEmail = nil }
                if let e = erreurEmail { MessageErreur(texte: e) }
                Button("Envoyer la demande", action: declarer).disabled(envoi)
            } header: {
                Text("Déclarer un autre compte")
            }
            Section("Pourquoi ?") {
                Text(Individu.pourquoi).font(.footnote)
            }
        }
        .navigationTitle("Mes autres comptes")
        .confirmationDialog("Confirmer votre autre compte", isPresented: present($aConfirmer), titleVisibility: .visible,
                            presenting: aConfirmer) { d in
            Button("Oui, c'est mon compte") { repondre(d, confirmer: true) }
            Button("Annuler", role: .cancel) {}
        } message: { d in
            Text("Confirmez-vous que le compte « \(d.declarant) » est bien le vôtre ? \(Individu.effetLien)")
        }
        // Sur le web, le refus part sans confirmation ; sur un téléphone, un appui de travers est plus probable et le
        // refus bloque ce compte 30 jours : on demande.
        .confirmationDialog("Refuser cette demande ?", isPresented: present($aRefuser), titleVisibility: .visible,
                            presenting: aRefuser) { d in
            Button("Refuser", role: .destructive) { repondre(d, confirmer: false) }
            Button("Annuler", role: .cancel) {}
        } message: { d in
            Text("Le compte « \(d.declarant) » n'est pas le vôtre ? \(Individu.effetRefus)")
        }
    }

    private func present<T>(_ b: Binding<T?>) -> Binding<Bool> {
        Binding(get: { b.wrappedValue != nil }, set: { if !$0 { b.wrappedValue = nil } })
    }

    /// Une écriture à la fois ; le résultat est annoncé à VoiceOver ; la session perdue ramène à la connexion.
    private func ecrire(_ intention: String, succes: String, _ bloc: @escaping (String) async throws -> Void, puis: @escaping () -> Void) {
        guard !envoi else { return }
        envoi = true
        annonce = nil
        erreur = nil
        Task { @MainActor in
            defer { envoi = false }
            do {
                try await modele.cles.executer(intention) { k in try await bloc(k) }
                annonce = succes
                UIAccessibility.post(notification: .announcement, argument: succes)
                puis()
            } catch ErreurApi.nonAuthentifie {
                modele.sessionPerdue()
            } catch {
                erreur = error.localizedDescription
                UIAccessibility.post(notification: .announcement, argument: error.localizedDescription)
            }
        }
    }

    private func repondre(_ d: DeclarationRecue, confirmer: Bool) {
        ecrire(Individu.intentionRepondre(d.id, confirmer: confirmer), succes: Individu.messageReponse(confirmer: confirmer), { k in
            _ = try await modele.client!.repondreDeclaration(d.id, confirmer: confirmer, cle: k)
        }, puis: { rechargement += 1 })
    }

    private func declarer() {
        switch Individu.adresseAEnvoyer(email) {
        case .failure(let e):
            erreurEmail = e.localizedDescription
            champEmail = true
        case .success(let e):
            erreurEmail = nil
            ecrire(Individu.intentionDeclarer(e), succes: Individu.messageDeclaration, { k in
                try await modele.client!.declarerAutreCompte(email: e, cle: k)
            }, puis: { email = "" }) // effacée seulement après l'accord du serveur
        }
    }
}
#endif
