//! Logique d'écran de la santé côté famille. Affichage seulement : le serveur contrôle le lien
//! et refuse toute signature d'un autre responsable que celui du compte.

use ent_types::sante::PaiCompletDto;
use ent_web_api::Enfant;
use uuid::Uuid;

/// Le bouton « Signer » n'est proposé que pour un PAI actif que la famille n'a pas encore signé
/// (dans sa version courante : toute modification du PAI invalide les signatures).
pub fn peut_signer(p: &PaiCompletDto) -> bool {
    p.pai.actif && !p.signatures.iter().any(|s| s.partie == "famille")
}

/// Enfant sélectionné ; le premier si le choix est vide ou inconnu.
pub fn enfant_choisi(enfants: &[Enfant], choix: &str) -> Option<Uuid> {
    enfants
        .iter()
        .find(|e| e.id.to_string() == choix)
        .or_else(|| enfants.first())
        .map(|e| e.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use ent_types::sante::{PaiBaseDto, SignaturePaiDto};

    fn pai(actif: bool, signe_famille: bool) -> PaiCompletDto {
        PaiCompletDto {
            pai: PaiBaseDto {
                id: Uuid::nil(),
                etablissement_id: Uuid::nil(),
                eleve_id: Uuid::nil(),
                motif: "m".into(),
                mesures_accueil: "a".into(),
                protocole_urgence: String::new(),
                note_interne: String::new(),
                date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                date_fin: None,
                actif,
                signe_famille_at: None,
                signe_famille_user_id: None,
                updated_by_user_id: Uuid::nil(),
                updated_at: Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap(),
            },
            version: 1,
            date_revision: None,
            revision_due: false,
            trousse_urgence: false,
            trousse_lieu: String::new(),
            trousse_peremption: None,
            trousse_perimee: false,
            signatures: if signe_famille {
                vec![SignaturePaiDto {
                    partie: "famille".into(),
                    signataire_nom: "Mme Potter".into(),
                    signe_at: Utc.with_ymd_and_hms(2026, 9, 2, 0, 0, 0).unwrap(),
                }]
            } else {
                Vec::new()
            },
            parties_manquantes: Vec::new(),
            complet: false,
        }
    }

    #[test]
    fn signature_proposee_une_seule_fois() {
        assert!(peut_signer(&pai(true, false)));
        assert!(!peut_signer(&pai(true, true)));
        assert!(!peut_signer(&pai(false, false)));
    }

    #[test]
    fn enfant_par_defaut() {
        let e = |n: u128| Enfant {
            id: Uuid::from_u128(n),
            nom: "N".into(),
            prenom: "P".into(),
            classe: None,
            status: "actif".into(),
            lien: "mere".into(),
            peut_voir_notes: false,
            peut_voir_vs: false,
            peut_justifier_absence: false,
            peut_autoriser_sorties: false,
        };
        let l = [e(1), e(2)];
        assert_eq!(
            enfant_choisi(&l, &Uuid::from_u128(2).to_string()),
            Some(Uuid::from_u128(2))
        );
        assert_eq!(enfant_choisi(&l, ""), Some(Uuid::from_u128(1)));
        assert_eq!(enfant_choisi(&l, "inconnu"), Some(Uuid::from_u128(1)));
        assert_eq!(enfant_choisi(&[], ""), None);
    }
}
