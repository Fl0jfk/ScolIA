package fr.scola.famille

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import java.util.UUID

/**
 * Écran famille — Compose : login, push, enfants, résumé, messagerie, sorties.
 */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                FamilleScreen()
            }
        }
    }
}

@Composable
fun FamilleScreen(client: EntApiClient = remember { EntApiClient() }) {
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var etab by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var enfants by remember { mutableStateOf<List<EntApiClient.Enfant>>(emptyList()) }
    var selected by remember { mutableStateOf<UUID?>(null) }
    var resume by remember { mutableStateOf<EntApiClient.Resume?>(null) }
    var sortie by remember { mutableStateOf<EntApiClient.RecapSortie?>(null) }
    var docsASigner by remember { mutableStateOf<List<EntApiClient.DocASigner>>(emptyList()) }
    var menus by remember { mutableStateOf<List<EntApiClient.MenuCantine>>(emptyList()) }
    var passagesRepas by remember { mutableStateOf<List<EntApiClient.PassageRepas>>(emptyList()) }
    var factures by remember { mutableStateOf<List<EntApiClient.Facture>>(emptyList()) }
    var quittances by remember { mutableStateOf<List<EntApiClient.Quittance>>(emptyList()) }
    var cahier by remember { mutableStateOf<List<EntApiClient.CahierTexte>>(emptyList()) }
    var alertes by remember { mutableStateOf<List<EntApiClient.AlerteCpe>>(emptyList()) }
    var internat by remember { mutableStateOf<EntApiClient.RecapInternat?>(null) }
    var conseils by remember { mutableStateOf<List<EntApiClient.ConseilDossier>>(emptyList()) }
    var ficheSante by remember { mutableStateOf<EntApiClient.FicheSante?>(null) }
    var pai by remember { mutableStateOf<EntApiClient.Pai?>(null) }
    var paiPieces by remember { mutableStateOf<List<EntApiClient.PaiPiece>>(emptyList()) }
    var dispensesVs by remember { mutableStateOf<List<EntApiClient.DispenseVs>>(emptyList()) }
    var voeux by remember { mutableStateOf<List<EntApiClient.Voeu>>(emptyList()) }
    var infoComp by remember { mutableStateOf<List<EntApiClient.InfoCompValeur>>(emptyList()) }
    var dispensesRepas by remember { mutableStateOf<List<EntApiClient.DispenseRepas>>(emptyList()) }
    var activiteRepasId by remember { mutableStateOf<UUID?>(null) }
    var pointsVs by remember { mutableStateOf<EntApiClient.SoldePoints?>(null) }
    var sanctions by remember { mutableStateOf<List<EntApiClient.Sanction>>(emptyList()) }
    var incidents by remember { mutableStateOf<List<EntApiClient.Incident>>(emptyList()) }
    var motifsAbsence by remember { mutableStateOf<List<EntApiClient.AbsenceMotif>>(emptyList()) }
    var commissions by remember { mutableStateOf<List<EntApiClient.CommissionDossier>>(emptyList()) }
    var dossierPsy by remember { mutableStateOf<EntApiClient.DossierPsy?>(null) }
    var rdvsPsy by remember { mutableStateOf<List<EntApiClient.RdvPsy>>(emptyList()) }
    var reinscription by remember { mutableStateOf<EntApiClient.ReinscriptionFamille?>(null) }
    var passagesInfirmerie by remember { mutableStateOf<List<EntApiClient.PassageInfirmerie>>(emptyList()) }
    var periodeNotes by remember { mutableStateOf<EntApiClient.NotePeriode?>(null) }
    var moyennes by remember { mutableStateOf<List<EntApiClient.MoyenneMatiere>>(emptyList()) }
    var appreciations by remember { mutableStateOf<List<EntApiClient.Appreciation>>(emptyList()) }
    var photoBytes by remember { mutableStateOf(0) }
    var trombi by remember { mutableStateOf<List<EntApiClient.TrombiEntree>>(emptyList()) }
    var conventions by remember { mutableStateOf<List<EntApiClient.ConventionStage>>(emptyList()) }
    var journal by remember { mutableStateOf<List<EntApiClient.JournalEntree>>(emptyList()) }
    var edt by remember { mutableStateOf<List<EntApiClient.EdtCreneau>>(emptyList()) }
    var fils by remember { mutableStateOf<List<EntApiClient.MsgFil>>(emptyList()) }
    var filOuvert by remember { mutableStateOf<EntApiClient.FilDetail?>(null) }
    var pdfStatus by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    fun weekBounds(): Pair<String, String> {
        val fmt = java.time.format.DateTimeFormatter.ISO_LOCAL_DATE
        val debut = java.time.LocalDate.now()
        return debut.format(fmt) to debut.plusDays(6).format(fmt)
    }
    fun passagesBounds(): Pair<String, String> {
        val fmt = java.time.format.DateTimeFormatter.ISO_LOCAL_DATE
        val fin = java.time.LocalDate.now()
        return fin.minusDays(14).format(fmt) to fin.format(fmt)
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("ScolIA", style = MaterialTheme.typography.headlineLarge)
        if (enfants.isEmpty()) {
            OutlinedTextField(email, { email = it }, label = { Text("E-mail") })
            OutlinedTextField(password, { password = it }, label = { Text("Mot de passe") })
            OutlinedTextField(etab, { etab = it }, label = { Text("Établissement UUID") })
            error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            Button(onClick = {
                scope.launch {
                    try {
                        val id = UUID.fromString(etab.trim())
                        client.login(email.trim(), password, id)
                        runCatching {
                            client.registerPush(
                                "android",
                                "android-dev-${UUID.randomUUID()}",
                                "scola.famille.android",
                            )
                        }
                        enfants = client.enfants()
                        fils = runCatching { client.messages() }.getOrDefault(emptyList())
                        motifsAbsence = runCatching { client.motifsAbsence() }.getOrDefault(emptyList())
                        factures = runCatching { client.facturesFamille() }.getOrDefault(emptyList())
                        quittances = runCatching { client.quittancesFamille() }.getOrDefault(emptyList())
                        reinscription = runCatching { client.reinscriptionFamille() }.getOrNull()
                        val (d, f) = weekBounds()
                        menus = runCatching { client.menusCantine(d, f) }.getOrDefault(emptyList())
                        enfants.firstOrNull()?.let {
                            selected = it.id
                            resume = runCatching { client.resume(it.id) }.getOrNull()
                            sortie = runCatching { client.recapSortie(it.id) }.getOrNull()
                            docsASigner = runCatching { client.docsASigner(it.id) }.getOrDefault(emptyList())
                            conventions = runCatching { client.conventionsStage(it.id) }.getOrDefault(emptyList())
                            journal = runCatching { client.journal(it.id) }.getOrDefault(emptyList())
                            edt = runCatching {
                                client.edt(it.id, java.time.LocalDate.now().toString())
                            }.getOrDefault(emptyList())
                            val (pd, pf) = passagesBounds()
                            passagesRepas = runCatching {
                                client.passagesRepas(it.id, pd, pf)
                            }.getOrDefault(emptyList())
                            cahier = runCatching { client.cahierTexte(it.id) }.getOrDefault(emptyList())
                            alertes = runCatching { client.alertesCpe(it.id) }.getOrDefault(emptyList())
                            internat = runCatching {
                                client.internat(it.id, java.time.LocalDate.now().toString())
                            }.getOrNull()
                            conseils = runCatching { client.conseils(it.id) }.getOrDefault(emptyList())
                            ficheSante = runCatching { client.ficheSante(it.id) }.getOrNull()
                            pai = runCatching { client.pai(it.id) }.getOrNull()
                            paiPieces = runCatching { client.paiPieces(it.id) }.getOrDefault(emptyList())
                            dispensesVs = runCatching { client.dispensesVs(it.id) }.getOrDefault(emptyList())
                            voeux = runCatching { client.voeux(it.id) }.getOrDefault(emptyList())
                            infoComp = runCatching { client.infoComp(it.id) }.getOrDefault(emptyList())
                            dispensesRepas = runCatching { client.dispensesRepas(it.id) }.getOrDefault(emptyList())
                            activiteRepasId = runCatching { client.activiteRepasMidi(it.id) }.getOrNull()
                            pointsVs = runCatching { client.pointsVs(it.id) }.getOrNull()
                            sanctions = runCatching { client.sanctions(it.id) }.getOrDefault(emptyList())
                            incidents = runCatching { client.incidents(it.id) }.getOrDefault(emptyList())
                            commissions = runCatching { client.commissions(it.id) }.getOrDefault(emptyList())
                                .filter { c -> c.signaleFamille }
                            dossierPsy = runCatching { client.dossierPsy(it.id) }.getOrNull()
                            rdvsPsy = runCatching { client.rdvsPsy(it.id) }.getOrDefault(emptyList())
                                .filter { r -> r.signaleFamille }
                            passagesInfirmerie = runCatching { client.passagesInfirmerie(it.id) }.getOrDefault(emptyList())
                            runCatching { client.moyennesCourantes(it.id) }.getOrNull()?.let { (p, m) ->
                                periodeNotes = p
                                moyennes = m
                            }
                            appreciations = runCatching { client.appreciationsCourantes(it.id) }
                                .getOrNull()?.second.orEmpty()
                            photoBytes = runCatching { client.photoEleve(it.id).size }.getOrDefault(0)
                            trombi = it.classe?.let { c ->
                                runCatching { client.trombi(c) }.getOrDefault(emptyList())
                            }.orEmpty()
                        }
                        error = null
                    } catch (e: Exception) {
                        error = e.message
                    }
                }
            }) {
                Text("Se connecter")
            }
        } else {
            Text("Mes enfants", style = MaterialTheme.typography.titleLarge)
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(enfants, key = { it.id }) { e ->
                    Column(
                        Modifier.clickable {
                            scope.launch {
                                selected = e.id
                                resume = runCatching { client.resume(e.id) }.getOrNull()
                                sortie = runCatching { client.recapSortie(e.id) }.getOrNull()
                                docsASigner = runCatching { client.docsASigner(e.id) }.getOrDefault(emptyList())
                                conventions = runCatching { client.conventionsStage(e.id) }.getOrDefault(emptyList())
                                journal = runCatching { client.journal(e.id) }.getOrDefault(emptyList())
                                edt = runCatching {
                                    client.edt(e.id, java.time.LocalDate.now().toString())
                                }.getOrDefault(emptyList())
                                val (pd, pf) = passagesBounds()
                                passagesRepas = runCatching {
                                    client.passagesRepas(e.id, pd, pf)
                                }.getOrDefault(emptyList())
                                cahier = runCatching { client.cahierTexte(e.id) }.getOrDefault(emptyList())
                                alertes = runCatching { client.alertesCpe(e.id) }.getOrDefault(emptyList())
                                internat = runCatching {
                                    client.internat(e.id, java.time.LocalDate.now().toString())
                                }.getOrNull()
                                conseils = runCatching { client.conseils(e.id) }.getOrDefault(emptyList())
                                ficheSante = runCatching { client.ficheSante(e.id) }.getOrNull()
                                pai = runCatching { client.pai(e.id) }.getOrNull()
                                paiPieces = runCatching { client.paiPieces(e.id) }.getOrDefault(emptyList())
                                dispensesVs = runCatching { client.dispensesVs(e.id) }.getOrDefault(emptyList())
                                voeux = runCatching { client.voeux(e.id) }.getOrDefault(emptyList())
                                infoComp = runCatching { client.infoComp(e.id) }.getOrDefault(emptyList())
                                dispensesRepas = runCatching { client.dispensesRepas(e.id) }.getOrDefault(emptyList())
                                activiteRepasId = runCatching { client.activiteRepasMidi(e.id) }.getOrNull()
                                pointsVs = runCatching { client.pointsVs(e.id) }.getOrNull()
                                sanctions = runCatching { client.sanctions(e.id) }.getOrDefault(emptyList())
                                incidents = runCatching { client.incidents(e.id) }.getOrDefault(emptyList())
                                commissions = runCatching { client.commissions(e.id) }.getOrDefault(emptyList())
                                    .filter { c -> c.signaleFamille }
                                dossierPsy = runCatching { client.dossierPsy(e.id) }.getOrNull()
                                rdvsPsy = runCatching { client.rdvsPsy(e.id) }.getOrDefault(emptyList())
                                    .filter { r -> r.signaleFamille }
                                passagesInfirmerie = runCatching { client.passagesInfirmerie(e.id) }.getOrDefault(emptyList())
                                runCatching { client.moyennesCourantes(e.id) }.getOrNull()?.let { (p, m) ->
                                    periodeNotes = p
                                    moyennes = m
                                }
                                appreciations = runCatching { client.appreciationsCourantes(e.id) }
                                    .getOrNull()?.second.orEmpty()
                                photoBytes = runCatching { client.photoEleve(e.id).size }.getOrDefault(0)
                                trombi = e.classe?.let { c ->
                                    runCatching { client.trombi(c) }.getOrDefault(emptyList())
                                }.orEmpty()
                                val (d, f) = weekBounds()
                                menus = runCatching { client.menusCantine(d, f) }.getOrDefault(emptyList())
                            }
                        },
                    ) {
                        Text("${e.prenom} ${e.nom}", style = MaterialTheme.typography.titleMedium)
                        Text(e.classe ?: "—", style = MaterialTheme.typography.bodyMedium)
                        if (selected == e.id) {
                            Text("sélectionné", style = MaterialTheme.typography.labelSmall)
                        }
                    }
                }
            }
            resume?.let { r ->
                Text("Prévenir une absence", style = MaterialTheme.typography.titleMedium)
                val motifs = motifsAbsence.ifEmpty {
                    listOf(EntApiClient.AbsenceMotif("maladie", "Maladie"))
                }
                motifs.forEach { m ->
                    Button(onClick = {
                        scope.launch {
                            selected?.let { id ->
                                val jour = java.time.LocalDate.now().toString()
                                runCatching { client.declarerAbsence(id, jour, jour, m.code) }
                                    .onFailure { error = it.message }
                                resume = runCatching { client.resume(id) }.getOrNull()
                            }
                        }
                    }) { Text("Aujourd’hui — ${m.libelle}") }
                }
                Text("Absences à justifier: ${r.absencesAJustifier.size}", style = MaterialTheme.typography.titleMedium)
                val motifsJustif = motifsAbsence.ifEmpty {
                    listOf(EntApiClient.AbsenceMotif("maladie", "Maladie"))
                }
                r.absencesAJustifier.forEach { a ->
                    Text("${a.debut} → ${a.fin} · ${a.motif}", style = MaterialTheme.typography.bodyMedium)
                    motifsJustif.forEach { m ->
                        Button(onClick = {
                            scope.launch {
                                runCatching { client.justifierAbsence(a.id, m.code) }
                                    .onFailure { error = it.message }
                                selected?.let { resume = runCatching { client.resume(it) }.getOrNull() }
                            }
                        }) {
                            Text("Justifier · ${m.libelle}")
                        }
                    }
                }
                Text("Carnet à signer: ${r.carnetASigner.size}")
                r.carnetASigner.forEach { c ->
                    Button(onClick = {
                        scope.launch {
                            runCatching { client.signerCarnet(c.id) }
                            selected?.let { resume = runCatching { client.resume(it) }.getOrNull() }
                        }
                    }) {
                        Text("Signer ${c.titre ?: c.id}")
                    }
                }
            }
            Text("Documents à signer: ${docsASigner.size}", style = MaterialTheme.typography.titleMedium)
            docsASigner.forEach { d ->
                Button(onClick = {
                    scope.launch {
                        runCatching { client.signerDoc(d.id) }
                        selected?.let {
                            docsASigner = runCatching { client.docsASigner(it) }.getOrDefault(emptyList())
                        }
                    }
                }) {
                    Text("Signer ${d.titre}")
                }
            }
            Text("Menus cantine: ${menus.size}", style = MaterialTheme.typography.titleMedium)
            menus.forEach { m ->
                Text("${m.jour} — ${m.plat}")
            }
            Text("Passages badge: ${passagesRepas.size}", style = MaterialTheme.typography.titleMedium)
            passagesRepas.forEach { p ->
                Text("${p.jour} · ${p.source}")
            }
            activiteRepasId?.let { actId ->
                Text("Prévenance cantine", style = MaterialTheme.typography.titleMedium)
                Button(onClick = {
                    scope.launch {
                        selected?.let { id ->
                            runCatching {
                                client.poserDispenseRepas(
                                    id,
                                    actId,
                                    java.time.LocalDate.now().toString(),
                                    "sortie scolaire",
                                )
                            }
                            dispensesRepas = runCatching { client.dispensesRepas(id) }.getOrDefault(emptyList())
                        }
                    }
                }) { Text("Dispenser aujourd’hui") }
                dispensesRepas.filter { !it.annulee }.forEach { d ->
                    Button(onClick = {
                        scope.launch {
                            selected?.let { id ->
                                runCatching { client.annulerDispenseRepas(id, d.id) }
                                dispensesRepas = runCatching { client.dispensesRepas(id) }.getOrDefault(emptyList())
                            }
                        }
                    }) { Text("Annuler ${d.jour} — ${d.motif}") }
                }
            }
            pointsVs?.let { p ->
                Text("Permis à points: ${p.solde} / ${p.depart}", style = MaterialTheme.typography.titleMedium)
            }
            if (sanctions.isNotEmpty()) {
                Text("Sanctions", style = MaterialTheme.typography.titleMedium)
                sanctions.forEach { s ->
                    val duree = s.dureeMinutes?.let { " · $it min" } ?: ""
                    val note = s.note?.let { " — $it" } ?: ""
                    Text("${s.jour} · ${s.statut}$duree$note")
                }
            }
            if (incidents.isNotEmpty()) {
                Text("Incidents", style = MaterialTheme.typography.titleMedium)
                incidents.forEach { i ->
                    Text("${i.jour}${i.description?.let { " — $it" } ?: ""}")
                }
            }
            if (commissions.isNotEmpty()) {
                Text("Commissions VS", style = MaterialTheme.typography.titleMedium)
                commissions.forEach { c ->
                    Text("${c.motifs} — ${c.decisionLibelle ?: "en attente"}")
                }
            }
            val psy = dossierPsy
            if (psy != null && psy.signaleFamille) {
                Text("Suivi psy", style = MaterialTheme.typography.titleMedium)
                Text(if (psy.suiviActif) "Suivi actif" else "Suivi inactif")
                if (psy.axesSuivi.isNotBlank()) Text(psy.axesSuivi)
                Button(onClick = {
                    scope.launch {
                        selected?.let { id ->
                            runCatching { client.pdfCourrierPsy(id) }
                                .onSuccess { pdfStatus = "Courrier psy: ${it.size} octets" }
                                .onFailure { error = it.message }
                        }
                    }
                }) { Text("PDF courrier psy") }
            }
            if (rdvsPsy.isNotEmpty()) {
                Text("RDV psy: ${rdvsPsy.size}", style = MaterialTheme.typography.titleMedium)
                rdvsPsy.forEach { r ->
                    Text("${r.jour} ${r.heureDebut}–${r.heureFin} · ${r.kindLibelle}")
                }
            }
            reinscription?.let { reins ->
                Text("Réinscription ${reins.anneeN1}", style = MaterialTheme.typography.titleMedium)
                if (!reins.campagneOuverte) {
                    Text("Campagne non ouverte")
                } else {
                    reins.enfants.forEach { row ->
                        Text("${row.prenom} ${row.nom}")
                        row.projectionAnnee?.let { Text("Projection $it · ${row.projectionNiveau ?: "—"}") }
                        if (row.confirmee) {
                            Text("Confirmée")
                        } else {
                            row.options.filter { it.statut != "retenue" }.forEach { o ->
                                Button(onClick = {
                                    scope.launch {
                                        runCatching { client.retenirOptionReinscription(row.enfantId, o.code, o.libelle) }
                                        reinscription = runCatching { client.reinscriptionFamille() }.getOrNull()
                                    }
                                }) { Text("Retenir ${o.libelle}") }
                            }
                            row.documentsManquants.forEach { code ->
                                Button(onClick = {
                                    scope.launch {
                                        runCatching { client.deposerDocReinscription(row.enfantId, code, code) }
                                        reinscription = runCatching { client.reinscriptionFamille() }.getOrNull()
                                    }
                                }) { Text("Déposer $code") }
                            }
                            if (row.documentsManquants.isEmpty() && row.acompteOk) {
                                Button(onClick = {
                                    scope.launch {
                                        runCatching { client.confirmerReinscription(row.enfantId) }
                                        reinscription = runCatching { client.reinscriptionFamille() }.getOrNull()
                                    }
                                }) { Text("Confirmer") }
                            }
                        }
                    }
                }
            }
            Text("Factures: ${factures.size}", style = MaterialTheme.typography.titleMedium)
            factures.forEach { f ->
                Text("${f.numero} — ${f.libelle} · ${"%.2f".format(f.totalCentimes / 100.0)} € (${f.statut})")
                Button(onClick = {
                    scope.launch {
                        runCatching { client.pdfFacture(f.id) }
                            .onSuccess { pdfStatus = "PDF ${f.numero}: ${it.size} octets" }
                            .onFailure { error = it.message }
                    }
                }) { Text("PDF facture") }
            }
            Text("Quittances: ${quittances.size}", style = MaterialTheme.typography.titleMedium)
            quittances.forEach { q ->
                Text("${q.numero} · ${"%.2f".format(q.montantCentimes / 100.0)} €")
                Button(onClick = {
                    scope.launch {
                        runCatching { client.pdfQuittance(q.id) }
                            .onSuccess { pdfStatus = "PDF ${q.numero}: ${it.size} octets" }
                            .onFailure { error = it.message }
                    }
                }) { Text("PDF quittance") }
            }
            pdfStatus?.let { Text(it) }
            Text("Cahier de texte: ${cahier.size}", style = MaterialTheme.typography.titleMedium)
            cahier.forEach { c ->
                Text("${c.matiereLibelle} — ${c.titre} (pour le ${c.pourLe})")
            }
            Text("Alertes CPE: ${alertes.size}", style = MaterialTheme.typography.titleMedium)
            alertes.forEach { a ->
                Text("${a.jour} · ${a.titre} (${a.severite})")
            }
            internat?.let { r ->
                if (r.chambreCode != null || r.suivis.isNotEmpty()) {
                    Text("Internat", style = MaterialTheme.typography.titleMedium)
                    r.chambreCode?.let { Text("Chambre $it (${r.chambreBatiment ?: "—"})") }
                    r.presenceStatut?.let { Text("Présence: $it") }
                    r.suivis.forEach { s -> Text("${s.jour} — ${s.texte}") }
                }
            }
            Text("Conseil de classe: ${conseils.size}", style = MaterialTheme.typography.titleMedium)
            conseils.forEach { c ->
                Text("${c.mention ?: "Décision"} — ${c.appreciation}")
            }
            Text("Passages infirmière: ${passagesInfirmerie.size}", style = MaterialTheme.typography.titleMedium)
            passagesInfirmerie.forEach { p ->
                Text("${p.jour} — ${p.motifLibelle}")
            }
            if (photoBytes > 0) {
                Text("Photo: $photoBytes octets", style = MaterialTheme.typography.titleMedium)
            }
            if (trombi.isNotEmpty()) {
                Text("Trombi: ${trombi.size}", style = MaterialTheme.typography.titleMedium)
                trombi.forEach { t -> Text("${t.prenom} ${t.nom}") }
            }
            if (moyennes.isNotEmpty()) {
                Text(
                    "Moyennes${periodeNotes?.let { " — ${it.libelle}" } ?: ""}",
                    style = MaterialTheme.typography.titleMedium,
                )
                moyennes.forEach { m ->
                    Text("${m.matiereLibelle ?: "Matière"} — %.1f / 20".format(m.moyenneSur20))
                }
            }
            if (appreciations.isNotEmpty()) {
                Text(
                    "Appréciations${periodeNotes?.let { " — ${it.libelle}" } ?: ""}",
                    style = MaterialTheme.typography.titleMedium,
                )
                appreciations.forEach { a ->
                    Text("${a.matiereLibelle ?: if (a.matiereId == null) "Générale" else "Matière"} · ${a.texte}")
                }
            }
            ficheSante?.let { fs ->
                Text("Fiche santé", style = MaterialTheme.typography.titleMedium)
                Text("Allergies: ${fs.allergies.ifEmpty { "—" }}")
                Text("Traitements: ${fs.traitements.ifEmpty { "—" }}")
                if (fs.paiActif) Text("PAI actif")
                Button(onClick = {
                    scope.launch {
                        selected?.let { id ->
                            runCatching { client.pdfFicheSante(id) }
                                .onSuccess { pdfStatus = "Fiche santé: ${it.size} octets" }
                                .onFailure { error = it.message }
                        }
                    }
                }) { Text("PDF fiche santé") }
            }
            pai?.let { p ->
                Text("PAI: ${p.motif}", style = MaterialTheme.typography.titleMedium)
                Text(p.mesuresAccueil)
                if (p.signeFamille) {
                    Text("Signé")
                } else {
                    Button(onClick = {
                        scope.launch {
                            selected?.let { id ->
                                pai = runCatching { client.signerPai(id) }.getOrNull() ?: pai
                            }
                        }
                    }) {
                        Text("Signer le PAI")
                    }
                }
                Button(onClick = {
                    scope.launch {
                        selected?.let { id ->
                            val pdf = "%PDF-1.1\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n".toByteArray()
                            runCatching {
                                client.deposerPaiPiece(id, "ordonnance-demo.pdf", "application/pdf", pdf)
                                paiPieces = client.paiPieces(id)
                            }.onFailure { error = it.message }
                        }
                    }
                }) { Text("Joindre ordonnance démo") }
                paiPieces.forEach { piece ->
                    Button(onClick = {
                        scope.launch {
                            runCatching { client.pdfPaiPiece(piece.id) }
                                .onSuccess { pdfStatus = "Ordonnance ${piece.nom}: ${it.size} octets" }
                                .onFailure { error = it.message }
                        }
                    }) { Text("PDF ${piece.nom}") }
                }
            }
            Text("Dispenses VS (EPS)", style = MaterialTheme.typography.titleMedium)
            Button(onClick = {
                scope.launch {
                    selected?.let { id ->
                        val jour = java.time.LocalDate.now().toString()
                        runCatching {
                            client.poserDispenseVs(id, "eps", jour, jour, "blessure légère")
                            dispensesVs = client.dispensesVs(id)
                        }.onFailure { error = it.message }
                    }
                }
            }) { Text("Déclarer EPS aujourd’hui") }
            if (dispensesVs.isEmpty()) {
                Text("Aucune")
            } else {
                dispensesVs.forEach { d ->
                    Text("${d.motif} · ${d.debut} → ${d.fin} · ${d.statut}")
                }
            }
            Text("Vœux: ${voeux.size}", style = MaterialTheme.typography.titleMedium)
            voeux.forEach { v ->
                Text("Rang ${v.rang} — ${v.filiereLabel ?: "Filière"} · ${v.anneeScolaire}")
            }
            if (infoComp.isNotEmpty()) {
                Text("Infos complémentaires", style = MaterialTheme.typography.titleMedium)
                infoComp.forEach { v ->
                    Text("${v.champLabel ?: "Champ"} · ${v.valeur}")
                }
            }
            Text("Stages: ${conventions.size}", style = MaterialTheme.typography.titleMedium)
            conventions.forEach { c ->
                Text("${c.entrepriseLabel ?: "Stage"} · ${c.debut}→${c.fin} · ${c.tuteurNom} (${c.statut})")
            }
            Text("Journal: ${journal.size}", style = MaterialTheme.typography.titleMedium)
            journal.forEach { j ->
                val when_ = j.createdAt?.take(16)?.let { " · $it" } ?: ""
                Text("${j.libelle} (${j.domaine}$when_)")
            }
            Text("EDT: ${edt.size}", style = MaterialTheme.typography.titleMedium)
            edt.forEach { c ->
                Text("${c.heureDebut}–${c.heureFin} ${c.matiereLibelle}${c.profLabel?.let { " · $it" } ?: ""}")
            }
            sortie?.let { s ->
                Text("Sorties", style = MaterialTheme.typography.titleMedium)
                if (s.autorisation?.interditEtablissement == true) {
                    Text("Interdit établissement", color = MaterialTheme.colorScheme.error)
                } else {
                    val a = s.autorisation
                    Text("midi=${a?.midi == true} soir=${a?.soir == true}")
                    Button(onClick = {
                        scope.launch {
                            selected?.let { id ->
                                runCatching {
                                    client.upsertAutorisationSortie(
                                        id,
                                        matin = false,
                                        midi = !(a?.midi ?: false),
                                        soir = a?.soir ?: false,
                                    )
                                }
                                sortie = runCatching { client.recapSortie(id) }.getOrNull()
                            }
                        }
                    }) {
                        Text("Basculer midi")
                    }
                    Button(onClick = {
                        scope.launch {
                            selected?.let { id ->
                                val jour = java.time.LocalDate.now().toString()
                                runCatching {
                                    client.demanderSortie(id, jour, "matin", "RDV médical")
                                }
                                sortie = runCatching { client.recapSortie(id) }.getOrNull()
                            }
                        }
                    }) {
                        Text("Demande exceptionnelle matin")
                    }
                }
                s.demandesOuvertes.forEach { d ->
                    Text("${d.jour} ${d.creneau} — ${d.statut}")
                }
            }
            Text("Messagerie (${fils.size})", style = MaterialTheme.typography.titleMedium)
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
                    Text(
                        m.auteurLabel ?: "Correspondant",
                        style = MaterialTheme.typography.labelSmall,
                    )
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
