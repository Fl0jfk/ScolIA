# ent-pdf — moteur PDF maison (D-062, socle 7, lot D1)

Aucune dépendance (`Cargo.lock` ne gagne que l'entrée `ent-pdf`). Aucune cryptographie.

| Brique | Fichier | Contenu |
|---|---|---|
| Encodage | `winansi.rs` | Unicode → WinAnsi (CP1252), NFD recomposé, repli lisible (`ł`→`l`, `→`→`->`, `4ᵉ`→`4e`), `?` en dernier recours |
| Mesure | `afm.rs` | Largeurs AFM de Helvetica, Helvetica-Bold, Times-Roman (codes 0x20–0xFF) |
| Mise en page | `doc.rs` | Pages A4, en-tête/pied répétés, « Page n sur N », paragraphes, alignements, tableaux à en-tête répété (rangée plus haute qu'une page coupée), images, QR |
| JPEG | `jpeg.rs` | Lecture de l'en-tête, `DCTDecode` sans décodage (gris/RVB, pas de CMJN) |
| QR | `qr.rs` | ISO 18004, octet + alphanumérique, niveau M, versions 1–10, 8 masques à pénalités, dessin vectoriel |
| Fichier | `ecrit.rs` | Objets, xref exacte, trailer, `startxref` ; métadonnées (UTF-16BE si accents) |
| Relecture | `releve.rs` | Contrôle de structure + texte décodé, pour les tests des consommateurs |

Flux **non compressés** : aucun compresseur n'est dans le graphe de dépendances.

Vérification locale facultative (jamais une dépendance) :
`ENT_PDF_SORTIE=/dossier cargo test -p ent-pdf sortie_locale` puis `pdfinfo`, `pdftotext`, `qpdf --check`.

Limites connues : pas de césure, pas de justification, pas de crénage ; polices non incorporées
(standard 14) ; la mesure des glyphes hors ASCII vient des AFM saisis à la main (la mise en
page seule en dépend, pas la lisibilité) ; QR limité à 213 octets (10-M).
