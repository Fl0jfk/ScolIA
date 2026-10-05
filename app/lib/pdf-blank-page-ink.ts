/**
 * Détection prudente de pages blanches (scans recto-verso).
 * Pas d’OCR / IA : mesure du taux d’encre sur une rasterisation.
 *
 * Objectif : ne retirer que les versos vraiment vides ; en cas de doute, on garde.
 */

export type BlankPageInkStats = {
  sampledPixels: number;
  softInkPixels: number;
  darkInkPixels: number;
  softInkRatio: number;
  darkInkPixelsRatio: number;
};

export type BlankPageDetectOptions = {
  /** Marge ignorée (trous de perforatrice, bords scanner). */
  marginFraction: number;
  /** Luminance sous laquelle un pixel n’est plus « presque blanc ». */
  nearWhiteLuma: number;
  /** Luminance sous laquelle un pixel compte comme encre réelle. */
  darkLuma: number;
  /** Ratio max d’encre douce pour considérer la page vide. */
  maxSoftInkRatio: number;
  /** Ratio max d’encre sombre pour considérer la page vide. */
  maxDarkInkRatio: number;
};

/** Seuils prudents : mieux laisser une page blanche que supprimer une info. */
export const BLANK_PAGE_DETECT_DEFAULTS: BlankPageDetectOptions = {
  marginFraction: 0.05,
  nearWhiteLuma: 242,
  darkLuma: 200,
  maxSoftInkRatio: 0.001,
  maxDarkInkRatio: 0.0003,
};

function clamp01(n: number): number {
  if (!Number.isFinite(n)) return 0;
  if (n < 0) return 0;
  if (n > 1) return 1;
  return n;
}

export function resolveBlankPageDetectOptions(
  overrides?: Partial<BlankPageDetectOptions>,
): BlankPageDetectOptions {
  return {
    marginFraction: clamp01(overrides?.marginFraction ?? BLANK_PAGE_DETECT_DEFAULTS.marginFraction),
    nearWhiteLuma: Math.min(
      255,
      Math.max(0, overrides?.nearWhiteLuma ?? BLANK_PAGE_DETECT_DEFAULTS.nearWhiteLuma),
    ),
    darkLuma: Math.min(
      255,
      Math.max(0, overrides?.darkLuma ?? BLANK_PAGE_DETECT_DEFAULTS.darkLuma),
    ),
    maxSoftInkRatio: clamp01(
      overrides?.maxSoftInkRatio ?? BLANK_PAGE_DETECT_DEFAULTS.maxSoftInkRatio,
    ),
    maxDarkInkRatio: clamp01(
      overrides?.maxDarkInkRatio ?? BLANK_PAGE_DETECT_DEFAULTS.maxDarkInkRatio,
    ),
  };
}

/**
 * Calcule les ratios d’encre sur un buffer RGBA (stride 4).
 * Ignore une marge périphérique pour le bruit scanner / perforations.
 */
export function computeInkStatsFromRgba(
  rgba: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
  overrides?: Partial<BlankPageDetectOptions>,
): BlankPageInkStats {
  const opts = resolveBlankPageDetectOptions(overrides);
  const w = Math.max(0, Math.floor(width));
  const h = Math.max(0, Math.floor(height));
  if (w < 8 || h < 8 || rgba.length < w * h * 4) {
    return {
      sampledPixels: 0,
      softInkPixels: 0,
      darkInkPixels: 0,
      softInkRatio: 0,
      darkInkPixelsRatio: 0,
    };
  }

  const x0 = Math.floor(w * opts.marginFraction);
  const x1 = Math.ceil(w * (1 - opts.marginFraction));
  const y0 = Math.floor(h * opts.marginFraction);
  const y1 = Math.ceil(h * (1 - opts.marginFraction));

  let sampled = 0;
  let soft = 0;
  let dark = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const i = (y * w + x) * 4;
      const r = rgba[i] ?? 255;
      const g = rgba[i + 1] ?? 255;
      const b = rgba[i + 2] ?? 255;
      const luma = 0.299 * r + 0.587 * g + 0.114 * b;
      sampled++;
      if (luma < opts.nearWhiteLuma) soft++;
      if (luma < opts.darkLuma) dark++;
    }
  }

  return {
    sampledPixels: sampled,
    softInkPixels: soft,
    darkInkPixels: dark,
    softInkRatio: sampled > 0 ? soft / sampled : 0,
    darkInkPixelsRatio: sampled > 0 ? dark / sampled : 0,
  };
}

/** Page blanche uniquement si encre douce ET sombre sont sous les seuils. */
export function isBlankPageFromInkStats(
  stats: BlankPageInkStats,
  overrides?: Partial<BlankPageDetectOptions>,
): boolean {
  if (stats.sampledPixels < 1000) return false;
  const opts = resolveBlankPageDetectOptions(overrides);
  return (
    stats.darkInkPixelsRatio <= opts.maxDarkInkRatio &&
    stats.softInkRatio <= opts.maxSoftInkRatio
  );
}
