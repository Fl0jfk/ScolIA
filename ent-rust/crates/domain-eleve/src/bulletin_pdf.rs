//! Bulletin PDF minimal — invariant : période **fermée** uniquement.
//!
//! Templates stylés catalogue = profondeur ultérieure ; ici = geste métier
//! observable (période clôturée → PDF → dépôt docs famille).

use crate::{Appreciation, DomainError, MoyenneMatiere, NotePeriode, NotePeriodeStatut};
use chrono::NaiveDate;
use ent_pdf::{Align, Colonne, Document, Ligne, Tableau};

#[derive(Debug, Clone)]
pub struct LigneBulletinMatiere {
    pub matiere_libelle: String,
    pub moyenne_sur_20: f64,
    pub appreciation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BulletinEleve {
    pub eleve_prenom: String,
    pub eleve_nom: String,
    pub classe: Option<String>,
    pub periode_libelle: String,
    pub periode_code: String,
    pub date_document: NaiveDate,
    pub lignes: Vec<LigneBulletinMatiere>,
    pub moyenne_generale: Option<f64>,
    pub appreciation_generale: Option<String>,
}

/// Invariant produit : bulletin imprimable seulement si période clôturée.
pub fn assert_periode_bulletin(periode: &NotePeriode) -> Result<(), DomainError> {
    if periode.statut != NotePeriodeStatut::Fermee {
        return Err(DomainError::NoteBulletinPeriodeNonFermee);
    }
    Ok(())
}

pub fn plan_bulletin_eleve(
    periode: &NotePeriode,
    eleve_prenom: &str,
    eleve_nom: &str,
    classe: Option<&str>,
    moyennes: &[(MoyenneMatiere, String)],
    appreciations: &[Appreciation],
    date_document: NaiveDate,
) -> Result<BulletinEleve, DomainError> {
    assert_periode_bulletin(periode)?;
    let app_gen = appreciations
        .iter()
        .find(|a| a.matiere_id.is_none())
        .map(|a| a.texte.clone());
    let mut lignes = Vec::new();
    let mut sum = 0.0;
    let mut n = 0usize;
    for (m, lib) in moyennes {
        let app = appreciations
            .iter()
            .find(|a| a.matiere_id == Some(m.matiere_id))
            .map(|a| a.texte.clone());
        sum += m.moyenne_sur_20;
        n += 1;
        lignes.push(LigneBulletinMatiere {
            matiere_libelle: lib.clone(),
            moyenne_sur_20: m.moyenne_sur_20,
            appreciation: app,
        });
    }
    lignes.sort_by(|a, b| a.matiere_libelle.cmp(&b.matiere_libelle));
    let moyenne_generale = if n == 0 { None } else { Some(sum / n as f64) };
    Ok(BulletinEleve {
        eleve_prenom: eleve_prenom.trim().to_string(),
        eleve_nom: eleve_nom.trim().to_string(),
        classe: classe.map(|s| s.to_string()),
        periode_libelle: periode.libelle.clone(),
        periode_code: periode.code.clone(),
        date_document,
        lignes,
        moyenne_generale,
        appreciation_generale: app_gen,
    })
}

/// PDF texte (une ligne = une information, titre = première ligne) sur plusieurs pages A4,
/// avec « Page n sur N ». Les caractères français s'affichent tels quels (WinAnsi).
pub fn rendre_pdf_lignes(lines: &[String]) -> Vec<u8> {
    crate::pdf_commun::rendre_lignes("Document ScolIA", None, lines)
}

pub fn render_bulletin_pdf(b: &BulletinEleve) -> Vec<u8> {
    let mut d = Document::new(crate::pdf_commun::meta(
        &format!(
            "Bulletin — {} {} — {}",
            b.eleve_prenom, b.eleve_nom, b.periode_libelle
        ),
        Some(b.date_document),
    ))
    .avec_en_tete("ScolIA — Bulletin scolaire")
    .avec_pied(crate::pdf_commun::pied());
    d.titre("Bulletin scolaire");
    d.paragraphe(&format!("Date : {}", b.date_document));
    d.paragraphe(&format!(
        "Élève : {} {} · Classe : {}",
        b.eleve_prenom,
        b.eleve_nom,
        b.classe.as_deref().unwrap_or("—")
    ));
    d.paragraphe(&format!(
        "Période : {} ({})",
        b.periode_libelle, b.periode_code
    ));
    d.sous_titre("Moyennes /20");
    let mut t = Tableau::new(vec![
        Colonne::new("Matière", 3.0, Align::Gauche),
        Colonne::new("Moyenne", 1.6, Align::Droite),
        Colonne::new("Appréciation", 6.0, Align::Gauche),
    ]);
    for l in &b.lignes {
        t.lignes.push(Ligne::new(vec![
            l.matiere_libelle.clone(),
            format!("{:.2}", l.moyenne_sur_20),
            l.appreciation.clone().unwrap_or_default(),
        ]));
    }
    if let Some(mg) = b.moyenne_generale {
        t.lignes.push(Ligne::grasse(vec![
            "Moyenne générale".to_string(),
            format!("{mg:.2}"),
            String::new(),
        ]));
    }
    d.tableau(&t);
    if let Some(ag) = &b.appreciation_generale {
        d.sous_titre("Appréciation générale");
        d.paragraphe(ag);
    }
    d.espace(10.0);
    d.paragraphe("Document publié dans l'espace documents (famille).");
    d.octets()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NotePeriodeStatut;
    use uuid::Uuid;

    fn periode(statut: NotePeriodeStatut) -> NotePeriode {
        NotePeriode {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            code: "T1".into(),
            libelle: "Trimestre 1".into(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: NaiveDate::from_ymd_opt(2026, 12, 20).unwrap(),
            statut,
        }
    }

    #[test]
    fn pdf_multipage() {
        let lignes: Vec<String> = (0..130).map(|i| format!("Ligne {i} é")).collect();
        let pdf = rendre_pdf_lignes(&lignes);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(9).any(|w| w == b"/Count 3 "));
        assert!(ent_pdf::releve::texte(&pdf).contains("Page 3 sur 3"));
        assert!(rendre_pdf_lignes(&[]).windows(9).any(|w| w == b"/Count 1 "));
    }

    #[test]
    fn refuse_si_ouverte() {
        let err = assert_periode_bulletin(&periode(NotePeriodeStatut::Ouverte)).unwrap_err();
        assert_eq!(err, DomainError::NoteBulletinPeriodeNonFermee);
    }

    #[test]
    fn pdf_si_fermee() {
        let p = periode(NotePeriodeStatut::Fermee);
        let mid = Uuid::new_v4();
        let b = plan_bulletin_eleve(
            &p,
            "Harry",
            "Potter",
            Some("4B"),
            &[(
                MoyenneMatiere {
                    matiere_id: mid,
                    periode_id: p.id,
                    moyenne_sur_20: 14.5,
                },
                "Maths".into(),
            )],
            &[],
            NaiveDate::from_ymd_opt(2026, 12, 21).unwrap(),
        )
        .unwrap();
        let pdf = render_bulletin_pdf(&b);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
    }

    #[test]
    fn bulletin_accents_lisibles_et_trois_pages() {
        use ent_pdf::releve::{texte, texte_des_pages, verifier_structure};
        let p = periode(NotePeriodeStatut::Fermee);
        let moyennes: Vec<(MoyenneMatiere, String)> = (0..40)
            .map(|i| {
                (
                    MoyenneMatiere {
                        matiere_id: Uuid::new_v4(),
                        periode_id: p.id,
                        moyenne_sur_20: 8.0 + (i % 11) as f64,
                    },
                    format!("Matière {i:02} — Français œuvres"),
                )
            })
            .collect();
        let appreciations: Vec<Appreciation> = moyennes
            .iter()
            .map(|(m, _)| Appreciation {
                id: Uuid::new_v4(),
                etablissement_id: p.etablissement_id,
                eleve_id: Uuid::new_v4(),
                periode_id: p.id,
                matiere_id: Some(m.matiere_id),
                auteur_user_id: Uuid::new_v4(),
                texte: "Élève sérieux, à l'écoute ; progrès réguliers en français, œuvre soignée. \
                        Il faut poursuivre l'effort et soigner la présentation des évaluations écrites."
                    .into(),
            })
            .collect();
        let b = plan_bulletin_eleve(
            &p,
            "Hélène",
            "Élève",
            Some("4ᵉ B"),
            &moyennes,
            &appreciations,
            NaiveDate::from_ymd_opt(2026, 12, 21).unwrap(),
        )
        .unwrap();
        let pdf = render_bulletin_pdf(&b);
        verifier_structure(&pdf);
        let t = texte(&pdf);
        assert!(t.contains("Élève : Hélène Élève"), "{t}");
        assert!(t.contains("Période : Trimestre 1 (T1)"));
        assert!(
            t.contains("Matière 00 — Français") && t.contains("œuvres"),
            "{t}"
        );
        let pages = texte_des_pages(&pdf);
        assert_eq!(pages.len(), 3, "{} pages", pages.len());
        for (i, pg) in pages.iter().enumerate() {
            assert!(pg.contains(&format!("Page {} sur 3", i + 1)));
            assert!(
                pg.contains("Matière\nMoyenne\nAppréciation"),
                "en-tête du tableau répété, page {i}"
            );
        }
        assert!(pages[2].contains("Moyenne générale"));
    }
}
