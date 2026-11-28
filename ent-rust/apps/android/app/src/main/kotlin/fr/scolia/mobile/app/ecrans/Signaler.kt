package fr.scolia.mobile.app.ecrans

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import fr.scolia.mobile.app.AppModele
import fr.scolia.mobile.app.ui.Chargeur
import fr.scolia.mobile.app.ui.DIALOGUE_SECURISE
import fr.scolia.mobile.app.ui.Ligne
import fr.scolia.mobile.app.ui.MessageErreur
import fr.scolia.mobile.app.ui.MiseAJour
import fr.scolia.mobile.app.ui.Titre
import fr.scolia.mobile.app.ui.dateFr
import fr.scolia.mobile.core.ErreurApi
import fr.scolia.mobile.core.Lecture
import fr.scolia.mobile.core.MaDeclaration
import fr.scolia.mobile.core.Signalement
import kotlinx.coroutines.launch

/**
 * Signaler un incident (registre des violations de données, socle 27, complément G5) : tout personnel déclare en
 * trente secondes, depuis son téléphone, puis suit l'avancement. La qualification, le risque, la notification à la
 * CNIL et le registre restent sur le web (D-003). Envoi après confirmation (la direction est alertée).
 */
@Composable
fun EcranSignaler(modele: AppModele) {
    val client = modele.client!!
    var intitule by rememberSaveable { mutableStateOf("") }
    var description by rememberSaveable { mutableStateOf("") }
    var sansDonnee by rememberSaveable { mutableStateOf(false) }
    var tente by remember { mutableStateOf(false) }
    var confirmer by remember { mutableStateOf(false) }
    var envoi by remember { mutableStateOf(false) }
    var erreur by remember { mutableStateOf<String?>(null) }
    var recu by remember { mutableStateOf<MaDeclaration?>(null) }
    var version by remember { mutableIntStateOf(0) }
    val portee = rememberCoroutineScope()
    val erreurIntitule = Signalement.erreurIntitule(intitule)
    val erreurDescription = Signalement.erreurDescription(description)

    Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Titre("Signaler un incident")
        Text(
            "Une donnée personnelle perdue, envoyée à la mauvaise personne ou consultée sans droit ? Signalez-le tout de " +
                "suite : la direction a 72 heures pour agir. Aucune suite ne vise la personne qui signale.",
        )
        recu?.let { r ->
            Card(Modifier.fillMaxWidth().semantics { liveRegion = LiveRegionMode.Polite }.testTag("signaler-recu")) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Signalement ${r.reference} enregistré", style = MaterialTheme.typography.titleMedium)
                    Text(r.message.ifBlank { "Merci : la direction est prévenue." })
                }
            }
        }
        OutlinedTextField(
            intitule, { intitule = it.take(Signalement.INTITULE_MAX + 20) },
            label = { Text("Que s'est-il passé ? En une phrase") },
            placeholder = { Text("J'ai perdu une clé USB avec les notes de 4e B") },
            isError = tente && erreurIntitule != null,
            supportingText = { if (tente && erreurIntitule != null) Text(erreurIntitule) },
            modifier = Modifier.fillMaxWidth().testTag("signaler-intitule"),
        )
        OutlinedTextField(
            description, { description = it.take(Signalement.DESCRIPTION_MAX + 20) },
            label = { Text("Précisions (facultatif)") },
            isError = tente && erreurDescription != null,
            supportingText = {
                Text(
                    if (tente && erreurDescription != null) erreurDescription
                    else "Ce qui est touché, combien de personnes à peu près, ce que vous avez déjà fait. N'y recopiez pas les données elles-mêmes.",
                )
            },
            minLines = 3,
            modifier = Modifier.fillMaxWidth(),
        )
        Row(
            Modifier.fillMaxWidth().heightIn(min = 48.dp)
                .toggleable(value = sansDonnee, role = Role.Checkbox, onValueChange = { sansDonnee = it }),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Checkbox(checked = sansDonnee, onCheckedChange = null)
            Text("Aucune donnée personnelle n'est touchée (tentative d'hameçonnage, virus bloqué…)", Modifier.padding(start = 8.dp))
        }
        MessageErreur(erreur)
        Button(
            onClick = {
                tente = true
                if (erreurIntitule == null && erreurDescription == null) confirmer = true
            },
            enabled = !envoi,
            modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp).testTag("signaler-envoyer"),
        ) { Text("Signaler") }

        if (confirmer) {
            val corps = Signalement.corps(intitule, description, sansDonnee)
            AlertDialog(
                properties = DIALOGUE_SECURISE,
                onDismissRequest = { confirmer = false },
                title = { Text("Prévenir la direction ?") },
                text = { Text("« ${corps.intitule} » sera enregistré au registre de l'établissement et la direction sera alertée tout de suite.") },
                confirmButton = {
                    TextButton(
                        enabled = !envoi,
                        modifier = Modifier.testTag("signaler-confirmer"),
                        onClick = {
                            envoi = true
                            portee.launch {
                                erreur = null
                                recu = null
                                try {
                                    recu = modele.cles.executer(Signalement.intention(corps)) { k -> client.declarerIncident(corps, k) }
                                    intitule = ""
                                    description = ""
                                    sansDonnee = false
                                    tente = false
                                    version++
                                } catch (e: ErreurApi.NonAuthentifie) {
                                    modele.sessionPerdue()
                                } catch (e: ErreurApi.AppliTropAncienne) {
                                    MiseAJour.exiger(e.message)
                                } catch (e: kotlinx.coroutines.CancellationException) {
                                    throw e
                                } catch (e: ErreurApi.HorsLigne) {
                                    erreur = "Pas de connexion : rien n'est parti. Réessayez dès que possible ; en cas d'urgence, appelez la direction."
                                } catch (e: Exception) {
                                    erreur = e.message
                                } finally {
                                    envoi = false
                                    confirmer = false
                                }
                            }
                        },
                    ) { Text("Envoyer") }
                },
                dismissButton = { TextButton(onClick = { confirmer = false }) { Text("Annuler") } },
            )
        }

        HorizontalDivider(Modifier.padding(vertical = 8.dp))
        Titre("Mes signalements")
        Chargeur(version, modele::sessionPerdue, { Lecture(client.mesDeclarations(), false, System.currentTimeMillis()) }) { liste, _ ->
            if (liste.isEmpty()) Text("Vous n'avez rien signalé dans cet établissement.")
            liste.forEach { d ->
                Ligne("${d.reference} · ${d.intitule}", "Signalé le ${dateFr(d.connueLe)} · ${Signalement.libelleAvancement(d.avancement)}")
                HorizontalDivider()
            }
        }
    }
}
