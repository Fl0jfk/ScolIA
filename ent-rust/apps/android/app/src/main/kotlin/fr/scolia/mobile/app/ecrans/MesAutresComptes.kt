package fr.scolia.mobile.app.ecrans

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
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
import fr.scolia.mobile.core.ClesIntention
import fr.scolia.mobile.core.DeclarationRecue
import fr.scolia.mobile.core.ErreurApi
import fr.scolia.mobile.core.Individu
import fr.scolia.mobile.core.Lecture
import fr.scolia.mobile.core.declarerAutreCompte
import fr.scolia.mobile.core.individuMoi
import fr.scolia.mobile.core.repondreDeclaration
import kotlinx.coroutines.launch

/**
 * « Mes autres comptes » dans « Mon compte » (transverse individu, `TRANSVERSE_INDIVIDU.md` §11) : ce que
 * l'établissement reconnaît, les demandes reçues d'un autre compte (confirmer, refuser : chacun avec une
 * confirmation qui rappelle l'effet) et la déclaration d'un autre compte (réponse neutre). Mêmes textes que le web.
 * Jamais mis en cache hors ligne : les demandes sont datées et les réponses exigent le réseau.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SectionMesAutresComptes(modele: AppModele, monEmail: String? = null) {
    val client = modele.client ?: return
    val portee = rememberCoroutineScope()
    var annonce by remember { mutableStateOf<String?>(null) }
    var erreur by remember { mutableStateOf<String?>(null) }
    var envoi by remember { mutableStateOf(false) }
    var aConfirmer by remember { mutableStateOf<DeclarationRecue?>(null) }
    var aRefuser by remember { mutableStateOf<DeclarationRecue?>(null) }
    var declarer by rememberSaveable { mutableStateOf(false) }
    var email by rememberSaveable { mutableStateOf("") }
    var erreurEmail by remember { mutableStateOf<String?>(null) }
    var pourquoi by rememberSaveable { mutableStateOf(false) }

    /** Une écriture à la fois ; message annoncé ; la session perdue ramène à la connexion. */
    fun ecrire(intention: String, bloc: suspend (cle: String) -> Unit, succes: String, rechargerSiRefus: (() -> Unit)? = null, puis: () -> Unit) {
        if (envoi) return
        envoi = true
        annonce = null
        erreur = null
        portee.launch {
            try {
                modele.cles.executer(intention) { k -> bloc(k) }
                annonce = succes
                puis()
            } catch (e: ErreurApi.NonAuthentifie) {
                modele.sessionPerdue()
            } catch (e: ErreurApi.AppliTropAncienne) {
                MiseAJour.exiger(e.message)
            } catch (e: kotlinx.coroutines.CancellationException) {
                throw e
            } catch (e: ErreurApi) {
                erreur = e.message ?: "Envoi impossible."
                // Refus définitif (demande expirée, déjà traitée : 409, 404) : la liste n'est plus à jour.
                if (!ClesIntention.incertaine(e)) rechargerSiRefus?.invoke()
            } catch (e: Exception) {
                erreur = e.message ?: "Envoi impossible."
            } finally {
                envoi = false
            }
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.testTag("mes-autres-comptes")) {
        Titre("Mes autres comptes")
        Chargeur("individu-moi", modele::sessionPerdue, { Lecture(client.individuMoi(), false, System.currentTimeMillis()) }) { moi, recharger ->
            Text(Individu.resumeMoi(moi))
            Individu.resumeFiches(moi)?.let { Text(it) }
            Individu.trierLiens(moi.liens).forEach { l ->
                Ligne(Individu.libelleOrigine(l.origine), Individu.detailLien(l))
            }
            if (moi.liens.isNotEmpty()) {
                Text(Individu.LIEN_FAUX, style = MaterialTheme.typography.bodySmall)
            }

            val demandes = Individu.trierDemandes(moi.declarationsRecues)
            Text("Demandes à confirmer", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 8.dp).semantics { heading() })
            if (demandes.isEmpty()) {
                Text(Individu.AUCUNE_DEMANDE, style = MaterialTheme.typography.bodySmall)
            } else {
                Text(Individu.INTRO_DEMANDES)
                demandes.forEach { d ->
                    Ligne("Compte « ${d.declarant} »", Individu.detailDemande(d))
                    // Retour à la ligne si le texte est grand (taille de police du système).
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(
                            onClick = { aConfirmer = d },
                            enabled = !envoi,
                            modifier = Modifier.heightIn(min = 48.dp).testTag("individu-confirmer-${d.id}")
                                .semantics { contentDescription = "C'est moi : le compte ${d.declarant} est le mien" },
                        ) { Text("C'est moi") }
                        OutlinedButton(
                            onClick = { aRefuser = d },
                            enabled = !envoi,
                            modifier = Modifier.heightIn(min = 48.dp).testTag("individu-refuser-${d.id}")
                                .semantics { contentDescription = "Ce n'est pas moi : refuser la demande du compte ${d.declarant}" },
                        ) { Text("Ce n'est pas moi") }
                    }
                    HorizontalDivider()
                }
            }

            aConfirmer?.let { d ->
                AlertDialog(
                    properties = DIALOGUE_SECURISE,
                    onDismissRequest = { aConfirmer = null },
                    title = { Text("Confirmer votre autre compte") },
                    text = { Text("Confirmez-vous que le compte « ${d.declarant} » est bien le vôtre ?\n\n${Individu.EFFET_LIEN}") },
                    confirmButton = {
                        TextButton(onClick = {
                            aConfirmer = null
                            ecrire(
                                Individu.intentionRepondre(d.id, true),
                                { k -> client.repondreDeclaration(d.id, true, k) },
                                Individu.messageReponse(true),
                                recharger,
                                recharger,
                            )
                        }) { Text("Oui, c'est mon compte") }
                    },
                    dismissButton = { TextButton(onClick = { aConfirmer = null }) { Text("Annuler") } },
                )
            }
            aRefuser?.let { d ->
                // Sur le web, le refus part sans confirmation ; sur un téléphone, un appui de travers est plus
                // probable et le refus est définitif pour cette demande (30 jours de blocage) : on demande.
                AlertDialog(
                    properties = DIALOGUE_SECURISE,
                    onDismissRequest = { aRefuser = null },
                    title = { Text("Refuser cette demande ?") },
                    text = { Text("Le compte « ${d.declarant} » n'est pas le vôtre ? ${Individu.EFFET_REFUS}") },
                    confirmButton = {
                        TextButton(onClick = {
                            aRefuser = null
                            ecrire(
                                Individu.intentionRepondre(d.id, false),
                                { k -> client.repondreDeclaration(d.id, false, k) },
                                Individu.messageReponse(false),
                                recharger,
                                recharger,
                            )
                        }) { Text("Refuser") }
                    },
                    dismissButton = { TextButton(onClick = { aRefuser = null }) { Text("Annuler") } },
                )
            }
        }

        // Résultat de la dernière écriture, annoncé par TalkBack.
        annonce?.let {
            Text(it, modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite }.testTag("individu-annonce"))
        }
        MessageErreur(erreur)

        if (!declarer) {
            OutlinedButton(
                onClick = { declarer = true },
                modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp).testTag("individu-ouvrir-declarer"),
            ) { Text("Déclarer un autre compte…") }
        } else {
            Text("Déclarer un autre compte", style = MaterialTheme.typography.titleMedium, modifier = Modifier.semantics { heading() })
            Text(Individu.AIDE_DECLARER)
            val envoyer = {
                Individu.adresseAEnvoyer(email, monEmail).fold(
                    onSuccess = { e ->
                        erreurEmail = null
                        ecrire(
                            Individu.intentionDeclarer(e),
                            { k -> client.declarerAutreCompte(e, k) },
                            Individu.MESSAGE_DECLARATION,
                        ) { email = "" } // effacée seulement après l'accord du serveur
                    },
                    onFailure = { erreurEmail = it.message },
                )
            }
            OutlinedTextField(
                email,
                { email = it; erreurEmail = null },
                label = { Text("Adresse e-mail de l'autre compte") },
                singleLine = true,
                isError = erreurEmail != null,
                supportingText = erreurEmail?.let { m -> { Text(m) } },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email, imeAction = ImeAction.Send, autoCorrectEnabled = false),
                keyboardActions = KeyboardActions(onSend = { envoyer() }),
                modifier = Modifier.fillMaxWidth().testTag("individu-email"),
            )
            Button(
                onClick = { envoyer() },
                enabled = !envoi,
                modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp).testTag("individu-declarer"),
            ) { Text("Envoyer la demande") }
        }

        TextButton(onClick = { pourquoi = !pourquoi }, modifier = Modifier.heightIn(min = 48.dp)) {
            Text(if (pourquoi) "Masquer : pourquoi relier mes comptes ?" else "Pourquoi relier mes comptes ?")
        }
        if (pourquoi) Text(Individu.POURQUOI, style = MaterialTheme.typography.bodyMedium)
    }
}
