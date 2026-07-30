"use client";

import type { TravelsTrip } from "@/app/lib/travels-types";
import { resolveCuisineOrderSentAt } from "@/app/lib/travels-trip-helpers";
import { TRAVELS_STATUS_LABELS } from "@/app/lib/travels-types";
import { TripSection } from "@/app/components/travels/TripDetailUI";

type JournalEntry = {
  id: string;
  at: string;
  category: "transport" | "cuisine" | "workflow" | "effectif" | "annulation" | "communication";
  title: string;
  detail?: string;
  user?: string;
};

function buildJournalEntries(trip: TravelsTrip): JournalEntry[] {
  const entries: JournalEntry[] = [];

  if (trip.data.listeElevesConfirmedAt) {
    entries.push({
      id: `liste_${trip.data.listeElevesConfirmedAt}`,
      at: trip.data.listeElevesConfirmedAt,
      category: "effectif",
      title: `Liste élèves confirmée (${trip.data.participantEleves?.length || 0})`,
      detail: trip.data.listeEnvoyeeTransporteurAt
        ? `Envoyée au transporteur le ${new Date(trip.data.listeEnvoyeeTransporteurAt).toLocaleString("fr-FR")}`
        : undefined,
      user: trip.data.listeElevesConfirmedBy?.name,
    });
  }

  for (const log of trip.data.parentComLogs || []) {
    entries.push({
      id: log.id,
      at: log.sentAt,
      category: "communication",
      title: `Com’ parents — ${log.subject}`,
      detail: `${log.recipientCount} destinataire(s) · ${log.photoCount} photo(s)`,
      user: log.sentBy.name,
    });
  }

  for (const a of trip.data.transportAmendments || []) {
    const prev = a.previousEffectif;
    const next = a.newEffectif;
    entries.push({
      id: `ta_${a.sentAt}`,
      at: a.sentAt,
      category: "transport",
      title:
        a.recipientMode === "single"
          ? `Avenant transport → ${a.providerName || "transporteur"}`
          : "Avenant transport → tous les transporteurs",
      detail: prev
        ? `Effectif ${prev.nbEleves + prev.nbAccompagnateurs} → ${next.nbEleves + next.nbAccompagnateurs} pers.`
        : `Nouvel effectif : ${next.nbEleves + next.nbAccompagnateurs} pers.`,
    });
  }

  for (const a of trip.data.cuisineAmendments || []) {
    entries.push({
      id: `ca_${a.sentAt}`,
      at: a.sentAt,
      category: "cuisine",
      title: "Commande cuisine renvoyée (annule et remplace)",
      detail: a.actor ? `Par ${a.actor}` : undefined,
      user: a.actor,
    });
  }

  const cuisineSentAt = resolveCuisineOrderSentAt(trip);
  if (cuisineSentAt && !(trip.data.cuisineAmendments?.length)) {
    entries.push({
      id: `ci_${cuisineSentAt}`,
      at: cuisineSentAt,
      category: "cuisine",
      title: "Commande cuisine initiale envoyée au chef",
    });
  }

  if (trip.data.transportQuoteSnapshot?.sentAt) {
    entries.push({
      id: `ts_${trip.data.transportQuoteSnapshot.sentAt}`,
      at: trip.data.transportQuoteSnapshot.sentAt,
      category: "transport",
      title:
        trip.data.transportQuoteSnapshot.type === "amendment"
          ? "Snapshot effectif transport mis à jour"
          : "Demande de devis transport envoyée",
      detail: `${trip.data.transportQuoteSnapshot.nbEleves} él. + ${trip.data.transportQuoteSnapshot.nbAccompagnateurs || 0} acc.`,
    });
  }

  for (const h of trip.history || []) {
    if (!h.date) continue;
    const action = h.action || "";
    let category: JournalEntry["category"] = "workflow";
    if (action === "EFFECTIF_MODIFIE") category = "effectif";
    if (action === "ANNULE") category = "annulation";
    if (action.includes("cuisine")) category = "cuisine";
    if (action.includes("transport") || action.includes("Avenant") || action.includes("transporteur")) {
      category = "transport";
    }
    if (action.includes("Communication parents") || action.includes("Liste élèves")) category = "effectif";
    if (action.includes("Communication parents")) category = "communication";

    entries.push({
      id: `h_${h.date}_${action}`,
      at: h.date,
      category,
      title: TRAVELS_STATUS_LABELS[action] || action.replace(/_/g, " "),
      detail: h.note,
      user: h.user,
    });
  }

  if (trip.data.cancelledAt) {
    entries.push({
      id: `cancel_${trip.data.cancelledAt}`,
      at: trip.data.cancelledAt,
      category: "annulation",
      title: "Sortie annulée",
      detail: trip.data.cancelReason as string | undefined,
    });
  }

  return entries.sort((a, b) => new Date(b.at).getTime() - new Date(a.at).getTime());
}

const CATEGORY_STYLES: Record<JournalEntry["category"], string> = {
  transport: "bg-amber-50 border-amber-200 text-amber-900",
  cuisine: "bg-emerald-50 border-emerald-200 text-emerald-900",
  workflow: "bg-slate-50 border-slate-200 text-slate-800",
  effectif: "bg-indigo-50 border-indigo-200 text-indigo-900",
  annulation: "bg-red-50 border-red-200 text-red-900",
  communication: "bg-violet-50 border-violet-200 text-violet-900",
};

const CATEGORY_ICONS: Record<JournalEntry["category"], string> = {
  transport: "🚌",
  cuisine: "🍽️",
  workflow: "📋",
  effectif: "👥",
  annulation: "🚫",
  communication: "📸",
};

export function TripAmendmentJournal({ trip }: { trip: TravelsTrip }) {
  const entries = buildJournalEntries(trip);

  return (
    <TripSection title="Journal du dossier" subtitle="Avenants, relances et historique workflow" icon="📜">
      {entries.length === 0 ? (
        <p className="text-sm text-slate-400 italic text-center py-8">Aucun événement enregistré pour l&apos;instant.</p>
      ) : (
        <ol className="relative border-l-2 border-slate-200 ml-3 space-y-4">
          {entries.map((e) => (
            <li key={e.id} className="ml-6">
              <span className="absolute -left-[9px] flex h-4 w-4 items-center justify-center rounded-full bg-white border-2 border-slate-300 text-[8px]">
                {CATEGORY_ICONS[e.category]}
              </span>
              <div className={`rounded-xl border p-3 ${CATEGORY_STYLES[e.category]}`}>
                <div className="flex flex-wrap items-baseline justify-between gap-2">
                  <p className="font-bold text-sm">{e.title}</p>
                  <time className="text-[10px] opacity-70">
                    {new Date(e.at).toLocaleString("fr-FR", {
                      day: "2-digit",
                      month: "short",
                      year: "numeric",
                      hour: "2-digit",
                      minute: "2-digit",
                    })}
                  </time>
                </div>
                {e.detail && <p className="text-xs mt-1 opacity-90">{e.detail}</p>}
                {e.user && <p className="text-[10px] mt-1 opacity-60">Par {e.user}</p>}
              </div>
            </li>
          ))}
        </ol>
      )}
    </TripSection>
  );
}
