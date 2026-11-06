package fr.scolia.mobile.app.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import fr.scolia.mobile.core.EnfantFamille
import fr.scolia.mobile.core.ErreurApi
import fr.scolia.mobile.core.Lecture
import java.text.DateFormat
import java.util.Date

private val Bleu = Color(0xFF1D4E89)

/** Thème Material 3 ; tailles de texte du système respectées (pas de taille figée). */
@Composable
fun ThemeScolIA(contenu: @Composable () -> Unit) {
    val couleurs = if (isSystemInDarkTheme()) {
        darkColorScheme(primary = Color(0xFF9CC2FF))
    } else {
        lightColorScheme(primary = Bleu)
    }
    MaterialTheme(colorScheme = couleurs) {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) { contenu() }
    }
}

@Composable
fun Titre(texte: String, modifier: Modifier = Modifier) {
    Text(
        texte,
        style = MaterialTheme.typography.titleLarge,
        modifier = modifier.padding(vertical = 8.dp).semantics { heading() },
    )
}

/** Message d'erreur annoncé par les lecteurs d'écran (TalkBack). */
@Composable
fun MessageErreur(texte: String?) {
    if (texte == null) return
    Text(
        texte,
        color = MaterialTheme.colorScheme.error,
        modifier = Modifier.padding(vertical = 8.dp).semantics { liveRegion = LiveRegionMode.Polite },
    )
}

@Composable
fun BandeauHorsLigne(horodatageMs: Long) {
    val quand = DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(horodatageMs))
    Card(Modifier.fillMaxWidth().padding(bottom = 8.dp)) {
        Text(
            "Hors ligne — données du $quand, en lecture seule.",
            modifier = Modifier.padding(12.dp).semantics { liveRegion = LiveRegionMode.Polite },
        )
    }
}

/**
 * Charge une donnée (réseau, cache en secours) et gère attente, erreur, hors ligne
 * et nouvel essai. `cle` relance le chargement quand elle change.
 */
@Composable
fun <T> Chargeur(
    cle: Any?,
    surSessionPerdue: () -> Unit,
    charger: suspend () -> Lecture<T>,
    contenu: @Composable (T, recharger: () -> Unit) -> Unit,
) {
    var essai by remember { mutableIntStateOf(0) }
    var lecture by remember(cle) { mutableStateOf<Lecture<T>?>(null) }
    var erreur by remember(cle) { mutableStateOf<String?>(null) }
    LaunchedEffect(cle, essai) {
        erreur = null
        try {
            lecture = charger()
        } catch (e: ErreurApi.NonAuthentifie) {
            surSessionPerdue()
        } catch (e: kotlinx.coroutines.CancellationException) {
            throw e // « Réessayer » ou changement de clé : pas un message d'erreur.
        } catch (e: Exception) {
            erreur = e.message ?: "Chargement impossible."
        }
    }
    val l = lecture
    Column {
        if (l?.horsLigne == true) BandeauHorsLigne(l.horodatageMs)
        when {
            erreur != null -> Column {
                MessageErreur(erreur)
                Button(onClick = { essai++ }) { Text("Réessayer") }
            }
            l == null -> Box(Modifier.fillMaxWidth().padding(32.dp), Alignment.Center) {
                CircularProgressIndicator(Modifier.semantics { contentDescription = "Chargement" })
            }
            else -> contenu(l.valeur) { essai++ }
        }
    }
}

/** Sélecteur d'enfant (famille) : puces horizontales, cible tactile ≥ 48 dp. */
@Composable
fun ChoixEnfant(enfants: List<EnfantFamille>, choisi: EnfantFamille?, surChoix: (EnfantFamille) -> Unit) {
    if (enfants.size <= 1) return
    LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(bottom = 8.dp)) {
        items(enfants, key = { it.id }) { e ->
            FilterChip(
                selected = e.id == choisi?.id,
                onClick = { surChoix(e) },
                label = { Text(e.prenom) },
                modifier = Modifier.heightIn(min = 48.dp),
            )
        }
    }
}

@Composable
fun Ligne(titre: String, detail: String?, droite: String? = null, modifier: Modifier = Modifier) {
    Row(modifier.fillMaxWidth().heightIn(min = 48.dp).padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(titre, style = MaterialTheme.typography.bodyLarge)
            if (!detail.isNullOrBlank()) Text(detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (droite != null) Text(droite, style = MaterialTheme.typography.titleMedium)
    }
}

/** Date ISO `2026-11-06` → `06/11/2026` (affichage). */
fun dateFr(iso: String): String {
    val d = iso.take(10).split('-')
    return if (d.size == 3) "${d[2]}/${d[1]}/${d[0]}" else iso
}

fun note(v: Double): String = if (v % 1.0 == 0.0) v.toInt().toString() else "%.2f".format(v).trimEnd('0').trimEnd(',', '.')
