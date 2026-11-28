package fr.scolia.mobile.core

import kotlinx.serialization.DeserializationStrategy
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.descriptors.StructureKind
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.fail

/**
 * Socle 11, complément du palier MB2 : **clés des fixtures que le modèle Kotlin ne lit pas**.
 *
 * Le décodage du client est tolérant (`ignoreUnknownKeys`) : un champ que le serveur **renomme** passe inaperçu de
 * [ContratMb2Test] si le modèle lui donne une valeur par défaut (l'écran affiche alors la valeur par défaut, sans
 * erreur). Ce test parcourt chaque fixture dorée en même temps que le descripteur du modèle qui la décode et liste
 * chaque clé non nulle que le modèle ignore. Une clé ignorée volontairement est déclarée dans [ignorees] avec sa raison ;
 * toute autre clé rend le test rouge : le serveur a ajouté ou renommé un champ, il faut le lire ou le déclarer.
 *
 * Miroir Swift : `ContratClesTests.swift` (même liste de clés ignorées).
 */
class ContratClesTest {
    private val dossier: File = File(
        System.getProperty("scolia.contrat") ?: "../../../contrat/mobile",
    ).canonicalFile

    private val lecteur = Json { ignoreUnknownKeys = true }

    /** Le modèle qui décode chaque fixture (celui de la méthode d'[ApiClient] du cas de [ContratMb2Test]). */
    private val modeles: Map<String, DeserializationStrategy<*>> = mapOf(
        "auth_etablissement" to EtablissementPublic.serializer(),
        "auth_login" to LoginResponse.serializer(),
        "auth_passkey_login_start" to PasskeyChallenge.serializer(),
        "auth_passkey_register_start" to PasskeyChallengeEnrolement.serializer(),
        "auth_me" to Me.serializer(),
        "roles_mes_droits" to ListSerializer(Droit.serializer()),
        "auth_sessions" to ListSerializer(SessionAppareil.serializer()),
        "devices_push" to PushDevice.serializer(),
        "edt_mes_creneaux" to ListSerializer(EdtCreneau.serializer()),
        "eleves_edt" to ListSerializer(EdtCreneau.serializer()),
        "appels_ouvrir" to Appel.serializer(),
        "appel" to Appel.serializer(),
        "appels_clore" to Appel.serializer(),
        "famille_enfants" to ListSerializer(EnfantFamille.serializer()),
        "famille_resume" to FamilleResume.serializer(),
        "absences_justifier" to Absence.serializer(),
        "vs_absences" to ListSerializer(Absence.serializer()),
        "note_periodes" to ListSerializer(NotePeriode.serializer()),
        "eleves_notes" to ListSerializer(Note.serializer()),
        "eleves_moyennes" to ListSerializer(MoyenneMatiere.serializer()),
        "devoirs" to ListSerializer(Devoir.serializer()),
        "eleves_devoirs" to ListSerializer(Devoir.serializer()),
        "classes_effectif" to Effectif.serializer(),
        "cahier_texte" to ListSerializer(CahierTexte.serializer()),
        "cahier_texte_eleve" to ListSerializer(CahierTexte.serializer()),
        "messages_fils" to ListSerializer(MsgFil.serializer()),
        "messages_fil" to FilDetail.serializer(),
        "messages_repondre" to Message.serializer(),
        "annonces" to ListSerializer(Annonce.serializer()),
        "eleves_annonces" to ListSerializer(Annonce.serializer()),
        "notifications" to ListSerializer(Notification.serializer()),
        "eva_enseignements" to ListSerializer(EnseignementEva.serializer()),
        "eva_cours_periodes" to ListSerializer(PeriodeEva.serializer()),
        "eva_grille" to GrilleEva.serializer(),
        "eva_grille_apres_saisie" to GrilleEva.serializer(),
        "eva_evaluations" to EvaluationEva.serializer(),
        "eva_evaluations_publication" to EvaluationEva.serializer(),
        "eva_saisie" to SaisieEvaRetour.serializer(),
        "eva_saisie_conflit" to SaisieEvaRetour.serializer(),
        "eva_saisie_lot" to SaisieEvaRetour.serializer(),
    )

    /** Réponses sans corps (204) : rien à lire. */
    private val sansCorps = setOf("auth_sessions_revoquer", "auth_logout", "devices_push_supprimer", "appels_lignes")

    private fun fixtures(): List<File> {
        assertTrue(dossier.isDirectory, "dossier des fixtures introuvable : $dossier")
        return dossier.listFiles { f -> f.extension == "json" }.orEmpty().sortedBy { it.name }
    }

    @Test
    fun chaque_cle_non_nulle_des_fixtures_est_lue_par_le_modele_ou_declaree_ignoree() {
        val ignorees = ClesIgnorees.charger(dossier)
        val vues = mutableSetOf<String>()
        val echecs = mutableListOf<String>()
        for (f in fixtures()) {
            val nom = f.nameWithoutExtension
            val corps = lecteur.parseToJsonElement(f.readText()).jsonObject["corps"] ?: JsonNull
            if (nom in sansCorps) {
                if (corps !is JsonNull) echecs += "$nom : déclarée sans corps, mais la fixture en a un"
                continue
            }
            val modele = modeles[nom]
            if (modele == null) {
                echecs += "$nom : fixture sans modèle dans ContratClesTest"
                continue
            }
            // Contrôle de la table : le modèle déclaré décode bien la fixture.
            runCatching { lecteur.decodeFromJsonElement(modele, corps) }
                .onFailure { echecs += "$nom : le modèle déclaré ne décode pas la fixture (${it.message})" }
            val nonLues = mutableListOf<String>()
            clesNonLues(corps, modele.descriptor, "", nonLues)
            for (chemin in nonLues.distinct()) {
                val cle = "$nom:$chemin"
                vues += cle
                if (cle !in ignorees) echecs += "$cle : clé servie par le serveur, ignorée par le modèle Kotlin"
            }
        }
        for (cle in ignorees.keys - vues) echecs += "$cle : déclarée ignorée, mais absente des fixtures ou lue par le modèle (retirer la ligne)"
        if (echecs.isNotEmpty()) fail("clés de contrat non lues côté Kotlin :\n  " + echecs.joinToString("\n  "))
    }

    /** Le parcours lui-même : une clé renommée par le serveur est signalée, même si le modèle a une valeur par défaut. */
    @Test
    fun une_cle_renommee_par_le_serveur_est_signalee() {
        val me = lecteur.parseToJsonElement(File(dossier, "auth_me.json").readText()).jsonObject.getValue("corps").jsonObject
        val renomme = JsonObject(me.mapKeys { (k, _) -> if (k == "portee") "portee_session" else k })
        val sortie = mutableListOf<String>()
        clesNonLues(renomme, Me.serializer().descriptor, "", sortie)
        assertTrue("portee_session" in sortie && "portee" !in sortie, "$sortie")
        // Le décodage tolérant, lui, ne voit rien : c'est la limite que ce test comble.
        assertEquals("complete", lecteur.decodeFromJsonElement(Me.serializer(), renomme).portee)
        val liste = JsonArray(listOf(renomme, renomme))
        val sortieListe = mutableListOf<String>()
        clesNonLues(liste, ListSerializer(Me.serializer()).descriptor, "", sortieListe)
        assertTrue("[].portee_session" in sortieListe, "$sortieListe")
    }

    private fun clesNonLues(v: JsonElement, d: SerialDescriptor, chemin: String, sortie: MutableList<String>) {
        when (d.kind) {
            StructureKind.CLASS, StructureKind.OBJECT -> {
                val o = v as? JsonObject ?: return
                for ((k, x) in o) {
                    if (x is JsonNull) continue
                    val i = d.getElementIndex(k)
                    val ici = if (chemin.isEmpty()) k else "$chemin.$k"
                    if (i < 0) sortie += ici else clesNonLues(x, d.getElementDescriptor(i), ici, sortie)
                }
            }
            StructureKind.LIST -> {
                val a = v as? JsonArray ?: return
                for (x in a) clesNonLues(x, d.getElementDescriptor(0), "$chemin[]", sortie)
            }
            // Cartes, `JsonElement`, `JsonObject`, valeurs simples : le modèle prend tout ce qui vient.
            else -> Unit
        }
    }
}

/**
 * Clés ignorées volontairement, déclarées pour les deux cœurs dans `contrat/mobile/cles_ignorees.txt` : une ligne
 * « plateformes fixture:chemin — raison », plateformes = `tous`, `android` ou `ios`.
 */
object ClesIgnorees {
    fun charger(dossier: File, plateforme: String = "android"): Map<String, String> {
        val f = File(dossier, "cles_ignorees.txt")
        assertTrue(f.isFile, "liste des clés ignorées introuvable : $f")
        val sortie = mutableMapOf<String, String>()
        for (ligne in f.readLines().map { it.trim() }.filter { it.isNotEmpty() && !it.startsWith("#") }) {
            val (tete, raison) = ligne.split(" — ", limit = 2).let { it[0].trim() to it.getOrElse(1) { "" }.trim() }
            val (qui, cle) = tete.split(Regex("\\s+"), limit = 2).let { it[0] to it.getOrElse(1) { "" } }
            assertTrue(qui in setOf("tous", "android", "ios"), "« $ligne » : plateforme inconnue « $qui »")
            assertTrue(cle.contains(':') && raison.isNotEmpty(), "« $ligne » : attendu « plateformes fixture:chemin — raison »")
            assertTrue(cle !in sortie, "« $cle » déclarée deux fois")
            if (qui == "tous" || qui == plateforme) sortie[cle] = raison
        }
        return sortie
    }
}
