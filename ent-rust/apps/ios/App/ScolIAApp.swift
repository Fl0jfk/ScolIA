import ScolIAUI
import SwiftUI

/// Point d'entrée de l'appli iOS unique ScolIA (D-003).
@main
struct ScolIAApp: App {
    #if DEBUG
    // Labo local (`http://127.0.0.1:3001`) accepté uniquement en Debug.
    @State private var modele = AppModele(modeLabo: true)
    #else
    @State private var modele = AppModele(modeLabo: false)
    #endif

    var body: some Scene {
        WindowGroup {
            RacineScolIA(modele: modele)
        }
    }
}
