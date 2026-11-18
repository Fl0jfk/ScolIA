#!/usr/bin/env python3
"""Compresse les captures PNG (palette adaptative 96 couleurs, sans perte visible sur une interface plate).
Usage : python3 compresser.py <dossier>"""
import sys, pathlib
from PIL import Image
avant = apres = 0
for f in pathlib.Path(sys.argv[1]).rglob('*.png'):
    avant += f.stat().st_size
    im = Image.open(f).convert('RGB')
    im.quantize(colors=96, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(f, optimize=True)
    apres += f.stat().st_size
print(f'{avant/1e6:.1f} Mo -> {apres/1e6:.1f} Mo')
