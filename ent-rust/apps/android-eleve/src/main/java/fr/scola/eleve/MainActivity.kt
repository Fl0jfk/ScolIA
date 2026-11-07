package fr.scola.eleve

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import java.time.LocalDate
import java.util.UUID

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { EleveScreen() }
    }
}

@androidx.compose.runtime.Composable
fun EleveScreen() {
    val client = remember { EntApiClient() }
    val scope = rememberCoroutineScope()
    var email by remember { mutableStateOf("harry@ent.lab") }
    var password by remember { mutableStateOf("DevLocalPass1!") }
    var etab by remember { mutableStateOf("") }
    var connected by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var notes by remember { mutableStateOf<List<EntApiClient.Note>>(emptyList()) }
    var devoirs by remember { mutableStateOf<List<EntApiClient.Devoir>>(emptyList()) }
    var cahier by remember { mutableStateOf<List<EntApiClient.CahierTexte>>(emptyList()) }
    var fils by remember { mutableStateOf<List<EntApiClient.MsgFil>>(emptyList()) }
    var filOuvert by remember { mutableStateOf<EntApiClient.FilDetail?>(null) }
    var edt by remember { mutableStateOf<List<EntApiClient.EdtCreneau>>(emptyList()) }
    var absences by remember { mutableStateOf<List<EntApiClient.Absence>>(emptyList()) }
    var journal by remember { mutableStateOf<List<EntApiClient.JournalEntree>>(emptyList()) }
    var moyennes by remember { mutableStateOf<List<EntApiClient.MoyenneMatiere>>(emptyList()) }
    var periode by remember { mutableStateOf<EntApiClient.NotePeriode?>(null) }
    var appreciations by remember { mutableStateOf<List<EntApiClient.Appreciation>>(emptyList()) }
    var photoBytes by remember { mutableStateOf(0) }

    Column(
        Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
    ) {
        Text("ScolIA Élève", style = MaterialTheme.typography.headlineMedium)
        if (!connected) {
            OutlinedTextField(email, { email = it }, label = { Text("E-mail") })
            OutlinedTextField(password, { password = it }, label = { Text("Mot de passe") })
            OutlinedTextField(etab, { etab = it }, label = { Text("Établissement UUID") })
            Button(onClick = {
                scope.launch {
                    runCatching {
                        val id = UUID.fromString(etab.trim())
                        val s = client.login(email.trim(), password, id)
                        require(s.role == "eleve") { "Compte non élève" }
                        val eid = client.eleveId ?: error("eleve_id manquant")
                        runCatching {
                            client.registerPush("android", "android-eleve-${UUID.randomUUID()}", "scola.eleve.android")
                        }
                        notes = runCatching { client.notes(eid) }.getOrDefault(emptyList())
                        runCatching { client.devoirsCourants(eid) }.getOrNull()?.let { (p, d) ->
                            if (periode == null) periode = p
                            devoirs = d
                        }
                        cahier = runCatching { client.cahierTexte() }.getOrDefault(emptyList())
                        fils = runCatching { client.messages() }.getOrDefault(emptyList())
                        edt = runCatching { client.edt(eid, LocalDate.now().toString()) }.getOrDefault(emptyList())
                        absences = runCatching { client.absences(eid) }.getOrDefault(emptyList())
                        journal = runCatching { client.journal(eid) }.getOrDefault(emptyList())
                        runCatching { client.moyennesCourantes(eid) }.getOrNull()?.let { (p, m) ->
                            periode = p
                            moyennes = m
                        }
                        appreciations = runCatching { client.appreciationsCourantes(eid) }
                            .getOrNull()?.second.orEmpty()
                        photoBytes = runCatching { client.photoEleve(eid).size }.getOrDefault(0)
                        connected = true
                        error = null
                    }.onFailure { error = it.message }
                }
            }) { Text("Entrer") }
            error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        } else {
            Text("Notes: ${notes.size}", style = MaterialTheme.typography.titleMedium)
            notes.forEach { n ->
                Text("${n.matiereLibelle ?: "Matière"} · ${n.libelle ?: "Note"} — ${n.valeur ?: "—"}/${n.sur ?: 20}")
            }
            if (devoirs.isNotEmpty()) {
                Text("Devoirs${periode?.let { " — ${it.libelle}" } ?: ""}", style = MaterialTheme.typography.titleMedium)
                devoirs.forEach { d ->
                    Text("${d.matiereLibelle?.let { "$it · " } ?: ""}${d.titre} · ${d.dateDevoir} · coef ${d.coef} / ${d.sur}")
                }
            }
            if (photoBytes > 0) {
                Text("Photo: $photoBytes octets", style = MaterialTheme.typography.titleMedium)
            }
            if (moyennes.isNotEmpty()) {
                Text("Moyennes${periode?.let { " — ${it.libelle}" } ?: ""}", style = MaterialTheme.typography.titleMedium)
                moyennes.forEach { m ->
                    Text("${m.matiereLibelle ?: "Matière"} — %.1f / 20".format(m.moyenneSur20))
                }
            }
            if (appreciations.isNotEmpty()) {
                Text("Appréciations${periode?.let { " — ${it.libelle}" } ?: ""}", style = MaterialTheme.typography.titleMedium)
                appreciations.forEach { a ->
                    Text("${a.matiereLibelle ?: "Appréciation"} · ${a.texte}")
                }
            }
            Text("Cahier: ${cahier.size}", style = MaterialTheme.typography.titleMedium)
            cahier.forEach { c -> Text("${c.matiereLibelle} — ${c.titre} (pour ${c.pourLe})") }
            Text("EDT: ${edt.size}", style = MaterialTheme.typography.titleMedium)
            edt.forEach { c ->
                Text("${c.heureDebut}–${c.heureFin} ${c.matiereLibelle}${c.profLabel?.let { " · $it" } ?: ""}")
            }
            Text("Absences: ${absences.size}", style = MaterialTheme.typography.titleMedium)
            absences.forEach { a ->
                Text("${a.debut} → ${a.fin} · ${a.motif} · ${a.statut}${a.motifJustif?.let { " — $it" } ?: ""}")
            }
            Text("Journal: ${journal.size}", style = MaterialTheme.typography.titleMedium)
            journal.forEach { j ->
                Text("${j.libelle} · ${j.domaine} · ${j.createdAt?.take(16) ?: "—"}")
            }
            Text("Messages: ${fils.size}", style = MaterialTheme.typography.titleMedium)
            fils.forEach { f ->
                Text(
                    f.sujet,
                    modifier = Modifier.clickable {
                        scope.launch {
                            filOuvert = runCatching { client.filDetail(f.id) }.getOrNull()
                                ?: filOuvert
                        }
                    },
                )
            }
            filOuvert?.let { detail ->
                Text("Fil: ${detail.fil.sujet}", style = MaterialTheme.typography.titleSmall)
                detail.messages.forEach { m ->
                    m.auteurLabel?.let { Text(it, style = MaterialTheme.typography.labelSmall) }
                    Text(m.corps)
                    m.pieces.forEach { p ->
                        Text(
                            "📎 ${p.nom}",
                            modifier = Modifier.clickable {
                                scope.launch {
                                    runCatching {
                                        val bytes = client.telechargerPieceMessage(p.id)
                                        error = "PJ ${p.nom} : ${bytes.size} octets"
                                    }.onFailure { error = it.message }
                                }
                            },
                        )
                    }
                    Button(onClick = {
                        scope.launch {
                            runCatching {
                                client.attacherPieceMessage(
                                    m.id,
                                    "note.txt",
                                    "text/plain",
                                    "pièce jointe démo".toByteArray(),
                                )
                                filOuvert = client.filDetail(detail.fil.id)
                            }.onFailure { error = it.message }
                        }
                    }) { Text("Joindre note.txt") }
                }
                Button(onClick = {
                    scope.launch {
                        runCatching {
                            client.repondre(detail.fil.id, "Bien reçu, merci.")
                            filOuvert = client.filDetail(detail.fil.id)
                            fils = client.messages()
                        }.onFailure { error = it.message }
                    }
                }) { Text("Répondre « Bien reçu »") }
            }
            error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        }
    }
}
