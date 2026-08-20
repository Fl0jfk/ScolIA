import {
  clampPosterOffsets,
  defaultPosterDraft,
  defaultPosterOffsets,
  isPosterFormat,
  isPosterLayoutPreset,
  isPosterTemplateId,
} from "@/app/lib/posters/catalog";
import type { PosterDraft, PosterLogoSize, PosterTitleSize } from "@/app/lib/posters/types";

function asLogoSize(v: unknown): PosterLogoSize {
  return v === "S" || v === "L" ? v : "M";
}

function asTitleSize(v: unknown): PosterTitleSize {
  return v === "S" || v === "L" ? v : "L";
}

/** Parse / normalise un draft client → PosterDraft sûr. */
export function parsePosterDraft(raw: unknown): PosterDraft {
  const base = defaultPosterDraft();
  if (!raw || typeof raw !== "object") return base;
  const b = raw as Record<string, unknown>;

  const templateId = isPosterTemplateId(String(b.templateId || ""))
    ? (b.templateId as PosterDraft["templateId"])
    : base.templateId;

  const draft = defaultPosterDraft(templateId);

  if (isPosterFormat(String(b.format || ""))) draft.format = b.format as PosterDraft["format"];
  if (isPosterLayoutPreset(String(b.layoutPreset || ""))) {
    draft.layoutPreset = b.layoutPreset as PosterDraft["layoutPreset"];
  }

  draft.title = String(b.title ?? draft.title).slice(0, 120);
  draft.subtitle = String(b.subtitle ?? "").slice(0, 160);
  draft.body = String(b.body ?? "").slice(0, 800);
  draft.partnerName = String(b.partnerName ?? "").slice(0, 120);
  draft.dateLabel = String(b.dateLabel ?? "").slice(0, 80);
  draft.placeLabel = String(b.placeLabel ?? "").slice(0, 120);
  draft.qrUrl = String(b.qrUrl ?? "").slice(0, 500);

  const bgMode = String(b.backgroundMode || "");
  if (bgMode === "solid" || bgMode === "gradient" || bgMode === "image") {
    draft.backgroundMode = bgMode;
  }

  draft.backgroundColor = String(b.backgroundColor || draft.backgroundColor).slice(0, 20);
  draft.accentColor = String(b.accentColor || draft.accentColor).slice(0, 20);
  draft.textColor = String(b.textColor || draft.textColor).slice(0, 20);
  draft.gradientTo = String(b.gradientTo || draft.gradientTo).slice(0, 20);

  draft.backgroundImageKey =
    typeof b.backgroundImageKey === "string" && b.backgroundImageKey.trim()
      ? b.backgroundImageKey.trim().slice(0, 500)
      : null;
  draft.partnerLogoKey =
    typeof b.partnerLogoKey === "string" && b.partnerLogoKey.trim()
      ? b.partnerLogoKey.trim().slice(0, 500)
      : null;

  const op = Number(b.overlayOpacity);
  draft.overlayOpacity = Number.isFinite(op) ? Math.min(0.85, Math.max(0, op)) : 0.45;

  draft.logoSchoolSize = asLogoSize(b.logoSchoolSize);
  draft.logoPartnerSize = asLogoSize(b.logoPartnerSize);
  draft.titleSize = asTitleSize(b.titleSize);

  const blocks = b.blocks && typeof b.blocks === "object" ? (b.blocks as Record<string, unknown>) : {};
  draft.blocks = {
    showQr: Boolean(blocks.showQr),
    showDatePlace: blocks.showDatePlace !== false,
    showSchoolMention: blocks.showSchoolMention !== false,
  };

  draft.offsets = clampPosterOffsets(
    b.offsets && typeof b.offsets === "object"
      ? (b.offsets as Partial<ReturnType<typeof defaultPosterOffsets>>)
      : defaultPosterOffsets(),
  );

  return draft;
}
