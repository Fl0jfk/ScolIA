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
  const travelLabel =
    trip.type === "COMPLEX"
      ? `Du ${formatDate(trip, "travel")} au ${
          trip.data?.endDate ? new Date(trip.data.endDate).toLocaleDateString("fr-FR") : "—"
        }`
      : `Le ${formatDate(trip, "travel")}`;

  return (
    <div
      onClick={canOpenTrip ? onOpen : undefined}
      className={`group relative flex min-w-0 flex-col overflow-hidden rounded-[1.75rem] transition duration-300 transform-gpu ${
        canOpenTrip
          ? "cursor-pointer hover:-translate-y-0.5 hover:shadow-[0_22px_50px_-28px_rgba(0,0,0,0.35)]"
          : "cursor-default"
      } ${
        isPast
          ? "bg-slate-100/90 opacity-60 grayscale ring-1 ring-slate-200 hover:opacity-75 hover:grayscale-[0.85]"
          : "bg-white"
      }`}
      style={
        isPast
          ? undefined
          : {
              background: `linear-gradient(160deg, ${vis.washBg} 0%, #ffffff 48%, #ffffff 100%)`,
              boxShadow: `inset 0 0 0 1.5px ${vis.borderColor}, 0 1px 0 rgba(0,0,0,0.03)`,
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
        <div className="absolute inset-y-0 left-0 w-1.5" style={{ backgroundColor: vis.hex }} aria-hidden />
      ) : null}
      {!isPast ? (
        <div
          className="pointer-events-none absolute -right-8 -top-8 h-36 w-36 rounded-full blur-2xl transition duration-500 group-hover:scale-110"
          style={{ backgroundColor: vis.orbBg }}
          aria-hidden
        />
      ) : null}

      <div className="relative p-3 pl-4">
        <div className="relative isolate overflow-hidden rounded-[1.35rem] bg-slate-100">
          <div className="relative aspect-[16/10] w-full">
            {imageUrl ? (
              <Image
                src={imageUrl}
                alt={trip.data?.title || "Sortie scolaire"}
                className="h-full w-full object-cover transition-transform duration-700 group-hover:scale-105"
                width={640}
                height={400}
              />
            ) : (
              <div
                className="flex h-full w-full items-center justify-center text-4xl"
                style={{ background: `linear-gradient(145deg, ${vis.washBg}, #f8fafc)` }}
              >
                {isComplex ? "🚌" : "🍦"}
              </div>
            )}
            <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-black/10" />
          </div>
          <div className="absolute left-3 top-3 flex flex-wrap gap-1.5">
            {isPast ? (
              <span className="rounded-full border border-slate-300 bg-slate-200/95 px-2.5 py-1 text-[10px] font-black text-slate-600 backdrop-blur-md">
                Terminée
              </span>
            ) : null}
            <span
              className={`rounded-full border px-2.5 py-1 text-[10px] font-black backdrop-blur-md shadow-sm ${
                isComplex
                  ? "border-purple-200/80 bg-purple-50/95 text-purple-800"
                  : "border-white/40 bg-white/90 text-slate-700"
              }`}
            >
              {isComplex ? "Voyage scolaire" : "Sortie locale"}
              {trip.data?.recurrenceSeriesId && trip.data?.recurrenceTotal
                ? ` · ${trip.data.recurrenceIndex ?? "?"}/${trip.data.recurrenceTotal}`
                : ""}
            </span>
          </div>
          <div
            className="absolute right-3 top-3 inline-flex max-w-[48%] items-center gap-1.5 truncate rounded-full border px-2.5 py-1 text-[10px] font-black uppercase tracking-wide shadow-md backdrop-blur-md"
            style={{
              backgroundColor: vis.badgeBg,
              color: vis.textColor,
              borderColor: vis.hex,
            }}
          >
            <span
              className="h-2 w-2 shrink-0 rounded-full ring-2 ring-white/70"
              style={{ backgroundColor: vis.hex }}
              aria-hidden
            />
            <span className="truncate">{etabLabel}</span>
          </div>
        </div>

        <div className="relative flex flex-1 flex-col gap-3 px-1 pb-1 pt-4">
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <p className="text-[10px] font-bold uppercase tracking-[0.14em] text-neutral-400">
                Dossier du {formatDate(trip, "created")}
              </p>
              <h3
                className="mt-1 text-lg font-black leading-snug tracking-tight text-[var(--dash-ink)] line-clamp-2 transition-colors group-hover:text-indigo-600"
                title={trip.data?.title || "Sans titre"}
              >
                {trip.data?.title || "Sans titre"}
              </h3>
              <p className="mt-1.5 text-xs font-semibold text-neutral-500">{travelLabel}</p>
            </div>
            <span
              className={`mt-0.5 inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full text-lg transition group-hover:translate-x-0.5 ${
                isPast
                  ? "bg-slate-200 text-slate-500 group-hover:bg-slate-500 group-hover:text-white"
                  : "text-white"
              }`}
              style={isPast ? undefined : { backgroundColor: vis.hex }}
              aria-hidden
            >
              →
            </span>
          </div>

          <div
            className="rounded-2xl px-3 py-2.5"
            style={{
              backgroundColor: isPast ? "rgba(15,23,42,0.04)" : vis.washBg,
              boxShadow: `inset 0 0 0 1px ${isPast ? "rgba(15,23,42,0.06)" : vis.borderColor}`,
            }}
            title={pipeline.currentLabel}
          >
            <div className="mb-2 flex items-center justify-between gap-2">
              <p className="text-[10px] font-bold uppercase tracking-[0.14em] text-neutral-500">Avancement</p>
              <p
                className="truncate text-[11px] font-black"
                style={{ color: pipeline.blocked ? "#b45309" : isPast ? "#64748b" : vis.textColor }}
              >
                {isPast ? "Terminée" : pipeline.currentLabel}
              </p>
            </div>
            <div
              className="flex items-center gap-0"
              role="list"
              aria-label={`Avancement : ${pipeline.currentLabel}`}
            >
              {pipeline.steps.map((step, idx) => {
                const reached = pipeline.done || idx < pipeline.currentIndex;
                const current = !pipeline.done && idx === pipeline.currentIndex;
                const connectorDone = pipeline.done || idx < pipeline.currentIndex;
                return (
                  <div
                    key={step.id}
                    className="flex min-w-0 flex-1 items-center last:flex-none"
                    role="listitem"
                  >
                    <span
                      className={`relative z-[1] flex shrink-0 rounded-full transition ${
                        current ? "h-3.5 w-3.5 ring-2 ring-white" : "h-2.5 w-2.5"
                      }`}
                      style={{
                        backgroundColor:
                          reached || current
                            ? pipeline.blocked && current
                              ? "#f59e0b"
                              : vis.hex
                            : "rgba(15,23,42,0.15)",
                        boxShadow: current ? `0 0 0 4px ${hexToRgba(vis.hex, 0.28)}` : undefined,
                      }}
                      title={step.label}
                      aria-current={current ? "step" : undefined}
                    />
                    {idx < pipeline.steps.length - 1 ? (
                      <span
                        className="mx-1 h-0.5 min-w-[0.5rem] flex-1 rounded-full"
                        style={{
                          backgroundColor: connectorDone ? vis.hex : "rgba(15,23,42,0.12)",
                          opacity: connectorDone ? 0.85 : 1,
                        }}
                        aria-hidden
                      />
                    ) : null}
                  </div>
                );
              })}
            </div>
          </div>

          <div className="mt-auto grid grid-cols-2 gap-2">
            <div
              className="rounded-2xl px-3 py-2.5"
              style={{
                backgroundColor: isPast ? "rgba(15,23,42,0.04)" : vis.washBg,
                boxShadow: `inset 0 0 0 1px ${isPast ? "rgba(15,23,42,0.06)" : vis.borderColor}`,
              }}
            >
              <p className="text-[10px] font-bold uppercase tracking-wide text-neutral-400">Élèves</p>
              <p className="mt-0.5 text-base font-black text-[var(--dash-ink)]">
                {nbEleves != null ? nbEleves : "—"}
              </p>
            </div>
            <div
              className="rounded-2xl px-3 py-2.5"
              style={{
                backgroundColor: isPast ? "rgba(15,23,42,0.04)" : vis.washBg,
                boxShadow: `inset 0 0 0 1px ${isPast ? "rgba(15,23,42,0.06)" : vis.borderColor}`,
              }}
            >
              <p className="text-[10px] font-bold uppercase tracking-wide text-neutral-400">
                {budget.kind === "valide" ? "Budget" : "Prévisionnel"}
              </p>
              <p className="mt-0.5 text-base font-black text-[var(--dash-ink)]">
                {budget.amount != null ? `${Math.round(budget.amount)}€` : "—"}
              </p>
            </div>
          </div>

          <div className="flex items-center justify-between gap-2 border-t border-black/5 pt-3">
            <div className="flex min-w-0 items-center gap-2.5">
              <div
                className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-[10px] font-bold uppercase text-white"
                style={{ backgroundColor: isPast ? "#0f172a" : vis.hex }}
              >
                {trip.ownerName?.substring(0, 2)}
              </div>
              <div className="min-w-0">
                <p className="truncate text-sm font-bold text-[var(--dash-ink)]">{trip.ownerName}</p>
                <p className="text-[10px] font-semibold uppercase tracking-wide text-neutral-400">
                  {isPast ? "Facturation / dossier" : "Responsable"}
                </p>
              </div>
            </div>
            <span className="hidden text-[11px] font-semibold uppercase tracking-[0.12em] text-neutral-400 opacity-0 transition group-hover:opacity-100 sm:inline">
              Ouvrir
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
