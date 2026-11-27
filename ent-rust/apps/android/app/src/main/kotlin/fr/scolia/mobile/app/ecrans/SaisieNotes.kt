package fr.scolia.mobile.app.ecrans

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import fr.scolia.mobile.app.AppModele
import fr.scolia.mobile.app.ui.Chargeur
import fr.scolia.mobile.app.ui.DIALOGUE_SECURISE
import fr.scolia.mobile.app.ui.Ligne
import fr.scolia.mobile.app.ui.MessageErreur
import fr.scolia.mobile.app.ui.MiseAJour
import fr.scolia.mobile.app.ui.Titre
import fr.scolia.mobile.app.ui.dateFr
import fr.scolia.mobile.core.AnalyseCellule
import fr.scolia.mobile.core.BrouillonSaisie
import fr.scolia.mobile.core.CreerEvaluationEvaBody
import fr.scolia.mobile.core.EnseignementEva
import fr.scolia.mobile.core.ErreurApi
import fr.scolia.mobile.core.EvaluationEva
import fr.scolia.mobile.core.GrilleEva
import fr.scolia.mobile.core.Lecture
import fr.scolia.mobile.core.PeriodeEva
import fr.scolia.mobile.core.SaisieNotes
import kotlinx.coroutines.launch
import java.time.LocalDate

// Saisie des notes sur la grille d'évaluation `/eva/*` (palier « notes eva » du socle 11).
// Parcours court du quotidien : cours → période (celle du jour par défaut) → évaluations → saisie d'une évaluation.
// Le web garde la grille complète (modification, annulation, import, historique, corrections motivées).

private fun <T> direct(v: T): Lecture<T> = Lecture(v, false, System.currentTimeMillis())

@Composable
fun EcranSaisieNotes(modele: AppModele) {
    var cours by remember { mutableStateOf<EnseignementEva?>(null) }
    var periode by remember { mutableStateOf<PeriodeEva?>(null) }
    var evaluationId by remember { mutableStateOf<String?>(null) }
    // La période du jour s'ouvre d'elle-même une fois par cours ; « Autre période » rend ensuite le choix.
    var auto by remember { mutableStateOf(true) }
    val c = cours
    val p = periode
    when {
        c == null -> ChoixCours(modele) { cours = it; periode = null; auto = true }
        p == null -> ChoixPeriode(modele, c, auto, retour = { cours = null }) { periode = it; auto = false }
        else -> GrilleCours(modele, c, p, evaluationId, ouvrir = { evaluationId = it }, retour = {
            if (evaluationId != null) evaluationId = null else periode = null
        })
    }
}

@Composable
private fun ChoixCours(modele: AppModele, choisir: (EnseignementEva) -> Unit) {
    Titre("Saisir des notes")
    Chargeur("eva-cours", modele::sessionPerdue, { direct(modele.client!!.evaEnseignements()) }) { liste, _ ->
        val enSaisie = liste.filter { it.enSaisie }
        if (enSaisie.isEmpty()) Text("Aucun cours où vous saisissez des notes. Les grilles en lecture restent sur le web.")
        LazyColumn {
            items(enSaisie, key = { it.coursId }) { e ->
                Ligne(e.libelle, e.cibleCode, null, Modifier.clickable(role = Role.Button) { choisir(e) })
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun ChoixPeriode(modele: AppModele, cours: EnseignementEva, auto: Boolean, retour: () -> Unit, choisir: (PeriodeEva) -> Unit) {
    Titre(cours.libelle)
    OutlinedButton(onClick = retour, modifier = Modifier.heightIn(min = 48.dp)) { Text("Autre cours") }
    Chargeur("eva-periodes-${cours.coursId}", modele::sessionPerdue, { direct(modele.client!!.evaPeriodes(cours.coursId)) }) { liste, _ ->
        val jour = LocalDate.now().toString()
        val duJour = liste.firstOrNull { it.contient(jour) }
        if (auto && duJour != null) {
            androidx.compose.runtime.LaunchedEffect(duJour.id) { choisir(duJour) }
        }
        if (liste.isEmpty()) Text("Aucune période de notes pour ce cours.")
        liste.forEach { per ->
            Ligne(per.libelle, "${dateFr(per.dateDebut)} au ${dateFr(per.dateFin)}", if (per.id == duJour?.id) "en cours" else null, Modifier.clickable(role = Role.Button) { choisir(per) })
        }
    }
}

@Composable
private fun GrilleCours(
    modele: AppModele,
    cours: EnseignementEva,
    periode: PeriodeEva,
    evaluationId: String?,
    ouvrir: (String) -> Unit,
    retour: () -> Unit,
) {
    var essai by remember { mutableIntStateOf(0) }
    Chargeur("eva-grille-${cours.coursId}-${periode.id}-$essai", modele::sessionPerdue, {
        direct(modele.client!!.evaGrille(cours.coursId, periode.id))
    }) { g, _ ->
        val ev = evaluationId?.let { id -> g.evaluations.firstOrNull { it.id == id } }
        if (ev == null) {
            ListeEvaluations(modele, g, ouvrir, retour, recharger = { essai++ })
        } else {
            SaisieEvaluation(modele, g, ev, retour = { essai++; retour() })
        }
    }
}

@Composable
private fun ListeEvaluations(modele: AppModele, g: GrilleEva, ouvrir: (String) -> Unit, retour: () -> Unit, recharger: () -> Unit) {
    var creer by remember { mutableStateOf(false) }
    Titre("${g.enseignement.libelle} · ${g.periode.libelle}")
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(bottom = 8.dp)) {
        OutlinedButton(onClick = retour, modifier = Modifier.heightIn(min = 48.dp)) { Text("Autre période") }
        if (g.peutSaisir) Button(onClick = { creer = true }, modifier = Modifier.heightIn(min = 48.dp).testTag("eva-nouvelle")) { Text("Nouvelle évaluation") }
    }
    if (!g.peutSaisir) Text("Grille en lecture : la saisie n'est pas ouverte pour vous sur ce cours.")
    g.avertissements.forEach { Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant) }
    g.moyenneClasse?.let { Text("Moyenne de la classe : ${SaisieNotes.formater(it.moyenneM)} / ${SaisieNotes.formater(g.echelleM)}") }
    val visibles = g.evaluations.filter { it.etat != "brouillon" }.sortedByDescending { it.dateEval }
    if (visibles.isEmpty()) Text("Aucune évaluation sur cette période.")
    LazyColumn {
        items(visibles, key = { it.id }) { e ->
            Ligne(
                e.titre,
                "${dateFr(e.dateEval)} · sur ${SaisieNotes.formater(e.baremeM)} · coef ${SaisieNotes.formater(e.coefM)}" + etatEvaluation(e),
                e.stats?.let { "${SaisieNotes.formater(it.moyenneM)}" },
                Modifier.clickable(role = Role.Button) { ouvrir(e.id) },
            )
            HorizontalDivider()
        }
    }
    if (creer) NouvelleEvaluation(modele, g, fermer = { creer = false }) { id -> creer = false; recharger(); ouvrir(id) }
}

private fun etatEvaluation(e: EvaluationEva): String = when {
    e.etat == "annulee" -> " · annulée"
    e.publiee -> " · publiée"
    e.publicationPrevueLe != null -> " · publication programmée"
    else -> " · non publiée"
}

@Composable
private fun NouvelleEvaluation(modele: AppModele, g: GrilleEva, fermer: () -> Unit, creee: (String) -> Unit) {
    val jour = LocalDate.now().toString()
    var titre by remember { mutableStateOf("") }
    var date by remember { mutableStateOf(if (g.periode.contient(jour)) jour else g.periode.dateDebut) }
    var bareme by remember { mutableStateOf("20") }
    var coef by remember { mutableStateOf("1") }
    var erreur by remember { mutableStateOf<String?>(null) }
    var occupe by remember { mutableStateOf(false) }
    val portee = rememberCoroutineScope()
    val baremeM = SaisieNotes.milliemesDepuisTexte(bareme)?.takeIf { it > 0 }
    val coefM = SaisieNotes.milliemesDepuisTexte(coef)?.takeIf { it > 0 }
    val dateValide = runCatching { LocalDate.parse(date) }.isSuccess
    val pret = titre.isNotBlank() && titre.trim().length <= 200 && baremeM != null && coefM != null && dateValide && !occupe
    AlertDialog(
        onDismissRequest = fermer,
        properties = DIALOGUE_SECURISE,
        title = { Text("Nouvelle évaluation") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(titre, { titre = it }, label = { Text("Titre") }, singleLine = true)
                OutlinedTextField(date, { date = it.trim() }, label = { Text("Date (AAAA-MM-JJ)") }, singleLine = true, isError = !dateValide)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(bareme, { bareme = it }, label = { Text("Sur") }, singleLine = true, isError = baremeM == null,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal), modifier = Modifier.weight(1f))
                    OutlinedTextField(coef, { coef = it }, label = { Text("Coefficient") }, singleLine = true, isError = coefM == null,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal), modifier = Modifier.weight(1f))
                }
                Text("Non publiée : les familles ne la verront qu'après « Publier ».", style = MaterialTheme.typography.bodySmall)
                MessageErreur(erreur)
            }
        },
        confirmButton = {
            TextButton(enabled = pret, onClick = {
                val corps = CreerEvaluationEvaBody(g.enseignement.coursId, titre.trim(), date, baremeM!!, coefM!!)
                occupe = true
                portee.launch {
                    erreur = null
                    try {
                        val e = modele.cles.executer("eva-creer:${corps.coursId}:${corps.titre}|${corps.dateEval}|${corps.baremeM}|${corps.coefM}") { k ->
                            modele.client!!.creerEvaluation(corps, k)
                        }
                        creee(e.id)
                    } catch (e: ErreurApi.NonAuthentifie) {
                        modele.sessionPerdue()
                    } catch (e: ErreurApi.AppliTropAncienne) {
                        MiseAJour.exiger(e.message)
                    } catch (e: Exception) {
                        erreur = e.message
                    } finally {
                        occupe = false
                    }
                }
            }) { Text("Créer") }
        },
        dismissButton = { TextButton(onClick = fermer) { Text("Annuler") } },
    )
}

@Composable
private fun SaisieEvaluation(modele: AppModele, g: GrilleEva, ev: EvaluationEva, retour: () -> Unit) {
    val brouillon = remember(g, ev.id) { BrouillonSaisie(g, ev.id) }
    // Le brouillon n'est pas observable : `tick` force la recomposition après chaque changement.
    var tick by remember { mutableIntStateOf(0) }
    var erreur by remember { mutableStateOf<String?>(null) }
    var info by remember { mutableStateOf<String?>(null) }
    var occupe by remember { mutableStateOf(false) }
    var publier by remember { mutableStateOf(false) }
    var evaluation by remember(ev.id) { mutableStateOf(ev) }
    val portee = rememberCoroutineScope()
    tick.let { }

    fun lancer(bloc: suspend () -> Unit) = portee.launch {
        erreur = null
        occupe = true
        try {
            bloc()
        } catch (e: ErreurApi.NonAuthentifie) {
            modele.sessionPerdue()
        } catch (e: ErreurApi.AppliTropAncienne) {
            MiseAJour.exiger(e.message)
        } catch (e: ErreurApi.HorsLigne) {
            erreur = "Hors ligne : rien n'est perdu, réessayez « Enregistrer » une fois le réseau revenu."
        } catch (e: Exception) {
            erreur = e.message
        } finally {
            occupe = false
            tick++
        }
    }

    Titre(evaluation.titre)
    Text("${dateFr(evaluation.dateEval)} · sur ${SaisieNotes.formater(evaluation.baremeM)} · coef ${SaisieNotes.formater(evaluation.coefM)}" + etatEvaluation(evaluation))
    val lot = brouillon.lot()
    val erreurs = brouillon.enErreur().size
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(vertical = 8.dp)) {
        OutlinedButton(onClick = retour, modifier = Modifier.heightIn(min = 48.dp)) { Text("Retour") }
        Button(
            enabled = lot.cellules.isNotEmpty() && !occupe,
            onClick = {
                lancer {
                    val retourServeur = modele.cles.executer(brouillon.intention(lot), famille = brouillon.famille()) { k -> modele.client!!.saisirNotes(lot, k) }
                    val n = brouillon.appliquer(retourServeur, lot)
                    info = buildString {
                        append("$n case(s) enregistrée(s).")
                        if (brouillon.nbConflits > 0) append(" ${brouillon.nbConflits} conflit(s) à trancher.")
                    }
                }
            },
            modifier = Modifier.heightIn(min = 48.dp).testTag("eva-enregistrer"),
        ) { Text(if (lot.cellules.isEmpty()) "Enregistrer" else "Enregistrer (${lot.cellules.size})") }
        val peutPublier = g.peutSaisir && evaluation.ouverte && (!evaluation.publiee || evaluation.retraitPossible)
        if (peutPublier) {
            OutlinedButton(onClick = { publier = true }, enabled = !occupe && lot.cellules.isEmpty(), modifier = Modifier.heightIn(min = 48.dp)) {
                Text(if (evaluation.publiee) "Retirer" else "Publier")
            }
        }
    }
    if (erreurs > 0) MessageErreur("$erreurs case(s) non comprise(s) : corrigez-les avant d'enregistrer.")
    MessageErreur(erreur)
    info?.let { Text(it) }
    if (!evaluation.ouverte) Text("Évaluation ${if (evaluation.etat == "annulee") "annulée" else "non ouverte"} : lecture seule.")
    // Codes rapides : rappel, la frappe suffit (abs, ar, disp, nr, nn).
    LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        items(SaisieNotes.CODES_RAPIDES) { (code, _, libelle) -> AssistChip(onClick = {}, label = { Text("$code : $libelle") }) }
    }
    LazyColumn(Modifier.fillMaxWidth()) {
        items(brouillon.eleves, key = { it.eleveId }) { l ->
            CaseEleve(brouillon, l.eleveId, "${l.nom} ${l.prenom}", l.mouvement, ev.baremeM, tick) { tick++ }
            HorizontalDivider()
        }
    }
    if (publier) {
        ConfirmerPublication(evaluation, fermer = { publier = false }) { action, motif ->
            publier = false
            lancer {
                evaluation = modele.cles.executer("eva-pub:${evaluation.id}:$action:${motif.orEmpty()}", famille = "eva-pub:${evaluation.id}:") { k ->
                    modele.client!!.publierEvaluation(evaluation.id, action, motif, k)
                }
                info = if (evaluation.publiee) "Publiée : les élèves et les familles voient les notes." else "Publication retirée."
            }
        }
    }
}

@Composable
private fun CaseEleve(b: BrouillonSaisie, id: String, nom: String, mouvement: String?, baremeM: Long, tick: Int, change: () -> Unit) {
    tick.let { }
    val case = b.caseDe(id)
    val conflit = b.conflit(id)
    val refus = b.refus(id)
    val a = b.analyse(id)
    Column(Modifier.padding(vertical = 6.dp)) {
        Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(nom, style = MaterialTheme.typography.bodyLarge)
                mouvement?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
                if (case?.saisieOuverte == false) Text("Saisie fermée (période arrêtée) : sur le web, avec un motif.", style = MaterialTheme.typography.bodySmall)
            }
            OutlinedTextField(
                value = b.texte(id),
                onValueChange = { b.taper(id, it); change() },
                enabled = b.modifiable(id),
                singleLine = true,
                isError = a is AnalyseCellule.Erreur || refus != null,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Text, imeAction = ImeAction.Next),
                modifier = Modifier.weight(0.6f).semantics { contentDescription = "Note de $nom" },
            )
        }
        when {
            a is AnalyseCellule.Erreur && b.modifiee(id) -> MessageErreur(a.message)
            b.videeSansEffet(id) -> Text("Une note ne s'efface pas : saisissez un code (abs, nn…) ou corrigez sur le web.", style = MaterialTheme.typography.bodySmall)
            a is AnalyseCellule.Statut && b.modifiee(id) -> Text(SaisieNotes.libelleStatut(a.statut), style = MaterialTheme.typography.bodySmall)
        }
        refus?.let { MessageErreur(it) }
        if (conflit != null) {
            Card(Modifier.fillMaxWidth().padding(top = 4.dp)) {
                Column(Modifier.padding(8.dp)) {
                    Text(
                        "Modifiée entre-temps" + (conflit.parNom?.let { " par $it" } ?: "") + " : " +
                            SaisieNotes.resume(conflit.statut, conflit.valeurM, baremeM),
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        TextButton(onClick = { b.reprendreServeur(id); change() }, modifier = Modifier.heightIn(min = 48.dp)) { Text("Garder la sienne") }
                        TextButton(onClick = { b.garderLaMienne(id); change() }, modifier = Modifier.heightIn(min = 48.dp)) { Text("Garder la mienne") }
                    }
                }
            }
        }
    }
}

@Composable
private fun ConfirmerPublication(e: EvaluationEva, fermer: () -> Unit, confirmer: (action: String, motif: String?) -> Unit) {
    var motif by remember { mutableStateOf("") }
    val retirer = e.publiee
    val motifValide = motif.trim().length in 3..500
    AlertDialog(
        onDismissRequest = fermer,
        properties = DIALOGUE_SECURISE,
        title = { Text(if (retirer) "Retirer la publication ?" else "Publier les notes ?") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    if (retirer) "Les élèves et les familles ne verront plus ces notes. Un motif écrit est exigé."
                    else "Les élèves et les familles verront aussitôt les notes de « ${e.titre} ».",
                )
                if (retirer) OutlinedTextField(motif, { motif = it }, label = { Text("Motif") }, isError = !motifValide)
            }
        },
        confirmButton = {
            TextButton(
                enabled = !retirer || motifValide,
                onClick = { confirmer(if (retirer) "retirer" else "maintenant", if (retirer) motif.trim() else null) },
                modifier = Modifier.testTag("eva-publication-confirmer"),
            ) {
                Text(if (retirer) "Retirer" else "Publier")
            }
        },
        dismissButton = { TextButton(onClick = fermer) { Text("Annuler") } },
    )
}
