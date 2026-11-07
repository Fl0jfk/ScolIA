//! Registre des migrations : chaque brique déclare ses migrations et sa plage,
//! le registre les assemble dans l'ordre global des numéros et calcule
//! `MIGRATION_TAG`. Plus aucune liste ni constante tenue à la main.

use std::collections::BTreeMap;
use std::fmt;

include!(concat!(env!("OUT_DIR"), "/migrations_noyau.rs"));

/// Une migration SQL, incluse dans le binaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    pub numero: u16,
    /// Nom sans extension (`255_email_confirmation`).
    pub nom: &'static str,
    pub sql: &'static str,
    /// Brique propriétaire (`noyau` pour le répertoire historique).
    pub brique: &'static str,
}

/// Plage de numéros réservée à une brique (bornes incluses).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plage {
    pub debut: u16,
    pub fin: u16,
}

impl Plage {
    pub const fn new(debut: u16, fin: u16) -> Self {
        Self { debut, fin }
    }
    pub fn contient(&self, n: u16) -> bool {
        (self.debut..=self.fin).contains(&n)
    }
    pub fn chevauche(&self, autre: &Plage) -> bool {
        self.debut <= autre.fin && autre.debut <= self.fin
    }
}

/// Ce qu'une brique déclare pour ses migrations.
#[derive(Debug, Clone, Copy)]
pub struct DeclMigrations {
    pub brique: &'static str,
    pub plages: &'static [Plage],
    pub migrations: &'static [Migration],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurMigration {
    HorsPlage {
        brique: &'static str,
        nom: &'static str,
    },
    MauvaisProprietaire {
        brique: &'static str,
        nom: &'static str,
        declare: &'static str,
    },
    PlagesChevauchees {
        a: &'static str,
        b: &'static str,
    },
    NumeroEnDouble {
        numero: u16,
        a: &'static str,
        b: &'static str,
    },
}

impl fmt::Display for ErreurMigration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HorsPlage { brique, nom } => {
                write!(
                    f,
                    "{nom} est hors des plages réservées à la brique {brique}"
                )
            }
            Self::MauvaisProprietaire {
                brique,
                nom,
                declare,
            } => write!(
                f,
                "{nom} est déclarée par {brique} mais porte la brique {declare}"
            ),
            Self::PlagesChevauchees { a, b } => {
                write!(f, "les plages de {a} et de {b} se chevauchent")
            }
            Self::NumeroEnDouble { numero, a, b } => {
                write!(f, "le numéro {numero:03} est pris par {a} et par {b}")
            }
        }
    }
}

/// Assemble les déclarations de toutes les briques : contrôle des plages, des
/// propriétaires et des doublons, puis ordre global par numéro (c'est l'ordre
/// d'application, identique à celui de la liste historique de `migrate.rs`).
pub fn assembler(decls: &[DeclMigrations]) -> Result<Vec<Migration>, Vec<ErreurMigration>> {
    let mut erreurs = Vec::new();
    for (i, a) in decls.iter().enumerate() {
        for b in &decls[i + 1..] {
            if a.plages
                .iter()
                .any(|pa| b.plages.iter().any(|pb| pa.chevauche(pb)))
            {
                erreurs.push(ErreurMigration::PlagesChevauchees {
                    a: a.brique,
                    b: b.brique,
                });
            }
        }
    }
    let mut par_numero: BTreeMap<u16, Migration> = BTreeMap::new();
    for d in decls {
        for m in d.migrations {
            if m.brique != d.brique {
                erreurs.push(ErreurMigration::MauvaisProprietaire {
                    brique: d.brique,
                    nom: m.nom,
                    declare: m.brique,
                });
            }
            if !d.plages.iter().any(|p| p.contient(m.numero)) {
                erreurs.push(ErreurMigration::HorsPlage {
                    brique: d.brique,
                    nom: m.nom,
                });
            }
            if let Some(deja) = par_numero.insert(m.numero, *m) {
                erreurs.push(ErreurMigration::NumeroEnDouble {
                    numero: m.numero,
                    a: deja.nom,
                    b: m.nom,
                });
            }
        }
    }
    if erreurs.is_empty() {
        Ok(par_numero.into_values().collect())
    } else {
        Err(erreurs)
    }
}

/// `MIGRATION_TAG` calculé : nom de la migration de plus haut numéro.
pub fn tag(migrations: &[Migration]) -> Option<&'static str> {
    migrations.iter().max_by_key(|m| m.numero).map(|m| m.nom)
}

/// Garde F-009 : contraintes nommées définies (ou redéfinies) dans plus d'une
/// migration. Tant que toutes les migrations sont rejouées à chaque démarrage,
/// la dernière redéfinition rejouée gagne : une contrainte élargie plus loin
/// serait rétrécie par le rejeu d'une migration plus ancienne.
/// Renvoie `nom de contrainte → migrations` pour les noms vus plusieurs fois.
pub fn contraintes_redefinies(migrations: &[Migration]) -> BTreeMap<String, Vec<&'static str>> {
    let mut vues: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    for m in migrations {
        for nom in noms_de_contraintes(m.sql) {
            let liste = vues.entry(nom).or_default();
            if !liste.contains(&m.nom) {
                liste.push(m.nom);
            }
        }
    }
    vues.retain(|_, l| l.len() > 1);
    vues
}

/// Noms qui suivent le mot `CONSTRAINT`, sauf `DROP CONSTRAINT [IF EXISTS]`.
/// Les commentaires de ligne `--` sont ignorés.
fn noms_de_contraintes(sql: &str) -> Vec<String> {
    let mut noms = Vec::new();
    for ligne in sql.lines() {
        let code = ligne.split("--").next().unwrap_or("").to_ascii_lowercase();
        let mots: Vec<&str> = code
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .filter(|m| !m.is_empty())
            .collect();
        for (i, mot) in mots.iter().enumerate() {
            if *mot != "constraint" || i + 1 >= mots.len() {
                continue;
            }
            if i > 0 && mots[i - 1] == "drop" {
                continue;
            }
            noms.push(mots[i + 1].to_string());
        }
    }
    noms
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn m(numero: u16, nom: &'static str, brique: &'static str) -> Migration {
        Migration {
            numero,
            nom,
            sql: "",
            brique,
        }
    }

    static NOYAU: &[Migration] = &[m(1, "001_socle", "noyau"), m(255, "255_email", "noyau")];
    static SANTE: &[Migration] = &[m(210, "210_sante", "sante"), m(216, "216_module", "sante")];
    static INFIRMERIE: &[Migration] = &[m(400, "400_passages", "infirmerie")];

    fn decl(
        brique: &'static str,
        plages: &'static [Plage],
        migrations: &'static [Migration],
    ) -> DeclMigrations {
        DeclMigrations {
            brique,
            plages,
            migrations,
        }
    }

    #[test]
    fn ordre_global_par_numero_et_tag_calcule() {
        let liste = assembler(&[
            decl("infirmerie", const { &[Plage::new(400, 409)] }, INFIRMERIE),
            decl(
                "noyau",
                const { &[Plage::new(1, 134), Plage::new(250, 259)] },
                NOYAU,
            ),
            decl("sante", const { &[Plage::new(210, 219)] }, SANTE),
        ])
        .expect("assemblage valide");
        let noms: Vec<_> = liste.iter().map(|m| m.nom).collect();
        assert_eq!(
            noms,
            [
                "001_socle",
                "210_sante",
                "216_module",
                "255_email",
                "400_passages"
            ]
        );
        assert_eq!(tag(&liste), Some("400_passages"));
    }

    #[test]
    fn refuse_hors_plage_doublon_et_chevauchement() {
        static VOLEUSE: &[Migration] = &[m(216, "216_vol", "cantine")];
        let erreurs = assembler(&[
            decl("sante", const { &[Plage::new(210, 219)] }, SANTE),
            decl("cantine", const { &[Plage::new(215, 229)] }, VOLEUSE),
        ])
        .unwrap_err();
        assert!(erreurs.contains(&ErreurMigration::PlagesChevauchees {
            a: "sante",
            b: "cantine"
        }));
        assert!(erreurs.contains(&ErreurMigration::NumeroEnDouble {
            numero: 216,
            a: "216_module",
            b: "216_vol"
        }));

        let erreurs =
            assembler(&[decl("sante", const { &[Plage::new(210, 214)] }, SANTE)]).unwrap_err();
        assert_eq!(
            erreurs,
            [ErreurMigration::HorsPlage {
                brique: "sante",
                nom: "216_module"
            }]
        );

        let erreurs =
            assembler(&[decl("cantine", const { &[Plage::new(210, 219)] }, SANTE)]).unwrap_err();
        assert_eq!(
            erreurs.len(),
            2,
            "deux migrations déclarées sous le mauvais propriétaire"
        );
    }

    #[test]
    fn detecte_une_contrainte_redefinie() {
        static A: &[Migration] = &[
            Migration {
                numero: 92,
                nom: "092_ref",
                sql: "CREATE TABLE t (s TEXT,\n  CONSTRAINT t_source_chk CHECK (s IN ('a')));",
                brique: "noyau",
            },
            Migration {
                numero: 110,
                nom: "110_ref",
                sql: "ALTER TABLE t DROP CONSTRAINT IF EXISTS t_source_chk;\n\
                      ALTER TABLE t ADD CONSTRAINT t_source_chk CHECK (s IN ('a','b')); -- CONSTRAINT commentaire_ignore",
                brique: "noyau",
            },
        ];
        let r = contraintes_redefinies(A);
        assert_eq!(r.len(), 1);
        assert_eq!(r["t_source_chk"], ["092_ref", "110_ref"]);
    }
}
