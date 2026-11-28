//! `ent-api plateforme-violation …` — **console de l'éditeur** pour les violations de la plateforme (socle 27,
//! complément du palier G5 ; RGP-254, RGP-255, RGP-311, R-RG36, CL27-44).
//!
//! Même régime que `ent-api plateforme-domaine` : un outil de l'**opérateur ScolIA**, avec accès direct à la base
//! (`ENT_DATABASE_URL`), sans route HTTP ni droit d'établissement. Choix prudents (D-069), notés dans
//! `docs/socles/27_G5_COMPLEMENT_LIVRAISON.md` :
//! * une propagation alerte la direction d'un établissement réel : la commande montre d'abord ce qui partira et à qui,
//!   et n'agit qu'avec `--confirmer <slug de l'établissement>` (une faute de frappe sur l'identifiant ne prévient
//!   jamais le mauvais établissement) ;
//! * la clôture, sans retour possible, se confirme par la référence (`--confirmer P-2027-002`) ;
//! * les instants se saisissent en RFC 3339 **avec** leur décalage (`2027-03-01T08:00:00+01:00`) : jamais d'heure
//!   ambiguë ;
//! * l'éditeur voit à qui il a propagé, quand, et la référence locale ; jamais ce que l'établissement en a décidé.

use chrono::{DateTime, Utc};
use ent_db::violations_plateforme::DeclarationPlateforme;
use uuid::Uuid;

pub const AIDE: &str = "\
ent-api plateforme-violation <commande>
  Console de l'éditeur (opérateur ScolIA, accès direct à la base, ENT_DATABASE_URL) : violations de la
  plateforme ou d'un sous-traitant de la plateforme (RGPD art. 33.2). Aucune route HTTP.

  lister
      toutes les violations de la plateforme (ouvertes d'abord)
  voir <référence|uuid>
      le détail et les établissements déjà informés (référence locale, date)
  declarer --intitule <texte> --description <texte> --connue-le <AAAA-MM-JJThh:mm:ss+hh:mm>
           --atteinte <confidentialite,integrite,disponibilite> --par <nom du déclarant> [--couche-compte]
      déclare UNE fois la violation (référence P-AAAA-NNN) ; n'informe encore personne
  propager <référence|uuid> <etablissement-uuid> --elements <texte> [--personnes <n>] [--confirmer <slug>]
      informe UN établissement avec ses seuls éléments : sa direction est alertée, son horloge de 72 h part.
      Sans --confirmer : aperçu, rien n'est envoyé. Idempotent (une seconde propagation ne réécrit rien).
  notifier-cnil <référence|uuid> --le <AAAA-MM-JJThh:mm:ss+hh:mm> --reference <référence CNIL> [--confirmer <référence CNIL>]
      ScolIA, responsable de la couche compte, saisit SA notification (une fois, couche compte seulement,
      sans retour possible). Sans --confirmer : aperçu, rien n'est saisi.
  clore <référence|uuid> --confirmer <référence>
      plus aucune propagation ; les violations propagées restent instruites par chaque établissement
Code de sortie : 0 succès ; 1 erreur ; 2 aperçu (confirmation attendue).
";

/// Une commande lue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commande {
    Aide,
    Lister,
    Voir(String),
    Declarer {
        intitule: String,
        description: String,
        connue_le: DateTime<Utc>,
        confidentialite: bool,
        integrite: bool,
        disponibilite: bool,
        couche_compte: bool,
        par: String,
    },
    Propager {
        violation: String,
        etablissement: Uuid,
        elements: String,
        personnes: Option<i32>,
        confirmer: Option<String>,
    },
    NotifierCnil {
        violation: String,
        le: DateTime<Utc>,
        reference: String,
        confirmer: Option<String>,
    },
    Clore {
        violation: String,
        confirmer: String,
    },
}

/// Options `--nom valeur` (et drapeaux sans valeur) après les positionnels. Une option répétée ou inconnue est refusée.
struct Options {
    positionnels: Vec<String>,
    valeurs: Vec<(String, String)>,
    drapeaux: Vec<String>,
}

fn lire_options(args: &[String], connues: &[&str], drapeaux: &[&str]) -> Result<Options, String> {
    let mut o = Options {
        positionnels: Vec::new(),
        valeurs: Vec::new(),
        drapeaux: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if let Some(nom) = a.strip_prefix("--") {
            if drapeaux.contains(&nom) {
                if o.drapeaux.iter().any(|d| d == nom) {
                    return Err(format!("--{nom} répété"));
                }
                o.drapeaux.push(nom.to_string());
            } else if connues.contains(&nom) {
                let v = it.next().ok_or(format!("--{nom} attend une valeur"))?;
                if o.valeurs.iter().any(|(n, _)| n == nom) {
                    return Err(format!("--{nom} répété"));
                }
                o.valeurs.push((nom.to_string(), v.clone()));
            } else {
                return Err(format!("option inconnue : --{nom}"));
            }
        } else {
            o.positionnels.push(a.clone());
        }
    }
    Ok(o)
}

impl Options {
    fn valeur(&self, nom: &str) -> Option<&str> {
        self.valeurs
            .iter()
            .find(|(n, _)| n == nom)
            .map(|(_, v)| v.as_str())
    }
    fn exiger(&self, nom: &str) -> Result<String, String> {
        self.valeur(nom)
            .map(str::to_string)
            .ok_or(format!("--{nom} requis"))
    }
    fn positionnels(&self, n: usize) -> Result<&[String], String> {
        if self.positionnels.len() != n {
            return Err(format!(
                "{n} argument(s) attendu(s) avant les options, {} reçu(s)",
                self.positionnels.len()
            ));
        }
        Ok(&self.positionnels)
    }
}

/// Instant RFC 3339 **avec** décalage explicite (`Z` ou `±hh:mm`).
fn instant(v: &str, nom: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(v.trim())
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| {
            format!("--{nom} : instant RFC 3339 avec décalage attendu (2027-03-01T08:00:00+01:00)")
        })
}

/// Lit la ligne de commande (sans le mot `plateforme-violation`).
pub fn lire(args: &[String]) -> Result<Commande, String> {
    let Some((commande, reste)) = args.split_first() else {
        return Ok(Commande::Aide);
    };
    match commande.as_str() {
        "--aide" | "--help" | "-h" | "aide" => Ok(Commande::Aide),
        "lister" => {
            lire_options(reste, &[], &[])?.positionnels(0)?;
            Ok(Commande::Lister)
        }
        "voir" => {
            let o = lire_options(reste, &[], &[])?;
            Ok(Commande::Voir(o.positionnels(1)?[0].clone()))
        }
        "declarer" => {
            let o = lire_options(
                reste,
                &["intitule", "description", "connue-le", "atteinte", "par"],
                &["couche-compte"],
            )?;
            o.positionnels(0)?;
            let (mut c, mut i, mut d) = (false, false, false);
            for a in o.exiger("atteinte")?.split(',').map(str::trim) {
                match a {
                    "confidentialite" => c = true,
                    "integrite" => i = true,
                    "disponibilite" => d = true,
                    autre => return Err(format!("--atteinte : « {autre} » inconnue (confidentialite, integrite, disponibilite)")),
                }
            }
            Ok(Commande::Declarer {
                intitule: o.exiger("intitule")?,
                description: o.exiger("description")?,
                connue_le: instant(&o.exiger("connue-le")?, "connue-le")?,
                confidentialite: c,
                integrite: i,
                disponibilite: d,
                couche_compte: o.drapeaux.iter().any(|f| f == "couche-compte"),
                par: o.exiger("par")?,
            })
        }
        "propager" => {
            let o = lire_options(reste, &["elements", "personnes", "confirmer"], &[])?;
            let p = o.positionnels(2)?;
            let etablissement = Uuid::parse_str(p[1].trim())
                .map_err(|_| "UUID d'établissement invalide".to_string())?;
            let personnes = match o.valeur("personnes") {
                Some(v) => Some(
                    v.trim()
                        .parse::<i32>()
                        .ok()
                        .filter(|n| *n >= 0)
                        .ok_or("--personnes : entier positif ou nul attendu")?,
                ),
                None => None,
            };
            Ok(Commande::Propager {
                violation: p[0].clone(),
                etablissement,
                elements: o.exiger("elements")?,
                personnes,
                confirmer: o.valeur("confirmer").map(str::to_string),
            })
        }
        "notifier-cnil" => {
            let o = lire_options(reste, &["le", "reference", "confirmer"], &[])?;
            Ok(Commande::NotifierCnil {
                violation: o.positionnels(1)?[0].clone(),
                le: instant(&o.exiger("le")?, "le")?,
                reference: o.exiger("reference")?,
                confirmer: o.valeur("confirmer").map(str::to_string),
            })
        }
        "clore" => {
            let o = lire_options(reste, &["confirmer"], &[])?;
            Ok(Commande::Clore {
                violation: o.positionnels(1)?[0].clone(),
                confirmer: o.exiger("confirmer")?,
            })
        }
        autre => Err(format!("commande inconnue : {autre} (voir --aide)")),
    }
}

fn atteintes(c: bool, i: bool, d: bool) -> String {
    [
        (c, "confidentialité"),
        (i, "intégrité"),
        (d, "disponibilité"),
    ]
    .iter()
    .filter(|(b, _)| *b)
    .map(|(_, l)| *l)
    .collect::<Vec<_>>()
    .join(", ")
}

fn date(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%d %H:%M UTC").to_string()
}

/// Issue d'une commande : texte à afficher et code de sortie.
pub struct Issue {
    pub texte: String,
    pub code: i32,
}

fn ok(texte: String) -> Result<Issue, String> {
    Ok(Issue { texte, code: 0 })
}

/// Exécute une commande contre la base.
pub async fn executer(ent: &ent_db::PgEnt, c: Commande) -> Result<Issue, String> {
    let e = |e: ent_db::EntDbError| e.to_string();
    let maintenant = Utc::now();
    match c {
        Commande::Aide => ok(AIDE.trim_end().to_string()),
        Commande::Lister => {
            let l = ent.violations_plateforme_lister().await.map_err(e)?;
            if l.is_empty() {
                return ok("aucune violation de la plateforme".into());
            }
            ok(l.into_iter()
                .map(|v| {
                    format!(
                        "{}\t{}\tconnue {}\t{}{}\t{}",
                        v.reference,
                        v.etat,
                        date(v.connue_le),
                        if v.couche_compte {
                            "couche compte"
                        } else {
                            "sous-traitance"
                        },
                        match &v.notifiee_cnil_le {
                            Some(n) => format!(", CNIL {}", date(*n)),
                            None => String::new(),
                        },
                        v.intitule
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        Commande::Voir(cle) => {
            let v = ent.violations_plateforme_trouver(&cle).await.map_err(e)?;
            let p = ent
                .violations_plateforme_propagations(v.id)
                .await
                .map_err(e)?;
            let mut t = vec![
                format!("{} ({}) — {}", v.reference, v.id, v.intitule),
                format!(
                    "état : {}{}",
                    v.etat,
                    v.close_le
                        .map(|c| format!(", close le {}", date(c)))
                        .unwrap_or_default()
                ),
                format!(
                    "connue le {} ; déclarée le {} par {}",
                    date(v.connue_le),
                    date(v.declaree_le),
                    v.declaree_par
                ),
                format!(
                    "atteinte : {}",
                    atteintes(v.confidentialite, v.integrite, v.disponibilite)
                ),
                format!(
                    "couche compte : {}{}",
                    if v.couche_compte {
                        "oui (ScolIA notifie pour sa part)"
                    } else {
                        "non (ScolIA sous-traitant)"
                    },
                    match (&v.notifiee_cnil_le, &v.reference_cnil) {
                        (Some(n), Some(r)) =>
                            format!(" ; notifiée à la CNIL le {} ({r})", date(*n)),
                        _ if v.couche_compte =>
                            " ; notification de ScolIA pas encore saisie".into(),
                        _ => String::new(),
                    }
                ),
                format!("description : {}", v.description),
                format!("établissements informés : {}", p.len()),
            ];
            for x in p {
                t.push(format!(
                    "  {}\t{} ({})\tinformé le {}\t{}{}",
                    x.etablissement_id,
                    x.etablissement_slug,
                    x.etablissement_nom,
                    date(x.informe_le),
                    x.reference_locale,
                    x.personnes_nombre
                        .map(|n| format!("\t{n} personnes"))
                        .unwrap_or_default()
                ));
            }
            ok(t.join("\n"))
        }
        Commande::Declarer {
            intitule,
            description,
            connue_le,
            confidentialite,
            integrite,
            disponibilite,
            couche_compte,
            par,
        } => {
            let (id, reference) = ent
                .violations_plateforme_declarer(
                    &par,
                    DeclarationPlateforme {
                        intitule: &intitule,
                        description: &description,
                        connue_le,
                        confidentialite,
                        integrite,
                        disponibilite,
                        couche_compte,
                    },
                    maintenant,
                )
                .await
                .map_err(e)?;
            ok(format!(
                "violation de la plateforme {reference} déclarée ({id}). Aucun établissement n'est encore informé : \
                 `propager {reference} <etablissement-uuid> --elements …` pour chacun."
            ))
        }
        Commande::Propager {
            violation,
            etablissement,
            elements,
            personnes,
            confirmer,
        } => {
            let v = ent
                .violations_plateforme_trouver(&violation)
                .await
                .map_err(e)?;
            let cible = ent
                .violations_plateforme_cible(etablissement)
                .await
                .map_err(|_| "établissement inconnu".to_string())?;
            if v.etat != "ouverte" {
                return Err(format!("{} est close : plus de propagation", v.reference));
            }
            // Mêmes bornes que le service, vérifiées dès l'aperçu (pas après la frappe du slug).
            let n = elements.trim().chars().count();
            if !(3..=4000).contains(&n) || elements.chars().any(|c| c.is_control() && c != '\n') {
                return Err("--elements : 3 à 4 000 caractères, sans caractère de contrôle".into());
            }
            if personnes.is_some_and(|p| p < 0) {
                return Err("--personnes : nombre négatif".into());
            }
            let deja = ent
                .violations_plateforme_propagations(v.id)
                .await
                .map_err(e)?
                .into_iter()
                .find(|p| p.etablissement_id == cible.id);
            if let (None, Some(p)) = (&confirmer, &deja) {
                return ok(format!(
                    "{} est déjà propagée à {} ({}) le {} (référence locale {}) : rien à faire.",
                    v.reference,
                    cible.nom,
                    cible.slug,
                    date(p.informe_le),
                    p.reference_locale
                ));
            }
            match confirmer {
                Some(slug) if slug.trim() == cible.slug => {
                    let (id, nouvelle) = ent
                        .violations_plateforme_propager(v.id, etablissement, &elements, personnes, maintenant)
                        .await
                        .map_err(e)?;
                    ok(if nouvelle {
                        format!(
                            "{} propagée à {} ({}) : violation {id} créée au registre, direction alertée, horloge de 72 h \
                             partie maintenant.",
                            v.reference, cible.nom, cible.slug
                        )
                    } else {
                        format!(
                            "{} était déjà propagée à {} ({}) : rien n'est réécrit (violation {id}).",
                            v.reference, cible.nom, cible.slug
                        )
                    })
                }
                Some(_) => Err(format!(
                    "--confirmer ne correspond pas au slug de l'établissement visé ({}) : rien n'est envoyé",
                    cible.nom
                )),
                None => Ok(Issue {
                    texte: format!(
                        "APERÇU, rien n'est envoyé.\n\
                         Violation : {} — {}\n\
                         Établissement : {} (slug {}, {})\n\
                         Éléments transmis : {}\n\
                         Personnes : {}\n\
                         Effet : une violation d'origine « plateforme » est créée à son registre, sa direction est \
                         alertée sur tous ses canaux, son horloge de 72 h part de maintenant.\n\
                         Pour envoyer : relancer avec --confirmer {}",
                        v.reference,
                        v.intitule,
                        cible.nom,
                        cible.slug,
                        cible.id,
                        elements.trim(),
                        personnes.map(|n| n.to_string()).unwrap_or_else(|| "non précisé".into()),
                        cible.slug
                    ),
                    code: 2,
                }),
            }
        }
        Commande::NotifierCnil {
            violation,
            le,
            reference,
            confirmer,
        } => {
            let v = ent
                .violations_plateforme_trouver(&violation)
                .await
                .map_err(e)?;
            match confirmer {
                None => {
                    return Ok(Issue {
                        texte: format!(
                            "APERÇU, rien n'est saisi.\n\
                             Violation : {} — {}\n\
                             Notification de ScolIA à la CNIL : le {} (UTC), référence {}\n\
                             Effet : saisie définitive, sans correction possible.\n\
                             Pour saisir : relancer avec --confirmer {}",
                            v.reference,
                            v.intitule,
                            date(le),
                            reference.trim(),
                            reference.trim()
                        ),
                        code: 2,
                    })
                }
                Some(c) if c.trim() != reference.trim() => {
                    return Err(
                        "--confirmer doit reprendre la référence CNIL : rien n'est saisi".into(),
                    )
                }
                Some(_) => {}
            }
            ent.violations_plateforme_notifier_cnil(v.id, le, &reference, maintenant)
                .await
                .map_err(e)?;
            ok(format!(
                "notification de ScolIA saisie pour {} : {} ({reference})",
                v.reference,
                date(le)
            ))
        }
        Commande::Clore {
            violation,
            confirmer,
        } => {
            let v = ent
                .violations_plateforme_trouver(&violation)
                .await
                .map_err(e)?;
            if !confirmer.trim().eq_ignore_ascii_case(&v.reference) {
                return Err(format!(
                    "--confirmer doit reprendre la référence ({}) : rien n'est clos",
                    v.reference
                ));
            }
            ent.violations_plateforme_clore(v.id, maintenant)
                .await
                .map_err(e)?;
            ok(format!(
                "{} close : plus de propagation. Chaque établissement informé instruit et clôt sa propre violation.",
                v.reference
            ))
        }
    }
}

/// Point d'entrée de `ent-api plateforme-violation …`.
pub async fn lancer(args: &[String]) -> i32 {
    let commande = match lire(args) {
        Ok(c) => c,
        Err(m) => {
            eprintln!("plateforme-violation : {m}");
            return 1;
        }
    };
    if commande == Commande::Aide {
        print!("{AIDE}");
        return 0;
    }
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("plateforme-violation : ENT_DATABASE_URL requis");
        return 1;
    };
    let ent = match ent_db::PgEnt::connect(&url).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("plateforme-violation : connexion : {e}");
            return 1;
        }
    };
    match executer(&ent, commande).await {
        Ok(i) => {
            if i.code == 0 {
                println!("{}", i.texte);
            } else {
                eprintln!("{}", i.texte);
            }
            i.code
        }
        Err(m) => {
            eprintln!("plateforme-violation : {m}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn aide_et_commandes_simples() {
        assert_eq!(lire(&[]).unwrap(), Commande::Aide);
        assert_eq!(lire(&a(&["--aide"])).unwrap(), Commande::Aide);
        assert_eq!(lire(&a(&["lister"])).unwrap(), Commande::Lister);
        assert!(lire(&a(&["lister", "x"])).is_err());
        assert_eq!(
            lire(&a(&["voir", "P-2027-002"])).unwrap(),
            Commande::Voir("P-2027-002".into())
        );
        assert!(lire(&a(&["supprimer", "P-2027-002"])).is_err());
    }

    #[test]
    fn declarer_exige_un_instant_avec_decalage_et_une_atteinte_connue() {
        let base = [
            "declarer",
            "--intitule",
            "Fuite chez l'hébergeur",
            "--description",
            "Sauvegarde exposée",
            "--connue-le",
            "2027-03-01T08:00:00+01:00",
            "--atteinte",
            "confidentialite,disponibilite",
            "--par",
            "Équipe sécurité",
        ];
        match lire(&a(&base)).unwrap() {
            Commande::Declarer {
                connue_le,
                confidentialite,
                integrite,
                disponibilite,
                couche_compte,
                ..
            } => {
                assert_eq!(connue_le.to_rfc3339(), "2027-03-01T07:00:00+00:00");
                assert!(confidentialite && !integrite && disponibilite && !couche_compte);
            }
            c => panic!("{c:?}"),
        }
        let mut avec = base.to_vec();
        avec.push("--couche-compte");
        assert!(matches!(
            lire(&a(&avec)).unwrap(),
            Commande::Declarer {
                couche_compte: true,
                ..
            }
        ));
        // Heure sans décalage : refusée (jamais d'heure ambiguë).
        let mut sans = base.to_vec();
        sans[6] = "2027-03-01T08:00:00";
        assert!(lire(&a(&sans)).unwrap_err().contains("décalage"));
        let mut inconnue = base.to_vec();
        inconnue[8] = "confidentialite,fuite";
        assert!(lire(&a(&inconnue)).is_err());
        // Option répétée ou inconnue : refusée.
        let mut repete = base.to_vec();
        repete.extend(["--par", "autre"]);
        assert!(lire(&a(&repete)).unwrap_err().contains("répété"));
        let mut inc = base.to_vec();
        inc.push("--forcer");
        assert!(lire(&a(&inc)).is_err());
    }

    #[test]
    fn propager_lit_la_confirmation_et_refuse_un_uuid_faux() {
        let id = "8d1f0b55-1b7a-4a59-9d2a-4c3b8f7e2a10";
        let c = lire(&a(&[
            "propager",
            "P-2027-002",
            id,
            "--elements",
            "12 comptes",
            "--personnes",
            "12",
        ]))
        .unwrap();
        assert_eq!(
            c,
            Commande::Propager {
                violation: "P-2027-002".into(),
                etablissement: Uuid::parse_str(id).unwrap(),
                elements: "12 comptes".into(),
                personnes: Some(12),
                confirmer: None,
            }
        );
        let c = lire(&a(&[
            "propager",
            "P-2027-002",
            id,
            "--elements",
            "x",
            "--confirmer",
            "lpnb",
        ]))
        .unwrap();
        assert!(matches!(c, Commande::Propager { confirmer: Some(ref s), .. } if s == "lpnb"));
        assert!(lire(&a(&[
            "propager",
            "P-2027-002",
            "pas-un-uuid",
            "--elements",
            "x"
        ]))
        .is_err());
        assert!(lire(&a(&[
            "propager",
            "P-2027-002",
            id,
            "--elements",
            "x",
            "--personnes",
            "-3"
        ]))
        .is_err());
        assert!(lire(&a(&["propager", "P-2027-002", id]))
            .unwrap_err()
            .contains("--elements"));
    }

    #[test]
    fn clore_exige_la_confirmation() {
        assert!(lire(&a(&["clore", "P-2027-002"]))
            .unwrap_err()
            .contains("--confirmer"));
        assert_eq!(
            lire(&a(&["clore", "P-2027-002", "--confirmer", "P-2027-002"])).unwrap(),
            Commande::Clore {
                violation: "P-2027-002".into(),
                confirmer: "P-2027-002".into()
            }
        );
        assert!(matches!(
            lire(&a(&[
                "notifier-cnil",
                "P-2027-002",
                "--le",
                "2027-03-02T10:00:00Z",
                "--reference",
                "CNIL-9"
            ]))
            .unwrap(),
            Commande::NotifierCnil {
                confirmer: None,
                ..
            }
        ));
        assert!(matches!(
            lire(&a(&[
                "notifier-cnil", "P-2027-002", "--le", "2027-03-02T10:00:00Z", "--reference", "CNIL-9", "--confirmer", "CNIL-9",
            ]))
            .unwrap(),
            Commande::NotifierCnil { confirmer: Some(c), .. } if c == "CNIL-9"
        ));
    }
}
