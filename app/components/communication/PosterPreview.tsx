"use client";

import { useMemo, type CSSProperties } from "react";
import { computePosterLayout } from "@/app/lib/posters/poster-layout";
import type { PosterDraft } from "@/app/lib/posters/types";

type Props = {
  draft: PosterDraft;
  schoolName: string;
  partnerLogoUrl?: string | null;
  backgroundImageUrl?: string | null;
};

function boxStyle(box: { x: number; y: number; w: number; h: number }): CSSProperties {
  return {
    position: "absolute",
    left: `${box.x * 100}%`,
    top: `${box.y * 100}%`,
    width: `${box.w * 100}%`,
    height: `${box.h * 100}%`,
  };
}

export default function PosterPreview({
  draft,
  schoolName,
  partnerLogoUrl,
  backgroundImageUrl,
}: Props) {
  const layout = useMemo(() => computePosterLayout(draft), [draft]);
  const aspect = layout.page.widthPt / layout.page.heightPt;

  const bgStyle: CSSProperties = (() => {
    if (draft.backgroundMode === "image" && backgroundImageUrl) {
      return {
        backgroundImage: `linear-gradient(rgba(15,23,42,${layout.overlayOpacity}), rgba(15,23,42,${layout.overlayOpacity})), url(${backgroundImageUrl})`,
        backgroundSize: "cover",
        backgroundPosition: "center",
      };
    }
    if (draft.backgroundMode === "gradient") {
      return {
        background: `linear-gradient(160deg, ${draft.backgroundColor}, ${draft.gradientTo})`,
      };
    }
    return { background: draft.backgroundColor };
  })();

  const datePlace = [draft.dateLabel, draft.placeLabel].map((s) => s.trim()).filter(Boolean).join("  ·  ");
  const mention = draft.partnerName.trim()
    ? `${schoolName}  ×  ${draft.partnerName.trim()}`
    : schoolName;

  return (
    <div className="mx-auto w-full max-w-md">
      <div
        className="relative overflow-hidden rounded-xl shadow-lg ring-1 ring-slate-200"
        style={{ aspectRatio: `${aspect}`, ...bgStyle }}
      >
        <div
          style={{
            ...boxStyle(layout.boxes.accentBar),
            background: draft.accentColor,
          }}
        />

        <div style={boxStyle(layout.boxes.logoSchool)} className="flex items-center justify-center p-1">
          <div
            className="flex h-full w-full items-center justify-center rounded-md bg-white/90 px-1 text-center text-[9px] font-bold text-slate-700"
            title="Logo établissement (auto)"
          >
            Logo école
          </div>
        </div>

        {layout.boxes.logoPartner && (partnerLogoUrl || draft.partnerLogoKey) ? (
          <div
            style={boxStyle(layout.boxes.logoPartner)}
            className="flex items-center justify-center p-1"
          >
            {partnerLogoUrl ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                src={partnerLogoUrl}
                alt="Partenaire"
                className="max-h-full max-w-full object-contain"
              />
            ) : (
              <div className="flex h-full w-full items-center justify-center rounded-md bg-white/80 text-[9px] font-bold text-slate-600">
                Partenaire
              </div>
            )}
          </div>
        ) : null}

        <div
          style={{
            ...boxStyle(layout.boxes.title),
            color: draft.textColor,
            fontSize: `clamp(14px, ${layout.titleFontSize * 0.35}px, 28px)`,
            fontWeight: 800,
            lineHeight: 1.15,
            textAlign: "center",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: "0 4px",
          }}
        >
          {draft.title || "Titre"}
        </div>

        {draft.subtitle.trim() ? (
          <div
            style={{
              ...boxStyle(layout.boxes.subtitle),
              color: draft.textColor,
              fontSize: `clamp(10px, ${layout.subtitleFontSize * 0.4}px, 16px)`,
              textAlign: "center",
              opacity: 0.95,
            }}
          >
            {draft.subtitle}
          </div>
        ) : null}

        {draft.body.trim() ? (
          <div
            style={{
              ...boxStyle(layout.boxes.body),
              color: draft.textColor,
              fontSize: `clamp(9px, ${layout.bodyFontSize * 0.4}px, 13px)`,
              textAlign: "center",
              lineHeight: 1.35,
              opacity: 0.92,
            }}
          >
            {draft.body}
          </div>
        ) : null}

        {layout.boxes.datePlace && datePlace ? (
          <div
            style={{
              ...boxStyle(layout.boxes.datePlace),
              color: draft.accentColor,
              fontSize: 11,
              fontWeight: 700,
              textAlign: "center",
            }}
          >
            {datePlace}
          </div>
        ) : null}

        {layout.boxes.schoolMention ? (
          <div
            style={{
              ...boxStyle(layout.boxes.schoolMention),
              color: draft.textColor,
              fontSize: 9,
              opacity: 0.85,
            }}
          >
            {mention}
          </div>
        ) : null}

        {layout.boxes.qr ? (
          <div
            style={boxStyle(layout.boxes.qr)}
            className="flex items-center justify-center rounded bg-white text-[8px] font-bold text-slate-500"
          >
            QR
          </div>
        ) : null}
      </div>
    </div>
  );
}
