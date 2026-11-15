//! Socle 42, lot V0d : restauration d'une sortie — **règles pures** (`docs/socles/42_SORTIES_VOYAGES.md` §14 ;
//! NX-38 à NX-40, NX-86). Aucune base ici : les commandes et lectures sont dans `voyages_restauration_repo.rs`.
//!
//! * un **panier** est attribué nominativement (participant ou adulte) : le nombre d'attributions plus celui
//!   des convives externes ne dépasse **jamais** la quantité commandée (plafond), et il l'égale quand la
//!   commande est passée (exact) ;
//! * une variante est un **code alimentaire**, jamais un nom de maladie ; aucune ligne ne porte d'INE (VD-16) ;
//! * l'avis à la cuisine est un **texte brut** composé par le serveur ; le destinataire n'y figure pas
//!   (VD-06) ; le corps est identique à l'aperçu (R-SV12).

use chrono::{NaiveDate, NaiveTime};
use std::collections::BTreeMap;

use crate::voyages_prestataires::ligne_propre;

pub const CATEGORIES: [&str; 5] = ["panier", "self_eleve", "self_adulte", "boisson", "gouter"];
pub const VARIANTES: [&str; 6] = [
    "standard",
    "sans_porc",
    "vegetarien",
    "sans_arachide",
    "sans_gluten",
    "sans_lactose",
];

/// Une ligne de commande : `quantite` unités de `categorie` et `variante` le `jour`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LigneTotal {
    pub jour: NaiveDate,
    pub categorie: String,
    pub variante: String,
    pub quantite: i32,
}

/// Écart entre les paniers commandés et les paniers attribués (nominativement ou à des convives externes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcartPanier {
    pub jour: NaiveDate,
    pub variante: String,
    pub commandes: i32,
    pub attribues: i32,
}

impl EcartPanier {
    /// Plus de paniers attribués que commandés : l'écriture est refusée.
    pub fn depasse(&self) -> bool {
        self.attribues > self.commandes
    }
}

pub fn categorie_valide(c: &str) -> bool {
    CATEGORIES.contains(&c)
}

pub fn variante_valide(v: &str) -> bool {
    VARIANTES.contains(&v)
}

/// Écarts panier par (jour, variante), pour les seuls couples commandés ou attribués. `attribues` : un
/// élément par panier attribué (participant, adulte ou externe).
pub fn ecarts_paniers(lignes: &[LigneTotal], attribues: &[(NaiveDate, String, i32)]) -> Vec<EcartPanier> {
    let mut m: BTreeMap<(NaiveDate, String), (i32, i32)> = BTreeMap::new();
    for l in lignes.iter().filter(|l| l.categorie == "panier") {
        m.entry((l.jour, l.variante.clone())).or_default().0 += l.quantite;
    }
    for (j, v, n) in attribues {
        m.entry((*j, v.clone())).or_default().1 += *n;
    }
    m.into_iter()
        .filter(|(_, (c, a))| c != a)
        .map(|((jour, variante), (commandes, attribues))| EcartPanier {
            jour,
            variante,
            commandes,
            attribues,
        })
        .collect()
}

/// Vrai si aucune attribution ne dépasse la commande (plafond) : condition d'écriture.
pub fn plafond_respecte(ecarts: &[EcartPanier]) -> bool {
    ecarts.iter().all(|e| !e.depasse())
}

/// Vrai si les paniers attribués égalent exactement les paniers commandés : condition pour passer la
/// commande ou émettre un avenant.
pub fn exacte(ecarts: &[EcartPanier]) -> bool {
    ecarts.is_empty()
}

/// Libellé d'une variante pour la cuisine.
pub fn libelle_variante(v: &str) -> &'static str {
    match v {
        "sans_porc" => "sans porc",
        "vegetarien" => "végétarien",
        "sans_arachide" => "sans arachide",
        "sans_gluten" => "sans gluten",
        "sans_lactose" => "sans lactose",
        _ => "standard",
    }
}

fn libelle_categorie(c: &str) -> &'static str {
    match c {
        "panier" => "Paniers repas",
        "self_eleve" => "Repas self (élèves)",
        "self_adulte" => "Repas self (adultes)",
        "boisson" => "Boissons",
        "gouter" => "Goûters",
        _ => "Autre",
    }
}

/// Un convive nominatif d'une liste « qui mange » : nom, prénom, classe, variante — **sans INE**, sans
/// motif, sans coordonnée de famille.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Convive {
    pub jour: NaiveDate,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub variante: String,
    pub etiquette: bool,
}

/// Faits qui composent un avis à la cuisine.
#[derive(Debug, Clone)]
pub struct FaitsAvis<'a> {
    pub etablissement: &'a str,
    pub reference: &'a str,
    pub titre: &'a str,
    pub numero: i32,
    pub avenant: bool,
    pub motif: Option<&'a str>,
    pub livraison_heure: Option<NaiveTime>,
    pub livraison_lieu: Option<&'a str>,
    pub lignes: &'a [LigneTotal],
    pub convives: &'a [Convive],
    pub externes: &'a [(NaiveDate, String, i32, String)],
}

/// Objet de l'avis : une seule ligne, sans caractère de contrôle (injection d'en-tête impossible).
pub fn objet_avis(reference: &str, numero: i32, avenant: bool) -> String {
    if avenant {
        ligne_propre(&format!("[{reference}] Restauration — avenant n° {numero} (annule et remplace)"), 200)
    } else {
        ligne_propre(&format!("[{reference}] Restauration — commande"), 200)
    }
}

fn jour_fr(j: NaiveDate) -> String {
    j.format("%d/%m/%Y").to_string()
}

/// Corps de l'avis en **texte brut** : tout texte venu de la base est nettoyé de ses sauts de ligne.
pub fn corps_avis(f: &FaitsAvis<'_>) -> String {
    let mut s = String::new();
    s.push_str("Bonjour,\n\n");
    if f.avenant {
        s.push_str(&format!(
            "Cet avenant n° {} ANNULE ET REMPLACE la commande précédente pour la sortie ci-dessous.\n",
            f.numero
        ));
        if let Some(m) = f.motif {
            s.push_str(&format!("Motif : {}\n", ligne_propre(m, 300)));
        }
        s.push('\n');
    }
    s.push_str(&format!("Établissement : {}\n", ligne_propre(f.etablissement, 120)));
    s.push_str(&format!("Sortie : {} ({})\n", ligne_propre(f.titre, 200), ligne_propre(f.reference, 40)));
    match (f.livraison_heure, f.livraison_lieu) {
        (None, None) => {}
        (h, l) => s.push_str(&format!(
            "Livraison : {}{}\n",
            h.map(|h| format!("{} ", h.format("%H:%M"))).unwrap_or_default(),
            l.map(|l| ligne_propre(l, 300)).unwrap_or_default()
        )),
    }
    let mut jours: Vec<NaiveDate> = f.lignes.iter().map(|l| l.jour).collect();
    jours.sort();
    jours.dedup();
    for j in jours {
        s.push_str(&format!("\n{}\n", jour_fr(j)));
        for l in f.lignes.iter().filter(|l| l.jour == j && l.quantite > 0) {
            s.push_str(&format!(
                "  - {} ({}) : {}\n",
                libelle_categorie(&l.categorie),
                libelle_variante(&l.variante),
                l.quantite
            ));
        }
        for (_, v, n, lib) in f.externes.iter().filter(|e| e.0 == j) {
            s.push_str(&format!(
                "  - dont convives extérieurs : {} × {} ({})\n",
                n,
                ligne_propre(lib, 100),
                libelle_variante(v)
            ));
        }
        for c in f.convives.iter().filter(|c| c.jour == j) {
            s.push_str(&format!(
                "  * {} {} — {} — panier {}{}\n",
                ligne_propre(&c.nom, 80),
                ligne_propre(&c.prenom, 80),
                c.classe.as_deref().map(|x| ligne_propre(x, 40)).unwrap_or_else(|| "classe non renseignée".into()),
                libelle_variante(&c.variante),
                if c.etiquette { " (à étiqueter au nom de l'élève)" } else { "" }
            ));
        }
    }
    s.push_str("\nMerci de nous accuser réception de ce message.\n");
    s
}

/// Différences lisibles entre deux versions de la commande (motif proposé d'un avenant).
pub fn differences(avant: &[LigneTotal], apres: &[LigneTotal]) -> Vec<String> {
    let cle = |l: &LigneTotal| (l.jour, l.categorie.clone(), l.variante.clone());
    let a: BTreeMap<_, _> = avant.iter().map(|l| (cle(l), l.quantite)).collect();
    let b: BTreeMap<_, _> = apres.iter().map(|l| (cle(l), l.quantite)).collect();
    let mut out = Vec::new();
    let mut cles: Vec<_> = a.keys().chain(b.keys()).cloned().collect();
    cles.sort();
    cles.dedup();
    for k in cles {
        let (x, y) = (a.get(&k).copied().unwrap_or(0), b.get(&k).copied().unwrap_or(0));
        if x != y {
            out.push(format!(
                "{} {} ({}) : {} → {}",
                jour_fr(k.0),
                libelle_categorie(&k.1),
                libelle_variante(&k.2),
                x,
                y
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, 5, j).unwrap()
    }

    fn l(j: u32, c: &str, v: &str, q: i32) -> LigneTotal {
        LigneTotal { jour: d(j), categorie: c.into(), variante: v.into(), quantite: q }
    }

    #[test]
    fn plafond_et_exactitude_des_paniers() {
        let lignes = [l(10, "panier", "standard", 3), l(10, "panier", "vegetarien", 1), l(10, "boisson", "standard", 40)];
        // Exact : 3 standard + 1 végétarien attribués.
        let ok = [(d(10), "standard".to_string(), 3), (d(10), "vegetarien".to_string(), 1)];
        assert!(exacte(&ecarts_paniers(&lignes, &ok)));
        // Un panier manque : plafond respecté, mais pas exact.
        let moins = [(d(10), "standard".to_string(), 2), (d(10), "vegetarien".to_string(), 1)];
        let e = ecarts_paniers(&lignes, &moins);
        assert!(plafond_respecte(&e) && !exacte(&e));
        // Trop attribués : plafond dépassé.
        let plus = [(d(10), "standard".to_string(), 4), (d(10), "vegetarien".to_string(), 1)];
        assert!(!plafond_respecte(&ecarts_paniers(&lignes, &plus)));
        // Une attribution sans commande est un dépassement.
        assert!(!plafond_respecte(&ecarts_paniers(&[], &[(d(11), "standard".to_string(), 1)])));
    }

    #[test]
    fn objet_sans_saut_de_ligne_et_corps_sans_ine() {
        let o = objet_avis("VOY-2027-AB\r\nBcc: x@y.fr", 2, true);
        assert!(!o.contains('\n') && !o.contains('\r'));
        let lignes = [l(10, "panier", "sans_gluten", 1)];
        let conv = [Convive {
            jour: d(10),
            nom: "Durand\nBcc: x".into(),
            prenom: "Léa".into(),
            classe: Some("4A".into()),
            variante: "sans_gluten".into(),
            etiquette: true,
        }];
        let c = corps_avis(&FaitsAvis {
            etablissement: "Collège",
            reference: "VOY-2027-AB",
            titre: "Sortie",
            numero: 1,
            avenant: true,
            motif: Some("Désistement"),
            livraison_heure: NaiveTime::from_hms_opt(8, 30, 0),
            livraison_lieu: Some("Portail"),
            lignes: &lignes,
            convives: &conv,
            externes: &[(d(10), "standard".into(), 2, "Parents accompagnateurs".into())],
        });
        assert!(c.contains("ANNULE ET REMPLACE") && c.contains("Durand Bcc: x Léa"));
        assert!(!c.contains("INE"), "aucun INE");
        assert!(c.contains("sans gluten") && c.contains("à étiqueter"));
    }

    #[test]
    fn differences_entre_deux_versions() {
        let av = [l(10, "panier", "standard", 3)];
        let ap = [l(10, "panier", "standard", 2), l(10, "boisson", "standard", 5)];
        let diff = differences(&av, &ap);
        assert_eq!(diff.len(), 2);
        assert!(diff.iter().any(|x| x.contains("3 → 2")));
    }
}
