//! Commande `anonymiser` (lot R1) : produit, à partir d'un export du Next.js, un export **sans
//! donnée réelle** qui garde la structure, les volumes, les liens et les cas difficiles.
//!
//! Principe **fermé par défaut** : la liste blanche (`donnees/liste_blanche.tsv`) déclare, pour chaque
//! colonne de chaque table, « garder », « pseudonymiser avec la clé locale » ou « effacer ». Une table
//! ou une colonne non déclarée fait échouer l'outil **avant d'écrire quoi que ce soit**.

pub mod cle;
pub mod cle_s3;
pub mod controle;
pub mod json;
pub mod moteur;
pub mod pseudo;
pub mod rapport;
