import SwiftUI
import ScolIACore

private func direct<T>(_ v: T) -> Lecture<T> { Lecture(valeur: v, horsLigne: false, horodatage: Date()) }

/// Signaler un incident (registre des violations de données, socle 27, complément G5), miroir de `Signaler.kt` :
/// tout personnel déclare en trente secondes puis suit l'avancement. La qualification, le risque, la notification
/// à la CNIL et le registre restent sur le web (D-003). Envoi après confirmation (la direction est alertée).
struct EcranSignaler: View {
    let modele: AppModele
    @State private var intitule = ""
    @State private var precisions = ""
    @State private var sansDonnee = false
    @State private var tente = false
    @State private var confirmer = false
    @State private var envoi = false
    @State private var erreur: String?
    @State private var recu: MaDeclaration?
    @State private var rechargement = 0

    private var erreurIntitule: String? { Signalement.erreurIntitule(intitule) }
    private var erreurPrecisions: String? { Signalement.erreurDescription(precisions) }

    var body: some View {
        Form {
            Section {
                Text("Une donnée personnelle perdue, envoyée à la mauvaise personne ou consultée sans droit ? Signalez-le tout de suite : la direction a 72 heures pour agir. Aucune suite ne vise la personne qui signale.")
                    .font(.subheadline)
            }
            if let r = recu {
                Section {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Signalement \(r.reference) enregistré").font(.headline)
                        Text(r.message.isEmpty ? "Merci : la direction est prévenue." : r.message)
                    }
                    .accessibilityElement(children: .combine)
                }
            }
            Section("Que s'est-il passé ?") {
                TextField("En une phrase", text: $intitule, axis: .vertical).lineLimit(1...3)
                if tente, let e = erreurIntitule { Text(e).font(.footnote).foregroundStyle(.red) }
                TextField("Précisions (facultatif)", text: $precisions, axis: .vertical).lineLimit(3...8)
                Text(tente ? (erreurPrecisions ?? "") : "Ce qui est touché, combien de personnes à peu près, ce que vous avez déjà fait. N'y recopiez pas les données elles-mêmes.")
                    .font(.footnote).foregroundStyle(tente && erreurPrecisions != nil ? .red : .secondary)
                Toggle("Aucune donnée personnelle n'est touchée (tentative d'hameçonnage, virus bloqué…)", isOn: $sansDonnee)
                if let e = erreur { MessageErreur(texte: e) }
                Button("Signaler") {
                    tente = true
                    if erreurIntitule == nil && erreurPrecisions == nil { confirmer = true }
                }
                .disabled(envoi)
            }
            Section("Mes signalements") {
                Chargeur(modele: modele, cle: "declarations#\(rechargement)", charger: { direct(try await modele.client!.mesDeclarations()) }) { liste, _ in
                    if liste.isEmpty { Text("Vous n'avez rien signalé dans cet établissement.") }
                    ForEach(liste) { d in
                        Ligne(titre: "\(d.reference) · \(d.intitule)", detail: "Signalé le \(dateFr(d.connueLe)) · \(Signalement.libelleAvancement(d.avancement))")
                    }
                }
            }
        }
        // Envoi : la direction est alertée tout de suite, confirmation humaine.
        .confirmationDialog("Prévenir la direction ?", isPresented: $confirmer, titleVisibility: .visible) {
            Button("Envoyer") {
                let corps = Signalement.corps(intitule: intitule, description: precisions, sansDonneePersonnelle: sansDonnee)
                envoi = true
                erreur = nil
                recu = nil
                Task { @MainActor in
                    defer { envoi = false }
                    do {
                        recu = try await modele.cles.executer(Signalement.intention(corps)) { k in try await modele.client!.declarerIncident(corps, cle: k) }
                        intitule = ""
                        precisions = ""
                        sansDonnee = false
                        tente = false
                        rechargement += 1
                    } catch ErreurApi.nonAuthentifie {
                        modele.sessionPerdue()
                    } catch ErreurApi.appliTropAncienne(let m) {
                        modele.miseAJourRequise = m
                    } catch ErreurApi.horsLigne {
                        erreur = "Pas de connexion : rien n'est parti. Réessayez dès que possible ; en cas d'urgence, appelez la direction."
                    } catch {
                        erreur = error.localizedDescription
                    }
                }
            }
        } message: {
            Text("« \(String(intitule.prefix(200))) » sera enregistré au registre de l'établissement et la direction sera alertée tout de suite.")
        }
        .navigationTitle("Signaler un incident")
    }
}
