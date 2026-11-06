package fr.scolia.mobile.app.ecrans

import fr.scolia.mobile.app.ui.DIALOGUE_SECURISE
import androidx.compose.foundation.clickable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import fr.scolia.mobile.app.AppModele
import fr.scolia.mobile.app.ui.ChoixEnfant
import fr.scolia.mobile.app.ui.Chargeur
import fr.scolia.mobile.app.ui.Ligne
import fr.scolia.mobile.app.ui.MessageErreur
import fr.scolia.mobile.app.ui.Titre
import fr.scolia.mobile.app.ui.dateFr
import fr.scolia.mobile.app.ui.note
import fr.scolia.mobile.core.Absence
import fr.scolia.mobile.core.Appel
import fr.scolia.mobile.core.Chemins
import fr.scolia.mobile.core.EdtCreneau
import fr.scolia.mobile.core.EnfantFamille
import fr.scolia.mobile.core.ErreurApi
import fr.scolia.mobile.core.EtatAppel
import fr.scolia.mobile.core.EtatConnexion
import fr.scolia.mobile.core.Lecture
import fr.scolia.mobile.core.Module
import fr.scolia.mobile.core.Profil
import fr.scolia.mobile.core.SaisirNoteBody
import kotlinx.coroutines.launch
import java.time.LocalDate

private fun aujourdHui(): String = LocalDate.now().toString()
private fun <T> direct(v: T): Lecture<T> = Lecture(v, false, System.currentTimeMillis())

@Composable
fun EcranModule(module: Module, modele: AppModele, c: EtatConnexion.Connecte, ouvrir: (Module) -> Unit) {
    when (module) {
        Module.ACCUEIL -> Accueil(modele, c, ouvrir)
        Module.APPEL -> EcranAppel(modele)
        Module.VIE_SCOLAIRE -> EcranVieScolaire(modele)
        Module.ABSENCES -> EcranAbsences(modele)
        Module.NOTES -> EcranNotes(modele, c)
        Module.SAISIE_NOTES -> EcranSaisieNotes(modele)
        Module.AGENDA -> EcranAgenda(modele, c)
        Module.EDT -> EcranEdt(modele, c)
        Module.MESSAGERIE -> EcranMessagerie(modele)
        Module.PROFIL -> EcranProfil(modele, c)
    }
}

// ---------- Accueil ----------

@Composable
private fun Accueil(modele: AppModele, c: EtatConnexion.Connecte, ouvrir: (Module) -> Unit) {
    val lecteur = modele.lecteur!!
    Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Titre("Bonjour")
        val raccourcis = c.espace.modules.filter { it != Module.ACCUEIL && it != Module.PROFIL }.take(4)
        raccourcis.chunked(2).forEach { paire ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                paire.forEach { m ->
                    Card(Modifier.weight(1f).heightIn(min = 72.dp).clickable(role = Role.Button) { ouvrir(m) }) {
                        Text(m.titre, Modifier.padding(16.dp), style = MaterialTheme.typography.titleMedium)
                    }
                }
            }
        }
        if (c.espace.enseigne) {
            Titre("Mes cours du jour")
            Chargeur(aujourdHui(), modele::sessionPerdue, { lecteur.lire(Chemins.mesCreneaux(aujourdHui()), Chemins.listeCreneaux) }) { cr, _ ->
                if (cr.isEmpty()) Text("Aucun cours aujourd'hui.")
                cr.sortedBy { it.heureDebut }.forEach { Ligne("${it.heureDebut.take(5)} · ${it.classe}", it.matiereLibelle, it.salle) }
            }
        }
        Titre("Annonces")
        Chargeur("annonces", modele::sessionPerdue, { direct(modele.client!!.annonces()) }) { a, _ ->
            if (a.isEmpty()) Text("Aucune annonce.")
            a.take(5).forEach { Ligne(it.titre, it.corps.take(140)) }
        }
    }
}

// ---------- Appel (professeur) ----------

@Composable
private fun EcranAppel(modele: AppModele) {
    val client = modele.client!!
    var appel by remember { mutableStateOf<Appel?>(null) }
    var erreur by remember { mutableStateOf<String?>(null) }
    var confirmer by remember { mutableStateOf(false) }
    val portee = rememberCoroutineScope()
    fun lancer(bloc: suspend () -> Unit) = portee.launch {
        erreur = null
        try {
            bloc()
        } catch (e: ErreurApi.NonAuthentifie) {
            modele.sessionPerdue()
        } catch (e: Exception) {
            erreur = e.message
        }
    }

    val a = appel
    if (a == null) {
        Titre("Séances du jour")
        MessageErreur(erreur)
        Chargeur(aujourdHui(), modele::sessionPerdue, { modele.lecteur!!.lire(Chemins.mesCreneaux(aujourdHui()), Chemins.listeCreneaux) }) { cr, _ ->
            if (cr.isEmpty()) Text("Aucune séance aujourd'hui.")
            LazyColumn {
                items(cr.sortedBy { it.heureDebut }, key = { it.id }) { s ->
                    Ligne(
                        "${s.heureDebut.take(5)}–${s.heureFin.take(5)} · ${s.classe}",
                        s.matiereLibelle + (s.salle?.let { " · salle $it" } ?: ""),
                        "Faire l'appel",
                        Modifier.clickable(role = Role.Button) { lancer { appel = client.ouvrirAppel(s.id, aujourdHui()) } },
                    )
                    HorizontalDivider()
                }
            }
        }
        return
    }

    Titre("${a.classe} · ${a.matiereLibelle ?: ""}")
    val absents = a.lignes.count { it.etat == "absent" }
    val retards = a.lignes.count { it.etat == "retard" }
    Text("${a.lignes.size} élèves · $absents absent(s) · $retards retard(s)" + if (a.estClos) " · appel clos" else "")
    MessageErreur(erreur)
    // Boutons au-dessus de la liste : une LazyColumn occupe toute la hauteur restante.
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(vertical = 8.dp)) {
        OutlinedButton(onClick = { appel = null }, modifier = Modifier.heightIn(min = 48.dp)) { Text("Retour") }
        if (!a.estClos) Button(onClick = { confirmer = true }, modifier = Modifier.heightIn(min = 48.dp)) { Text("Valider l'appel") }
    }
    LazyColumn(Modifier.fillMaxWidth()) {
        items(a.lignes, key = { it.eleveId }) { l ->
            Column(Modifier.padding(vertical = 6.dp)) {
                Text(l.eleveLabel ?: "Élève", style = MaterialTheme.typography.bodyLarge)
                SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                    EtatAppel.entries.forEachIndexed { i, e ->
                        SegmentedButton(
                            selected = l.etat == e.code,
                            onClick = {
                                lancer {
                                    client.marquer(a.id, l.eleveId, e)
                                    // Repartir de la valeur courante (deux marquages rapprochés).
                                    appel = appel?.let { cur -> cur.copy(lignes = cur.lignes.map { if (it.eleveId == l.eleveId) it.copy(etat = e.code) else it }) }
                                }
                            },
                            enabled = !a.estClos,
                            shape = SegmentedButtonDefaults.itemShape(i, EtatAppel.entries.size),
                            modifier = Modifier.heightIn(min = 48.dp).semantics {
                                contentDescription = "${l.eleveLabel ?: "Élève"} : ${e.libelle}"
                                stateDescription = if (l.etat == e.code) "sélectionné" else "non sélectionné"
                            },
                        ) { Text(e.libelle) }
                    }
                }
            }
            HorizontalDivider()
        }
    }
    if (confirmer) {
        // Action sensible : confirmation explicite (CLAUDE.md, sécurité).
        AlertDialog(properties = DIALOGUE_SECURISE, 
            onDismissRequest = { confirmer = false },
            title = { Text("Valider l'appel ?") },
            text = { Text("$absents absent(s), $retards retard(s). Les familles seront prévenues ; l'appel ne sera plus modifiable.") },
            confirmButton = { TextButton(onClick = { confirmer = false; lancer { appel = client.clore(a.id) } }) { Text("Valider") } },
            dismissButton = { TextButton(onClick = { confirmer = false }) { Text("Annuler") } },
        )
    }
}

// ---------- Vie scolaire ----------

@Composable
private fun EcranVieScolaire(modele: AppModele) {
    Titre("Absences à traiter")
    Chargeur("vs", modele::sessionPerdue, { direct(modele.client!!.absencesATraiter()) }) { liste, _ ->
        Text("Le traitement détaillé (décisions, courriers) se fait sur le web.", style = MaterialTheme.typography.bodySmall)
        if (liste.isEmpty()) Text("Rien à traiter.")
        LazyColumn {
            items(liste, key = { it.id }) { a ->
                Ligne(a.eleveLabel ?: "Élève", "Du ${dateFr(a.debut)} au ${dateFr(a.fin)} · ${a.motif}", a.statut)
                HorizontalDivider()
            }
        }
    }
}

// ---------- Famille : enfants, absences ----------

@Composable
private fun AvecEnfants(modele: AppModele, contenu: @Composable (EnfantFamille) -> Unit) {
    var choisi by rememberSaveable { mutableStateOf<String?>(null) }
    Chargeur("enfants", modele::sessionPerdue, { modele.lecteur!!.lire(Chemins.ENFANTS, Chemins.listeEnfants) }) { enfants, _ ->
        if (enfants.isEmpty()) {
            Text("Aucun enfant rattaché à votre compte dans cet établissement.")
        } else {
            val e = enfants.firstOrNull { it.id == choisi } ?: enfants.first()
            ChoixEnfant(enfants, e) { choisi = it.id }
            contenu(e)
        }
    }
}

@Composable
private fun EcranAbsences(modele: AppModele) {
    val client = modele.client!!
    var aJustifier by remember { mutableStateOf<Absence?>(null) }
    var motif by remember { mutableStateOf("") }
    var erreur by remember { mutableStateOf<String?>(null) }
    var envoi by remember { mutableStateOf(false) }
    val portee = rememberCoroutineScope()
    Column(Modifier.verticalScroll(rememberScrollState())) { AvecEnfants(modele) { enfant ->
        Chargeur(enfant.id, modele::sessionPerdue, { direct(client.resumeEnfant(enfant.id)) }) { r, recharger ->
            Titre("Absences à justifier")
            if (r.absencesAJustifier.isEmpty()) Text("Aucune absence à justifier pour ${enfant.prenom}.")
            r.absencesAJustifier.forEach { a ->
                Ligne(
                    if (a.debut == a.fin) "Le ${dateFr(a.debut)}" else "Du ${dateFr(a.debut)} au ${dateFr(a.fin)}",
                    a.motif,
                    if (enfant.peutJustifierAbsence) "Justifier" else null,
                    if (enfant.peutJustifierAbsence) Modifier.clickable(role = Role.Button) { aJustifier = a; motif = "" } else Modifier,
                )
                HorizontalDivider()
            }
            if (!enfant.peutJustifierAbsence) Text("Votre compte n'a pas le droit de justifier les absences de cet enfant.")
            if (r.carnetASigner.isNotEmpty()) {
                Titre("Carnet à signer")
                r.carnetASigner.forEach { Ligne(it.titre, dateFr(it.dateEntree)) }
                Text("La signature du carnet se fait sur le web pour le moment.", style = MaterialTheme.typography.bodySmall)
            }
            MessageErreur(erreur)
            val a = aJustifier
            if (a != null) {
                AlertDialog(properties = DIALOGUE_SECURISE, 
                    onDismissRequest = { aJustifier = null },
                    title = { Text("Justifier l'absence du ${dateFr(a.debut)}") },
                    text = {
                        OutlinedTextField(motif, { motif = it.take(500) }, label = { Text("Motif") }, minLines = 2)
                    },
                    confirmButton = {
                        TextButton(enabled = motif.isNotBlank() && !envoi, onClick = {
                            envoi = true
                            portee.launch {
                                erreur = null
                                try {
                                    client.justifierAbsence(a.id, motif)
                                    aJustifier = null
                                    recharger()
                                } catch (e: ErreurApi.NonAuthentifie) {
                                    modele.sessionPerdue()
                                } catch (e: kotlinx.coroutines.CancellationException) {
                                    throw e
                                } catch (e: Exception) {
                                    erreur = e.message
                                    aJustifier = null
                                } finally {
                                    envoi = false
                                }
                            }
                        }) { Text("Envoyer") }
                    },
                    dismissButton = { TextButton(onClick = { aJustifier = null }) { Text("Annuler") } },
                )
            }
        }
    } }
}

// ---------- Notes (élève, famille) ----------

@Composable
private fun ListeNotes(modele: AppModele, eleveId: String) {
    Chargeur(eleveId, modele::sessionPerdue, { modele.lecteur!!.lire(Chemins.notes(eleveId), Chemins.listeNotes) }) { notes, _ ->
        if (notes.isEmpty()) Text("Aucune note pour le moment.")
        LazyColumn {
            items(notes.sortedByDescending { it.dateNote }, key = { it.id }) { n ->
                Ligne(n.matiereLibelle ?: "Matière", listOfNotNull(n.libelle, dateFr(n.dateNote)).joinToString(" · "), "${note(n.valeur)}/${note(n.sur)}")
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun EcranNotes(modele: AppModele, c: EtatConnexion.Connecte) {
    when (c.espace.profil) {
        Profil.ELEVE -> c.me.eleveId?.let { ListeNotes(modele, it) } ?: Text("Compte élève non lié.")
        Profil.FAMILLE -> AvecEnfants(modele) { e ->
            if (e.peutVoirNotes) ListeNotes(modele, e.id) else Text("Votre compte n'a pas accès aux notes de cet enfant.")
        }
        else -> Text("Consultez les notes d'un élève depuis le web.")
    }
}

// ---------- Saisie de notes (professeur) ----------

@Composable
private fun EcranSaisieNotes(modele: AppModele) {
    val client = modele.client!!
    var classe by rememberSaveable { mutableStateOf<String?>(null) }
    var devoirId by rememberSaveable { mutableStateOf<String?>(null) }
    // Saisies propres au devoir choisi : changer de devoir repart de zéro.
    val saisies = remember(devoirId) { mutableStateMapOf<String, String>() }
    val enregistrees = remember(devoirId) { mutableStateListOf<String>() }
    var erreur by remember { mutableStateOf<String?>(null) }
    var enCours by remember { mutableStateOf(false) }
    var confirmer by remember { mutableStateOf(false) }
    val portee = rememberCoroutineScope()

    Column(Modifier.verticalScroll(rememberScrollState())) {
    Chargeur("classes", modele::sessionPerdue, { modele.lecteur!!.lire(Chemins.mesCreneaux(aujourdHui()), Chemins.listeCreneaux) }) { cr, _ ->
        val classes = cr.map { it.classe }.distinct().sorted()
        Titre("Classe")
        if (classes.isEmpty()) Text("Aucune classe aujourd'hui. La saisie complète est sur le web.")
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            classes.forEach { k -> OutlinedButton(onClick = { classe = k; devoirId = null }, modifier = Modifier.heightIn(min = 48.dp)) { Text(k) } }
        }
    }
    val k = classe ?: return@Column
    Chargeur(k, modele::sessionPerdue, {
        val periodes = client.periodesNotes()
        val p = periodes.firstOrNull { it.statut == "ouverte" } ?: periodes.firstOrNull()
        direct(p?.let { client.devoirsClasse(it.id, k) }.orEmpty())
    }) { devoirs, _ ->
        Titre("Devoir")
        if (devoirs.isEmpty()) Text("Aucun devoir pour $k sur la période ouverte. Créez-le sur le web.")
        devoirs.forEach { d ->
            Ligne(d.titre, "${d.matiereLibelle ?: ""} · ${dateFr(d.dateDevoir)} · sur ${note(d.sur)}", if (d.id == devoirId) "✓" else null,
                Modifier.clickable(role = Role.Button) { devoirId = d.id })
        }
        val d = devoirs.firstOrNull { it.id == devoirId } ?: return@Chargeur
        Chargeur(d.id, modele::sessionPerdue, { direct(client.effectif(k, aujourdHui())) }) { eff, _ ->
            Titre("Notes sur ${note(d.sur)}")
            MessageErreur(erreur)
            Column(Modifier.fillMaxWidth()) {
                eff.eleves.forEach { el ->
                    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                        Text("${el.nom} ${el.prenom}", Modifier.weight(1f).padding(top = 16.dp))
                        OutlinedTextField(
                            saisies[el.id] ?: "", { saisies[el.id] = it.replace(',', '.').take(5) },
                            label = { Text(if (el.id in enregistrees) "Enregistrée" else "Note") },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                            modifier = Modifier.weight(0.6f),
                        )
                    }
                }
            }
            val aEnvoyer = saisies.filter { (eleve, t) -> eleve !in enregistrees && t.isNotBlank() }
            Button(onClick = { confirmer = true }, enabled = !enCours && aEnvoyer.isNotEmpty(), modifier = Modifier.heightIn(min = 48.dp)) {
                Text("Enregistrer les notes")
            }
            if (confirmer) AlertDialog(properties = DIALOGUE_SECURISE, 
                onDismissRequest = { confirmer = false },
                title = { Text("Enregistrer ${aEnvoyer.size} note(s) ?") },
                text = { Text("« ${d.titre} », $k, notes sur ${note(d.sur)}.") },
                dismissButton = { TextButton(onClick = { confirmer = false }) { Text("Annuler") } },
                confirmButton = { TextButton(onClick = {
                confirmer = false
                enCours = true
                portee.launch {
                    erreur = null
                    try { for ((eleve, texte) in aEnvoyer) {
                        val v = texte.toDoubleOrNull()
                        if (v == null || !v.isFinite() || v < 0 || v > d.sur) { erreur = "Note invalide : « $texte »."; continue }
                        try {
                            client.saisirNote(eleve, SaisirNoteBody(d.matiereId, v, d.sur, aujourdHui(), d.titre, d.id, d.periodeId))
                            enregistrees += eleve
                        } catch (e: ErreurApi.NonAuthentifie) {
                            modele.sessionPerdue(); return@launch
                        } catch (e: kotlinx.coroutines.CancellationException) {
                            throw e
                        } catch (e: Exception) {
                            erreur = e.message
                        }
                    } } finally { enCours = false }
                }
            }) { Text("Enregistrer") } },
            )
        }
    }
    }
}

// ---------- Cahier de textes ----------

@Composable
private fun ListeCahier(modele: AppModele, eleveId: String?) {
    Chargeur(eleveId, modele::sessionPerdue, { modele.lecteur!!.lire(Chemins.cahier(eleveId), Chemins.listeCahier) }) { liste, _ ->
        val aVenir = liste.filter { it.pourLe >= aujourdHui() }.sortedBy { it.pourLe }
        if (aVenir.isEmpty()) Text("Rien à faire pour les prochains jours.")
        LazyColumn {
            items(aVenir, key = { it.id }) { e ->
                Ligne("${e.matiereLibelle} · pour le ${dateFr(e.pourLe)}", "${e.titre}\n${e.consigne}".trim())
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun EcranAgenda(modele: AppModele, c: EtatConnexion.Connecte) {
    when (c.espace.profil) {
        Profil.ELEVE -> ListeCahier(modele, c.me.eleveId)
        Profil.FAMILLE -> AvecEnfants(modele) { ListeCahier(modele, it.id) }
        else -> ListeCahier(modele, null)
    }
}

// ---------- Emploi du temps ----------

private val JOURS = listOf("", "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche")

@Composable
private fun ListeEdt(creneaux: List<EdtCreneau>) {
    if (creneaux.isEmpty()) Text("Aucun cours.")
    LazyColumn {
        creneaux.groupBy { it.jourSemaine }.toSortedMap().forEach { (j, cr) ->
            item(key = "j$j") { Titre(JOURS.getOrElse(j) { "" }) }
            items(cr.sortedBy { it.heureDebut }, key = { it.id }) { s ->
                Ligne("${s.heureDebut.take(5)}–${s.heureFin.take(5)} · ${s.matiereLibelle}", listOfNotNull(s.classe, s.profLabel, s.salle?.let { "salle $it" }).joinToString(" · "))
            }
        }
    }
}

@Composable
private fun EcranEdt(modele: AppModele, c: EtatConnexion.Connecte) {
    val l = modele.lecteur!!
    val jour = aujourdHui()
    when (c.espace.profil) {
        Profil.ELEVE -> c.me.eleveId?.let { id ->
            Chargeur(id, modele::sessionPerdue, { l.lire(Chemins.edtEleve(id, jour), Chemins.listeCreneaux) }) { cr, _ -> ListeEdt(cr) }
        }
        Profil.FAMILLE -> AvecEnfants(modele) { e ->
            Chargeur(e.id, modele::sessionPerdue, { l.lire(Chemins.edtEleve(e.id, jour), Chemins.listeCreneaux) }) { cr, _ -> ListeEdt(cr) }
        }
        else -> Chargeur(jour, modele::sessionPerdue, { l.lire(Chemins.mesCreneaux(jour), Chemins.listeCreneaux) }) { cr, _ -> ListeEdt(cr) }
    }
}
