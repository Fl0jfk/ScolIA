"use client";

import {
  hexToRgba,
  type EstablishmentVisual,
} from "@/app/lib/establishment-visual";
import {
  isTripTravelDatePast,
  travelsListNbEleves,
  travelsListPipeline,
} from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { normalizeTravelImageUrl } from "@/app/lib/travels-image-url";

const DONE_GREEN = "#059669";
const DONE_GREEN_SOFT = "rgba(5, 150, 105, 0.1)";

type Props = {
  trip: TravelsTrip;
  etabLabel: string;
  vis: EstablishmentVisual;
  canOpenTrip: boolean;
  index?: number;
  onOpen: () => void;
  formatDate: (trip: TravelsTrip, field: "created" | "travel") => string;
};

/**
 * Carte liste sorties — style listing property (image dominante, meta claire, peu de boîtes).
 * Réf. design : https://x.com/nizamdesign/status/2104696209369907518
 */
export default function TravelsTripBentoCard({
  trip,
  etabLabel,
  vis,
  canOpenTrip,
  index = 0,
  onOpen,
  formatDate,
}: Props) {
  const isComplex = trip.type === "COMPLEX" || Boolean((trip.data as { transport?: unknown })?.transport);
  const imageUrl = normalizeTravelImageUrl(
    (typeof trip.imageUrl === "string" && trip.imageUrl) ||
      (typeof trip.data?.imageUrl === "string" ? trip.data.imageUrl : undefined),
  );
  const isPast = isTripTravelDatePast(trip);
  const nbEleves = travelsListNbEleves(trip);
  const pipeline = travelsListPipeline(trip);
  const isFinalized = pipeline.done && !pipeline.blocked;
  const accent = isPast ? "#64748b" : isFinalized ? DONE_GREEN : vis.hex;
  const travelLabel =
    trip.type === "COMPLEX"
      ? `Du ${formatDate(trip, "travel")} au ${
          trip.data?.endDate ? new Date(trip.data.endDate).toLocaleDateString("fr-FR") : "—"
        }`
      : `Le ${formatDate(trip, "travel")}`;
  const seriesLabel =
    trip.data?.recurrenceSeriesId && trip.data?.recurrenceTotal
      ? `${trip.data.recurrenceIndex ?? "?"}/${trip.data.recurrenceTotal}`
      : null;
  const progressLabel = isPast ? "Terminée" : isFinalized ? "Finalisé" : pipeline.currentLabel;

  return (
    <article
      onClick={canOpenTrip ? onOpen : undefined}
      className={`group relative flex min-w-0 flex-col overflow-hidden rounded-[1.75rem] bg-white shadow-[0_8px_30px_-18px_rgba(15,23,42,0.35)] ring-1 transition duration-300 ${
        canOpenTrip
          ? "cursor-pointer hover:-translate-y-1 hover:shadow-[0_22px_48px_-22px_rgba(15,23,42,0.4)]"
          : "cursor-default"
      } ${
        isPast
          ? "opacity-60 grayscale ring-slate-200 hover:opacity-80 hover:grayscale-[0.85]"
          : isFinalized
            ? "ring-emerald-200/80"
            : "ring-black/5"
      }`}
      style={{ animationDelay: `${Math.min(index, 8) * 40}ms` }}
      title={
        canOpenTrip
          ? undefined
          : "Consultation liste uniquement — ouverture du dossier réservée à d’autres rôles"
      }
    >
      {/* Media — ~25 % moins haut qu’avant (16/10 → 16/7.5) */}
      <div className="relative isolate aspect-[16/7.5] w-full overflow-hidden bg-slate-100">
        {imageUrl ? (
          // eslint-disable-next-line @next/next/no-img-element -- covers CDN / Wikimedia hors optimizer Next
          <img
            src={imageUrl}
            alt={trip.data?.title || "Sortie scolaire"}
            className="h-full w-full object-cover transition-transform duration-700 ease-out group-hover:scale-[1.04]"
            loading="lazy"
            decoding="async"
          />
        ) : (
          <div
            className="flex h-full w-full items-center justify-center text-4xl"
            style={{ background: `linear-gradient(145deg, ${vis.washBg}, #e2e8f0)` }}
          >
            {isComplex ? "🚌" : "🍦"}
          </div>
        )}
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/50 via-black/5 to-black/10" />

        <div className="absolute left-3 top-3 flex max-w-[58%] flex-wrap gap-1.5">
          {isPast ? (
            <span className="rounded-full bg-slate-900/80 px-2.5 py-1 text-[11px] font-bold text-white backdrop-blur-sm">
              Terminée
            </span>
          ) : null}
          {seriesLabel ? (
            <span className="rounded-full bg-white/95 px-2.5 py-1 text-[11px] font-bold text-slate-800 shadow-sm">
              Série {seriesLabel}
            </span>
          ) : null}
        </div>

        <div
          className="absolute right-3 top-3 inline-flex max-w-[42%] items-center gap-1.5 truncate rounded-full px-2.5 py-1 text-[11px] font-bold shadow-sm backdrop-blur-sm"
          style={{
            backgroundColor: hexToRgba(vis.hex, 0.92),
            color: "#fff",
          }}
        >
          <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-white/90" aria-hidden />
          <span className="truncate">{etabLabel}</span>
        </div>

        <div className="absolute bottom-3 right-3">
          <span
            className="inline-flex h-10 w-10 items-center justify-center rounded-full text-white shadow-md transition group-hover:translate-x-0.5"
            style={{ backgroundColor: accent }}
            aria-hidden
          >
            →
          </span>
        </div>
      </div>

      {/* Corps */}
      <div className="flex flex-1 flex-col gap-3.5 p-5">
        <div className="min-w-0">
          <h3
            className="text-[1.25rem] font-black leading-snug tracking-tight text-slate-900 line-clamp-2 transition-colors group-hover:text-slate-700"
            title={trip.data?.title || "Sans titre"}
          >
            {trip.data?.title || "Sans titre"}
          </h3>
          <p className="mt-1.5 flex items-center gap-1.5 text-sm font-medium text-slate-500">
            <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-slate-100 text-[11px]" aria-hidden>
              📅
            </span>
            <span className="truncate">{travelLabel}</span>
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-x-4 gap-y-2 border-y border-slate-100 py-3 text-sm">
          <div className="flex items-center gap-1.5 font-semibold text-slate-700">
            <span className="text-slate-400" aria-hidden>
              👥
            </span>
            <span>{nbEleves != null ? nbEleves : "—"}</span>
            <span className="font-medium text-slate-400">élèves</span>
          </div>
        </div>

        {/* Frise avancement — fine, vert si finalisé */}
        <div title={progressLabel}>
          <div className="mb-2 flex items-center justify-between gap-2">
            <p className="text-[11px] font-bold uppercase tracking-[0.14em] text-slate-400">
              Avancement
            </p>
            <p
              className="truncate text-[13px] font-bold"
              style={{
                color: pipeline.blocked
                  ? "#b45309"
                  : isPast
                    ? "#64748b"
                    : isFinalized
                      ? DONE_GREEN
                      : "#334155",
              }}
            >
              {progressLabel}
            </p>
          </div>
          <div
            className="rounded-full px-1 py-2"
            style={{
              backgroundColor: isFinalized && !isPast ? DONE_GREEN_SOFT : "transparent",
            }}
          >
            <div
              className="flex items-center"
              role="list"
              aria-label={`Avancement : ${progressLabel}`}
            >
              {pipeline.steps.map((step, idx) => {
                const reached = pipeline.done || idx < pipeline.currentIndex;
                const current = !pipeline.done && idx === pipeline.currentIndex;
                const connectorDone = pipeline.done || idx < pipeline.currentIndex;
                const activeColor = pipeline.blocked && current ? "#f59e0b" : accent;
                return (
                  <div
                    key={step.id}
                    className="flex min-w-0 flex-1 items-center last:flex-none"
                    role="listitem"
                  >
                    <span
                      className={`relative z-[1] flex shrink-0 rounded-full ${
                        current || (isFinalized && idx === pipeline.steps.length - 1)
                          ? "h-3 w-3"
                          : "h-2 w-2"
                      }`}
                      style={{
                        backgroundColor: reached || current ? activeColor : "#cbd5e1",
                        boxShadow:
                          current || (isFinalized && idx === pipeline.steps.length - 1)
                            ? `0 0 0 3px ${hexToRgba(activeColor, 0.22)}`
                            : undefined,
                      }}
                      title={step.label}
                      aria-current={current ? "step" : undefined}
                    />
                    {idx < pipeline.steps.length - 1 ? (
                      <span
                        className="mx-1 h-0.5 min-w-[0.5rem] flex-1 rounded-full"
                        style={{
                          backgroundColor: connectorDone ? activeColor : "#e2e8f0",
                        }}
                        aria-hidden
                      />
                    ) : null}
                  </div>
                );
              })}
            </div>
          </div>
        </div>

        {/* Agent / responsable */}
        <div className="mt-auto flex items-center justify-between gap-3 pt-1">
          <div className="flex min-w-0 items-center gap-2.5">
            <div
              className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full text-[11px] font-bold uppercase text-white"
              style={{ backgroundColor: isPast ? "#0f172a" : vis.hex }}
            >
              {trip.ownerName?.substring(0, 2)}
            </div>
            <div className="min-w-0">
              <p className="truncate text-sm font-bold text-slate-800">{trip.ownerName}</p>
              <p className="text-[11px] font-medium text-slate-400">
                {isPast ? "Facturation / dossier" : "Responsable"}
              </p>
            </div>
          </div>
          <span
            className="shrink-0 rounded-full px-3 py-1.5 text-[11px] font-bold uppercase tracking-[0.08em] transition"
            style={{
              backgroundColor: isPast ? "#f1f5f9" : hexToRgba(accent, 0.12),
              color: isPast ? "#64748b" : accent,
            }}
          >
            Ouvrir
          </span>
        </div>
      </div>
    </article>
  );
}
