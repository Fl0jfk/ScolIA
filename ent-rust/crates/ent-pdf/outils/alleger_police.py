#!/usr/bin/env python3
"""Allège une police TrueType : ne garde que les tables utiles au moteur ent-pdf
(head hhea maxp hmtx cmap loca glyf post name OS/2 + cvt/fpgm/prep) et réduit `post`
au format 3 (sans noms de glyphes). Les glyphes ne sont PAS modifiés.
Usage : alleger_police.py entree.ttf sortie.ttf
Outil de développement ponctuel (non requis au build) ; seule la bibliothèque standard Python."""
import struct, sys

GARDER = [b"OS/2", b"cmap", b"cvt ", b"fpgm", b"glyf", b"head", b"hhea", b"hmtx", b"loca", b"maxp", b"name", b"post", b"prep"]

def somme(d):
    d += b"\0" * (-len(d) % 4)
    return sum(struct.unpack(">%dI" % (len(d) // 4), d)) & 0xFFFFFFFF

src = open(sys.argv[1], "rb").read()
n = struct.unpack(">H", src[4:6])[0]
tables = {}
for i in range(n):
    tag, _, off, ln = struct.unpack(">4sIII", src[12 + 16 * i:28 + 16 * i])
    tables[tag] = src[off:off + ln]
tables[b"post"] = tables[b"post"][:16] + b"\0" * 16
tables[b"post"] = struct.pack(">I", 0x00030000) + tables[b"post"][4:32]
tables[b"head"] = tables[b"head"][:8] + b"\0\0\0\0" + tables[b"head"][12:]
tags = sorted(t for t in GARDER if t in tables)
nt = len(tags)
p = 1
while p * 2 <= nt:
    p *= 2
out = struct.pack(">IHHHH", 0x00010000, nt, p * 16, p.bit_length() - 1, nt * 16 - p * 16)
off = 12 + 16 * nt
corps = b""
for t in tags:
    d = tables[t]
    out += struct.pack(">4sIII", t, somme(d), off + len(corps), len(d))
    corps += d + b"\0" * (-len(d) % 4)
fichier = bytearray(out + corps)
adj = (0xB1B0AFBA - somme(bytes(fichier))) & 0xFFFFFFFF
# checkSumAdjustment à l'offset 8 de `head`
for i in range(nt):
    if fichier[12 + 16 * i:16 + 16 * i] == b"head":
        o = struct.unpack(">I", fichier[20 + 16 * i:24 + 16 * i])[0]
        fichier[o + 8:o + 12] = struct.pack(">I", adj)
open(sys.argv[2], "wb").write(fichier)
