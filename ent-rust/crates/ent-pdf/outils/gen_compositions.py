#!/usr/bin/env python3
"""Génère src/compositions.rs : table des compositions canoniques (base + marque → lettre
composée) pour le latin (Latin-1, étendu A/B, étendu additionnel dont le vietnamien), le grec
et le cyrillique, et classes de combinaison (ccc) des marques U+0300–U+036F.
Données : base Unicode de la bibliothèque standard Python (15.1). Outil de développement,
non requis au build ; le fichier généré est versionné."""
import unicodedata as u

PLAGES = [(0xC0, 0x24F), (0x370, 0x3FF), (0x400, 0x4FF), (0x1E00, 0x1EFF), (0x1F00, 0x1FFF)]
paires = []
for a, b in PLAGES:
    for c in range(a, b + 1):
        ch = chr(c)
        d = u.decomposition(ch)
        if not d or d.startswith("<"):
            continue
        p = d.split()
        if len(p) != 2:
            continue
        base, marque = int(p[0], 16), int(p[1], 16)
        if base > 0xFFFF or not (0x300 <= marque <= 0x36F):
            continue
        if u.normalize("NFC", chr(base) + chr(marque)) != ch:
            continue  # exclusion de composition
        paires.append((base, marque, c))
paires.sort()
ccc = [(m, u.combining(chr(m))) for m in range(0x300, 0x370)]
with open("src/compositions.rs", "w") as f:
    f.write("//! GÉNÉRÉ par outils/gen_compositions.py (Unicode %s) — ne pas modifier à la main.\n" % u.unidata_version)
    f.write("//! (base, marque combinante, lettre composée), trié par (base, marque).\n\n")
    f.write("#[rustfmt::skip]\npub(crate) static COMPOSITIONS: [(u16, u16, u16); %d] = [\n" % len(paires))
    for i in range(0, len(paires), 4):
        f.write("    " + " ".join("(0x%04X, 0x%04X, 0x%04X)," % p for p in paires[i:i + 4]) + "\n")
    f.write("];\n\n")
    f.write("/// Classe de combinaison canonique de U+0300 + index (0 = pas de réordonnancement).\n")
    f.write("#[rustfmt::skip]\npub(crate) static CCC: [u8; 112] = [\n")
    for i in range(0, 112, 16):
        f.write("    " + " ".join("%d," % c for _, c in ccc[i:i + 16]) + "\n")
    f.write("];\n")
print(len(paires))
