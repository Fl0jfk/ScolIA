"use client";

/**
 * Onglet « Page familles » (circuit v2) : trame fixe + champs structurés remplis par l'organisateur,
 * aperçu fidèle de ce que verront les parents (simple étiquette « Aperçu »), bloc prix (compta) et fenêtre de
 * validation finale de la direction. Aucun texte libre, aucun appel IA, aucun envoi aux familles dans ce lot.
 */

import { useMemo, useState } from "react";
import { TripAlert, TripButton, TripSection } from "@/app/components/travels/TripDetailUI";
import { FamilyDocumentsEditor, newFamilyDocId, RetroplanningCard } from "@/app/components/travels/TravelsV2Creation";
import {
  buildFamilyPagePreviewModel,
  dateFr,
} from "@/app/lib/travels-family-page";
import { FAMILY_PAGE_TEMPLATE } from "@/app/lib/travels-family-page-template";
import { FamilyPageView } from "@/app/components/travels/family/FamilyPageView";
import { defaultParentCalendarFromTrip, newCalendarPointId } from "@/app/lib/travels-parent-calendar";
import type {
  TravelsCalendarPoint,
  TravelsFamilyDocRequest,
  TravelsFamilyPage,
  TravelsParentCalendar,
  TravelsPriceBlock,
  TravelsPriceInstallment,
  TravelsTrip,
} from "@/app/lib/travels-types";
import {
  addDaysIso,
  FAMILY_PROGRAM_MAX_CHARS,
  familyPageReadiness,
  getFamilyMeetingPoints,
  isFamilyPageEditableStatus,
  isFamilyPageOpened,
  REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE,
  REGISTRATION_MIN_OPEN_DAYS,
  todayIsoParis,
  tripDepartureIso,
  validateFamilyStructured,
  validatePriceBlock,
} from "@/app/lib/travels-v2";

export type SaveResult = { ok: boolean; error?: string };

function eur(n: number): string {
  return `${n.toFixed(2).replace(".", ",")} €`;
}

/* ─── Bloc prix (compta uniquement) ─── */

export function TripPriceBlockEditor({
  trip,
  canEdit,
  onSave,
}: {
  trip: TravelsTrip;
  canEdit: boolean;
  onSave: (block: TravelsPriceBlock) => Promise<SaveResult>;
}) {
  const existing = trip.data.priceBlock;
  const [price, setPrice] = useState(existing ? String(existing.pricePerStudent) : "");
  const [installments, setInstallments] = useState<TravelsPriceInstallment[]>(existing?.installments || []);
  const [paymentText, setPaymentText] = useState(existing?.paymentText || "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const draft = (): TravelsPriceBlock => ({
    pricePerStudent: Number(String(price).replace(",", ".")),
    installments: installments.map((i) => ({ ...i, amount: Number(i.amount) })),
    paymentText: paymentText.trim(),
  });

  const save = async () => {
    const block = draft();
    const check = validatePriceBlock(block);
    if (!check.ok) {
      setError(check.error);
      return;
    }
    setError(null);
    setBusy(true);
    try {
      const res = await onSave(block);
      if (!res.ok) setError(res.error || "Enregistrement impossible. Réessayez.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <TripSection
      title="Bloc prix de la page des familles"
      subtitle="Seule la comptabilité fixe le prix, les échéances et le texte de paiement affichés aux familles."
      icon="💶"
      accent="amber"
    >
      <div className="p-6 space-y-4">
        {!canEdit ? (
          <p className="text-sm text-slate-600">
            {existing
              ? `Prix par élève : ${eur(existing.pricePerStudent)} — saisi par ${existing.authorName || "la comptabilité"}.`
              : "Le bloc prix n’est pas encore renseigné par la comptabilité."}
          </p>
        ) : (
          <>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-xs font-bold mb-1">Prix par élève (€) — 0 € accepté</label>
                <input
                  value={price}
                  inputMode="decimal"
                  onChange={(e) => setPrice(e.target.value)}
                  className="w-full p-2.5 bg-white border rounded-xl"
                />
              </div>
              <div className="md:col-span-2">
                <label className="block text-xs font-bold mb-1">Texte de paiement (modalités, ordre du chèque, RIB…)</label>
                <textarea
                  value={paymentText}
                  rows={3}
                  onChange={(e) => setPaymentText(e.target.value)}
                  className="w-full p-2.5 bg-white border rounded-xl text-sm"
                />
              </div>
            </div>
            <div className="space-y-2">
              <p className="text-xs font-bold">Échéances (facultatif — le total doit égaler le prix par élève)</p>
              {installments.map((inst) => (
                <div key={inst.id} className="flex flex-wrap items-center gap-2">
                  <input
                    type="date"
                    value={inst.dueDate}
                    onChange={(e) =>
                      setInstallments((prev) => prev.map((x) => (x.id === inst.id ? { ...x, dueDate: e.target.value } : x)))
                    }
                    className="p-2 border rounded-lg bg-white"
                  />
                  <input
                    value={String(inst.amount ?? "")}
                    inputMode="decimal"
                    placeholder="Montant €"
                    onChange={(e) =>
                      setInstallments((prev) =>
                        prev.map((x) => (x.id === inst.id ? { ...x, amount: e.target.value as unknown as number } : x)),
                      )
                    }
                    className="w-28 p-2 border rounded-lg bg-white"
                  />
                  <button
                    type="button"
                    className="text-red-500 text-xs font-bold"
                    onClick={() => setInstallments((prev) => prev.filter((x) => x.id !== inst.id))}
                  >
                    Retirer
                  </button>
                </div>
              ))}
              <button
                type="button"
                className="text-xs font-bold text-indigo-600 hover:underline"
                onClick={() => setInstallments((prev) => [...prev, { id: newFamilyDocId(), dueDate: "", amount: 0 }])}
              >
                + Ajouter une échéance
              </button>
            </div>
            {error ? (
              <p className="text-sm font-semibold text-red-700" role="alert">
                {error}
              </p>
            ) : null}
            <TripButton variant="primary" onClick={() => void save()} disabled={busy}>
              {busy ? "Enregistrement…" : "Enregistrer le bloc prix"}
            </TripButton>
          </>
        )}
      </div>
    </TripSection>
  );
}

/* ─── Onglet « Page familles » ─── */

type MeetingDraft = { place: string; date: string; time: string };

function meetingFromCalendar(trip: TravelsTrip): { outbound: MeetingDraft; inbound: MeetingDraft } {
  const cal = defaultParentCalendarFromTrip(trip.data);
  const { outbound, inbound } = getFamilyMeetingPoints({ parentCalendar: cal });
  return {
    outbound: { place: outbound?.place || "", date: outbound?.date || "", time: outbound?.time || "" },
    inbound: { place: inbound?.place || "", date: inbound?.date || "", time: inbound?.time || "" },
  };
}

/** Réécrit les points « depot » / « recuperation » du calendrier parents existant (les autres points sont conservés). */
function applyMeetingsToCalendar(
  cal: TravelsParentCalendar,
  outbound: MeetingDraft,
  inbound: MeetingDraft,
): TravelsParentCalendar {
  const upsert = (points: TravelsCalendarPoint[], kind: "depot" | "recuperation", m: MeetingDraft, label: string) => {
    const idx = points.findIndex((p) => p.kind === kind);
    const base: TravelsCalendarPoint =
      idx >= 0
        ? points[idx]
        : { id: newCalendarPointId(), kind, label, date: m.date, time: m.time, durationMinutes: 30 };
    const next = { ...base, place: m.place.trim() || undefined, date: m.date || base.date, time: m.time || base.time };
    if (idx >= 0) points[idx] = next;
    else points.push(next);
  };
  const points = cal.points.map((p) => ({ ...p }));
  upsert(points, "depot", outbound, "Dépôt / départ");
  upsert(points, "recuperation", inbound, "Récupération / retour");
  return { ...cal, points };
}

export function TripFamilyPagePanel({
  trip,
  canEditPage,
  saveFamilyPage,
}: {
  trip: TravelsTrip;
  /** Organisateur, direction, administratif ou admin général (la compta est en lecture seule). */
  canEditPage: boolean;
  saveFamilyPage: (page: TravelsFamilyPage, parentCalendar: TravelsParentCalendar) => Promise<SaveResult>;
}) {
  const stored: TravelsFamilyPage = trip.data.familyPage || { documents: [] };
  const opened = isFamilyPageOpened(stored);
  const editableStatus = isFamilyPageEditableStatus(String(trip.status));
  const editable = canEditPage && editableStatus && !opened;
  const departureIso = tripDepartureIso(trip.data);
  const todayIso = todayIsoParis();
  const initialMeetings = useMemo(() => meetingFromCalendar(trip), [trip]);

  const [outbound, setOutbound] = useState<MeetingDraft>(initialMeetings.outbound);
  const [inbound, setInbound] = useState<MeetingDraft>(initialMeetings.inbound);
  const [equipment, setEquipment] = useState<string[]>(stored.equipment || []);
  const [newItem, setNewItem] = useState("");
  const [cashAmount, setCashAmount] = useState(stored.cash ? String(stored.cash.amount) : "");
  const [cashReason, setCashReason] = useState(stored.cash?.reason || "");
  const [program, setProgram] = useState(stored.program || "");
  const [deadline, setDeadline] = useState(stored.deadline || "");
  const [documents, setDocuments] = useState<TravelsFamilyDocRequest[]>(stored.documents || []);
  const [previewWidth, setPreviewWidth] = useState<"phone" | "desktop">("phone");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ tone: "ok" | "error"; text: string } | null>(null);

  const draftCash = (): TravelsFamilyPage["cash"] | undefined => {
    const amount = Number(cashAmount.replace(",", "."));
    if (!cashAmount.trim() && !cashReason.trim()) return undefined;
    return { amount: Number.isFinite(amount) ? amount : 0, reason: cashReason.trim() };
  };
  const draftPage = (): TravelsFamilyPage => ({
    ...stored,
    equipment,
    cash: draftCash(),
    program: program.trim() || undefined,
    deadline: deadline || undefined,
    documents,
  });
  const draftCalendar = () => applyMeetingsToCalendar(defaultParentCalendarFromTrip(trip.data), outbound, inbound);

  const dirty =
    JSON.stringify(draftPage()) !== JSON.stringify({ ...stored, documents: stored.documents || [] }) ||
    JSON.stringify([outbound, inbound]) !== JSON.stringify([initialMeetings.outbound, initialMeetings.inbound]);

  const model = useMemo(
    () =>
      buildFamilyPagePreviewModel(
        trip,
        opened ? undefined : { familyPage: draftPage(), parentCalendar: draftCalendar() },
      ),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [trip, opened, outbound, inbound, equipment, cashAmount, cashReason, program, deadline, documents],
  );

  const deadlineMin = addDaysIso(todayIso, REGISTRATION_MIN_OPEN_DAYS);
  const deadlineMax = departureIso ? addDaysIso(departureIso, -REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE) : undefined;

  const save = async () => {
    const check = validateFamilyStructured(draftPage());
    if (!check.ok) {
      setMessage({ tone: "error", text: check.error });
      return;
    }
    setBusy(true);
    setMessage(null);
    try {
      const res = await saveFamilyPage(draftPage(), draftCalendar());
      setMessage(res.ok ? { tone: "ok", text: "Page enregistrée." } : { tone: "error", text: res.error || "Enregistrement impossible — réessayez." });
    } finally {
      setBusy(false);
    }
  };

  const addItem = (label: string) => {
    const v = label.trim();
    if (!v || equipment.some((e) => e.toLowerCase() === v.toLowerCase())) return;
    setEquipment((prev) => [...prev, v]);
    setNewItem("");
  };

  const meetingFields = (label: string, m: MeetingDraft, set: (v: MeetingDraft) => void, timeLabel: string) => (
    <div className="rounded-2xl border bg-slate-50 p-3 space-y-2">
      <p className="text-sm font-bold">{label}</p>
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
        <input value={m.place} placeholder="Lieu" onChange={(e) => set({ ...m, place: e.target.value })} className="p-2 border rounded-lg bg-white text-sm" />
        <input type="date" value={m.date} onChange={(e) => set({ ...m, date: e.target.value })} className="p-2 border rounded-lg bg-white text-sm" />
        <input type="time" value={m.time} aria-label={timeLabel} title={timeLabel} onChange={(e) => set({ ...m, time: e.target.value })} className="p-2 border rounded-lg bg-white text-sm" />
      </div>
      <p className="text-[11px] text-slate-500">{timeLabel}</p>
    </div>
  );

  return (
    <div className="space-y-6">
      {opened ? (
        <TripAlert tone="success" icon="🔒" title="Page ouverte — verrouillée en écriture">
          La page a été ouverte aux inscriptions le {stored.openedAt ? new Date(stored.openedAt).toLocaleDateString("fr-FR") : "—"}.
          Ses informations ne peuvent plus être modifiées.
        </TripAlert>
      ) : !editableStatus ? (
        <TripAlert tone="info" icon="ℹ️" title="Remplissage possible après la validation pédagogique">
          La page se remplit dès que la direction a validé la pédagogie (en parallèle du transport).
        </TripAlert>
      ) : !canEditPage ? (
        <TripAlert tone="muted" icon="👀" title="Lecture seule">
          Seuls l’organisateur, la direction et l’administration remplissent la page. Le prix est saisi par la comptabilité.
        </TripAlert>
      ) : null}

      <div className="grid grid-cols-1 items-start gap-6 xl:grid-cols-2">
      <div className="space-y-6 min-w-0">
      {editable ? (
        <p className="rounded-xl border border-indigo-200 bg-indigo-50 px-4 py-3 text-sm font-semibold text-indigo-900">
          Tout ce qui apparaît dans l’aperçu (prix, pièces, paniers, signature…) est ajouté automatiquement : remplissez seulement les champs ci-dessous.
        </p>
      ) : null}
      {editable ? (
        <TripSection
          title="Compléter la page"
          subtitle="Le texte de la page est fixe et identique pour tous les voyages ; vous renseignez seulement les informations ci-dessous."
          icon="✍️"
        >
          <div className="p-6 space-y-6">
            <div className="space-y-3">
              <p className="text-sm font-bold">Rendez-vous et horaires (obligatoires)</p>
              {meetingFields("Aller", outbound, setOutbound, "Heure de dépôt des élèves")}
              {meetingFields("Retour", inbound, setInbound, "Heure de récupération des élèves")}
            </div>

            <div className="space-y-2">
              <p className="text-sm font-bold">Dans la valise</p>
              <div className="flex flex-wrap gap-2">
                {FAMILY_PAGE_TEMPLATE.equipmentSuggestions
                  .filter((s) => !equipment.some((e) => e.toLowerCase() === s.toLowerCase()))
                  .map((s) => (
                    <button key={s} type="button" onClick={() => addItem(s)} className="px-2.5 py-1 rounded-full border border-indigo-200 bg-indigo-50 text-xs font-semibold text-indigo-700 hover:bg-indigo-100">
                      + {s}
                    </button>
                  ))}
              </div>
              <ul className="flex flex-wrap gap-2">
                {equipment.map((e) => (
                  <li key={e} className="flex items-center gap-2 rounded-full bg-slate-100 px-3 py-1 text-sm">
                    {e}
                    <button type="button" className="text-red-500 text-xs" aria-label={`Retirer ${e}`} onClick={() => setEquipment((prev) => prev.filter((x) => x !== e))}>
                      ✕
                    </button>
                  </li>
                ))}
              </ul>
              <div className="flex gap-2">
                <input
                  value={newItem}
                  maxLength={80}
                  placeholder="Ajouter un élément…"
                  onChange={(e) => setNewItem(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addItem(newItem);
                    }
                  }}
                  className="flex-1 p-2.5 bg-white border rounded-xl text-sm"
                />
                <TripButton variant="secondary" size="sm" onClick={() => addItem(newItem)} disabled={!newItem.trim()}>
                  Ajouter
                </TripButton>
              </div>
            </div>

            <div className="space-y-2">
              <p className="text-sm font-bold">Somme en liquide à prévoir (facultatif)</p>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                <input value={cashAmount} inputMode="decimal" placeholder="Montant (€)" onChange={(e) => setCashAmount(e.target.value)} className="p-2.5 bg-white border rounded-xl text-sm" />
                <input value={cashReason} placeholder="Motif (ex. sandwich sur la route du retour)" onChange={(e) => setCashReason(e.target.value)} className="sm:col-span-2 p-2.5 bg-white border rounded-xl text-sm" />
              </div>
              <p className="text-[11px] text-slate-500">Affichée aux familles comme « {FAMILY_PAGE_TEMPLATE.cashNotIncluded} »</p>
            </div>

            <div className="space-y-1">
              <p className="text-sm font-bold">Programme en quelques lignes (facultatif)</p>
              <textarea
                value={program}
                rows={4}
                maxLength={FAMILY_PROGRAM_MAX_CHARS}
                onChange={(e) => setProgram(e.target.value)}
                className="w-full p-3 bg-white border rounded-xl text-sm"
              />
              <p className="text-[11px] text-slate-500">{program.length}/{FAMILY_PROGRAM_MAX_CHARS} caractères</p>
            </div>

            <div>
              <label className="block text-sm font-bold mb-1">Date limite de réponse des familles</label>
              <input type="date" value={deadline} min={deadlineMin} max={deadlineMax} onChange={(e) => setDeadline(e.target.value)} className="p-2.5 bg-white border rounded-xl" />
              <p className="text-[11px] text-slate-500 mt-1">
                Au moins 7 jours après l’ouverture et au plus tard 14 jours avant le départ
                {deadlineMax ? ` (${dateFr(deadlineMax)})` : ""}. Contrôlé par la direction à la validation finale.
              </p>
            </div>

            <div>
              <p className="text-sm font-bold mb-2">Pièces demandées aux familles</p>
              <FamilyDocumentsEditor value={documents} onChange={setDocuments} departureIso={departureIso} />
            </div>
            {departureIso ? <RetroplanningCard departureIso={departureIso} todayIso={todayIso} /> : null}
            <div className="flex flex-wrap items-center gap-3">
              <TripButton variant="primary" onClick={() => void save()} disabled={busy || !dirty}>
                {busy ? "Enregistrement…" : "Enregistrer la page"}
              </TripButton>
              {dirty ? <span className="text-xs text-amber-700 font-semibold">Modifications non enregistrées</span> : null}
            </div>
          </div>
        </TripSection>
      ) : null}

      {(trip.data.preListeEleves?.length ?? 0) > 0 ? (
        <details className="rounded-2xl border bg-white px-4 py-3 text-sm">
          <summary className="cursor-pointer font-bold text-slate-700">
            Élèves invités : {trip.data.preListeEleves!.length} — pré-liste provisoire (la liste officielle sera celle des familles qui auront signé)
          </summary>
          <p className="mt-2 text-slate-600">
            {trip.data.preListeEleves!
              .map((p) => `${p.nom} ${p.prenom}${p.classe ? ` (${p.classe})` : ""}`)
              .join(" · ")}
          </p>
        </details>
      ) : null}

      {message ? (
        <p className={`text-sm font-semibold ${message.tone === "ok" ? "text-emerald-700" : "text-red-700"}`} role="status">
          {message.text}
        </p>
      ) : null}

      </div>

      <div className="min-w-0 xl:sticky xl:top-4">
        <TripSection
          title="Aperçu — la vraie page, telle que les parents la verront"
          subtitle={opened ? "Page ouverte." : "Mis à jour en direct pendant la saisie."}
          icon="👁️"
          action={
            <div className="inline-flex overflow-hidden rounded-lg border text-xs font-bold" role="group" aria-label="Largeur de l'aperçu">
              {(["phone", "desktop"] as const).map((w) => (
                <button
                  key={w}
                  type="button"
                  onClick={() => setPreviewWidth(w)}
                  className={`px-3 py-1.5 ${previewWidth === w ? "bg-indigo-600 text-white" : "bg-white text-slate-600"}`}
                >
                  {w === "phone" ? "Téléphone" : "Ordinateur"}
                </button>
              ))}
            </div>
          }
        >
          <div className="p-3 sm:p-5 bg-slate-100/60">
            <div className={previewWidth === "phone" ? "mx-auto w-full max-w-[390px]" : "w-full"}>
              <FamilyPageView model={model} mode="preview" />
            </div>
          </div>
        </TripSection>
      </div>
      </div>
    </div>
  );
}

/* ─── Validation finale (direction) : aperçu complet avant ouverture ─── */

export function TripOpenRegistrationsModal({
  trip,
  busy,
  onCancel,
  onConfirm,
}: {
  trip: TravelsTrip;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const todayIso = todayIsoParis();
  const readiness = useMemo(() => familyPageReadiness(trip.data, todayIso), [trip.data, todayIso]);
  const model = useMemo(() => buildFamilyPagePreviewModel(trip), [trip]);
  return (
    <div className="fixed inset-0 z-[80] flex items-center justify-center bg-slate-900/60 backdrop-blur-sm p-4">
      <div className="w-full max-w-3xl max-h-[92vh] overflow-y-auto rounded-3xl bg-white p-6 shadow-2xl space-y-4">
        <div>
          <h2 className="text-xl font-black text-slate-900">Validation finale — ouverture des inscriptions</h2>
          <p className="text-sm text-slate-500">
            Relisez l’aperçu complet. Aucun envoi aux familles n’est fait à cette étape ; la page sera simplement verrouillée en écriture.
          </p>
        </div>
        {readiness.ready ? (
          <p className="rounded-xl bg-emerald-50 border border-emerald-200 p-3 text-sm font-semibold text-emerald-800">
            ✅ Texte, bloc prix et date limite sont renseignés et conformes.
          </p>
        ) : (
          <div className="rounded-xl bg-red-50 border border-red-200 p-3 text-sm text-red-800 space-y-1" role="alert">
            <p className="font-bold">Ouverture impossible pour l’instant :</p>
            <ul className="list-disc list-inside">
              {readiness.problems.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          </div>
        )}
        <div className="mx-auto w-full max-w-[420px]">
          <FamilyPageView model={model} mode="preview" />
        </div>
        <div className="flex flex-wrap justify-end gap-3">
          <TripButton variant="secondary" onClick={onCancel} disabled={busy}>
            Annuler
          </TripButton>
          <TripButton variant="success" onClick={onConfirm} disabled={busy || !readiness.ready}>
            {busy ? "Ouverture…" : "Valider et ouvrir les inscriptions"}
          </TripButton>
        </div>
      </div>
    </div>
  );
}
