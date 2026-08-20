import { clampPosterOffsets } from "@/app/lib/posters/catalog";
import type {
  ComputedPosterLayout,
  PosterBox,
  PosterDraft,
  PosterFormat,
  PosterLogoSize,
  PosterTitleSize,
} from "@/app/lib/posters/types";

/** Points PDF (1 pt = 1/72"). */
export function pageSizePt(format: PosterFormat): { widthPt: number; heightPt: number } {
  if (format === "a3-landscape") {
    return { widthPt: 1190.55, heightPt: 841.89 };
  }
  return { widthPt: 595.28, heightPt: 841.89 };
}

function logoFrac(size: PosterLogoSize): number {
  if (size === "S") return 0.1;
  if (size === "L") return 0.16;
  return 0.13;
}

function titlePt(size: PosterTitleSize, format: PosterFormat): number {
  const base = format === "a3-landscape" ? 36 : 28;
  if (size === "S") return base * 0.75;
  if (size === "L") return base * 1.15;
  return base;
}

function shiftBox(box: PosterBox, dx: number, dy: number): PosterBox {
  return {
    x: Math.min(0.95, Math.max(0.02, box.x + dx)),
    y: Math.min(0.95, Math.max(0.02, box.y + dy)),
    w: box.w,
    h: box.h,
  };
}

function scaleBoxAroundCenter(box: PosterBox, scale: number): PosterBox {
  const cx = box.x + box.w / 2;
  const cy = box.y + box.h / 2;
  const w = box.w * scale;
  const h = box.h * scale;
  return {
    x: cx - w / 2,
    y: cy - h / 2,
    w,
    h,
  };
}

/**
 * Placement unique preview + PDF.
 * Coordonnées normalisées 0–1, origine haut-gauche.
 */
export function computePosterLayout(draft: PosterDraft): ComputedPosterLayout {
  const page = pageSizePt(draft.format);
  const offsets = clampPosterOffsets(draft.offsets);
  const schoolW = logoFrac(draft.logoSchoolSize);
  const partnerW = logoFrac(draft.logoPartnerSize);
  const logoH = 0.1;

  let logoSchool: PosterBox;
  let logoPartner: PosterBox | null = null;
  let title: PosterBox;
  let subtitle: PosterBox;
  let body: PosterBox;
  let accentBar: PosterBox;

  const preset = draft.layoutPreset;

  if (preset === "partner-right") {
    logoSchool = { x: 0.06, y: 0.06, w: schoolW, h: logoH };
    logoPartner = draft.partnerLogoKey
      ? { x: 0.94 - partnerW, y: 0.06, w: partnerW, h: logoH }
      : null;
    title = { x: 0.08, y: 0.28, w: 0.55, h: 0.12 };
    subtitle = { x: 0.08, y: 0.42, w: 0.55, h: 0.06 };
    body = { x: 0.08, y: 0.52, w: 0.55, h: 0.22 };
    accentBar = { x: 0, y: 0.22, w: 1, h: 0.012 };
  } else if (preset === "photo-full") {
    logoSchool = { x: 0.06, y: 0.05, w: schoolW, h: logoH };
    logoPartner = draft.partnerLogoKey
      ? { x: 0.94 - partnerW, y: 0.05, w: partnerW, h: logoH }
      : null;
    title = { x: 0.08, y: 0.38, w: 0.84, h: 0.14 };
    subtitle = { x: 0.1, y: 0.54, w: 0.8, h: 0.06 };
    body = { x: 0.12, y: 0.64, w: 0.76, h: 0.16 };
    accentBar = { x: 0.35, y: 0.52, w: 0.3, h: 0.008 };
  } else {
    // logos-top
    const gap = 0.04;
    const pairW = schoolW + (draft.partnerLogoKey ? partnerW + gap : 0);
    const startX = (1 - pairW) / 2;
    logoSchool = { x: startX, y: 0.06, w: schoolW, h: logoH };
    logoPartner = draft.partnerLogoKey
      ? { x: startX + schoolW + gap, y: 0.06, w: partnerW, h: logoH }
      : null;
    title = { x: 0.08, y: 0.26, w: 0.84, h: 0.14 };
    subtitle = { x: 0.1, y: 0.42, w: 0.8, h: 0.06 };
    body = { x: 0.12, y: 0.52, w: 0.76, h: 0.2 };
    accentBar = { x: 0, y: 0.2, w: 1, h: 0.012 };
  }

  // V2 offsets
  title = shiftBox(title, offsets.contentShiftX, offsets.titleOffsetY + offsets.contentShiftY);
  subtitle = shiftBox(subtitle, offsets.contentShiftX, offsets.contentShiftY);
  body = shiftBox(body, offsets.contentShiftX, offsets.contentShiftY);
  if (logoPartner) {
    logoPartner = scaleBoxAroundCenter(logoPartner, offsets.logoPartnerScale);
  }

  const datePlace =
    draft.blocks.showDatePlace && (draft.dateLabel.trim() || draft.placeLabel.trim())
      ? shiftBox(
          { x: 0.1, y: 0.78, w: 0.8, h: 0.06 },
          offsets.contentShiftX,
          offsets.contentShiftY,
        )
      : null;

  const schoolMention = draft.blocks.showSchoolMention
    ? { x: 0.08, y: 0.92, w: 0.6, h: 0.04 }
    : null;

  const qr =
    draft.blocks.showQr && draft.qrUrl.trim()
      ? { x: 0.82, y: 0.82, w: 0.12, h: 0.12 }
      : null;

  return {
    page,
    boxes: {
      background: { x: 0, y: 0, w: 1, h: 1 },
      overlay: { x: 0, y: 0, w: 1, h: 1 },
      logoSchool,
      logoPartner,
      title,
      subtitle,
      body,
      datePlace,
      schoolMention,
      qr,
      accentBar,
    },
    colors: {
      background: draft.backgroundColor,
      accent: draft.accentColor,
      text: draft.textColor,
      gradientTo: draft.gradientTo,
    },
    backgroundMode: draft.backgroundMode,
    overlayOpacity:
      draft.backgroundMode === "image"
        ? Math.min(0.85, Math.max(0, draft.overlayOpacity))
        : 0,
    titleFontSize: titlePt(draft.titleSize, draft.format),
    subtitleFontSize: titlePt(draft.titleSize, draft.format) * 0.45,
    bodyFontSize: titlePt(draft.titleSize, draft.format) * 0.38,
  };
}
