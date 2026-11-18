#!/usr/bin/env python3
"""Génère les fichiers réels des tests de la messagerie (lots M5 et M6).

Chaque fichier est produit par une vraie bibliothèque (reportlab, pypdf, Pillow, python-docx), pas écrit à la main
octet par octet, sauf un cas signalé : le DOCX « à macro » (un vrai DOCX auquel on ajoute `vbaProject.bin`). L'exécutable ELF est
compilé par gcc (voir la fin du script). Rejouable : `python3 generer.py` dans ce dossier.
Aucune donnée réelle : noms et textes de fiction.
"""
import io, struct, zipfile
from PIL import Image, ImageDraw
from pypdf import PdfReader, PdfWriter
from reportlab.lib.pagesizes import A4
from reportlab.pdfgen import canvas
import docx

# PDF réel (texte sur deux pages), flux non compressés en objets (pas de flux d'objets : désarmable).
def pdf_reel(nom, titre):
    c = canvas.Canvas(nom, pagesize=A4, pageCompression=0)
    c.setTitle(titre)
    c.drawString(72, 780, titre)
    c.drawString(72, 760, "Document de fiction pour les tests de la messagerie ScolIA.")
    c.showPage()
    c.drawString(72, 780, "Page 2")
    c.save()

pdf_reel("certificat_medical.pdf", "Certificat de fiction")

# PDF chiffré (mot de passe) par pypdf.
w = PdfWriter(clone_from=PdfReader("certificat_medical.pdf"))
w.encrypt("secret-de-test", algorithm="AES-128")
with open("pdf_chiffre.pdf", "wb") as f:
    w.write(f)

# PDF à contenu actif : JavaScript déclaré par pypdf.
w = PdfWriter(clone_from=PdfReader("certificat_medical.pdf"))
w.add_js("app.alert('contenu actif');")
with open("pdf_javascript.pdf", "wb") as f:
    w.write(f)

# Photo JPEG réelle avec EXIF : orientation, appareil et position GPS (Saint-Denis de La Réunion, fiction).
img = Image.new("RGB", (96, 64), (200, 220, 240))
d = ImageDraw.Draw(img)
d.rectangle([8, 8, 88, 56], outline=(10, 60, 120), width=3)
d.text((16, 24), "photo", fill=(10, 60, 120))
exif = Image.Exif()
exif[0x0112] = 6                      # orientation
exif[0x010F] = "Fabricant de fiction"  # Make
gps = {1: "S", 2: (20.0, 52.0, 40.0), 3: "E", 4: (55.0, 27.0, 0.0)}
exif[0x8825] = gps
img.save("photo_gps.jpg", "JPEG", exif=exif, quality=85)

# PNG réel avec bloc texte (tEXt) et date (tIME) : métadonnées à retirer.
from PIL.PngImagePlugin import PngInfo
info = PngInfo()
info.add_text("Comment", "capture-de-fiction-localisation-secrete")
img.save("capture.png", "PNG", pnginfo=info)

# DOCX réel (python-docx), puis la même chose avec une macro VBA ajoutée (hand-made : seul le conteneur est vrai).
doc = docx.Document()
doc.add_heading("Lettre de fiction", 1)
doc.add_paragraph("Bonjour, ceci est un document de test pour la messagerie ScolIA.")
doc.save("lettre.docx")
with zipfile.ZipFile("lettre.docx") as src, zipfile.ZipFile("lettre_macro.docx", "w", zipfile.ZIP_DEFLATED) as dst:
    for n in src.namelist():
        dst.writestr(n, src.read(n))
    dst.writestr("word/vbaProject.bin", b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1" + b"macro de fiction")

# Texte brut.
open("note.txt", "w", encoding="utf-8").write("Rendez-vous lundi à 8 h devant la salle 12.\n")

# Archive ZIP réelle (refusée en pièce jointe : les archives ne sont pas admises).
with zipfile.ZipFile("archive.zip", "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("a.txt", "contenu")

# Exécutable ELF réel (refusé à l'entrée), compilé hors de ce script :
#   gcc -static -nostdlib -Os -s -Wl,--build-id=none -o executable.elf t.c
# avec `void _start(void){ __asm__ volatile("mov $60,%eax; mov $42,%edi; syscall"); }` (exit(42) : il s'exécute).
