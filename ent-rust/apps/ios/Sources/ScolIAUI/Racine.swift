#if os(iOS)
import ScolIACore
import SwiftUI

/// Aiguillage : rattachement → connexion → verrou → enrôlement → onglets selon le rôle.
public struct RacineScolIA: View {
    @Bindable var modele: AppModele
    @Environment(\.scenePhase) private var phase
    @State private var arrierePlan: Date?

    public init(modele: AppModele) { self.modele = modele }

    public var body: some View {
        Group {
            if modele.actif == nil {
                EcranRattachement(modele: modele)
            } else {
                switch modele.etat {
                case .deconnecte, .secondFacteur:
                    EcranConnexion(modele: modele)
                case .connecte(let me, let espace):
                    if modele.verrouille {
                        EcranVerrou(modele: modele)
                    } else if espace.enrolementRequis {
                        EcranEnrolement(modele: modele)
                    } else {
                        Coquille(modele: modele, me: me, espace: espace)
                    }
                }
            }
        }
        // Lien d'invitation (Universal Link https://<etab>.scolia.fr/rejoindre?invitation=…).
        .onOpenURL { modele.rattacher($0.absoluteString) }
        .onChange(of: phase) { _, nouvelle in
            switch nouvelle {
            case .background: arrierePlan = Date()
            case .active:
                if let t = arrierePlan, Date().timeIntervalSince(t) > Verrou.delai { modele.verrouille = true }
                arrierePlan = nil
            default: break
            }
        }
        // Masque le contenu dans le sélecteur d'applis (données d'élèves).
        .overlay { if phase != .active { Color(.systemBackground).ignoresSafeArea() } }
    }
}

struct EcranRattachement: View {
    @Bindable var modele: AppModele
    @State private var saisie = ""
    @State private var scanner = false

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Text("Ouvrez le lien d'invitation reçu par e-mail ou SMS, ou scannez le QR code remis par l'établissement.")
                    Button("Scanner un QR code") { scanner = true }
                }
                Section("Adresse de l'établissement") {
                    TextField("lpnb.scolia.fr", text: $saisie)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)
                    Button("Continuer") { modele.rattacher(saisie) }
                        .disabled(saisie.isEmpty || modele.occupe)
                    // Recherche par nom ou ville : API d'annuaire public manquante (docs/MOBILE.md).
                    Text("Recherche par nom ou ville : bientôt disponible.").font(.footnote).foregroundStyle(.secondary)
                }
                if let e = modele.erreur { Section { MessageErreur(texte: e) } }
                if !modele.rattachements.isEmpty {
                    Section("Mes établissements") {
                        ForEach(modele.rattachements) { r in Button(r.nom ?? r.slug) { modele.choisir(r) } }
                    }
                }
            }
            .navigationTitle("Rejoindre")
            .sheet(isPresented: $scanner) {
                ScannerQR { code in
                    scanner = false
                    modele.rattacher(code)
                }
                .ignoresSafeArea()
            }
        }
    }
}

struct EcranConnexion: View {
    @Bindable var modele: AppModele
    @State private var email = ""
    @State private var mdp = ""
    @State private var avecMotDePasse = false

    var body: some View {
        NavigationStack {
            Form {
                if case .secondFacteur(let defi) = modele.etat {
                    Section {
                        Text("Mot de passe accepté. Confirmez avec votre passkey.")
                        Button("Utiliser ma passkey") { modele.secondFacteur(defi) }.disabled(modele.occupe)
                    }
                } else {
                    Section {
                        TextField("Adresse e-mail", text: $email)
                            .textContentType(.username)
                            .keyboardType(.emailAddress)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                        Button("Se connecter avec une passkey") { modele.connexionPasskey(email) }
                            .disabled(email.isEmpty || modele.occupe)
                    }
                    Section {
                        if avecMotDePasse {
                            SecureField("Mot de passe", text: $mdp).textContentType(.password)
                            Button("Se connecter") {
                                modele.connexionMotDePasse(email, mdp)
                                mdp = ""
                            }
                            .disabled(email.isEmpty || mdp.isEmpty || modele.occupe)
                        } else {
                            Button("Pas encore de passkey ? Utiliser mon mot de passe") { avecMotDePasse = true }
                        }
                    }
                }
                if let e = modele.erreur { Section { MessageErreur(texte: e) } }
                Section {
                    Button("Changer d'établissement", role: .destructive) { if let a = modele.actif { modele.oublier(a) } }
                }
            }
            .navigationTitle(modele.actif?.nom ?? modele.actif?.slug ?? "ScolIA")
            .onAppear { if email.isEmpty { email = modele.dernierEmail } }
            .overlay { if modele.occupe { ProgressView().accessibilityLabel("Connexion en cours") } }
        }
    }
}

struct EcranVerrou: View {
    @Bindable var modele: AppModele

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "lock.fill").font(.largeTitle).accessibilityHidden(true)
            Text("ScolIA est verrouillée").font(.title2).accessibilityAddTraits(.isHeader)
            Button("Déverrouiller") { Task { await deverrouiller() } }.buttonStyle(.borderedProminent)
        }
        .task { await deverrouiller() }
    }

    private func deverrouiller() async {
        if await Verrou.deverrouiller() { modele.verrouille = false }
    }
}

/// Passkey obligatoire (D-009) : seul écran accessible tant qu'elle n'est pas créée.
struct EcranEnrolement: View {
    @Bindable var modele: AppModele
    @State private var mdp = ""

    var body: some View {
        NavigationStack {
            Form {
                Text("Votre établissement demande une passkey : elle remplace le mot de passe et protège votre compte.")
                SecureField("Mot de passe actuel", text: $mdp).textContentType(.password)
                Button("Créer ma passkey") {
                    modele.enroler(mdp)
                    mdp = ""
                }
                .disabled(mdp.isEmpty || modele.occupe)
                if let e = modele.erreur { MessageErreur(texte: e) }
                Button("Se déconnecter", role: .destructive) { modele.deconnexion() }
            }
            .navigationTitle("Créer votre passkey")
        }
    }
}

/// Onglets selon le rôle. Au-delà de 5, iOS regroupe automatiquement dans « Plus ».
struct Coquille: View {
    @Bindable var modele: AppModele
    let me: Me
    let espace: Espace
    @State private var courant: Module = .accueil

    var body: some View {
        TabView(selection: $courant) {
            ForEach(espace.modules) { m in
                NavigationStack {
                    EcranModule(module: m, modele: modele, me: me, espace: espace, ouvrir: { courant = $0 })
                        .navigationTitle(m.titre)
                }
                .tabItem { Label(m.titre, systemImage: m.symbole) }
                .tag(m)
            }
        }
    }
}

struct MessageErreur: View {
    let texte: String
    var body: some View {
        Text(texte)
            .foregroundStyle(.red)
            .accessibilityAddTraits(.updatesFrequently)
            .onAppear { UIAccessibility.post(notification: .announcement, argument: texte) }
    }
}
#endif
