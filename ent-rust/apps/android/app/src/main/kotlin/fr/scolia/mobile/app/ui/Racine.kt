package fr.scolia.mobile.app.ui

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountCircle
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import fr.scolia.mobile.app.AppModele
import fr.scolia.mobile.app.ecrans.EcranModule
import fr.scolia.mobile.core.EtatConnexion
import fr.scolia.mobile.core.Module

/** Aiguillage : rattachement → connexion → verrou → enrôlement → coquille selon le rôle. */
@Composable
fun RacineScolIA(modele: AppModele, onDeverrouiller: () -> Unit) {
    val actif = modele.actif
    when {
        actif == null -> EcranRattachement(modele)
        modele.etat is EtatConnexion.Deconnecte || modele.etat is EtatConnexion.SecondFacteur -> EcranConnexion(modele)
        modele.verrouille -> EcranVerrou(onDeverrouiller)
        else -> {
            val c = modele.etat as EtatConnexion.Connecte
            if (c.espace.enrolementRequis) EcranEnrolement(modele) else Coquille(modele, c)
        }
    }
}

@Composable
private fun Page(contenu: @Composable () -> Unit) {
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) { contenu() }
}

/** Rattachement à un établissement : lien d'invitation, QR code, ou adresse (D-012). */
@Composable
fun EcranRattachement(modele: AppModele) {
    var saisie by rememberSaveable { mutableStateOf("") }
    val scanner = rememberLauncherForActivityResult(ScanContract()) { r -> r.contents?.let { modele.rattacher(it) } }
    Page {
        Titre("Rejoindre mon établissement")
        Text("Ouvrez le lien d'invitation reçu par e-mail ou SMS, ou scannez le QR code remis par l'établissement.")
        Button(
            onClick = {
                scanner.launch(
                    ScanOptions().setDesiredBarcodeFormats(ScanOptions.QR_CODE).setPrompt("Visez le QR code ScolIA").setBeepEnabled(false),
                )
            },
            modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp),
        ) { Text("Scanner un QR code") }
        Spacer(Modifier.height(8.dp))
        OutlinedTextField(
            saisie, { saisie = it },
            label = { Text("Adresse de l'établissement") },
            supportingText = { Text("Exemple : lpnb.scolia.fr") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
            modifier = Modifier.fillMaxWidth(),
        )
        OutlinedButton(onClick = { modele.rattacher(saisie) }, enabled = !modele.occupe && saisie.isNotBlank(), modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) {
            Text("Continuer")
        }
        // Recherche par nom ou ville : API d'annuaire public manquante (docs/MOBILE.md).
        Text("Recherche par nom ou ville : bientôt disponible.")
        MessageErreur(modele.erreur)
        if (modele.rattachements.isNotEmpty()) {
            Titre("Mes établissements")
            modele.rattachements.forEach { r ->
                TextButton(onClick = { modele.choisir(r) }, modifier = Modifier.heightIn(min = 48.dp)) { Text(r.nom ?: r.slug) }
            }
        }
    }
}

/** Connexion : passkey d'abord (D-009), mot de passe en secours, session mobile (D-011). */
@Composable
fun EcranConnexion(modele: AppModele) {
    var email by rememberSaveable { mutableStateOf(modele.dernierEmail ?: "") }
    var mdp by remember { mutableStateOf("") }
    var avecMotDePasse by rememberSaveable { mutableStateOf(false) }
    val etat = modele.etat
    Page {
        Titre(modele.actif?.nom ?: modele.actif?.slug ?: "ScolIA")
        if (etat is EtatConnexion.SecondFacteur) {
            Text("Mot de passe accepté. Confirmez avec votre passkey.")
            Button(onClick = { modele.secondFacteur(etat.defi) }, enabled = !modele.occupe, modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) {
                Text("Utiliser ma passkey")
            }
        } else {
            OutlinedTextField(
                email, { email = it },
                label = { Text("Adresse e-mail") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
                modifier = Modifier.fillMaxWidth(),
            )
            Button(
                onClick = { modele.connexionPasskey(email) },
                enabled = !modele.occupe && email.isNotBlank(),
                modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp),
            ) { Text("Se connecter avec une passkey") }
            if (avecMotDePasse) {
                OutlinedTextField(
                    mdp, { mdp = it },
                    label = { Text("Mot de passe") },
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedButton(
                    onClick = { modele.connexionMotDePasse(email, mdp).also { mdp = "" } },
                    enabled = !modele.occupe && email.isNotBlank() && mdp.isNotEmpty(),
                    modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp),
                ) { Text("Se connecter") }
            } else {
                TextButton(onClick = { avecMotDePasse = true }, modifier = Modifier.heightIn(min = 48.dp)) {
                    Text("Pas encore de passkey ? Utiliser mon mot de passe")
                }
            }
        }
        MessageErreur(modele.erreur)
        TextButton(onClick = { modele.oublierEtablissement(modele.actif!!) }, modifier = Modifier.heightIn(min = 48.dp)) {
            Text("Changer d'établissement")
        }
    }
}

@Composable
fun EcranVerrou(onDeverrouiller: () -> Unit) {
    LaunchedEffect(Unit) { onDeverrouiller() }
    Page {
        Titre("ScolIA est verrouillée")
        Text("Déverrouillez avec votre empreinte, votre visage ou le code de l'appareil.")
        Button(onClick = onDeverrouiller, modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) { Text("Déverrouiller") }
    }
}

/** Passkey obligatoire (D-009) : seul cet écran est accessible tant qu'elle n'est pas créée. */
@Composable
fun EcranEnrolement(modele: AppModele) {
    var mdp by remember { mutableStateOf("") }
    Page {
        Titre("Créer votre passkey")
        Text("Votre établissement demande une passkey : elle remplace le mot de passe et protège votre compte. Elle est enregistrée dans ce téléphone (ou votre gestionnaire de mots de passe).")
        OutlinedTextField(
            mdp, { mdp = it },
            label = { Text("Mot de passe actuel") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth(),
        )
        Button(onClick = { modele.enroler(mdp).also { mdp = "" } }, enabled = !modele.occupe && mdp.isNotEmpty(), modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) {
            Text("Créer ma passkey")
        }
        MessageErreur(modele.erreur)
        TextButton(onClick = { modele.deconnexion() }) { Text("Se déconnecter") }
    }
}

fun icone(m: Module): ImageVector = when (m) {
    Module.ACCUEIL -> Icons.Filled.Home
    Module.APPEL -> Icons.Filled.CheckCircle
    Module.VIE_SCOLAIRE -> Icons.Filled.Warning
    Module.ABSENCES -> Icons.Filled.Info
    Module.NOTES -> Icons.Filled.Star
    Module.SAISIE_NOTES -> Icons.Filled.Edit
    Module.AGENDA -> Icons.Filled.List
    Module.EDT -> Icons.Filled.DateRange
    Module.MESSAGERIE -> Icons.Filled.Email
    Module.PROFIL -> Icons.Filled.AccountCircle
}

/** Coquille : barre du bas (4 onglets) + menu « Plus » pour les autres modules. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Coquille(modele: AppModele, connecte: EtatConnexion.Connecte) {
    val espace = connecte.espace
    var courant by rememberSaveable { mutableStateOf(espace.modules.first()) }
    var menu by remember { mutableStateOf(false) }
    BackHandler(enabled = courant != espace.modules.first()) { courant = espace.modules.first() }
    Scaffold(
        topBar = { TopAppBar(title = { Text(courant.titre) }) },
        bottomBar = {
            NavigationBar {
                espace.onglets.forEach { m ->
                    NavigationBarItem(
                        selected = courant == m,
                        onClick = { courant = m },
                        icon = { Icon(icone(m), contentDescription = null) },
                        label = { Text(m.titre, maxLines = 1) },
                    )
                }
                if (espace.autres.isNotEmpty()) {
                    NavigationBarItem(
                        selected = courant in espace.autres,
                        onClick = { menu = true },
                        icon = {
                            Icon(Icons.Filled.Menu, contentDescription = null)
                            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                                espace.autres.forEach { m ->
                                    DropdownMenuItem(
                                        text = { Text(m.titre) },
                                        leadingIcon = { Icon(icone(m), contentDescription = null) },
                                        onClick = { courant = m; menu = false },
                                    )
                                }
                            }
                        },
                        label = { Text("Plus") },
                    )
                }
            }
        },
    ) { marges ->
        Column(Modifier.padding(marges).padding(horizontal = 16.dp).fillMaxSize()) {
            EcranModule(courant, modele, connecte, ouvrir = { courant = it })
        }
    }
}
