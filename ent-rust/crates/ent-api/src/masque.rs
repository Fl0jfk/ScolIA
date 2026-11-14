//! `Debug` à champs masqués pour les types qui portent un mot de passe, un
//! jeton, un secret ou une identité de connexion : un `{:?}` (journal, panic,
//! `dbg!`) ne doit jamais les écrire.
//!
//! ```ignore
//! debug_masque!(LoginBody { etablissement_id } masque { email, password });
//! ```
//! Les champs « visibles » sont affichés ; les « masqués » apparaissent comme
//! `"[masqué]"` ; les champs non listés sont omis (`..`).

macro_rules! debug_masque {
    ($ty:ident { $($visible:ident),* $(,)? } masque { $($masque:ident),* $(,)? }) => {
        impl ::std::fmt::Debug for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!($ty))
                    $(.field(stringify!($visible), &self.$visible))*
                    $(.field(stringify!($masque), &"[masqué]"))*
                    .finish_non_exhaustive()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[allow(dead_code)]
    struct Exemple {
        id: u32,
        secret: String,
    }
    debug_masque!(Exemple { id } masque { secret });

    #[test]
    fn debug_masque_cache_les_champs_secrets() {
        let e = Exemple {
            id: 7,
            secret: "hunter2-très-secret".into(),
        };
        let s = format!("{e:?}");
        assert!(s.contains("id: 7"));
        assert!(s.contains("[masqué]"));
        assert!(!s.contains("hunter2"));
    }
}
