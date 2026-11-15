//! Rendu PDF (A4) — facture & quittance, via la brique `ent-pdf` (D-062).

use crate::facturation::{reste_a_payer, Facture, FactureLigne, Quittance};
use ent_pdf::{Align, Colonne, Document, Ligne, Tableau};

/// Formate des centimes en « 70,00 EUR ».
pub fn euros_centimes(centimes: i64) -> String {
    let neg = centimes < 0;
    let abs = centimes.unsigned_abs();
    let euros = abs / 100;
    let cts = abs % 100;
    let s = format!("{euros},{cts:02} EUR");
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

/// PDF facture (émise ou soldée) avec lignes + soldes.
pub fn render_facture_pdf(
    facture: &Facture,
    lignes: &[FactureLigne],
    encaisse_centimes: i64,
) -> Vec<u8> {
    let mut d = Document::new(crate::pdf_commun::meta(
        &format!("Facture {}", facture.numero),
        None,
    ))
    .avec_en_tete("ScolIA — Facture")
    .avec_pied(crate::pdf_commun::pied());
    d.titre("Facture");
    d.paragraphe(&format!("Numéro : {}", facture.numero));
    d.paragraphe(&format!("Libellé : {}", facture.libelle));
    d.paragraphe(&format!("Statut : {}", facture.statut.as_str()));
    if let Some(n) = &facture.nature {
        d.paragraphe(&format!("Nature : {n}"));
    }
    if let Some(a) = &facture.annee_scolaire {
        d.paragraphe(&format!("Année : {a}"));
    }
    d.sous_titre("Lignes");
    let mut t = Tableau::new(vec![
        Colonne::new("Libellé", 5.0, Align::Gauche),
        Colonne::new("Qté", 1.0, Align::Droite),
        Colonne::new("Prix unitaire", 2.0, Align::Droite),
        Colonne::new("Total", 2.0, Align::Droite),
    ]);
    for l in lignes {
        t.lignes.push(Ligne::new(vec![
            l.libelle.clone(),
            l.quantite.to_string(),
            euros_centimes(l.montant_unitaire_centimes),
            euros_centimes(l.total_centimes()),
        ]));
    }
    d.tableau(&t);
    d.espace(6.0);
    d.ligne_droite("Total", &euros_centimes(facture.total_centimes), true);
    d.ligne_droite("Encaissé", &euros_centimes(encaisse_centimes), false);
    d.ligne_droite(
        "Reste à payer",
        &euros_centimes(reste_a_payer(facture.total_centimes, encaisse_centimes)),
        true,
    );
    d.espace(10.0);
    d.paragraphe("Document générique ENT — sans valeur fiscale OGEC.");
    d.octets()
}

/// PDF quittance (reçu de paiement).
pub fn render_quittance_pdf(quittance: &Quittance, facture_numero: &str) -> Vec<u8> {
    let lines = vec![
        "ScolIA — Quittance".into(),
        String::new(),
        format!("Numéro quittance : {}", quittance.numero),
        format!("Facture : {facture_numero}"),
        format!("Montant : {}", euros_centimes(quittance.montant_centimes)),
        format!("Moyen : {}", quittance.moyen.as_str()),
        format!(
            "Émise le : {}",
            quittance.emise_at.format("%Y-%m-%d %H:%M UTC")
        ),
        String::new(),
        "Reçu de paiement — document générique ENT.".into(),
    ];
    crate::pdf_commun::rendre_lignes(&format!("Quittance {}", quittance.numero), None, &lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facturation::{FactureStatut, MoyenPaiement};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn facture_pdf_commence_par_header() {
        let f = Facture {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-PDF-1".into(),
            libelle: "Cantine octobre".into(),
            statut: FactureStatut::Emise,
            total_centimes: 7000,
            nature: None,
            annee_scolaire: None,
            created_by_user_id: None,
            created_at: Utc::now(),
            emise_at: Some(Utc::now()),
            soldee_at: None,
        };
        let lignes = vec![FactureLigne {
            id: Uuid::new_v4(),
            etablissement_id: f.etablissement_id,
            facture_id: f.id,
            eleve_id: None,
            libelle: "Cantine".into(),
            quantite: 20,
            montant_unitaire_centimes: 450,
        }];
        let pdf = render_facture_pdf(&f, &lignes, 0);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("F-PDF-1"));
        assert!(s.contains("%%EOF"));
    }

    #[test]
    fn quittance_pdf_ok() {
        let q = Quittance {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            facture_id: Uuid::new_v4(),
            encaissement_id: Uuid::new_v4(),
            numero: "Q-F-1-01".into(),
            montant_centimes: 3500,
            moyen: MoyenPaiement::Cheque,
            emise_at: Utc::now(),
            created_by_user_id: None,
        };
        let pdf = render_quittance_pdf(&q, "F-1");
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(String::from_utf8_lossy(&pdf).contains("Q-F-1-01"));
    }

    #[test]
    fn facture_accents_lisibles() {
        use ent_pdf::releve::{texte, verifier_structure};
        let f = Facture {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-É-1".into(),
            libelle: "Cantine — octobre, élèves demi-pensionnaires".into(),
            statut: FactureStatut::Emise,
            total_centimes: 9000,
            nature: Some("Restauration".into()),
            annee_scolaire: Some("2026-2027".into()),
            created_by_user_id: None,
            created_at: Utc::now(),
            emise_at: None,
            soldee_at: None,
        };
        let lignes = vec![FactureLigne {
            id: Uuid::new_v4(),
            etablissement_id: f.etablissement_id,
            facture_id: f.id,
            eleve_id: None,
            libelle: "Déjeuner « élève » — Hélène".into(),
            quantite: 20,
            montant_unitaire_centimes: 450,
        }];
        let pdf = render_facture_pdf(&f, &lignes, 3000);
        verifier_structure(&pdf);
        let t = texte(&pdf);
        assert!(t.contains("Numéro : F-É-1"), "{t}");
        assert!(t.contains("Déjeuner « élève » — Hélène"));
        assert!(t.contains("Reste à payer") && t.contains("60,00 EUR"));
        assert!(t.contains("Année : 2026-2027"));
        let q = Quittance {
            id: Uuid::new_v4(),
            etablissement_id: f.etablissement_id,
            foyer_id: f.foyer_id,
            facture_id: f.id,
            encaissement_id: Uuid::new_v4(),
            numero: "Q-É-1".into(),
            montant_centimes: 3000,
            moyen: MoyenPaiement::Cheque,
            emise_at: Utc::now(),
            created_by_user_id: None,
        };
        let tq = texte(&render_quittance_pdf(&q, "F-É-1"));
        assert!(
            tq.contains("Reçu de paiement") && tq.contains("Émise le"),
            "{tq}"
        );
    }
}
