/**
 * Fixtures PNG pour tests (hors production).
 * Le marqueur d’attestation est un PNG 1×1 blanc ; la signature « dessinée » de test est un PNG 2×2 distinct.
 */

/** PNG 2×2 — valide pour `parseSignaturePngDataUrl`, distinct du marqueur 1×1. */
export const SST_DRAWN_SIGNATURE_TEST_DATA_URL =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAAAdgausAAAAEklEQVQImWNgYGD4DwABBAEAAP9/Af4B/wAAAABJRU5ErkJggg==";
