package fr.scolia.mobile.app

import android.app.Application
import android.content.Intent
import android.os.Bundle
import android.os.SystemClock
import android.view.WindowManager
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.fragment.app.FragmentActivity
import androidx.lifecycle.lifecycleScope
import fr.scolia.mobile.app.securite.Verrou
import fr.scolia.mobile.app.ui.RacineScolIA
import fr.scolia.mobile.app.ui.ThemeScolIA
import kotlinx.coroutines.launch

class ScolIAApplication : Application()

/**
 * Activité unique. `FragmentActivity` pour `BiometricPrompt`.
 * `FLAG_SECURE` : pas de capture d'écran ni d'aperçu dans les applis récentes
 * (données d'élèves affichées).
 */
class MainActivity : FragmentActivity() {
    private val modele: AppModele by viewModels()
    private var miseEnArrierePlan = 0L

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE)
        enableEdgeToEdge()
        traiterLien(intent)
        setContent {
            ThemeScolIA { RacineScolIA(modele, onDeverrouiller = ::deverrouiller) }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        traiterLien(intent)
    }

    override fun onResume() {
        super.onResume()
        ActiviteCourante.definir(this)
        val delai = SystemClock.elapsedRealtime() - miseEnArrierePlan
        if (miseEnArrierePlan == 0L || delai > Verrou.DELAI_MS) modele.verrouille = true
    }

    override fun onPause() {
        super.onPause()
        miseEnArrierePlan = SystemClock.elapsedRealtime()
        ActiviteCourante.definir(null)
    }

    private fun deverrouiller() {
        val v = Verrou(this)
        if (!v.disponible()) {
            // Appareil sans verrouillage : on n'empêche pas l'usage (le jeton reste chiffré).
            modele.verrouille = false
            return
        }
        lifecycleScope.launch { if (v.deverrouiller()) modele.verrouille = false }
    }

    /** Lien d'invitation (App Link vérifié `https://<etab>.scolia.fr/rejoindre?invitation=…`). */
    private fun traiterLien(intent: Intent?) {
        val data = intent?.data ?: return
        if (intent.action == Intent.ACTION_VIEW) modele.rattacher(data.toString())
    }
}
