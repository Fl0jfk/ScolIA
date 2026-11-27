package fr.scola.eleve

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.util.UUID

/** Client HTTP ENT — face élève (notes, cahier, messagerie, EDT, absences). */
class EntApiClient(
    private val baseUrl: String = "http://10.0.2.2:3001",
) {
    var token: String? = null
        private set
    var etablissementId: UUID? = null
        private set
    var eleveId: UUID? = null
        private set

    data class Session(
        val token: String,
        val userId: UUID,
        val email: String,
        val etablissementId: UUID,
        val role: String,
    )

    data class Note(
        val id: UUID,
        val valeur: Double?,
        val libelle: String?,
        val matiereLibelle: String?,
        val sur: Double?,
    )

    data class Devoir(
        val id: UUID,
        val titre: String,
        val matiereLibelle: String?,
        val classe: String,
        val dateDevoir: String,
        val coef: Double,
        val sur: Double,
    )

    data class CahierTexte(
        val id: UUID,
        val matiereLibelle: String,
        val titre: String,
        val consigne: String,
        val pourLe: String,
    )

    data class MsgFil(
        val id: UUID,
        val sujet: String,
        val lastMessageAt: String?,
    )

    data class MsgPiece(
        val id: UUID,
        val nom: String,
    )

    data class Message(
        val id: UUID,
        val auteurLabel: String?,
        val corps: String,
        val createdAt: String,
        val pieces: List<MsgPiece> = emptyList(),
    )

    data class FilDetail(
        val fil: MsgFil,
        val messages: List<Message>,
    )

    data class EdtCreneau(
        val id: UUID,
        val matiereLibelle: String,
        val profLabel: String?,
        val heureDebut: String,
        val heureFin: String,
    )

    data class MoyenneMatiere(
        val matiereLibelle: String?,
        val moyenneSur20: Double,
    )

    data class NotePeriode(
        val id: UUID,
        val libelle: String,
        val statut: String,
    )

    data class Absence(
        val id: UUID,
        val debut: String,
        val fin: String,
        val motif: String,
        val statut: String,
        val motifJustif: String?,
    )

    data class JournalEntree(
        val id: UUID,
        val domaine: String,
        val action: String,
        val libelle: String,
        val createdAt: String?,
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
            val me = getJson("/auth/me")
            eleveId = me.optString("eleve_id").takeIf { it.isNotBlank() }?.let(UUID::fromString)
            session
        }

    suspend fun registerPush(platform: String, pushToken: String, appId: String?) =
        withContext(Dispatchers.IO) {
            val body = JSONObject().put("platform", platform).put("token", pushToken)
            if (appId != null) body.put("app_id", appId)
            postJson("/devices/push", body, authed = true)
            Unit
        }

    suspend fun notes(eleveId: UUID): List<Note> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/notes")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
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
                            sur = if (o.has("sur") && !o.isNull("sur")) o.getDouble("sur") else null,
                        ),
                    )
                }
            }
        }

    suspend fun devoirsCourants(eleveId: UUID): Pair<NotePeriode?, List<Devoir>> =
        withContext(Dispatchers.IO) {
            val periodesArr = getJsonArray("/note-periodes")
            var periode: NotePeriode? = null
            for (i in 0 until periodesArr.length()) {
                val o = periodesArr.getJSONObject(i)
                val p = NotePeriode(
                    id = UUID.fromString(o.getString("id")),
                    libelle = o.getString("libelle"),
                    statut = o.getString("statut"),
                )
                if (periode == null || p.statut == "ouverte") periode = p
                if (p.statut == "ouverte") break
            }
            if (periode == null) return@withContext null to emptyList()
            val arr = getJsonArray("/eleves/$eleveId/devoirs?periode_id=${periode.id}")
            val list = buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Devoir(
                            id = UUID.fromString(o.getString("id")),
                            titre = o.getString("titre"),
                            matiereLibelle = if (o.has("matiere_libelle") && !o.isNull("matiere_libelle")) {
                                o.getString("matiere_libelle")
                            } else {
                                null
                            },
                            classe = o.getString("classe"),
                            dateDevoir = o.getString("date_devoir"),
                            coef = o.getDouble("coef"),
                            sur = o.getDouble("sur"),
                        ),
                    )
                }
            }
            periode to list
        }

    suspend fun cahierTexte(): List<CahierTexte> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/cahier-texte")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        CahierTexte(
                            id = UUID.fromString(o.getString("id")),
                            matiereLibelle = o.getString("matiere_libelle"),
                            titre = o.getString("titre"),
                            consigne = o.optString("consigne", ""),
                            pourLe = o.getString("pour_le"),
                        ),
                    )
                }
            }
        }

    suspend fun messages(): List<MsgFil> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/messages/fils")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        MsgFil(
                            id = UUID.fromString(o.getString("id")),
                            sujet = o.getString("sujet"),
                            lastMessageAt = o.optString("last_message_at").takeIf { it.isNotBlank() },
                        ),
                    )
                }
            }
        }

    suspend fun filDetail(filId: UUID): FilDetail =
        withContext(Dispatchers.IO) {
            val o = getJson("/messages/fils/$filId")
            val f = o.getJSONObject("fil")
            val fil = MsgFil(
                id = UUID.fromString(f.getString("id")),
                sujet = f.getString("sujet"),
                lastMessageAt = f.optString("last_message_at").takeIf { it.isNotBlank() },
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

    suspend fun repondre(filId: UUID, corps: String) =
        withContext(Dispatchers.IO) {
            postJson(
                "/messages/fils/$filId/repondre",
                JSONObject().put("corps", corps),
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
            val conn = open("/messages/pieces/$pieceId", "GET", authed = true)
            val code = conn.responseCode
            val bytes = (if (code in 200..299) conn.inputStream else conn.errorStream)
                ?.readBytes()
                .orEmpty()
            if (code !in 200..299) error("HTTP $code")
            bytes
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
                            matiereLibelle = o.getString("matiere_libelle"),
                            profLabel = if (o.has("prof_label") && !o.isNull("prof_label")) {
                                o.getString("prof_label")
                            } else {
                                null
                            },
                            heureDebut = o.getString("heure_debut"),
                            heureFin = o.getString("heure_fin"),
                        ),
                    )
                }
            }
        }

    suspend fun absences(eleveId: UUID): List<Absence> =
        withContext(Dispatchers.IO) {
            val arr = getJsonArray("/eleves/$eleveId/absences")
            buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Absence(
                            id = UUID.fromString(o.getString("id")),
                            debut = o.getString("debut"),
                            fin = o.getString("fin"),
                            motif = o.getString("motif"),
                            statut = o.getString("statut"),
                            motifJustif = o.optString("motif_justif").takeIf { it.isNotBlank() },
                        ),
                    )
                }
            }
        }

    suspend fun journal(eleveId: UUID, limit: Int = 50): List<JournalEntree> =
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

    suspend fun moyennesCourantes(eleveId: UUID): Pair<NotePeriode?, List<MoyenneMatiere>> =
        withContext(Dispatchers.IO) {
            val periodes = getJsonArray("/note-periodes")
            val list = buildList {
                for (i in 0 until periodes.length()) {
                    val o = periodes.getJSONObject(i)
                    add(
                        NotePeriode(
                            id = UUID.fromString(o.getString("id")),
                            libelle = o.getString("libelle"),
                            statut = o.getString("statut"),
                        ),
                    )
                }
            }
            val p = list.firstOrNull { it.statut == "ouverte" } ?: list.firstOrNull()
                ?: return@withContext null to emptyList()
            val arr = getJsonArray("/eleves/$eleveId/moyennes?periode_id=${p.id}")
            val moy = buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        MoyenneMatiere(
                            matiereLibelle = if (o.has("matiere_libelle") && !o.isNull("matiere_libelle")) {
                                o.getString("matiere_libelle")
                            } else {
                                null
                            },
                            moyenneSur20 = o.getDouble("moyenne_sur_20"),
                        ),
                    )
                }
            }
            p to moy
        }

    data class Appreciation(
        val id: UUID,
        val matiereLibelle: String?,
        val texte: String,
    )

    suspend fun appreciationsCourantes(eleveId: UUID): Pair<NotePeriode?, List<Appreciation>> =
        withContext(Dispatchers.IO) {
            val periodes = getJsonArray("/note-periodes")
            val list = buildList {
                for (i in 0 until periodes.length()) {
                    val o = periodes.getJSONObject(i)
                    add(
                        NotePeriode(
                            id = UUID.fromString(o.getString("id")),
                            libelle = o.getString("libelle"),
                            statut = o.getString("statut"),
                        ),
                    )
                }
            }
            val p = list.firstOrNull { it.statut == "ouverte" } ?: list.firstOrNull()
                ?: return@withContext null to emptyList()
            val arr = getJsonArray("/eleves/$eleveId/appreciations?periode_id=${p.id}")
            val apps = buildList {
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    add(
                        Appreciation(
                            id = UUID.fromString(o.getString("id")),
                            matiereLibelle = if (o.has("matiere_libelle") && !o.isNull("matiere_libelle")) {
                                o.getString("matiere_libelle")
                            } else {
                                null
                            },
                            texte = o.getString("texte"),
                        ),
                    )
                }
            }
            p to apps
        }

    suspend fun photoEleve(eleveId: UUID): ByteArray =
        withContext(Dispatchers.IO) {
            val conn = open("/eleves/$eleveId/photo", "GET", authed = true)
            val code = conn.responseCode
            val bytes = (if (code in 200..299) conn.inputStream else conn.errorStream)
                ?.readBytes()
                .orEmpty()
            if (code !in 200..299) error("HTTP $code")
            bytes
        }

    private fun getJson(path: String): JSONObject {
        val conn = open(path, "GET", authed = true)
        return JSONObject(conn.inputStream.bufferedReader().readText())
    }

    private fun getJsonArray(path: String): JSONArray {
        val conn = open(path, "GET", authed = true)
        return JSONArray(conn.inputStream.bufferedReader().readText())
    }

    private fun postJson(path: String, body: JSONObject, authed: Boolean): JSONObject {
        val conn = open(path, "POST", authed)
        conn.doOutput = true
        conn.setRequestProperty("Content-Type", "application/json")
        conn.outputStream.use { it.write(body.toString().toByteArray()) }
        val code = conn.responseCode
        val stream = if (code in 200..299) conn.inputStream else conn.errorStream
        val text = stream?.bufferedReader()?.readText() ?: ""
        if (code !in 200..299) error(text.ifBlank { "HTTP $code" })
        return if (text.isBlank()) JSONObject() else JSONObject(text)
    }

    private fun open(path: String, method: String, authed: Boolean): HttpURLConnection {
        val conn = (URL("$baseUrl$path").openConnection() as HttpURLConnection)
        conn.requestMethod = method
        conn.connectTimeout = 15_000
        conn.readTimeout = 30_000
        if (authed) {
            val t = token ?: error("non authentifié")
            conn.setRequestProperty("Authorization", "Bearer $t")
        }
        return conn
    }
}
