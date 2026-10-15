package fr.scola.famille

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.util.UUID

/**
 * Client HTTP pour l’API ENT Rust (Bearer) — famille.
 * Surfaces : login, push, enfants, résumé, messagerie, signature carnet, sorties.
 */
class EntApiClient(
    private val baseUrl: String = "http://10.0.2.2:3001",
) {
    var token: String? = null
        private set
    var etablissementId: UUID? = null
        private set

    data class Session(
        val token: String,
        val userId: UUID,
        val email: String,
        val etablissementId: UUID,
        val role: String,
    )

    data class Enfant(
        val id: UUID,
        val nom: String,
        val prenom: String,
        val classe: String?,
        val peutJustifier: Boolean,
    )

    data class Absence(
        val id: UUID,
        val debut: String,
        val fin: String,
        val statut: String,
        val motif: String,
    )

    data class CarnetEntree(
        val id: UUID,
        val titre: String?,
        val corps: String?,
    )

    data class Note(
        val id: UUID,
        val valeur: Double?,
        val libelle: String?,
        val matiereLibelle: String?,
    )

    data class Resume(
        val eleveId: UUID,
        val absencesAJustifier: List<Absence>,
        val carnetASigner: List<CarnetEntree>,
        val notes: List<Note>,
    )

    data class MsgFil(
        val id: UUID,
        val eleveId: UUID,
        val sujet: String,
        val lastMessageAt: String?,
    )

    data class MsgPiece(
        val id: UUID,
        val nom: String,
        val mimeType: String,
        val tailleOctets: Int?,
    )

    data class Message(
        val id: UUID,
        val filId: UUID,
        val auteurUserId: UUID,
        val auteurLabel: String?,
        val corps: String,
        val createdAt: String,
        val pieces: List<MsgPiece> = emptyList(),
    )

    data class FilDetail(
        val fil: MsgFil,
        val messages: List<Message>,
    )

    data class AutorisationSortie(
        val id: UUID,
        val eleveId: UUID,
        val matin: Boolean,
        val midi: Boolean,
        val soir: Boolean,
        val interditEtablissement: Boolean,
    )

    data class DemandeSortie(
        val id: UUID,
        val eleveId: UUID,
        val jour: String,
        val creneau: String,
        val motif: String,
        val statut: String,
    )

    data class RecapSortie(
        val eleveId: UUID,
        val autorisation: AutorisationSortie?,
        val demandesOuvertes: List<DemandeSortie>,
    )

    data class DocASigner(
        val id: UUID,
        val eleveId: UUID,
        val code: String,
        val titre: String,
        val statut: String,
    )

    data class MenuCantine(
        val id: UUID,
        val jour: String,
        val entree: String,
        val plat: String,
        val dessert: String,
        val notes: String,
        val publie: Boolean,
    )

    data class ConventionStage(
        val id: UUID,
        val eleveId: UUID,
        val entrepriseId: UUID,
        val entrepriseLabel: String?,
        val debut: String,
        val fin: String,
        val tuteurNom: String,
        val statut: String,
    )

    data class JournalEntree(
        val id: UUID,
        val domaine: String,
        val action: String,
        val libelle: String,
        val createdAt: String?,
    )

    data class EdtCreneau(
        val id: UUID,
        val classe: String,
        val matiereLibelle: String,
        val profLabel: String?,
        val heureDebut: String,
        val heureFin: String,
        val salle: String?,
    )

    data class PassageRepas(
        val id: UUID,
        val eleveId: UUID,
        val jour: String,
        val badgeLu: String,
        val source: String,
    )

    data class Facture(
        val id: UUID,
        val numero: String,
        val libelle: String,
        val statut: String,
        val totalCentimes: Long,
        val nature: String?,
    )

    data class Quittance(
        val id: UUID,
        val factureId: UUID,
        val numero: String,
        val montantCentimes: Long,
        val moyen: String,
        val emiseAt: String,
    )

    data class CahierTexte(
        val id: UUID,
        val classe: String,
        val matiereLibelle: String,
        val titre: String,
        val consigne: String,
        val pourLe: String,
    )

    data class AlerteCpe(
        val id: UUID,
        val titre: String,
        val detail: String,
        val severite: String,
        val statut: String,
        val jour: String,
    )

    data class RecapInternat(
        val eleveId: UUID,
        val chambreCode: String?,
        val chambreBatiment: String?,
        val presenceStatut: String?,
        val suivis: List<SuiviInternat>,
    )

    data class SuiviInternat(
        val id: UUID,
        val jour: String,
        val texte: String,
    )

    data class ConseilDossier(
        val id: UUID,
        val appreciation: String,
        val mention: String?,
        val decisionTexte: String,
    )

    data class FicheSante(
        val allergies: String,
        val traitements: String,
        val contreIndications: String,
        val paiActif: Boolean,
        val medecinNom: String?,
        val urgenceTel: String?,
    )

    data class Pai(
        val id: UUID,
        val motif: String,
        val mesuresAccueil: String,
        val protocoleUrgence: String,
        val dateDebut: String,
        val actif: Boolean,
        val signeFamille: Boolean,
    )

    data class Voeu(
        val id: UUID,
        val filiereId: UUID,
        val filiereLabel: String?,
        val rang: Int,
        val anneeScolaire: String,
    )

    data class InfoCompValeur(
        val champId: UUID,
        val champLabel: String?,
        val valeur: String,
    )

    data class DispenseRepas(
        val id: UUID,
        val jour: String,
        val motif: String,
        val annulee: Boolean,
    )

    data class SoldePoints(
        val eleveId: UUID,
        val solde: Int,
        val depart: Int,
    )

    data class Sanction(
        val id: UUID,
        val jour: String,
        val dureeMinutes: Int?,
        val note: String?,
        val statut: String,
    )

    data class Incident(
        val id: UUID,
        val jour: String,
        val description: String?,
    )

    data class CommissionDossier(
        val id: UUID,
        val motifs: String,
        val decisionLibelle: String?,
        val pointsRetires: Int,
        val signaleFamille: Boolean,
    )

    data class DossierPsy(
        val axesSuivi: String,
        val suiviActif: Boolean,
        val signaleFamille: Boolean,
    )

    data class RdvPsy(
        val id: UUID,
        val jour: String,
        val heureDebut: String,
        val heureFin: String,
        val kindLibelle: String,
        val statut: String,
        val signaleFamille: Boolean,
    )

    data class ReinscriptionOption(
        val id: UUID,
        val code: String,
        val libelle: String,
        val statut: String,
    )

    data class ReinscriptionEnfant(
        val enfantId: UUID,
        val prenom: String,
        val nom: String,
        val confirmee: Boolean,
        val documentsManquants: List<String>,
        val acompteOk: Boolean,
        val options: List<ReinscriptionOption>,
        val projectionNiveau: String?,
        val projectionAnnee: String?,
    )

    data class ReinscriptionFamille(
        val anneeN1: String,
        val campagneOuverte: Boolean,
        val enfants: List<ReinscriptionEnfant>,
    )

    data class PassageInfirmerie(
        val id: UUID,
        val jour: String,
        val motif: String,
        val motifLibelle: String,
        val noteSoins: String,
    )

    data class NotePeriode(
        val id: UUID,
        val code: String,
        val libelle: String,
        val statut: String,
    )

    data class MoyenneMatiere(
        val matiereId: UUID,
        val matiereLibelle: String?,
        val periodeId: UUID,
        val moyenneSur20: Double,
    )

    suspend fun login(email: String, password: String, etablissementId: UUID): Session =
        withContext(Dispatchers.IO) {
            val body = JSONObject()
                .put("email", email)
                .put("password", password)
                .put("etablissement_id", etablissementId.toString())
            val json = postJson("/auth/login", body, authed = false)
            val session = Session(
                token = json.getString("token"),
                userId = UUID.fromString(json.getString("user_id")),
                email = json.getString("email"),
                etablissementId = UUID.fromString(json.getString("etablissement_id")),
                role = json.getString("role"),
            )
            this@EntApiClient.token = session.token
            this@EntApiClient.etablissementId = session.etablissementId
            session
        }

    suspend fun registerPush(platform: String, token: String, appId: String?) =
        withContext(Dispatchers.IO) {
            val body = JSONObject()
                .put("platform", platform)
                .put("token", token)
            if (appId != null) body.put("app_id", appId)
            postJson("/devices/push", body, authed = true)
            Unit
        }

    suspend fun enfants(): List<Enfant> = withContext(Dispatchers.IO) {
        val arr = getJsonArray("/famille/enfants")
        buildList {
            for (i in 0 until arr.length()) {
                val o = arr.getJSONObject(i)
                add(
                    Enfant(
                        id = UUID.fromString(o.getString("id")),
                        nom = o.getString("nom"),
                        prenom = o.getString("prenom"),
                        classe = o.optString("classe", null),
                        peutJustifier = o.optBoolean("peut_justifier_absence", false),
                    ),
                )
            }
        }
    }

    suspend fun resume(eleveId: UUID): Resume = withContext(Dispatchers.IO) {
        val o = getJson("/famille/enfants/$eleveId/resume")
        Resume(
            eleveId = UUID.fromString(o.getString("eleve_id")),
            absencesAJustifier = o.getJSONArray("absences_a_justifier").toAbsences(),
            carnetASigner = o.getJSONArray("carnet_a_signer").toCarnet(),
            notes = o.getJSONArray("notes").toNotes(),
        )
    }

    suspend fun justifierAbsence(absenceId: UUID, motif: String) = withContext(Dispatchers.IO) {
        postJson(
            "/absences/$absenceId/justifier",
            JSONObject().put("motif_justif", motif),
            authed = true,
        )
        Unit
    }

    data class AbsenceMotif(val code: String, val libelle: String)

    suspend fun motifsAbsence(): List<AbsenceMotif> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/motifs-absence")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(AbsenceMotif(code = o.getString("code"), libelle = o.getString("libelle")))
                }
            }
        }

    suspend fun declarerAbsence(eleveId: UUID, debut: String, fin: String, motif: String) =
        withContext(Dispatchers.IO) {
            postJson(
                "/eleves/$eleveId/absences",
                JSONObject()
                    .put("debut", debut)
                    .put("fin", fin)
                    .put("motif", motif),
                authed = true,
            )
            Unit
        }

    suspend fun signerCarnet(entreeId: UUID) = withContext(Dispatchers.IO) {
        postJson("/carnet/$entreeId/signer", JSONObject(), authed = true)
        Unit
    }

    suspend fun messages(): List<MsgFil> = withContext(Dispatchers.IO) {
        val arr = getJsonArray("/messages/fils")
        buildList {
            for (i in 0 until arr.length()) {
                val o = arr.getJSONObject(i)
                add(
                    MsgFil(
                        id = UUID.fromString(o.getString("id")),
                        eleveId = UUID.fromString(o.getString("eleve_id")),
                        sujet = o.getString("sujet"),
                        lastMessageAt = o.optString("last_message_at", null),
                    ),
                )
            }
        }
    }

    suspend fun filDetail(filId: UUID): FilDetail = withContext(Dispatchers.IO) {
        val o = getJson("/messages/fils/$filId")
        val f = o.getJSONObject("fil")
        val fil = MsgFil(
            id = UUID.fromString(f.getString("id")),
            eleveId = UUID.fromString(f.getString("eleve_id")),
            sujet = f.getString("sujet"),
            lastMessageAt = f.optString("last_message_at", null),
        )
        val arr = o.getJSONArray("messages")
        val messages = buildList {
            for (i in 0 until arr.length()) {
                val m = arr.getJSONObject(i)
                val pcs = if (m.has("pieces") && !m.isNull("pieces")) {
                    val pa = m.getJSONArray("pieces")
                    buildList {
                        for (j in 0 until pa.length()) {
                            val p = pa.getJSONObject(j)
                            add(
                                MsgPiece(
                                    id = UUID.fromString(p.getString("id")),
                                    nom = p.getString("nom"),
                                    mimeType = p.optString("mime_type", "application/octet-stream"),
                                    tailleOctets = if (p.has("taille_octets") && !p.isNull("taille_octets")) {
                                        p.getInt("taille_octets")
                                    } else {
                                        null
                                    },
                                ),
                            )
                        }
                    }
                } else {
                    emptyList()
                }
                add(
                    Message(
                        id = UUID.fromString(m.getString("id")),
                        filId = UUID.fromString(m.getString("fil_id")),
                        auteurUserId = UUID.fromString(m.getString("auteur_user_id")),
                        auteurLabel = if (m.has("auteur_label") && !m.isNull("auteur_label")) {
                            m.getString("auteur_label").takeIf { it.isNotBlank() }
                        } else {
                            null
                        },
                        corps = m.getString("corps"),
                        createdAt = m.getString("created_at"),
                        pieces = pcs,
                    ),
                )
            }
        }
        FilDetail(fil = fil, messages = messages)
    }

    suspend fun repondre(filId: UUID, corps: String) = withContext(Dispatchers.IO) {
        postJson(
            "/messages/fils/$filId/repondre",
            JSONObject().put("corps", corps),
            authed = true,
        )
        Unit
    }

    suspend fun pdfFacture(factureId: UUID): ByteArray = withContext(Dispatchers.IO) {
        getBytes("/facturation/factures/$factureId/pdf")
    }

    suspend fun pdfQuittance(quittanceId: UUID): ByteArray = withContext(Dispatchers.IO) {
        getBytes("/facturation/quittances/$quittanceId/pdf")
    }

    data class PaiPiece(val id: UUID, val nom: String)

    suspend fun pdfFicheSante(eleveId: UUID): ByteArray = withContext(Dispatchers.IO) {
        getBytes("/eleves/$eleveId/fiche-sante.pdf")
    }

    suspend fun pdfCourrierPsy(eleveId: UUID): ByteArray = withContext(Dispatchers.IO) {
        getBytes("/eleves/$eleveId/dossier-psy/courrier.pdf")
    }

    suspend fun paiPieces(eleveId: UUID): List<PaiPiece> = withContext(Dispatchers.IO) {
        val arr = getJsonArray("/eleves/$eleveId/pai/pieces")
        buildList {
            for (i in 0 until arr.length()) {
                val o = arr.getJSONObject(i)
                add(
                    PaiPiece(
                        id = UUID.fromString(o.getString("id")),
                        nom = o.getString("nom"),
                    ),
                )
            }
        }
    }

    suspend fun pdfPaiPiece(pieceId: UUID): ByteArray = withContext(Dispatchers.IO) {
        getBytes("/pai/pieces/$pieceId")
    }

    suspend fun deposerPaiPiece(eleveId: UUID, nom: String, mimeType: String, bytes: ByteArray) =
        withContext(Dispatchers.IO) {
            val b64 = android.util.Base64.encodeToString(bytes, android.util.Base64.NO_WRAP)
            postJson(
                "/eleves/$eleveId/pai/pieces",
                JSONObject()
                    .put("nom", nom)
                    .put("mime_type", mimeType)
                    .put("contenu_base64", b64),
                authed = true,
            )
            Unit
        }

    data class DispenseVs(
        val id: UUID,
        val motif: String,
        val debut: String,
        val fin: String,
        val statut: String,
    )

    suspend fun dispensesVs(eleveId: UUID): List<DispenseVs> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/dispenses-vs")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        DispenseVs(
                            id = UUID.fromString(o.getString("id")),
                            motif = o.getString("motif"),
                            debut = o.getString("debut"),
                            fin = o.getString("fin"),
                            statut = o.getString("statut"),
                        ),
                    )
                }
            }
        }

    suspend fun poserDispenseVs(
        eleveId: UUID,
        motif: String,
        debut: String,
        fin: String,
        commentaire: String,
    ) = withContext(Dispatchers.IO) {
        postJson(
            "/eleves/$eleveId/dispenses-vs",
            JSONObject()
                .put("motif", motif)
                .put("debut", debut)
                .put("fin", fin)
                .put("commentaire", commentaire),
            authed = true,
        )
        Unit
    }

    suspend fun attacherPieceMessage(messageId: UUID, nom: String, mimeType: String, bytes: ByteArray) =
        withContext(Dispatchers.IO) {
            val b64 = android.util.Base64.encodeToString(bytes, android.util.Base64.NO_WRAP)
            postJson(
                "/messages/$messageId/pieces",
                JSONObject()
                    .put("nom", nom)
                    .put("mime_type", mimeType)
                    .put("contenu_base64", b64),
                authed = true,
            )
            Unit
        }

    suspend fun telechargerPieceMessage(pieceId: UUID): ByteArray =
        withContext(Dispatchers.IO) {
            getBytes("/messages/pieces/$pieceId")
        }

    suspend fun recapSortie(eleveId: UUID): RecapSortie = withContext(Dispatchers.IO) {
        val o = getJson("/eleves/$eleveId/sorties")
        val auth = if (o.isNull("autorisation")) null else o.getJSONObject("autorisation").let { a ->
            AutorisationSortie(
                id = UUID.fromString(a.getString("id")),
                eleveId = UUID.fromString(a.getString("eleve_id")),
                matin = a.getBoolean("matin"),
                midi = a.getBoolean("midi"),
                soir = a.getBoolean("soir"),
                interditEtablissement = a.getBoolean("interdit_etablissement"),
            )
        }
        val demandes = o.getJSONArray("demandes_ouvertes")
        RecapSortie(
            eleveId = UUID.fromString(o.getString("eleve_id")),
            autorisation = auth,
            demandesOuvertes = buildList {
                for (i in 0 until demandes.length()) {
                    val d = demandes.getJSONObject(i)
                    add(
                        DemandeSortie(
                            id = UUID.fromString(d.getString("id")),
                            eleveId = UUID.fromString(d.getString("eleve_id")),
                            jour = d.getString("jour"),
                            creneau = d.getString("creneau"),
                            motif = d.getString("motif"),
                            statut = d.getString("statut"),
                        ),
                    )
                }
            },
        )
    }

    suspend fun upsertAutorisationSortie(
        eleveId: UUID,
        matin: Boolean,
        midi: Boolean,
        soir: Boolean,
    ): AutorisationSortie = withContext(Dispatchers.IO) {
        val body = JSONObject()
            .put("matin", matin)
            .put("midi", midi)
            .put("soir", soir)
            .put("interdit_etablissement", false)
        val a = putJson("/eleves/$eleveId/sorties", body)
        AutorisationSortie(
            id = UUID.fromString(a.getString("id")),
            eleveId = UUID.fromString(a.getString("eleve_id")),
            matin = a.getBoolean("matin"),
            midi = a.getBoolean("midi"),
            soir = a.getBoolean("soir"),
            interditEtablissement = a.getBoolean("interdit_etablissement"),
        )
    }

    suspend fun demanderSortie(
        eleveId: UUID,
        jour: String,
        creneau: String,
        motif: String,
    ): DemandeSortie = withContext(Dispatchers.IO) {
        val body = JSONObject()
            .put("jour", jour)
            .put("creneau", creneau)
            .put("motif", motif)
        val d = postJson("/eleves/$eleveId/sorties/demandes", body, authed = true)
        DemandeSortie(
            id = UUID.fromString(d.getString("id")),
            eleveId = UUID.fromString(d.getString("eleve_id")),
            jour = d.getString("jour"),
            creneau = d.getString("creneau"),
            motif = d.getString("motif"),
            statut = d.getString("statut"),
        )
    }

    suspend fun docsASigner(eleveId: UUID): List<DocASigner> = withContext(Dispatchers.IO) {
        val arr = getJsonArray("/eleves/$eleveId/docs-a-signer?actives=true")
        buildList {
            for (i in 0 until arr.length()) {
                val o = arr.getJSONObject(i)
                add(
                    DocASigner(
                        id = UUID.fromString(o.getString("id")),
                        eleveId = UUID.fromString(o.getString("eleve_id")),
                        code = o.getString("code"),
                        titre = o.getString("titre"),
                        statut = o.getString("statut"),
                    ),
                )
            }
        }
    }

    suspend fun signerDoc(docId: UUID): DocASigner = withContext(Dispatchers.IO) {
        val o = postJson("/docs-a-signer/$docId/signer", JSONObject(), authed = true)
        DocASigner(
            id = UUID.fromString(o.getString("id")),
            eleveId = UUID.fromString(o.getString("eleve_id")),
            code = o.getString("code"),
            titre = o.getString("titre"),
            statut = o.getString("statut"),
        )
    }

    suspend fun menusCantine(debut: String, fin: String): List<MenuCantine> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/menus-cantine?debut=$debut&fin=$fin")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        MenuCantine(
                            id = UUID.fromString(o.getString("id")),
                            jour = o.getString("jour"),
                            entree = o.optString("entree"),
                            plat = o.getString("plat"),
                            dessert = o.optString("dessert"),
                            notes = o.optString("notes"),
                            publie = o.optBoolean("publie", true),
                        ),
                    )
                }
            }
        }

    suspend fun conventionsStage(eleveId: UUID): List<ConventionStage> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/conventions-stage")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        ConventionStage(
                            id = UUID.fromString(o.getString("id")),
                            eleveId = UUID.fromString(o.getString("eleve_id")),
                            entrepriseId = UUID.fromString(o.getString("entreprise_id")),
                            entrepriseLabel = if (o.has("entreprise_label") && !o.isNull("entreprise_label")) {
                                o.getString("entreprise_label")
                            } else {
                                null
                            },
                            debut = o.getString("debut"),
                            fin = o.getString("fin"),
                            tuteurNom = o.getString("tuteur_nom"),
                            statut = o.getString("statut"),
                        ),
                    )
                }
            }
        }

    suspend fun journal(eleveId: UUID, limit: Int = 80): List<JournalEntree> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/journal?limit=$limit")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        JournalEntree(
                            id = UUID.fromString(o.getString("id")),
                            domaine = o.getString("domaine"),
                            action = o.getString("action"),
                            libelle = o.getString("libelle"),
                            createdAt = o.optString("created_at").takeIf { it.isNotBlank() },
                        ),
                    )
                }
            }
        }

    suspend fun passagesInfirmerie(eleveId: UUID): List<PassageInfirmerie> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/infirmerie/passages?eleve_id=$eleveId")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        PassageInfirmerie(
                            id = UUID.fromString(o.getString("id")),
                            jour = o.getString("jour"),
                            motif = o.getString("motif"),
                            motifLibelle = o.optString("motif_libelle", o.getString("motif")),
                            noteSoins = o.optString("note_soins", ""),
                        ),
                    )
                }
            }
        }

    suspend fun notePeriodes(): List<NotePeriode> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/note-periodes")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        NotePeriode(
                            id = UUID.fromString(o.getString("id")),
                            code = o.getString("code"),
                            libelle = o.getString("libelle"),
                            statut = o.getString("statut"),
                        ),
                    )
                }
            }
        }

    suspend fun moyennes(eleveId: UUID, periodeId: UUID): List<MoyenneMatiere> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/moyennes?periode_id=$periodeId")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        MoyenneMatiere(
                            matiereId = UUID.fromString(o.getString("matiere_id")),
                            matiereLibelle = if (o.has("matiere_libelle") && !o.isNull("matiere_libelle")) {
                                o.getString("matiere_libelle")
                            } else {
                                null
                            },
                            periodeId = UUID.fromString(o.getString("periode_id")),
                            moyenneSur20 = o.getDouble("moyenne_sur_20"),
                        ),
                    )
                }
            }
        }

    suspend fun moyennesCourantes(eleveId: UUID): Pair<NotePeriode?, List<MoyenneMatiere>> =
        withContext(Dispatchers.IO) {
            val periodes = notePeriodes()
            val p = periodes.firstOrNull { it.statut == "ouverte" || it.statut == "ouvert" }
                ?: periodes.firstOrNull()
                ?: return@withContext null to emptyList()
            p to moyennes(eleveId, p.id)
        }

    data class Appreciation(
        val id: UUID,
        val matiereId: UUID?,
        val matiereLibelle: String?,
        val texte: String,
        val auteurLabel: String?,
    )

    data class TrombiEntree(
        val eleveId: UUID,
        val nom: String,
        val prenom: String,
        val classe: String?,
    )

    suspend fun appreciations(eleveId: UUID, periodeId: UUID): List<Appreciation> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/appreciations?periode_id=$periodeId")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Appreciation(
                            id = UUID.fromString(o.getString("id")),
                            matiereId = if (o.has("matiere_id") && !o.isNull("matiere_id")) {
                                UUID.fromString(o.getString("matiere_id"))
                            } else {
                                null
                            },
                            matiereLibelle = if (o.has("matiere_libelle") && !o.isNull("matiere_libelle")) {
                                o.getString("matiere_libelle")
                            } else {
                                null
                            },
                            texte = o.getString("texte"),
                            auteurLabel = if (o.has("auteur_label") && !o.isNull("auteur_label")) {
                                o.getString("auteur_label")
                            } else {
                                null
                            },
                        ),
                    )
                }
            }
        }

    suspend fun appreciationsCourantes(eleveId: UUID): Pair<NotePeriode?, List<Appreciation>> =
        withContext(Dispatchers.IO) {
            val periodes = notePeriodes()
            val p = periodes.firstOrNull { it.statut == "ouverte" || it.statut == "ouvert" }
                ?: periodes.firstOrNull()
                ?: return@withContext null to emptyList()
            p to appreciations(eleveId, p.id)
        }

    suspend fun photoEleve(eleveId: UUID): ByteArray = withContext(Dispatchers.IO) {
        getBytes("/eleves/$eleveId/photo")
    }

    suspend fun trombi(classe: String): List<TrombiEntree> =
        withContext(Dispatchers.IO) {
            val enc = java.net.URLEncoder.encode(classe, Charsets.UTF_8)
            val arr = getJsonArray("/classes/$enc/trombi")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        TrombiEntree(
                            eleveId = UUID.fromString(o.getString("eleve_id")),
                            nom = o.getString("nom"),
                            prenom = o.getString("prenom"),
                            classe = o.optString("classe", null),
                        ),
                    )
                }
            }
        }

    suspend fun edt(eleveId: UUID, jour: String): List<EdtCreneau> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/edt?jour=$jour")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        EdtCreneau(
                            id = UUID.fromString(o.getString("id")),
                            classe = o.getString("classe"),
                            matiereLibelle = o.getString("matiere_libelle"),
                            profLabel = if (o.has("prof_label") && !o.isNull("prof_label")) {
                                o.getString("prof_label")
                            } else {
                                null
                            },
                            heureDebut = o.getString("heure_debut"),
                            heureFin = o.getString("heure_fin"),
                            salle = o.optString("salle", null),
                        ),
                    )
                }
            }
        }

    suspend fun passagesRepas(eleveId: UUID, debut: String, fin: String): List<PassageRepas> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/repas/passages?debut=$debut&fin=$fin")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        PassageRepas(
                            id = UUID.fromString(o.getString("id")),
                            eleveId = UUID.fromString(o.getString("eleve_id")),
                            jour = o.getString("jour"),
                            badgeLu = o.getString("badge_lu"),
                            source = o.getString("source"),
                        ),
                    )
                }
            }
        }

    suspend fun facturesFamille(): List<Facture> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/facturation/famille")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Facture(
                            id = UUID.fromString(o.getString("id")),
                            numero = o.getString("numero"),
                            libelle = o.getString("libelle"),
                            statut = o.getString("statut"),
                            totalCentimes = o.getLong("total_centimes"),
                            nature = o.optString("nature", null),
                        ),
                    )
                }
            }
        }

    suspend fun quittancesFamille(): List<Quittance> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/facturation/famille/quittances")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Quittance(
                            id = UUID.fromString(o.getString("id")),
                            factureId = UUID.fromString(o.getString("facture_id")),
                            numero = o.getString("numero"),
                            montantCentimes = o.getLong("montant_centimes"),
                            moyen = o.getString("moyen"),
                            emiseAt = o.getString("emise_at"),
                        ),
                    )
                }
            }
        }

    suspend fun cahierTexte(eleveId: UUID): List<CahierTexte> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/cahier-texte?eleve_id=$eleveId")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        CahierTexte(
                            id = UUID.fromString(o.getString("id")),
                            classe = o.getString("classe"),
                            matiereLibelle = o.getString("matiere_libelle"),
                            titre = o.getString("titre"),
                            consigne = o.getString("consigne"),
                            pourLe = o.getString("pour_le"),
                        ),
                    )
                }
            }
        }

    suspend fun alertesCpe(eleveId: UUID): List<AlerteCpe> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/alertes-cpe")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        AlerteCpe(
                            id = UUID.fromString(o.getString("id")),
                            titre = o.getString("titre"),
                            detail = o.optString("detail", ""),
                            severite = o.getString("severite"),
                            statut = o.getString("statut"),
                            jour = o.getString("jour"),
                        ),
                    )
                }
            }
        }

    suspend fun internat(eleveId: UUID, jour: String): RecapInternat =
        withContext(Dispatchers.IO) {
            val o = getJson("/eleves/$eleveId/internat?jour=$jour")
            val chambre = o.optJSONObject("chambre")
            val presence = o.optJSONObject("presence_du_jour")
            val suivisArr = o.optJSONArray("suivis")
            val suivis = buildList {
                if (suivisArr != null) {
                    for (i in 0 until suivisArr.length()) {
                        val s = suivisArr.getJSONObject(i)
                        add(
                            SuiviInternat(
                                id = UUID.fromString(s.getString("id")),
                                jour = s.getString("jour"),
                                texte = s.getString("texte"),
                            ),
                        )
                    }
                }
            }
            RecapInternat(
                eleveId = eleveId,
                chambreCode = chambre?.optString("code"),
                chambreBatiment = chambre?.optString("batiment"),
                presenceStatut = presence?.optString("statut"),
                suivis = suivis,
            )
        }

    suspend fun conseils(eleveId: UUID): List<ConseilDossier> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/conseils")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        ConseilDossier(
                            id = UUID.fromString(o.getString("id")),
                            appreciation = o.getString("appreciation"),
                            mention = o.optString("mention", null),
                            decisionTexte = o.optString("decision_texte", ""),
                        ),
                    )
                }
            }
        }

    suspend fun ficheSante(eleveId: UUID): FicheSante? =
        withContext(Dispatchers.IO) {
            val raw = getRaw("/eleves/$eleveId/fiche-sante")
            if (raw == "null" || raw.isBlank()) return@withContext null
            val o = JSONObject(raw)
            FicheSante(
                allergies = o.optString("allergies", ""),
                traitements = o.optString("traitements", ""),
                contreIndications = o.optString("contre_indications", ""),
                paiActif = o.optBoolean("pai_actif", false),
                medecinNom = o.optString("medecin_nom", null),
                urgenceTel = o.optString("urgence_tel", null),
            )
        }

    suspend fun pai(eleveId: UUID): Pai? =
        withContext(Dispatchers.IO) {
            val raw = getRaw("/eleves/$eleveId/pai")
            if (raw == "null" || raw.isBlank()) return@withContext null
            val o = JSONObject(raw)
            Pai(
                id = UUID.fromString(o.getString("id")),
                motif = o.getString("motif"),
                mesuresAccueil = o.optString("mesures_accueil", ""),
                protocoleUrgence = o.optString("protocole_urgence", ""),
                dateDebut = o.getString("date_debut"),
                actif = o.optBoolean("actif", true),
                signeFamille = o.optBoolean("signe_famille", false),
            )
        }

    suspend fun signerPai(eleveId: UUID): Pai =
        withContext(Dispatchers.IO) {
            val o = postJson("/eleves/$eleveId/pai/signer", JSONObject(), authed = true)
            Pai(
                id = UUID.fromString(o.getString("id")),
                motif = o.getString("motif"),
                mesuresAccueil = o.optString("mesures_accueil", ""),
                protocoleUrgence = o.optString("protocole_urgence", ""),
                dateDebut = o.getString("date_debut"),
                actif = o.optBoolean("actif", true),
                signeFamille = o.optBoolean("signe_famille", false),
            )
        }

    suspend fun voeux(eleveId: UUID): List<Voeu> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/voeux")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Voeu(
                            id = UUID.fromString(o.getString("id")),
                            filiereId = UUID.fromString(o.getString("filiere_id")),
                            filiereLabel = if (o.has("filiere_label") && !o.isNull("filiere_label")) {
                                o.getString("filiere_label")
                            } else {
                                null
                            },
                            rang = o.getInt("rang"),
                            anneeScolaire = o.getString("annee_scolaire"),
                        ),
                    )
                }
            }
        }

    suspend fun infoComp(eleveId: UUID): List<InfoCompValeur> =
        withContext(Dispatchers.IO) {
            val root = getJson("/eleves/$eleveId/info-comp")
            val arr = when {
                root.has("valeurs") && !root.isNull("valeurs") -> root.getJSONArray("valeurs")
                else -> org.json.JSONArray()
            }
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        InfoCompValeur(
                            champId = UUID.fromString(o.getString("champ_id")),
                            champLabel = if (o.has("champ_label") && !o.isNull("champ_label")) {
                                o.getString("champ_label")
                            } else {
                                null
                            },
                            valeur = o.getString("valeur"),
                        ),
                    )
                }
            }
        }

    suspend fun dispensesRepas(eleveId: UUID): List<DispenseRepas> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/repas/dispenses")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        DispenseRepas(
                            id = UUID.fromString(o.getString("id")),
                            jour = o.getString("jour"),
                            motif = o.getString("motif"),
                            annulee = o.optBoolean("annulee", false),
                        ),
                    )
                }
            }
        }

    suspend fun activiteRepasMidi(eleveId: UUID): UUID? =
        withContext(Dispatchers.IO) {
            val o = getJson("/eleves/$eleveId/dossier")
            val grille = o.optJSONArray("grille_repas") ?: return@withContext null
            if (grille.length() == 0) return@withContext null
            UUID.fromString(grille.getJSONObject(0).getString("activite_id"))
        }

    suspend fun poserDispenseRepas(
        eleveId: UUID,
        activiteId: UUID,
        jour: String,
        motif: String,
    ): DispenseRepas =
        withContext(Dispatchers.IO) {
            val body = JSONObject()
                .put("activite_id", activiteId.toString())
                .put("jour", jour)
                .put("motif", motif)
            val o = postJson("/eleves/$eleveId/repas/dispenses", body, authed = true)
            DispenseRepas(
                id = UUID.fromString(o.getString("id")),
                jour = o.getString("jour"),
                motif = o.getString("motif"),
                annulee = o.optBoolean("annulee", false),
            )
        }

    suspend fun annulerDispenseRepas(eleveId: UUID, dispenseId: UUID): DispenseRepas =
        withContext(Dispatchers.IO) {
            val o = postJson(
                "/eleves/$eleveId/repas/dispenses/$dispenseId/annuler",
                JSONObject(),
                authed = true,
            )
            DispenseRepas(
                id = UUID.fromString(o.getString("id")),
                jour = o.getString("jour"),
                motif = o.getString("motif"),
                annulee = o.optBoolean("annulee", false),
            )
        }

    suspend fun pointsVs(eleveId: UUID): SoldePoints =
        withContext(Dispatchers.IO) {
            val o = getJson("/eleves/$eleveId/points-vs")
            SoldePoints(
                eleveId = UUID.fromString(o.getString("eleve_id")),
                solde = o.getInt("solde"),
                depart = o.getInt("depart"),
            )
        }

    suspend fun sanctions(eleveId: UUID): List<Sanction> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/sanctions")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Sanction(
                            id = UUID.fromString(o.getString("id")),
                            jour = o.getString("jour"),
                            dureeMinutes = if (o.has("duree_minutes") && !o.isNull("duree_minutes")) {
                                o.getInt("duree_minutes")
                            } else {
                                null
                            },
                            note = o.optString("note").takeIf { it.isNotBlank() },
                            statut = o.getString("statut"),
                        ),
                    )
                }
            }
        }

    suspend fun incidents(eleveId: UUID): List<Incident> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/incidents")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Incident(
                            id = UUID.fromString(o.getString("id")),
                            jour = o.getString("jour"),
                            description = o.optString("description").takeIf { it.isNotBlank() },
                        ),
                    )
                }
            }
        }

    suspend fun commissions(eleveId: UUID): List<CommissionDossier> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/commissions")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        CommissionDossier(
                            id = UUID.fromString(o.getString("id")),
                            motifs = o.getString("motifs"),
                            decisionLibelle = o.optString("decision_libelle").takeIf { it.isNotBlank() },
                            pointsRetires = o.optInt("points_retires", 0),
                            signaleFamille = o.optBoolean("signale_famille", false),
                        ),
                    )
                }
            }
        }

    suspend fun dossierPsy(eleveId: UUID): DossierPsy? =
        withContext(Dispatchers.IO) {
            val text = runCatching { getRaw("/eleves/$eleveId/dossier-psy") }.getOrNull()
                ?: return@withContext null
            if (text.isBlank() || text == "null") return@withContext null
            val o = JSONObject(text)
            DossierPsy(
                axesSuivi = o.optString("axes_suivi", ""),
                suiviActif = o.optBoolean("suivi_actif", false),
                signaleFamille = o.optBoolean("signale_famille", false),
            )
        }

    suspend fun rdvsPsy(eleveId: UUID): List<RdvPsy> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/psy/rdvs?eleve_id=$eleveId")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        RdvPsy(
                            id = UUID.fromString(o.getString("id")),
                            jour = o.getString("jour"),
                            heureDebut = o.getString("heure_debut"),
                            heureFin = o.getString("heure_fin"),
                            kindLibelle = o.optString("kind_libelle", o.getString("kind")),
                            statut = o.getString("statut"),
                            signaleFamille = o.optBoolean("signale_famille", false),
                        ),
                    )
                }
            }
        }

    suspend fun reinscriptionFamille(): ReinscriptionFamille =
        withContext(Dispatchers.IO) {
            val o = getJson("/reinscription/famille")
            val campagne = o.optJSONObject("campagne")
            val enfantsArr = o.getJSONArray("enfants")
            val enfants = buildList {
                for (i in 0 until enfantsArr.length()) {
                    val row = enfantsArr.getJSONObject(i)
                    val enfant = row.getJSONObject("enfant")
                    val proj = row.optJSONObject("projection_n1")
                    val optsArr = row.optJSONArray("options") ?: JSONArray()
                    val opts = buildList {
                        for (j in 0 until optsArr.length()) {
                            val opt = optsArr.getJSONObject(j)
                            add(
                                ReinscriptionOption(
                                    id = UUID.fromString(opt.getString("id")),
                                    code = opt.getString("code"),
                                    libelle = opt.getString("libelle"),
                                    statut = opt.getString("statut"),
                                ),
                            )
                        }
                    }
                    val manqArr = row.optJSONArray("documents_manquants") ?: JSONArray()
                    val manq = buildList {
                        for (j in 0 until manqArr.length()) add(manqArr.getString(j))
                    }
                    add(
                        ReinscriptionEnfant(
                            enfantId = UUID.fromString(enfant.getString("id")),
                            prenom = enfant.getString("prenom"),
                            nom = enfant.getString("nom"),
                            confirmee = row.optBoolean("confirmee", false),
                            documentsManquants = manq,
                            acompteOk = row.optBoolean("acompte_ok", true),
                            options = opts,
                            projectionNiveau = proj?.optString("niveau")?.takeIf { it.isNotBlank() },
                            projectionAnnee = proj?.optString("annee_scolaire")?.takeIf { it.isNotBlank() },
                        ),
                    )
                }
            }
            ReinscriptionFamille(
                anneeN1 = o.getString("annee_n1"),
                campagneOuverte = campagne?.optBoolean("ouverte", false) == true,
                enfants = enfants,
            )
        }

    suspend fun retenirOptionReinscription(eleveId: UUID, code: String, libelle: String) =
        withContext(Dispatchers.IO) {
            postJson(
                "/reinscription/famille/$eleveId/options",
                JSONObject()
                    .put("code", code)
                    .put("libelle", libelle)
                    .put("nature", "academique")
                    .put("retenir", true),
                authed = true,
            )
            Unit
        }

    suspend fun deposerDocReinscription(eleveId: UUID, code: String, titre: String) =
        withContext(Dispatchers.IO) {
            postJson(
                "/reinscription/famille/$eleveId/documents",
                JSONObject().put("code", code).put("titre", titre),
                authed = true,
            )
            Unit
        }

    suspend fun confirmerReinscription(eleveId: UUID) =
        withContext(Dispatchers.IO) {
            postJson("/reinscription/famille/$eleveId/confirmer", JSONObject(), authed = true)
            Unit
        }

    private fun JSONArray.toAbsences(): List<Absence> = buildList {
        for (i in 0 until length()) {
            val o = getJSONObject(i)
            add(
                Absence(
                    id = UUID.fromString(o.getString("id")),
                    debut = o.getString("debut"),
                    fin = o.optString("fin", o.getString("debut")),
                    statut = o.getString("statut"),
                    motif = o.optString("motif", "—"),
                ),
            )
        }
    }

    private fun JSONArray.toCarnet(): List<CarnetEntree> = buildList {
        for (i in 0 until length()) {
            val o = getJSONObject(i)
            add(
                CarnetEntree(
                    id = UUID.fromString(o.getString("id")),
                    titre = o.optString("titre", null),
                    corps = o.optString("corps", null),
                ),
            )
        }
    }

    private fun JSONArray.toNotes(): List<Note> = buildList {
        for (i in 0 until length()) {
            val o = getJSONObject(i)
            add(
                Note(
                    id = UUID.fromString(o.getString("id")),
                    valeur = if (o.has("valeur") && !o.isNull("valeur")) o.getDouble("valeur") else null,
                    libelle = o.optString("libelle", null),
                    matiereLibelle = if (o.has("matiere_libelle") && !o.isNull("matiere_libelle")) {
                        o.getString("matiere_libelle")
                    } else {
                        null
                    },
                ),
            )
        }
    }

    private fun postJson(path: String, body: JSONObject, authed: Boolean): JSONObject =
        sendJson("POST", path, body, authed)

    private fun putJson(path: String, body: JSONObject): JSONObject =
        sendJson("PUT", path, body, authed = true)

    private fun sendJson(method: String, path: String, body: JSONObject, authed: Boolean): JSONObject {
        val conn = (URL("$baseUrl$path").openConnection() as HttpURLConnection).apply {
            requestMethod = method
            setRequestProperty("Content-Type", "application/json")
            if (authed) token?.let { setRequestProperty("Authorization", "Bearer $it") }
            doOutput = true
            connectTimeout = 8_000
            readTimeout = 12_000
        }
        conn.outputStream.use { it.write(body.toString().toByteArray(Charsets.UTF_8)) }
        val code = conn.responseCode
        val stream = if (code in 200..299) conn.inputStream else conn.errorStream
        val text = stream?.bufferedReader()?.readText().orEmpty()
        if (code !in 200..299) error("HTTP $code: $text")
        return if (text.isBlank()) JSONObject() else JSONObject(text)
    }

    private fun getRaw(path: String): String {
        val conn = (URL("$baseUrl$path").openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            token?.let { setRequestProperty("Authorization", "Bearer $it") }
            connectTimeout = 8_000
            readTimeout = 12_000
        }
        val code = conn.responseCode
        val text = conn.inputStream.bufferedReader().readText()
        if (code !in 200..299) error("HTTP $code: $text")
        return text.trim()
    }

    private fun getJson(path: String): JSONObject {
        val text = getRaw(path)
        return JSONObject(text)
    }

    private fun getJsonArray(path: String): JSONArray {
        val conn = (URL("$baseUrl$path").openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            token?.let { setRequestProperty("Authorization", "Bearer $it") }
            connectTimeout = 8_000
            readTimeout = 12_000
        }
        val code = conn.responseCode
        val text = conn.inputStream.bufferedReader().readText()
        if (code !in 200..299) error("HTTP $code: $text")
        return JSONArray(text)
    }

    private fun getBytes(path: String): ByteArray {
        val conn = (URL("$baseUrl$path").openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            token?.let { setRequestProperty("Authorization", "Bearer $it") }
            connectTimeout = 8_000
            readTimeout = 20_000
        }
        val code = conn.responseCode
        val bytes = (if (code in 200..299) conn.inputStream else conn.errorStream)
            ?.readBytes()
            .orEmpty()
        if (code !in 200..299) error("HTTP $code: ${bytes.toString(Charsets.UTF_8)}")
        return bytes
    }
}
