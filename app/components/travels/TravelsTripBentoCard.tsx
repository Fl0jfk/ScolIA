"use client";

import Image from "next/image";
import {
  hexToRgba,
  type EstablishmentVisual,
} from "@/app/lib/establishment-visual";
import {
  isTripTravelDatePast,
  travelsListBudget,
  travelsListNbEleves,
  travelsListPipeline,
} from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { normalizeTravelImageUrl } from "@/app/lib/travels-image-url";

const DONE_GREEN = "#059669";
const DONE_GREEN_SOFT = "rgba(5, 150, 105, 0.12)";
const DONE_GREEN_RING = "rgba(5, 150, 105, 0.28)";

type Props = {
  trip: TravelsTrip;
  etabLabel: string;
  vis: EstablishmentVisual;
  canOpenTrip: boolean;
  index?: number;
  onOpen: () => void;
  formatDate: (trip: TravelsTrip, field: "created" | "travel") => string;
};

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
  const budget = travelsListBudget(trip);
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
      ? `Série ${trip.data.recurrenceIndex ?? "?"}/${trip.data.recurrenceTotal}`
      : null;
  const progressLabel = isPast ? "Terminée" : isFinalized ? "Finalisé" : pipeline.currentLabel;

  return (
    <div
      onClick={canOpenTrip ? onOpen : undefined}
      className={`group relative flex min-w-0 flex-col overflow-hidden rounded-[1.75rem] bg-white transition duration-300 transform-gpu ${
        canOpenTrip
          ? "cursor-pointer hover:-translate-y-0.5 hover:shadow-[0_18px_40px_-24px_rgba(0,0,0,0.35)]"
          : "cursor-default"
      } ${
        isPast
          ? "opacity-60 grayscale ring-1 ring-slate-200 hover:opacity-80 hover:grayscale-[0.85]"
          : ""
      }`}
      style={
        isPast
          ? undefined
          : {
              boxShadow: `inset 0 0 0 1.5px ${isFinalized ? "rgba(5,150,105,0.35)" : vis.borderColor}`,
              animationDelay: `${Math.min(index, 8) * 40}ms`,
            }
      }
      title={
        canOpenTrip
          ? undefined
          : "Consultation liste uniquement — ouverture du dossier réservée à d’autres rôles"
      }
    >
      {!isPast ? (
        <div className="absolute inset-y-0 left-0 w-1.5" style={{ backgroundColor: accent }} aria-hidden />
      ) : null}

      <div className="relative flex flex-1 flex-col p-4 pl-5">
        <div className="relative isolate overflow-hidden rounded-[1.25rem] bg-slate-100">
          <div className="relative aspect-[2/1] w-full sm:aspect-[21/9]">
            {imageUrl ? (
              <Image
                src={imageUrl}
                alt={trip.data?.title || "Sortie scolaire"}
                className="h-full w-full object-cover transition-transform duration-700 group-hover:scale-[1.03]"
                width={720}
                height={320}
              />
            ) : (
              <div className="flex h-full w-full items-center justify-center bg-slate-100 text-3xl text-slate-400">
                {isComplex ? "🚌" : "🍦"}
              </div>
            )}
            <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/25 via-transparent to-transparent" />
          </div>
          <div className="absolute left-3 top-3 flex flex-wrap gap-1.5">
            {isPast ? (
              <span className="rounded-full border border-slate-300 bg-slate-200/95 px-2.5 py-1 text-[11px] font-bold text-slate-600">
                Terminée
              </span>
            ) : null}
            {seriesLabel ? (
              <span className="rounded-full border border-white/50 bg-white/95 px-2.5 py-1 text-[11px] font-bold text-slate-700 shadow-sm">
                {seriesLabel}
              </span>
            ) : null}
          </div>
          <div
            className="absolute right-3 top-3 inline-flex max-w-[55%] items-center gap-1.5 truncate rounded-full border px-2.5 py-1 text-[11px] font-bold shadow-sm"
            style={{
              backgroundColor: vis.badgeBg,
              color: vis.textColor,
              borderColor: vis.hex,
            }}
          >
            <span
              className="h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: vis.hex }}
              aria-hidden
            />
            <span className="truncate">{etabLabel}</span>
          </div>
        </div>

        <div className="relative mt-4 flex flex-1 flex-col gap-4">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <h3
                className="text-xl font-black leading-snug tracking-tight text-slate-900 line-clamp-2 transition-colors group-hover:text-indigo-700"
                title={trip.data?.title || "Sans titre"}
              >
                {trip.data?.title || "Sans titre"}
              </h3>
              <p className="mt-1.5 text-sm font-semibold text-slate-600">{travelLabel}</p>
              <p className="mt-0.5 text-xs font-medium text-slate-400">
                Dossier du {formatDate(trip, "created")}
              </p>
            </div>
            <span
              className={`mt-0.5 inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-full text-lg text-white transition group-hover:translate-x-0.5 ${
                isPast ? "bg-slate-400" : ""
              }`}
              style={isPast ? undefined : { backgroundColor: accent }}
              aria-hidden
            >
              →
            </span>
          </div>

          <div
            className="rounded-2xl border px-3.5 py-3"
            style={{
              backgroundColor: isPast
                ? "rgba(15,23,42,0.03)"
                : isFinalized
                  ? DONE_GREEN_SOFT
                  : "rgba(15,23,42,0.03)",
              borderColor: isPast
                ? "rgba(15,23,42,0.08)"
                : isFinalized
                  ? "rgba(5,150,105,0.28)"
                  : "rgba(15,23,42,0.08)",
            }}
            title={progressLabel}
          >
            <div className="mb-2.5 flex items-center justify-between gap-2">
              <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-slate-500">
                Avancement
              </p>
              <p
                className="truncate text-sm font-black"
                style={{
                  color: pipeline.blocked
                    ? "#b45309"
                    : isPast
                      ? "#64748b"
                      : isFinalized
                        ? DONE_GREEN
                        : "#0f172a",
                }}
              >
                {progressLabel}
              </p>
            </div>
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
                          ? "h-3.5 w-3.5"
                          : "h-2.5 w-2.5"
                      }`}
                      style={{
                        backgroundColor: reached || current ? activeColor : "rgba(15,23,42,0.18)",
                        boxShadow:
                          current || (isFinalized && idx === pipeline.steps.length - 1)
                            ? `0 0 0 4px ${isFinalized ? DONE_GREEN_RING : hexToRgba(vis.hex, 0.25)}`
                            : undefined,
                      }}
                      title={step.label}
                      aria-current={current ? "step" : undefined}
                    />
                    {idx < pipeline.steps.length - 1 ? (
                      <span
                        className="mx-1.5 h-1 min-w-[0.75rem] flex-1 rounded-full"
                        style={{
                          backgroundColor: connectorDone ? activeColor : "rgba(15,23,42,0.12)",
                        }}
                        aria-hidden
                      />
                    ) : null}
                  </div>
                );
              })}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-2xl border border-slate-100 bg-slate-50 px-3.5 py-3">
              <p className="text-[11px] font-bold uppercase tracking-wide text-slate-400">Élèves</p>
              <p className="mt-1 text-lg font-black text-slate-900">
                {nbEleves != null ? nbEleves : "—"}
              </p>
            </div>
            <div className="rounded-2xl border border-slate-100 bg-slate-50 px-3.5 py-3">
              <p className="text-[11px] font-bold uppercase tracking-wide text-slate-400">
                {budget.kind === "valide" ? "Budget" : "Prévisionnel"}
              </p>
              <p className="mt-1 text-lg font-black text-slate-900">
                {budget.amount != null ? `${Math.round(budget.amount)}€` : "—"}
              </p>
            </div>
          </div>

          <div className="mt-auto flex items-center justify-between gap-2 border-t border-slate-100 pt-3">
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
            <span className="text-[11px] font-bold uppercase tracking-[0.12em] text-slate-400 opacity-0 transition group-hover:opacity-100">
              Ouvrir
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
