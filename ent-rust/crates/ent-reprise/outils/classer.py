#!/usr/bin/env python3
"""Génère `donnees/schema_attendu.tsv` et `donnees/liste_blanche.tsv` à partir de `cols_brut.tsv`
(table, rang, colonne, type, nullable : extrait de `information_schema` après rejeu des migrations
`drizzle/` lues sur `origin/main`, sans aucune base réelle).

Règles de classement : une colonne par ligne, explicite. Toute colonne qui ne correspond à aucune
règle fait ÉCHOUER ce script (fermé par défaut) : il faut la classer ici, à la main.
Usage : python3 classer.py   (depuis le dossier outils/)
"""
import re, sys

# --- Tables entières traitées de la même façon ------------------------------------------
# (aucune : toutes les colonnes sont classées une à une, y compris pour les tables de santé,
#  que l'outil EXCLUT en plus du flux par défaut, voir `TABLES_SANTE` côté Rust.)

# --- Règles par nom de colonne (valables partout sauf surcharge par table) ------------------
GARDER_NOMS = set("""
id etablissement_id created_at updated_at at status statut state source kind type category categorie
action role roles scope level mode enabled active actif actif_ bookable has_pap confidence
count counter size width height seq sort_order ordre rang position retard_minutes
start_date end_date start_time end_time start_at end_at starts_at ends_at starts_on ends_on date_debut date_fin
date_seance date_devoir date_appel date_entree date_sanction date_accident date_depense date_mouvement
date_encaissement date_emission date_echeance a_rendre_le date_sortie date_ouverture date_fermeture
valid_from valid_to date_import horodatage arrivee sortie pris_at last_seen_at last_message_at last_read_at
last_delivered_at hidden_at pinned_at marked_unread_at deleted_at edited_at opened_at closed_at expires_at
used_at revoked_at signed_at consulted_at lu_at published_at activated_at purged_at migrated_at
valide_at decided_at traite_at admin_treated_at closed_at cancelled_at completed_at visited_at
accepted_at refused_at frozen_at conseil_date mandat_date genere_at envoye_at
last_sent_at last_reminder_at reminder_count follow_up_due_at follow_up_email_sent_at
engagement_accepted_at confirm_expires_at confirmed_at reconfirm_mail_sent_at reconfirmed_at
reschedule_token_expires_at release_min_hours horizon_days delai_famille_jours follow_up_delay_minutes
parents_notified_at justificatif_relance_at makeup_slots_relance_at privacy_documents_purged_at
justification_uploaded_at last_success_at failure_count locked_until failed_verification_count
inscription_opens_at inscription_closes_at rsvp_closes_at purge_at issued_at access_token_expires_at
refresh_token_expires_at manual_until quiet_start_minute quiet_end_minute invitation_sent_at
slug_ code libelle_ montant quantite prix_unitaire remise total_ht total_ttc tva_taux coef coef_defaut
coefficient moyenne moyenne_figee nb_notes valeur absent dispense niveau absent_ quotient_familial
repas_par_semaine demi_pension capacite capacity max_places max_total_persons max_persons_per_eleve
present_count nb_inserts nb_updates nb_deletes duration_days nb_eleves nb_accompagnateurs
sens jour_semaine day week_type semaine periode periodicite portee nature compte_produit gravite cycle
etage aile building_ is_current is_broadcast calendar_visible privacy_reason_redacted prof_own_classes_only
allow_broadcast allow_parent_attachments muted verified connections user_verified
email_verified platform_admin org_admin must_change_password two_factor_enabled two_factor_verified
droit_image_ok panier_repas peut_recuperer contact_urgence autorite_parentale payeur payeur_est_foyer
accepte_prelevement consent consent_ booked_for_other google_linked require_eligible require_signature
create_new pap_bring_to_rdv optionnelle gelee backed_up
site_id class_key matiere_id periode_id annee_scolaire_id annee_source_id annee_cible_id classe_ group_id
groupe_id creneau_id foyer_id eleve_id type_devoir_id tarif_id facture_id facture_origine_id encaissement_id
depense_id compte_id campagne_id etape_id etape_courante_id fiche_id seance_id page_id offre_id offer_id
thread_id message_id conversation_id batiment_id chambre_id scolarite_id appel_id absence_id comment_id
document_id extrait_id domain_id session_id slot_id room_id post_id travel_id request_id program_id
award_id convention_id application_id planning_id item_id devoir_id direction_id niveau_id bulletin_id
aggregate aggregate_id eleve_id_ resource_type resource_id user_id actor_user_id created_by_user_id
decided_by_user_id valide_par_user_id traite_par_user_id enseignant_user_id auteur_user_id
requester_user_id recorded_by_user_id last_modified_by_user_id booked_by_user_id author_user_id
declarant_user_id activated_by_user_id from_user_id to_user_id user_a_id user_b_id sender_id reply_to_id
forwarded_from_id pinned_by_id last_read_message_id last_delivered_message_id duplicate_group_id eligible_id
tarif_id route_id assigned_route_id image_config_id calendrier_mode starter_mode canal match_status
reconfirm_status workflow_status manager_decision hours_treatment staff_preferred_treatment
auteur_cote diploma_mode theme_ response niveau_modele constraint_kind phase suite_ permissions_
auteur_role marque situation_status liste_eleves_status confidentialite tiroir demarche_ purpose provider_id
mime mime_type content_type method device_type regime sexe mention decision pap_mime_type extension_
secteur_ code_nature code_type code_secteur sigle_ site_key template_key
""".split())

# Valeur "garder" pour des colonnes dont le nom est ambigu : listées par (table, colonne).
GARDER_TC = {
 ("etablissement","name"),("etablissement","slug"),("etablissement","data_bucket"),
 ("etablissement_site","label"),("etablissement_site","kind"),("etablissement_site","color_hex"),
 ("etablissement_site","grades"),("etablissement_site","site_id"),
 ("annee_scolaire","label"),("calendrier_scolaire","label"),
 ("classe_site_mapping","class_name"),("classe_site_mapping","siecle_code"),
 ("school_class_assignment","class_name"),("school_class_assignment","class_key"),
 ("ref_etablissement","code_rne"),("ref_etablissement","denom_princ"),("ref_etablissement","denom_compl"),
 ("ref_etablissement","adresse"),("ref_etablissement","sigle"),
 ("ref_nomenclature","code"),("ref_nomenclature","libelle_court"),("ref_nomenclature","libelle_long"),
 ("groupe_pedagogique","code"),("groupe_pedagogique","libelle"),
 ("note_matiere","libelle"),("note_matiere","groupe_matiere"),("note_competence_domaine","libelle"),
 ("note_competence_item","libelle"),("note_periode","libelle"),("note_type_devoir","libelle"),
 ("vs_sanction_type","libelle"),("tarif","libelle"),("tresorerie_compte","libelle"),
 ("reservation_room","name"),("reservation_room","building"),("internat_batiment","label"),
 ("internat_chambre","label"),("domain_planning_domain","name"),("domain_planning_domain","color"),
 ("domain_planning_domain","description"),("paie_periode","label"),("fd_etape","label"),
 ("fd_campagne","label"),("fd_campagne","annee_label"),("sst_campagne","title"),("sst_campagne","annee_label"),
 ("travel_image_catalog","label"),("travel_image_catalog","url"),("travel_image_catalog","keywords"),
 ("travel_image_catalog","normalize_key"),("travel_image_catalog","source"),("travel_image_catalog","author"),
 ("travel_image_catalog","license"),("travel_image_catalog","attribution_url"),("travel_image_catalog","source_page_url"),
 ("eleve_scolarite","classe"),("eleve","classe"),("eleve","mef"),("eleve","secteur"),("eleve","lv1"),("eleve","lv2"),
 ("enseignant","secteur"),("eleve_foyer_link","relation"),("eleve_document","annee_label"),
 ("edt_creneau","classe"),("edt_creneau","salle"),("cahier_texte","classe"),("vs_appel","classe"),
 ("conseil_seance","classe"),("note_devoir","classe"),("note_matiere_classe","classe"),
 ("vs_appel","matiere_libelle"),("cahier_texte","matiere_libelle"),("bulletin_ligne","libelle"),
 ("facture_ligne","libelle"),("facture_ligne","periode"),("tarif","portee_valeur"),
 ("fd_fiche","classe_actuelle"),("fd_fiche","eleve_mef"),("partenariat_inscription","eleve_classe"),
 ("partenariat_inscription","eleve_niveau"),("travel_participant","classe"),("travel","classes"),
 ("travel","site_label"),("absence","site_label"),("personnel","category"),
 ("personnel","job_title"),("personnel","establishment_label"),("portes_ouvertes_slot","label"),
 ("portes_ouvertes_registration","slot_label"),("portes_ouvertes_registration","classe_souhaitee"),
 ("portes_ouvertes_registration","cycle"),("portes_ouvertes_slot","cycle"),
 ("portes_ouvertes_slot_staff","role"),("portes_ouvertes_slot_staff","ref_id"),
 ("partenariat_offre","slug"),("partenariat_offre","kind"),("partenariat_offre","category_label"),
 ("invitation_page","slug"),("invitation_page","theme"),("invitation_page","diploma_mode"),
 ("invitation_eligible","diploma"),("invitation_rsvp","diploma"),("rdv_inscription_direction","slug"),
 ("rdv_inscription_direction","label"),("rdv_inscription_booking","direction_slug"),
 ("rdv_inscription_booking","niveau_label"),("rdv_inscription_email_gate","direction_slug"),
 ("preinscription","niveau_vise"),("preinscription","filiere_visee"),("teacher_planning_slot","subject"),
 ("teacher_planning_slot","room"),("teacher_planning_slot","group_label"),("teacher_planning_slot","group_id"),
 ("teacher_planning_replacement","subject"),("teacher_planning_replacement","room"),
 ("teacher_planning_replacement","date"),("teacher_planning_slot","classes"),("teacher_planning_replacement","classes"),
 ("teacher_planning","source"),("passkey","transports"),("passkey","aaguid"),
 ("auth_user_mapping","migrated_at"),("metier_event","type"),("certificate_program","title"),
 ("stage_offer","title"),("reservation_room_booking","class_name"),("reservation_room_booking","group_id"),
 ("reservation_room_booking","room_id"),("reservation_room_booking","starts_at"),("reservation_room_booking","ends_at"),
 ("reservation_room_booking","cancelled_at"),("reservation_room_booking","status"),
 ("travel","start_date"),("travel","end_date"),("travel","start_time"),("travel","end_time"),
 ("travel","nb_eleves"),("travel","nb_accompagnateurs"),("travel","image_url"),("travel","image_config_id"),
 ("travel","type"),("personnel","hire_date_"),("domain_planning_session","constraint_kind"),
 ("domain_planning_booking","starts_at"),("domain_planning_booking","ends_at"),
 ("domain_planning_session","starts_at"),("domain_planning_session","ends_at"),
 ("domain_planning_signup","created_at"),("personnel_document","uploaded_at"),("personnel_document","expires_at"),
 ("personnel_document","kind"),("personnel_entretien","kind"),("personnel_entretien","completed_at"),
 ("personnel_entretien","next_due_at"),("personnel_formation","completed_at"),("personnel_formation","expires_at"),
 ("personnel_habilitation","issued_at"),("personnel_habilitation","expires_at"),("personnel_shared_doc","uploaded_at"),
 ("personnel_leave","type"),("personnel_leave","start_date"),("personnel_leave","end_date"),
 ("personnel_leave","created_at"),("personnel_leave","decided_at"),("personnel_leave","status"),
 ("absence","display_name_"),("scolia_message","role"),("scolia_conversation","state_"),
 ("tenant_setting_section","section"),("platform_document","doc_key"),("ent_collection_record","collection"),
 ("ent_collection_record","status"),("ent_collection_attr","collection"),("ent_collection_attr","path"),
 ("tenant_setting_attr","section"),("tenant_setting_attr","path"),("personnel_attr","path"),
 ("travel_attr","path"),("request_attr","path"),("request_org_attr","path"),("request_routing_attr","path"),
 ("platform_document_attr","path"),("platform_document_attr","doc_key"),("stage_application_attr","path"),
 ("stage_convention_attr","path"),("stage_offer_attr","path"),("stage_token_attr","path"),
 ("certificate_award_attr","path"),("certificate_program_attr","path"),
 ("messaging_message","type"),("messaging_call_ring","kind"),("messaging_call_ring","status"),
 ("messaging_reaction","emoji"),("messaging_presence","status"),("messaging_presence","manual_status"),
 ("famille_notif","kind"),("user_membership","context"),("user_role","role"),
 ("rate_limit","key_"),("rate_limit","last_request"),("app_rate_limit","reset_at"),
 ("web_push_signal_state","signal_id"),("note_valeur","valeur"),
 ("nomenclature_import_log","source"),("nomenclature_import_log","statut"),("nomenclature_import_log","fichier_"),
 ("request","category"),("request","assigned_unit"),("request","assigned_role_label"),
 ("absence_ingest_job","phase"),("absence_ingest_job","code"),("absence_ingest_job","status"),
 ("absence_ingest_job","started_at"),("absence_ingest_job","updated_at"),("absence_ingest_job","processing_started_at"),
 ("absence_ingest_job","locked_at"),("absence_ingest_job","locked_by"),
}

# Règles de transformation par motif de nom de colonne (appliquées dans l'ordre).
# (motif regex sur le nom, règle)
REGLES = [
 # secrets et jetons : effacés
 (r"^(password|access_token|refresh_token|id_token|secret|backup_codes|google_refresh_token|p256dh|auth|public_key)$","effacer"),
 # jetons à usage unique : remplacés par un jeton aléatoire déterministe (unicité conservée)
 (r"^(token|secure_code|confirm_token|reconfirm_token|reschedule_token|value_token|credential_id|endpoint)$","pseudo:jeton"),
 # images et PDF en base64
 (r"^(signature_png_base64)$","pseudo:image"),
 (r"^(pdf_base64)$","pseudo:pdf"),
 (r"^(content_base64)$","pseudo:pdf"),
 # clés d'objets S3 et fichiers
 (r"^(s3_key|pdf_key|doc_key_|document_key|justificatif_key|photo_key|eleve_photo_key|pap_s3_key|signature_s3_key|logo_s3_key|file_url|justification_file_url|attribution_url_|source_document|document_keys)$","cle_s3"),
 (r"^(file_name|justification_file_name|pap_file_name|source_file_name|fichier|folder_name_)$","pseudo:nom_fichier"),
 # identité
 (r"^(nom|last_name|.*_last_name|eleve_nom)$","pseudo:nom"),
 (r"^(prenom|first_name|.*_first_name|eleve_prenom)$","pseudo:prenom"),
 (r"^(name|display_name|owner_name|personnel_name|director_name|contact_name|partner_contact_name|signer_name|invite_nom|requester_full_name|directrice_display_name|user_name|user_label|author_name|by_name|from_name|creator_name|auteur_label|eleve_name_norm|folder_name|signe_par_nom|valide_par_nom|label_famille|.*_nom|by|created_by_name|recorded_by_name|last_modified_by_name|booked_by_name)$","pseudo:nom_complet"),
 (r"^(email|.*_email|email_perso|email_pro|parent_emails_|assigned_email|notify_email|by_email|signer_email|google_linked_email)$","pseudo:email"),
 (r"^(phone|telephone|.*_phone|parent_phone|parent1_phone|parent2_phone)$","pseudo:telephone"),
 (r"^(ine|eleve_ine)$","pseudo:ine"),
 (r"^(iban)$","pseudo:iban"),(r"^(bic)$","pseudo:bic"),(r"^(rum)$","pseudo:rum"),
 (r"^(date_naissance|birth_date|eleve_birth_date|eleve_date_naissance|hire_date)$","pseudo:date_naissance"),
 (r"^(lieu_naissance|ville)$","pseudo:ville"),
 (r"^(adresse|address_famille)$","pseudo:adresse"),
 (r"^(code_postal)$","pseudo:code_postal"),
 (r"^(ip_address)$","pseudo:ip"),(r"^(user_agent)$","pseudo:agent"),
 (r"^(external_user_id|assignee_external_user_id|director_external_user_id|coordinator_external_user_ids)$","pseudo:identifiant"),
 (r"^(personnel_id|enseignant_id|manager_id|owner_id|record_id|eleve_key|source_key|pilotage_key|code_auxiliaire|enseignant_nom_key|assigned_to)$","pseudo:id_souple"),
 (r"^(teacher_catalog)$","pseudo:nom_complet"),
 # textes libres : remplacés par un texte fictif de même longueur
 (r"^(reason|details|admin_note|manager_note|note|notes|comment|comments|commentaire|body|corps|content|contenu|text|description|sujet|subject|intro|preview|motif|motif_court|observations|suggestions|remarques|circonstances|soins|soins_notes|suite|contexte|antecedents|personnes_a_prevenir|protocole|traitements_autorises|appreciation|appreciation_generale|note_cpe|decision_note|cancel_reason|admin_cancel_note|pv|children_info|engagement_text|situation_detail|situation_establishment|medicament|dose|error|title|titre|titre_|short_description|libelle_extrait|location|lieu|room|salle|destination|label|libelle|response|justification|extra)$","pseudo:texte"),
 (r"^(situation_|ask_situation|consent_label|event_title_pattern|diploma|diplome|demarche|travail|maps_url|preinscription_url)$","pseudo:texte"),
]

def classer(table, col, typ):
    if (table,col) in GARDER_TC: return "garder"
    # surcharges
    sur = SURCHARGES.get((table,col))
    if sur: return sur
    for pat, r in REGLES:
        if re.match(pat, col):
            if r=="pseudo:texte" and typ not in("text",): pass
            return r
    if col in GARDER_NOMS: return "garder"
    if typ=="jsonb": return "json"
    if typ in ("boolean","integer","numeric","double precision","bigint","timestamp with time zone","date"): return "garder"
    if col in ("created_by_roles","creator_roles","heure_debut","heure_fin"): return "garder"
    if col=="id" or col.endswith("_id") or col.endswith("_ids"): return "garder"
    return None

SURCHARGES = {
 # identités et textes dont le nom est ambigu
 ("user","name"):"pseudo:nom_complet",("user","image"):"cle_s3",("user","email_verified"):"garder",
 ("eleve","folder_name"):"pseudo:nom_complet",("enseignant","folder_name"):"pseudo:nom_complet",
 ("school_class_assignment","name"):"pseudo:nom_complet",("school_class_assignment","email"):"pseudo:email",
 ("passkey","name"):"pseudo:texte",("passkey","public_key"):"pseudo:jeton",("passkey","credential_id"):"pseudo:jeton",
 ("foyer","label"):"pseudo:nom_complet",
 ("personnel","hire_date"):"pseudo:date_naissance",
 ("etablissement_site","director_name"):"pseudo:nom_complet",("etablissement_site","director_email"):"pseudo:email",
 ("travel","owner_name"):"pseudo:nom_complet",("travel","owner_email"):"pseudo:email",("travel","owner_id"):"garder",
 ("travel","destination"):"garder",("travel","title"):"pseudo:texte",
 ("portes_ouvertes_config","address"):"garder",("portes_ouvertes_config","title"):"garder",("portes_ouvertes_config","intro"):"pseudo:texte",
 ("portes_ouvertes_config","maps_url"):"garder",("portes_ouvertes_config","preinscription_url"):"garder",
 ("rdv_inscription_booking","etablissement_origine_adresse"):"garder",("rdv_inscription_booking","etablissement_origine_label"):"garder",
 ("rdv_inscription_booking","etablissement_origine_rne"):"garder",
 ("rdv_inscription_booking","rdv_attendee"):"pseudo:email",
 ("rdv_inscription_booking","google_event_id"):"pseudo:jeton",("rdv_inscription_booking","google_calendar_id"):"pseudo:jeton",
 ("rdv_inscription_booking","google_html_link"):"pseudo:jeton",
 ("rdv_inscription_booking","has_pap"):"garder",
 ("rdv_inscription_direction","google_calendar_id"):"pseudo:jeton",("rdv_inscription_direction","directrice_display_name"):"pseudo:nom_complet",
 ("rdv_inscription_direction","title"):"garder",("rdv_inscription_direction","intro"):"pseudo:texte",
 ("rdv_inscription_direction","event_title_pattern"):"garder",("rdv_inscription_direction","location"):"garder",
 ("rdv_inscription_direction","consent_label"):"garder",
 ("rdv_inscription_config","title"):"garder",("rdv_inscription_config","intro"):"pseudo:texte",
 ("rdv_inscription_config","event_title_pattern"):"garder",("rdv_inscription_config","location"):"garder",
 ("rdv_inscription_config","consent_label"):"garder",("rdv_inscription_config","google_linked"):"garder",
 ("rdv_inscription_config","google_linked_at"):"garder",
 ("invitation_page","title"):"garder",("invitation_page","intro"):"pseudo:texte",("invitation_page","location"):"garder",
 ("invitation_page","ask_situation"):"garder",("invitation_page","notify_email"):"pseudo:email",("invitation_page","enabled"):"garder",
 ("invitation_page","require_eligible"):"garder",
 ("invitation_rsvp","response"):"garder",("invitation_rsvp","situation_status"):"garder",
 ("invitation_rsvp","situation_detail"):"pseudo:texte",("invitation_rsvp","situation_establishment"):"pseudo:texte",
 ("partenariat_offre","title"):"garder",("partenariat_offre","short_description"):"garder",("partenariat_offre","body"):"garder",
 ("partenariat_offre","demarche"):"garder",("partenariat_offre","engagement_text"):"garder",
 ("partenariat_offre","notify_email"):"pseudo:email",("partenariat_offre","contact_role"):"garder",
 ("partenariat_offre","partner_contact_role"):"garder",("partenariat_offre","enabled"):"garder",
 ("partenariat_evenement","title"):"garder",("partenariat_evenement","location"):"garder",("partenariat_evenement","notes"):"pseudo:texte",
 ("partenariat_inscription","status"):"garder",("partenariat_inscription","admin_note"):"pseudo:texte",
 ("fd_campagne","contact_pp_label"):"pseudo:nom_complet",("fd_campagne","niveau_actuel"):"garder",
 ("fd_campagne","site_key"):"garder",("fd_campagne","template_key"):"garder",
 ("fd_etape","description"):"garder",("fd_etape","kind"):"garder",
 ("fd_signature","role"):"garder",("fd_signature","method"):"garder",("fd_token","purpose"):"garder",
 ("fd_reponse","auteur_label"):"pseudo:nom_complet",("fd_pdf_archive","title"):"garder",("fd_pdf_archive","kind"):"garder",
 ("eleve_document","title"):"pseudo:texte",("eleve_document","source"):"garder",
 ("sst_fiche","lieu"):"pseudo:texte",("sst_fiche","observed_date"):"garder",("sst_fiche","observed_time"):"garder",
 ("sst_fiche","declarant_fonction"):"garder",("sst_consultation","fonction"):"garder",("sst_emargement","fonction"):"garder",
 ("sst_consultation","comments"):"pseudo:texte",("sst_emargement","remarques"):"pseudo:texte",
 ("sst_fiche","numero"):"garder",("sst_fiche","status"):"garder",
 ("request","subject"):"pseudo:texte",("request","description"):"pseudo:texte",
 ("request_comment","body"):"pseudo:texte",("request_comment","by_name"):"pseudo:nom_complet",
 ("request_history","note"):"pseudo:texte",("request_history","by"):"pseudo:nom_complet",
 ("absence_history","by"):"pseudo:nom_complet",("absence_history","note"):"pseudo:texte",
 ("travel_history","by"):"pseudo:nom_complet",("travel_history","note"):"pseudo:texte",
 ("travel_message","user_label"):"pseudo:nom_complet",("travel_message","body"):"pseudo:texte",
 ("travel_parent_blog_post","body"):"pseudo:texte",("travel_parent_blog_post","author_name"):"pseudo:nom_complet",
 ("travel_participant","nom"):"pseudo:nom",("travel_participant","prenom"):"pseudo:prenom",
 ("travel_participant","eleve_key"):"pseudo:id_souple",
 ("absence","display_name"):"pseudo:nom_complet",("absence","reason"):"pseudo:texte",("absence","details"):"pseudo:texte",
 ("absence","source_document"):"cle_s3",("absence","scope"):"garder",("absence","period_type"):"garder",
 ("absence","justification_uploaded_by"):"pseudo:nom_complet",("absence","admin_treated_by"):"pseudo:nom_complet",
 ("absence","created_by_name"):"pseudo:nom_complet",("absence","created_by_email"):"pseudo:email",
 ("absence","admin_note"):"pseudo:texte",("absence","manager_note"):"pseudo:texte",
 ("absence","staff_preferred_makeup_slots"):"pseudo:texte",("absence","direction_confirmed_makeup_slots"):"pseudo:texte",
 ("absence_ingest_job","creator_name"):"pseudo:nom_complet",("absence_ingest_job","creator_email"):"pseudo:email",
 ("absence_ingest_job","source_file_name"):"pseudo:nom_fichier",("absence_ingest_job","document_key"):"cle_s3",
 ("absence_ingest_job","error"):"pseudo:texte",("absence_ingest_job","created_payload"):"pseudo:texte",
 ("absence_ingest_job","parsed_payload"):"pseudo:texte",("absence_ingest_job","job_id"):"garder",
 ("absence_ingest_job","creator_roles"):"garder",
 ("absence_message","text"):"pseudo:texte",("absence_message","user_name"):"pseudo:nom_complet",("absence_message","role_label"):"garder",
 ("personnel","first_name"):"pseudo:prenom",("personnel","last_name"):"pseudo:nom",("personnel","display_name"):"pseudo:nom_complet",
 ("personnel","email"):"pseudo:email",("personnel","payload"):"json",
 ("personnel_document","name"):"pseudo:nom_fichier",("personnel_shared_doc","name"):"pseudo:nom_fichier",
 ("personnel_shared_doc","uploaded_by"):"pseudo:nom_complet",("personnel_document","file_url"):"cle_s3",
 ("personnel_shared_doc","file_url"):"cle_s3",("personnel_entretien","notes"):"pseudo:texte",
 ("personnel_entretien","status"):"garder",("personnel_formation","title"):"pseudo:texte",
 ("personnel_formation","organism"):"pseudo:texte",("personnel_habilitation","title"):"pseudo:texte",
 ("personnel_leave","personnel_name"):"pseudo:nom_complet",("personnel_leave","reason"):"pseudo:texte",
 ("personnel_leave","decided_by"):"pseudo:nom_complet",("personnel_leave","decision_note"):"pseudo:texte",
 ("personnel_attr","value"):"attr_valeur",("ent_collection_attr","value"):"attr_valeur",
 ("tenant_setting_attr","value"):"attr_valeur",("travel_attr","value"):"attr_valeur",("request_attr","value"):"attr_valeur",
 ("request_org_attr","value"):"attr_valeur",("request_routing_attr","value"):"attr_valeur",
 ("platform_document_attr","value"):"attr_valeur",("stage_application_attr","value"):"attr_valeur",
 ("stage_convention_attr","value"):"attr_valeur",("stage_offer_attr","value"):"attr_valeur",
 ("stage_token_attr","value"):"attr_valeur",("certificate_award_attr","value"):"attr_valeur",
 ("certificate_program_attr","value"):"attr_valeur",
 ("ent_collection_attr","record_id"):"pseudo:id_souple",("ent_collection_record","record_id"):"pseudo:id_souple",
 ("personnel_attr","personnel_id"):"pseudo:id_souple",
 ("stage_token","token"):"pseudo:jeton",("stage_token_attr","token"):"pseudo:jeton",("stage_token","kind"):"garder",
 ("stage_offer","status"):"garder",("travel_parent_blog","token"):"pseudo:jeton",
 ("session","token"):"pseudo:jeton",("session","id"):"pseudo:jeton",
 ("verification","value"):"effacer",("verification","identifier"):"pseudo:email",
 ("rate_limit","key"):"pseudo:jeton",("app_rate_limit","key"):"pseudo:jeton",("rate_limit","id"):"pseudo:jeton",
 ("account","account_id"):"pseudo:identifiant",("account","provider_id"):"garder",("account","issuer"):"garder",
 ("account","scope"):"garder",("account","id"):"garder",
 ("two_factor","id"):"garder",
 ("auth_user_mapping","external_user_id"):"pseudo:identifiant",
 ("data_access_audit","metadata"):"json",("eleve_access_audit","metadata"):"json",("security_audit_log","metadata"):"json",
 ("data_access_audit","action"):"garder",("security_audit_log","action"):"garder",
 ("metier_event","payload"):"json",("scolia_conversation","state"):"json",("scolia_conversation","title"):"pseudo:texte",
 ("scolia_message","content"):"pseudo:texte",
 ("messaging_message","body"):"pseudo:texte",("messaging_signal_relay","event"):"json",
 ("messaging_attachment","file_name"):"pseudo:nom_fichier",("messaging_call_ring","from_name"):"pseudo:nom_complet",
 ("messaging_call_ring","title"):"pseudo:texte",
 ("messaging_participant","muted"):"garder",
 ("famille_thread","sujet"):"pseudo:texte",("famille_thread","created_by_nom"):"pseudo:nom_complet",
 ("famille_thread_message","corps"):"pseudo:texte",("famille_thread_message","auteur_nom"):"pseudo:nom_complet",
 ("famille_thread_attachment","file_name"):"pseudo:nom_fichier",("famille_notif","titre"):"pseudo:texte",
 ("famille_notif","preview"):"pseudo:texte",
 ("famille_messaging_settings","roles_can_initiate"):"json",
 ("cahier_texte","contenu"):"pseudo:texte",("cahier_texte","travail"):"pseudo:texte",("cahier_texte","enseignant_nom"):"pseudo:nom_complet",
 ("edt_creneau","enseignant_nom"):"pseudo:nom_complet",("vs_appel","enseignant_nom"):"pseudo:nom_complet",
 ("note_matiere_classe","enseignant_nom"):"pseudo:nom_complet",
 ("vs_absence_eleve","motif"):"pseudo:texte",("vs_absence_eleve","note_cpe"):"pseudo:texte",
 ("vs_absence_eleve","created_by_nom"):"pseudo:nom_complet",("vs_absence_eleve","type"):"garder",("vs_absence_eleve","source"):"garder",
 ("vs_absence_eleve","heure_debut"):"garder",("vs_absence_eleve","heure_fin"):"garder",
 ("vs_appel","heure_debut"):"garder",("vs_appel","heure_fin"):"garder",("vs_appel_ligne","note"):"pseudo:texte",
 ("vs_carnet_entree","titre"):"pseudo:texte",("vs_carnet_entree","corps"):"pseudo:texte",
 ("vs_carnet_entree","created_by_nom"):"pseudo:nom_complet",("vs_carnet_entree","signe_par_nom"):"pseudo:nom_complet",
 ("vs_sanction","motif"):"pseudo:texte",("vs_sanction","created_by_nom"):"pseudo:nom_complet",
 ("vs_sanction_type","code"):"garder",
 ("internat_appel_ligne","note"):"pseudo:texte",("internat_appel_ligne","marque"):"garder",
 ("internat_batiment","notes"):"pseudo:texte",("internat_sortie","motif"):"pseudo:texte",
 ("infirmerie_fiche","antecedents"):"pseudo:texte",("infirmerie_fiche","personnes_a_prevenir"):"pseudo:texte",
 ("infirmerie_fiche","notes"):"pseudo:texte",("infirmerie_passage","motif_court"):"pseudo:texte",
 ("infirmerie_passage","soins_notes"):"pseudo:texte",("infirmerie_passage","suite"):"pseudo:texte",
 ("infirmerie_passage","contexte"):"pseudo:texte",("infirmerie_passage","auteur_nom"):"pseudo:nom_complet",
 ("sante_accident","circonstances"):"pseudo:texte",("sante_accident","soins"):"pseudo:texte",("sante_accident","suite"):"pseudo:texte",
 ("sante_accident","lieu"):"pseudo:texte",("sante_accident","auteur_nom"):"pseudo:nom_complet",
 ("sante_extrait","libelle"):"pseudo:texte",("sante_extrait","portee"):"garder",
 ("sante_inaptitude_eps","motif"):"pseudo:texte",("sante_inaptitude_eps","libelle_extrait"):"pseudo:texte",
 ("sante_inaptitude_eps","auteur_nom"):"pseudo:nom_complet",
 ("sante_medicament_prise","medicament"):"pseudo:texte",("sante_medicament_prise","dose"):"pseudo:texte",
 ("sante_medicament_prise","notes"):"pseudo:texte",("sante_medicament_prise","auteur_nom"):"pseudo:nom_complet",
 ("sante_pai","protocole"):"pseudo:texte",("sante_pai","traitements_autorises"):"pseudo:texte",
 ("sante_pai","notes"):"pseudo:texte",("sante_pai","valide_par_nom"):"pseudo:nom_complet",("sante_pai","statut"):"garder",
 ("bulletin","appreciation_generale"):"pseudo:texte",("bulletin_ligne","appreciation"):"pseudo:texte",
 ("conseil_avis","appreciation"):"pseudo:texte",("conseil_avis","decision"):"garder",("conseil_avis","mention"):"garder",
 ("conseil_seance","pv"):"pseudo:texte",
 ("note_valeur","appreciation"):"pseudo:texte",("note_competence_valeur","appreciation"):"pseudo:texte",
 ("note_competence_valeur","niveau"):"garder",
 ("depense","fournisseur"):"pseudo:nom_complet",("depense","libelle"):"pseudo:texte",("depense","portee"):"garder",
 ("encaissement","reference"):"pseudo:identifiant",("encaissement","mode"):"garder",
 ("mouvement_tresorerie","libelle"):"pseudo:texte",("facture","numero"):"garder",
 ("foyer_facturation","categorie_quotient"):"garder",
 ("passage","invite_nom"):"pseudo:nom_complet",("passage","lieu"):"pseudo:texte",
 ("teacher_planning_replacement","note"):"pseudo:texte",("teacher_planning_replacement","created_by"):"pseudo:nom_complet",
 ("teacher_planning","updated_by"):"pseudo:nom_complet",("teacher_planning","source_file_name"):"pseudo:nom_fichier",
 ("school_roster_meta","updated_by"):"pseudo:nom_complet",
 ("teacher_planning_slot","day"):"garder",("teacher_planning_slot","week_type"):"garder",
 ("reservation_room_booking","first_name"):"pseudo:prenom",("reservation_room_booking","last_name"):"pseudo:nom",
 ("reservation_room_booking","email"):"pseudo:email",("reservation_room_booking","subject"):"pseudo:texte",
 ("reservation_room_booking","comment"):"pseudo:texte",("reservation_room_booking","cancelled_by"):"pseudo:nom_complet",
 ("reservation_room_booking","cancel_reason"):"pseudo:texte",("reservation_room_booking","user_id"):"garder",
 ("reservation_room","kind"):"garder",("reservation_room","id"):"garder",
 ("domain_planning_booking","title"):"pseudo:texte",("domain_planning_booking","notes"):"pseudo:texte",
 ("domain_planning_session","title"):"pseudo:texte",("domain_planning_signup","display_name"):"pseudo:nom_complet",
 ("domain_planning_signup","email"):"pseudo:email",
 ("domain_planning_domain","coordinator_external_user_ids"):"pseudo:identifiant",
 ("web_push_subscription","endpoint"):"pseudo:jeton",("web_push_subscription","auth"):"effacer",
 ("web_push_subscription","p256dh"):"effacer",
 ("web_push_preference","muted_kinds"):"garder",
 ("ref_nomenclature","type"):"garder",("ref_nomenclature","source"):"garder",("ref_nomenclature","metadata_json"):"json",
 ("ref_etablissement","source"):"garder",("ref_etablissement","code_nature"):"garder",("ref_etablissement","code_type"):"garder",
 ("ref_etablissement","code_secteur"):"garder",
 ("nomenclature_import_log","fichier"):"pseudo:nom_fichier",("nomenclature_import_log","rapport_json"):"json",
 ("eleve","options"):"json",("eleve_scolarite","grille_repas"):"json",("eleve_scolarite","etablissement_precedent"):"pseudo:texte",
 ("eleve_scolarite","statut"):"garder",("preinscription","etablissement_precedent"):"pseudo:texte",
 ("preinscription","payload"):"json",("preinscription","nom"):"pseudo:nom",("preinscription","prenom"):"pseudo:prenom",
 ("preinscription","lieu_naissance"):"pseudo:ville",
 ("fd_campagne","catalogue"):"json",("fd_campagne","appel_config"):"json",("fd_campagne","classes_cibles"):"json",
 ("fd_campagne","eleve_ids_cibles"):"json",("fd_fiche","options_actuelles"):"json",("fd_fiche","parent_emails"):"json",
 ("fd_fiche","acceptation"):"json",("fd_fiche","appel"):"json",("fd_fiche","parent_accord"):"json",
 ("fd_fiche","conflict_payload"):"json",("fd_reponse","payload"):"json",
 ("partenariat_offre","cycles"):"json",("partenariat_offre","niveaux"):"json",("partenariat_offre","tarifs"):"json",
 ("partenariat_offre","cta_links"):"json",("portes_ouvertes_slot_staff","meta"):"json",
 ("portes_ouvertes_slot_staff","display_name"):"pseudo:nom_complet",
 ("sst_fiche","workflow"):"json",
 ("etablissement_site","signature_s3_key"):"cle_s3",("etablissement_site","role_slugs"):"garder",
 ("etablissement_site","director_external_user_id"):"pseudo:identifiant",
 ("request","requester_full_name"):"pseudo:nom_complet",("request","assigned_email"):"pseudo:email",
 ("request","assigned_route_id"):"garder",("request","id"):"garder",
 ("request_attachment","file_name"):"pseudo:nom_fichier",("request_attachment","content_type"):"garder",
 ("travel","id"):"garder",("travel","status"):"garder",("travel","liste_eleves_status"):"garder",
 ("travel","image_config_id"):"garder",("personnel","id"):"pseudo:id_souple",("enseignant","id"):"pseudo:id_souple",
 ("enseignant","nom"):"pseudo:nom",("enseignant","prenom"):"pseudo:prenom",
 ("eleve","pilotage_key"):"pseudo:id_souple",("eleve","source_key"):"pseudo:id_souple",
 ("eleve","regime"):"garder",("eleve","sexe"):"garder",("eleve","photo_key"):"cle_s3",("eleve","status"):"garder",
 ("foyer","adresse"):"pseudo:adresse",("foyer","code_postal"):"pseudo:code_postal",("foyer","ville"):"pseudo:ville",
 ("foyer_responsable","user_id"):"garder",
 ("invitation_eligible","eleve_name_norm"):"pseudo:nom_complet",("invitation_rsvp","eleve_name_norm"):"pseudo:nom_complet",
 ("partenariat_inscription","eleve_birth_date"):"pseudo:date_naissance",
 ("partenariat_inscription","signature_s3_key"):"cle_s3",
 ("rdv_inscription_booking","student_first_name"):"pseudo:prenom",("rdv_inscription_booking","student_last_name"):"pseudo:nom",
 ("rdv_inscription_booking","pap_file_name"):"pseudo:nom_fichier",("rdv_inscription_booking","pap_mime_type"):"garder",
 ("rdv_inscription_booking","match_status"):"garder",("rdv_inscription_booking","reconfirm_status"):"garder",
 ("rdv_inscription_booking","regime"):"garder",("rdv_inscription_booking","admin_cancel_note"):"pseudo:texte",
 ("rdv_inscription_booking","status"):"garder",
 ("portes_ouvertes_registration","children_info"):"pseudo:texte",("portes_ouvertes_registration","source"):"garder",
 ("portes_ouvertes_registration","recorded_by_name"):"pseudo:nom_complet",("portes_ouvertes_registration","last_modified_by_name"):"pseudo:nom_complet",
 ("portes_ouvertes_registration","slot_label"):"garder",("portes_ouvertes_registration","id"):"garder",
 ("portes_ouvertes_registration","first_name"):"pseudo:prenom",("portes_ouvertes_registration","last_name"):"pseudo:nom",
 ("portes_ouvertes_registration","child_first_name"):"pseudo:prenom",("portes_ouvertes_registration","child_last_name"):"pseudo:nom",
 ("portes_ouvertes_registration","email"):"pseudo:email",("portes_ouvertes_registration","phone"):"pseudo:telephone",
 ("portes_ouvertes_config","consent_label"):"garder",("portes_ouvertes_config","notify_email"):"pseudo:email",
 ("sst_consultation","signature_png_base64"):"pseudo:image",("sst_emargement","signature_png_base64"):"pseudo:image",
 ("sst_fiche","signature_png_base64"):"pseudo:image",("fd_signature","signature_png_base64"):"pseudo:image",
 ("famille_thread_attachment","content_base64"):"pseudo:pdf",("fd_pdf_archive","pdf_base64"):"pseudo:pdf",
 ("fd_signature","signer_name"):"pseudo:nom_complet",("fd_signature","signer_email"):"pseudo:email",
 ("fd_token","email"):"pseudo:email",("fd_token","token"):"pseudo:jeton",("fd_token","secure_code"):"pseudo:jeton",
 ("rdv_inscription_email_gate","parent_email"):"pseudo:email",("rdv_inscription_email_gate","token"):"pseudo:jeton",
 ("rdv_inscription_booking","parent_email"):"pseudo:email",
 ("export_academique","canal"):"garder",("document_access_request","note"):"pseudo:texte",
 ("tarif","code"):"garder",("tarif","compte_produit"):"garder",
 ("web_push_signal_state","user_id"):"garder",("user","id"):"garder",
}

def main():
    cols=[l.rstrip("\n").split("\t") for l in open("cols_brut.tsv")]
    out_s=[]; out_w=[]; bad=[]
    for table,rang,col,typ,nul in cols:
        r=classer(table,col,typ)
        if r is None:
            bad.append((table,col,typ)); continue
        # les tableaux (_text) : la règle s'applique élément par élément ; json : colonne jsonb
        if typ=="jsonb" and r not in("json","garder","effacer"):
            r="json"
        if col=="path" and table.endswith("_attr"):
            r="chemin"
        out_s.append("\t".join([table,col,typ,"1" if nul=="YES" else "0"]))
        out_w.append("\t".join([table,col,r]))
    if bad:
        print("COLONNES NON CLASSÉES :",file=sys.stderr)
        for b in bad: print("  ",*b,file=sys.stderr)
        sys.exit(1)
    # Journal des migrations de Drizzle : schéma `drizzle`, technique, rien de personnel.
    for col,typ,nul in (("id","integer",False),("hash","text",False),("created_at","bigint",True)):
        out_s.append("\t".join(["drizzle.__drizzle_migrations",col,typ,"1" if nul else "0"]))
        out_w.append("\t".join(["drizzle.__drizzle_migrations",col,"garder"]))
    open("../donnees/schema_attendu.tsv","w").write("# table\tcolonne\ttype\tnullable\n"+"\n".join(out_s)+"\n")
    open("../donnees/liste_blanche.tsv","w").write("# table\tcolonne\tregle\n"+"\n".join(out_w)+"\n")
    print(len(out_s),"colonnes classées")
main()
