//! Sous-ensemble TrueType : ne garde que les glyphes utilisés (composants des glyphes
//! composites compris) et réécrit `head`, `hhea`, `maxp`, `hmtx`, `loca`, `glyf` (+ `cvt `,
//! `fpgm`, `prep` recopiées, exigées par la spécification PDF pour FontFile2).
//! Les glyphes sont renumérotés : le glyphe n° `i` de la liste `principaux` devient le glyphe
//! `i` du sous-ensemble (CID = GID, `CIDToGIDMap /Identity`) ; les composants absents de la
//! liste sont ajoutés à la suite.

use crate::ttf::Ttf;
use std::collections::HashMap;

fn somme(d: &[u8]) -> u32 {
    let mut s: u32 = 0;
    for c in d.chunks(4) {
        let mut b = [0u8; 4];
        b[..c.len()].copy_from_slice(c);
        s = s.wrapping_add(u32::from_be_bytes(b));
    }
    s
}

fn aligner(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(4) {
        v.push(0);
    }
}

pub(crate) struct Sortie {
    pub octets: Vec<u8>,
}

/// `principaux[0]` doit être le glyphe 0 (.notdef). Les identifiants doivent être uniques.
pub(crate) fn sous_ensemble(ttf: &Ttf, principaux: &[u16]) -> Sortie {
    let mut anciens: Vec<u16> = principaux.to_vec();
    let mut nouveau: HashMap<u16, u16> = anciens
        .iter()
        .enumerate()
        .map(|(i, &g)| (g, i as u16))
        .collect();
    // Fermeture par les composants (la liste grandit pendant le parcours).
    let mut i = 0;
    while i < anciens.len() {
        for (_, c) in Ttf::composants(ttf.glyphe(anciens[i])) {
            if let std::collections::hash_map::Entry::Vacant(v) = nouveau.entry(c) {
                v.insert(anciens.len() as u16);
                anciens.push(c);
            }
        }
        i += 1;
    }
    let n = anciens.len();
    let mut glyf: Vec<u8> = Vec::new();
    let mut loca: Vec<u8> = Vec::with_capacity(4 * (n + 1));
    let mut hmtx: Vec<u8> = Vec::with_capacity(4 * n);
    for &g in &anciens {
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        let mut d = ttf.glyphe(g).to_vec();
        for (pos, c) in Ttf::composants(&d) {
            d[pos..pos + 2].copy_from_slice(&nouveau[&c].to_be_bytes());
        }
        glyf.extend_from_slice(&d);
        aligner(&mut glyf);
        let (av, lsb) = ttf.metrique(g);
        hmtx.extend_from_slice(&av.to_be_bytes());
        hmtx.extend_from_slice(&lsb.to_be_bytes());
    }
    loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());

    let mut head = ttf.table(b"head").unwrap().to_vec();
    head[8..12].copy_from_slice(&[0; 4]);
    head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut hhea = ttf.table(b"hhea").unwrap().to_vec();
    hhea[34..36].copy_from_slice(&(n as u16).to_be_bytes());
    let mut maxp = ttf.table(b"maxp").unwrap().to_vec();
    maxp[4..6].copy_from_slice(&(n as u16).to_be_bytes());

    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"cvt ", ttf.table(b"cvt ").unwrap_or(&[]).to_vec()),
        (b"fpgm", ttf.table(b"fpgm").unwrap_or(&[]).to_vec()),
        (b"glyf", glyf),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"loca", loca),
        (b"maxp", maxp),
        (b"prep", ttf.table(b"prep").unwrap_or(&[]).to_vec()),
    ];
    tables.retain(|(_, d)| !d.is_empty());
    let nt = tables.len() as u16;
    let mut p = 1u16;
    let mut log = 0u16;
    while p * 2 <= nt {
        p *= 2;
        log += 1;
    }
    let mut sortie = Vec::new();
    sortie.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    sortie.extend_from_slice(&nt.to_be_bytes());
    sortie.extend_from_slice(&(p * 16).to_be_bytes());
    sortie.extend_from_slice(&log.to_be_bytes());
    sortie.extend_from_slice(&(nt * 16 - p * 16).to_be_bytes());
    let mut off = 12 + 16 * tables.len();
    let mut corps = Vec::new();
    let mut pos_head = 0;
    for (tag, d) in &tables {
        sortie.extend_from_slice(*tag);
        sortie.extend_from_slice(&somme(d).to_be_bytes());
        sortie.extend_from_slice(&(off as u32).to_be_bytes());
        sortie.extend_from_slice(&(d.len() as u32).to_be_bytes());
        if *tag == b"head" {
            pos_head = off;
        }
        corps.extend_from_slice(d);
        aligner(&mut corps);
        off = 12 + 16 * tables.len() + corps.len();
    }
    sortie.extend_from_slice(&corps);
    let ajust = 0xB1B0_AFBAu32.wrapping_sub(somme(&sortie));
    sortie[pos_head + 8..pos_head + 12].copy_from_slice(&ajust.to_be_bytes());
    Sortie { octets: sortie }
}
