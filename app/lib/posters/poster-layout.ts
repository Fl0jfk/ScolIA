import type { PosterFormat } from "@/app/lib/posters/types";

/** Points PDF (1 pt = 1/72"). */
export function pageSizePt(format: PosterFormat): { widthPt: number; heightPt: number } {
  if (format === "a3-landscape") {
    return { widthPt: 1190.55, heightPt: 841.89 };
  }
  if (format === "a5-portrait") {
    return { widthPt: 419.53, heightPt: 595.28 };
  }
  return { widthPt: 595.28, heightPt: 841.89 };
}

/** Page physique d’export (A4 pour planche A5×4). */
export function exportSheetSizePt(format: PosterFormat): {
  widthPt: number;
  heightPt: number;
  tiles: number;
} {
  if (format === "a5-portrait") {
    return { widthPt: 595.28, heightPt: 841.89, tiles: 4 };
  }
  const page = pageSizePt(format);
  return { ...page, tiles: 1 };
}
