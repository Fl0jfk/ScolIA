package fr.scolia.mobile.app.ecrans

import androidx.activity.compose.BackHandler
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
import androidx.compose.material3.AlertDialog
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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import fr.scolia.mobile.app.AppModele
import fr.scolia.mobile.app.ui.Chargeur
import fr.scolia.mobile.app.ui.Ligne
import fr.scolia.mobile.app.ui.MessageErreur
import fr.scolia.mobile.app.ui.Titre
import fr.scolia.mobile.app.ui.dateFr
import fr.scolia.mobile.core.Chemins
import fr.scolia.mobile.core.ErreurApi
import fr.scolia.mobile.core.EtatConnexion
import fr.scolia.mobile.core.Lecture
import kotlinx.coroutines.launch

// ---------- Messagerie ----------

@Composable
fun EcranMessagerie(modele: AppModele) {
    var filOuvert by rememberSaveable { mutableStateOf<String?>(null) }
    val f = filOuvert
    if (f != null) {
        BackHandler { filOuvert = null }
        EcranFil(modele, f) { filOuvert = null }
        return
    }
    Chargeur("fils", modele::sessionPerdue, { modele.lecteur!!.lire(Chemins.FILS, Chemins.listeFils) }) { fils, _ ->
        Text("Pour écrire un nouveau message, utilisez le web (choix des destinataires).", style = MaterialTheme.typography.bodySmall)
        if (fils.isEmpty()) Text("Aucun message.")
        LazyColumn {
            items(fils.sortedByDescending { it.lastMessageAt }, key = { it.id }) { fil ->
                val nonLus = if (fil.nbNonLus > 0) "${fil.nbNonLus} non lu(s)" else null
                Ligne(
                    fil.sujet,
                    listOfNotNull(fil.eleveLabel, dateFr(fil.lastMessageAt)).joinToString(" · "),
                    nonLus,
                    Modifier.clickable(role = Role.Button) { filOuvert = fil.id }.semantics {
                        contentDescription = "${fil.sujet}${nonLus?.let { ", $it" } ?: ""}"
                    },
                )
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun EcranFil(modele: AppModele, filId: String, retour: () -> Unit) {
    val client = modele.client!!
    var reponse by remember { mutableStateOf("") }
    var confirmer by remember { mutableStateOf(false) }
    var envoi by remember { mutableStateOf(false) }
    var erreur by remember { mutableStateOf<String?>(null) }
    val portee = rememberCoroutineScope()
    OutlinedButton(onClick = retour, modifier = Modifier.heightIn(min = 48.dp)) { Text("Retour aux messages") }
    Chargeur(filId, modele::sessionPerdue, { Lecture(client.fil(filId), false, System.currentTimeMillis()) }) { d, recharger ->
        Titre(d.fil.sujet)
        LazyColumn(Modifier.fillMaxWidth().heightIn(max = 480.dp)) {
            items(d.messages, key = { it.id }) { m ->
                Card(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                    Column(Modifier.padding(12.dp)) {
                        Text("${m.auteurLabel ?: "—"} · ${dateFr(m.createdAt)}", style = MaterialTheme.typography.labelLarge)
                        Text(m.corps)
                        if (m.pieces.isNotEmpty()) Text("${m.pieces.size} pièce(s) jointe(s) : à ouvrir sur le web", style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
        }
        OutlinedTextField(reponse, { reponse = it.take(5000) }, label = { Text("Répondre") }, minLines = 2, modifier = Modifier.fillMaxWidth())
        MessageErreur(erreur)
        Button(onClick = { confirmer = true }, enabled = reponse.isNotBlank() && !envoi, modifier = Modifier.heightIn(min = 48.dp)) { Text("Envoyer") }
        if (confirmer) {
            // Envoi de message : confirmation humaine (CLAUDE.md).
            AlertDialog(
                onDismissRequest = { confirmer = false },
                title = { Text("Envoyer ce message ?") },
                text = { Text(reponse.take(300)) },
                confirmButton = {
                    TextButton(onClick = {
                        confirmer = false
                        envoi = true // pas de double envoi
                        portee.launch {
                            erreur = null
                            try {
                                client.repondre(filId, reponse.trim())
                                reponse = ""
                                recharger()
                            } catch (e: ErreurApi.NonAuthentifie) {
                                modele.sessionPerdue()
                            } catch (e: kotlinx.coroutines.CancellationException) {
                                throw e
                            } catch (e: Exception) {
                                erreur = e.message
                            } finally {
                                envoi = false
                            }
                        }
                    }) { Text("Envoyer") }
                },
                dismissButton = { TextButton(onClick = { confirmer = false }) { Text("Annuler") } },
            )
        }
    }
}

// ---------- Mon compte : appareils, déconnexion, établissements ----------

@Composable
fun EcranProfil(modele: AppModele, c: EtatConnexion.Connecte) {
    val client = modele.client!!
    var erreur by remember { mutableStateOf<String?>(null) }
    val portee = rememberCoroutineScope()
    Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Titre("Mon compte")
        Ligne(c.me.email, "${modele.actif?.nom ?: ""} · rôle : ${c.me.role}")
        Ligne(
            "Passkeys : ${c.me.passkeyCount}",
            if (c.me.passkeyCount == 0L) "Ajoutez une passkey : elle sera bientôt obligatoire." else "Connexion sans mot de passe active.",
        )
        Titre("Appareils connectés")
        Chargeur("sessions", modele::sessionPerdue, { Lecture(client.sessions(), false, System.currentTimeMillis()) }) { sessions, recharger ->
            sessions.forEach { s ->
                Row(Modifier.fillMaxWidth()) {
                    Column(Modifier.weight(1f)) {
                        // Libellé issu du User-Agent : affiché comme texte brut (E8).
                        Ligne(
                            (s.appareil ?: "Appareil inconnu").take(60) + if (s.courante) " (cet appareil)" else "",
                            "${if (s.client == "mobile") "Appli" else "Web"} · vu le ${dateFr(s.lastSeenAt)}",
                        )
                    }
                    if (!s.courante) {
                        TextButton(onClick = {
                            portee.launch {
                                try {
                                    client.revoquerSession(s.id)
                                    recharger()
                                } catch (e: ErreurApi.NonAuthentifie) {
                                    modele.sessionPerdue()
                                } catch (e: ErreurApi) {
                                    erreur = e.message
                                }
                            }
                        }, modifier = Modifier.heightIn(min = 48.dp)) { Text("Déconnecter") }
                    }
                }
                HorizontalDivider()
            }
        }
        MessageErreur(erreur)
        if (modele.rattachements.size > 1) {
            Titre("Changer d'établissement")
            modele.rattachements.filter { it.baseUrl != modele.actif?.baseUrl }.forEach { r ->
                TextButton(onClick = { modele.choisir(r) }, modifier = Modifier.heightIn(min = 48.dp)) { Text(r.nom ?: r.slug) }
            }
        }
        Button(onClick = { modele.deconnexion() }, modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) { Text("Se déconnecter") }
    }
}
