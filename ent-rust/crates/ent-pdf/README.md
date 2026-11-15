# ent-pdf — moteur PDF maison (D-062, socle 7, lots D1 et D1b)

Aucune dépendance (`Cargo.lock` ne gagne que l'entrée `ent-pdf`). Aucune cryptographie.

| Brique | Fichier | Contenu |
|---|---|---|
| Police incorporée (D1b) | `ttf.rs`, `sous_ensemble.rs`, `fonte.rs` | Lecteur TrueType à la main (`head`, `hhea`, `maxp`, `hmtx`, `cmap` 4 et 12, `loca`, `glyf`, `post`, `name`, `OS/2`), sous-ensemble (glyphes utilisés + composants, CID = GID), police CID Type 2 `Identity-H` avec `/W`, `CIDToGIDMap /Identity` et `ToUnicode` (texte copiable, recherchable) |
| Normalisation (D1b) | `normalise.rs`, `compositions.rs` | NFC maison (réordonnancement des marques puis composition) latin, grec, cyrillique, vietnamien ; table générée par `outils/gen_compositions.py` |
| Encodage (repli) | `winansi.rs` | Unicode → WinAnsi (CP1252), NFD recomposé, repli lisible (`ł`→`l`, `→`→`->`, `4ᵉ`→`4e`), `?` en dernier recours |
| Mesure | `afm.rs` | Largeurs AFM de Helvetica, Helvetica-Bold, Times-Roman (codes 0x20–0xFF) |
| Mise en page | `doc.rs` | Pages A4, en-tête/pied répétés, « Page n sur N », paragraphes, alignements, tableaux à en-tête répété (rangée plus haute qu'une page coupée), images, QR |
| JPEG | `jpeg.rs` | Lecture de l'en-tête, `DCTDecode` sans décodage (gris/RVB, pas de CMJN) |
| QR | `qr.rs` | ISO 18004, octet + alphanumérique, niveau M, versions 1–10, 8 masques à pénalités, dessin vectoriel |
| Fichier | `ecrit.rs` | Objets, xref exacte, trailer, `startxref` ; métadonnées (UTF-16BE si accents) |
| Relecture | `releve.rs` | Contrôle de structure + texte décodé, pour les tests des consommateurs |

**Polices** : `polices/DejaVuSans.ttf` et `DejaVuSans-Bold.ttf` (DejaVu 2.37, licence Bitstream Vera, texte dans
`polices/LICENCE-DejaVu.txt`), allégées par `outils/alleger_police.py` (GPOS, GSUB, kern… retirés ; glyphes intacts ;
≈ 1,2 Mo pour les deux, compilés dans le binaire). Elles sont des **actifs**, pas des crates. Mode par défaut de
`Document::new` : police incorporée ; `Document::avec_polices_standard()` = repli Helvetica/Times en WinAnsi.
Un caractère absent de la police sort translittéré (accent retiré) ou en glyphe de remplacement U+FFFD, jamais en « ? »
silencieux : `Document::octets_avec_avertissements()` rend la liste (codes seuls, pas de donnée nominative) et
`octets()` l'écrit sur la sortie d'erreur.

Flux **non compressés** : aucun compresseur n'est dans le graphe de dépendances.

Vérification locale facultative (jamais une dépendance) :
`ENT_PDF_SORTIE=/dossier cargo test -p ent-pdf sortie_locale` puis `pdfinfo`, `pdftotext`, `pdffonts`, `qpdf --check` (les tests `outils_externes_si_presents` les utilisent s'ils existent).

Limites connues : pas de césure, pas de justification, pas de crénage ; pas de scripts à
lettres liées ni de droite-à-gauche (arabe, hébreu : glyphes isolés, non reliés), pas de CJK (glyphe U+FFFD +
avertissement), `Police::Times` rendue en DejaVu Sans en mode incorporé, pas de crénage (GPOS retirée), marques
combinantes sans composition posées par la police sans repositionnement ; en mode standard la mesure des glyphes
hors ASCII vient des AFM saisis à la main ; QR limité à 213 octets (10-M).
