//! Modification du projet d'une sortie : préremplissage depuis le détail, composition de
//! `SortieModifDto` et lecture du conflit de version. **Aucune règle métier** : le serveur reste
//! l'autorité (droits, états modifiables, champs figés après la présentation du dossier).

use ent_types::voyages::{SortieDetailDto, SortieModifDto};
use ent_web_api::ApiError;

use super::modele::{composer_public, Erreurs, SaisieVoyage};
use crate::noyau::temps::a_paris;

/// Valeurs actuelles de la sortie, telles qu'elles apparaissent dans le formulaire (heure de Paris).
pub fn saisie_depuis(d: &SortieDetailDto) -> SaisieVoyage {
    let s = &d.sortie;
    let (debut, fin) = (a_paris(s.debut), a_paris(s.fin));
    SaisieVoyage {
        titre: s.titre.clone(),
        nature: s.nature.clone(),
        forme: s.forme.clone(),
        destination: s.destination.clone(),
        pays: s.pays.clone().unwrap_or_default(),
        lieu: s.destination_libelle.clone().unwrap_or_default(),
        objectifs: s.objectifs.clone().unwrap_or_default(),
        date_debut: debut.format("%Y-%m-%d").to_string(),
        heure_debut: debut.format("%H:%M").to_string(),
        date_fin: fin.format("%Y-%m-%d").to_string(),
        heure_fin: fin.format("%H:%M").to_string(),
        effectif: s
            .effectif_eleves_declare
            .map(|n| n.to_string())
            .unwrap_or_default(),
        adultes: s
            .effectif_adultes_declare
            .map(|n| n.to_string())
            .unwrap_or_default(),
        point_rendez_vous: s.point_rendez_vous.clone().unwrap_or_default(),
        besoin_car: s.besoin_car,
        classes: d.publics.iter().filter_map(|p| p.classe_id).collect(),
    }
}

/// Corps de `PUT /voyages/{id}` : les champs du formulaire, plus ceux que le formulaire ne montre
/// pas (groupes, mode d'effectif, places, programme, consignes…) **repris tels quels** pour que la
/// modification ne les efface pas, et la `version` lue avec le détail (verrou optimiste).
pub fn composer_modification(
    saisie: &SaisieVoyage,
    d: &SortieDetailDto,
) -> Result<SortieModifDto, Erreurs> {
    let s = &d.sortie;
    let groupes: Vec<_> = d.publics.iter().filter_map(|p| p.groupe_id).collect();
    let mut saisie = saisie.clone();
    // Un champ masqué n'est pas envoyé : le pays n'a de sens qu'à l'étranger.
    if saisie.destination != "etranger" {
        saisie.pays = String::new();
    }
    let mut dto = composer_public(&saisie, !groupes.is_empty())?;
    dto.groupes = groupes;
    dto.effectif_mode = Some(s.effectif_mode.clone());
    dto.places = s.places;
    dto.programme_libelle = s.programme_libelle.clone();
    dto.car_sur_place = s.car_sur_place;
    dto.consignes_transport = s.consignes_transport.clone();
    dto.places_encadrant_a_pourvoir = s.places_encadrant_a_pourvoir;
    Ok(SortieModifDto {
        version: s.version,
        saisie: dto,
    })
}

/// Vrai pour le refus « modifié entre-temps » du verrou optimiste (409). Les autres refus
/// (dossier plus modifiable, champ figé…) ne se règlent pas en rechargeant.
pub fn est_conflit_de_version(e: &ApiError) -> bool {
    matches!(e, ApiError::Conflit(m) if m.contains("entre-temps"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use ent_types::voyages::{AccesDto, ProchaineEtapeDto, PublicDto, SortieDetailDto, SortieDto};
    use uuid::Uuid;

    pub(crate) fn detail() -> SortieDetailDto {
        let debut = Utc.with_ymd_and_hms(2027, 3, 10, 6, 0, 0).unwrap();
        SortieDetailDto {
            sortie: SortieDto {
                id: Uuid::nil(),
                annee_id: Uuid::nil(),
                serie_id: None,
                reference: "VOY-2027-AAAAAAAA".into(),
                titre: "Voyage à Rome".into(),
                objectifs: Some("Histoire".into()),
                nature: "facultative".into(),
                forme: "voyage".into(),
                destination: "etranger".into(),
                pays: Some("Italie".into()),
                destination_libelle: Some("Rome".into()),
                programme_libelle: Some("Forum".into()),
                debut,
                fin: debut + chrono::Duration::days(4),
                porteur_personne_id: Uuid::nil(),
                etat: "preparation".into(),
                etat_retour: None,
                motif_decision: None,
                effectif_mode: "liste".into(),
                places: Some(50),
                effectif_eleves_declare: Some(42),
                effectif_adultes_declare: None,
                besoin_car: true,
                car_sur_place: true,
                point_rendez_vous: Some("Parvis".into()),
                consignes_transport: Some("Valise".into()),
                places_encadrant_a_pourvoir: 2,
                contournement_devis_motif: None,
                liste_confirmee_le: None,
                version: 7,
                cree_le: debut,
                modifie_le: debut,
            },
            porteur_nom: "A B".into(),
            publics: vec![
                PublicDto {
                    id: Uuid::nil(),
                    classe_id: Some(Uuid::from_u128(1)),
                    groupe_id: None,
                    libelle: "3A".into(),
                },
                PublicDto {
                    id: Uuid::nil(),
                    classe_id: None,
                    groupe_id: Some(Uuid::from_u128(2)),
                    libelle: "Latin".into(),
                },
            ],
            etapes: Vec::new(),
            encadrants: Vec::new(),
            nb_inscrits: 0,
            nb_desistes: 0,
            prochaine_etape: ProchaineEtapeDto {
                qui: "porteur".into(),
                libelle: String::new(),
                commande: None,
            },
            audit: Vec::new(),
            acces: AccesDto::default(),
            commandes: Vec::new(),
        }
    }

    #[test]
    fn le_formulaire_est_prerempli_en_heure_de_paris() {
        let s = saisie_depuis(&detail());
        assert_eq!(s.titre, "Voyage à Rome");
        assert_eq!(s.pays, "Italie");
        assert_eq!(s.lieu, "Rome");
        assert_eq!(s.date_debut, "2027-03-10");
        assert_eq!(s.heure_debut, "07:00");
        assert_eq!(s.date_fin, "2027-03-14");
        assert_eq!(s.effectif, "42");
        assert_eq!(s.adultes, "");
        assert!(s.besoin_car);
        assert_eq!(s.classes, vec![Uuid::from_u128(1)]);
    }

    #[test]
    fn la_modification_garde_la_version_et_les_champs_non_montres() {
        let d = detail();
        let m = composer_modification(&saisie_depuis(&d), &d).expect("valide");
        assert_eq!(m.version, 7);
        assert_eq!(m.saisie.debut, d.sortie.debut);
        assert_eq!(m.saisie.groupes, vec![Uuid::from_u128(2)]);
        assert_eq!(m.saisie.effectif_mode.as_deref(), Some("liste"));
        assert_eq!(m.saisie.places, Some(50));
        assert_eq!(m.saisie.programme_libelle.as_deref(), Some("Forum"));
        assert!(m.saisie.car_sur_place);
        assert_eq!(m.saisie.consignes_transport.as_deref(), Some("Valise"));
        assert_eq!(m.saisie.places_encadrant_a_pourvoir, 2);
    }

    #[test]
    fn la_modification_dit_pourquoi_elle_refuse() {
        let d = detail();
        let mut s = saisie_depuis(&d);
        s.titre = " ".into();
        s.date_fin = "2027-03-01".into();
        let champs: Vec<String> = composer_modification(&s, &d)
            .unwrap_err()
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        assert!(champs.contains(&"voy-titre".to_string()));
        assert!(champs.contains(&"voy-date-fin".to_string()));
    }

    #[test]
    fn un_public_de_groupes_seuls_n_exige_pas_de_classe() {
        let mut d = detail();
        d.publics.remove(0);
        let s = saisie_depuis(&d);
        assert!(s.classes.is_empty());
        assert!(composer_modification(&s, &d).is_ok());
        // Sans aucun groupe, une classe reste demandée.
        d.publics.clear();
        assert!(composer_modification(&saisie_depuis(&d), &d).is_err());
    }

    #[test]
    fn le_pays_masque_n_est_pas_envoye() {
        let d = detail();
        let mut s = saisie_depuis(&d);
        s.destination = "france".into();
        let m = composer_modification(&s, &d).unwrap();
        assert_eq!(m.saisie.pays, None);
    }

    #[test]
    fn seul_le_conflit_de_version_propose_de_recharger() {
        let v = ApiError::Conflit(
            "Le dossier a été modifié entre-temps : rechargez-le avant de recommencer.".into(),
        );
        assert!(est_conflit_de_version(&v));
        assert!(!est_conflit_de_version(&ApiError::Conflit(
            "Ce dossier n'est plus modifiable.".into()
        )));
        assert!(!est_conflit_de_version(&ApiError::Interdit("x".into())));
    }
}
