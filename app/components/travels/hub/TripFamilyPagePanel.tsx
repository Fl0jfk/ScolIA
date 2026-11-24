"use client";

/**
 * Onglet « Page familles » (circuit v2) : rédaction par l'organisateur, aperçu fidèle de ce que verront
 * les parents (bandeau PROJET tant que la page n'est pas ouverte), export PDF filigrané, bloc prix (compta),
 * et fenêtre de validation finale de la direction. Aucun envoi aux familles dans ce lot.
 */

import { useMemo, useState } from "react";
import { TripAlert, TripButton, TripSection } from "@/app/components/travels/TripDetailUI";
import { FamilyDocumentsEditor, newFamilyDocId, RetroplanningCard } from "@/app/components/travels/TravelsV2Creation";
import {
  buildFamilyPagePreviewModel,
  FAMILY_PAGE_PROJECT_BANNER,
  type FamilyPagePreviewModel,
} from "@/app/lib/travels-family-page";
import type {
  TravelsFamilyDocRequest,
  TravelsFamilyPage,
  TravelsPriceBlock,
  TravelsPriceInstallment,
  TravelsTrip,
} from "@/app/lib/travels-types";
import {
  addDaysIso,
  familyPageReadiness,
  isFamilyPageEditableStatus,
  isFamilyPageOpened,
  REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE,
  REGISTRATION_MIN_OPEN_DAYS,
  textMentionsPrice,
  todayIsoParis,
  tripDepartureIso,
  validatePriceBlock,
} from "@/app/lib/travels-v2";

export type SaveResult = { ok: boolean; error?: string };

function eur(n: number): string {
  return `${n.toFixed(2).replace(".", ",")} €`;
}
function dateFr(iso: string): string {
  return /^\d{4}-\d{2}-\d{2}$/.test(iso) ? `${iso.slice(8, 10)}/${iso.slice(5, 7)}/${iso.slice(0, 4)}` : iso;
}

/* ─── Aperçu tel que les parents le verront ─── */

export function FamilyPagePreview({ model }: { model: FamilyPagePreviewModel }) {
  return (
    <div className="relative overflow-hidden rounded-2xl border-2 border-slate-200 bg-white shadow-sm">
      {model.projectBanner ? (
        <>
          <div className="bg-red-600 text-white text-center text-sm font-black tracking-wide uppercase px-3 py-2" role="alert">
            ⚠️ {FAMILY_PAGE_PROJECT_BANNER}
          </div>
          <div
            aria-hidden
            className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center"
          >
            <span className="-rotate-[28deg] text-6xl sm:text-7xl font-black uppercase text-red-600/10 select-none text-center leading-tight">
              Projet<br />non validé
            </span>
          </div>
        </>
      ) : (
        <div className="bg-emerald-600 text-white text-center text-xs font-bold px-3 py-1.5">
          Page ouverte aux inscriptions{model.openedAt ? ` le ${new Date(model.openedAt).toLocaleDateString("fr-FR")}` : ""}
        </div>
      )}
      {model.imageUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={model.imageUrl} alt="" className="w-full h-48 sm:h-60 object-cover" />
      ) : null}
      <div className="p-5 sm:p-8 space-y-5">
        <div>
          <h2 className="text-2xl font-black text-slate-900">{model.title}</h2>
          {model.destination ? <p className="text-slate-500 text-sm mt-1">{model.destination}</p> : null}
          <p className="mt-2 text-indigo-700 font-bold">{model.dateLine}</p>
        </div>
        {model.meetingLines.length > 0 ? (
          <div className="rounded-xl bg-slate-50 border p-3 text-sm space-y-1">
            {model.meetingLines.map((l) => (
              <p key={l}>{l}</p>
            ))}
          </div>
        ) : null}
        <div className="space-y-3 text-slate-700 leading-relaxed">
          {model.hasText ? (
            model.paragraphs.map((p, i) => (
              <p key={i} className="whitespace-pre-line">
                {p}
              </p>
            ))
          ) : (
            <p className="text-slate-400 italic">Le texte de la page n’est pas encore rédigé.</p>
          )}
        </div>
        {model.documents.length > 0 ? (
          <div>
            <h3 className="font-bold text-slate-900 mb-2">Pièces à fournir</h3>
            <ul className="space-y-1 text-sm">
              {model.documents.map((d) => (
                <li key={d.id} className="flex flex-wrap gap-x-2">
                  <span className="font-semibold">{d.label}</span>
                  <span className="text-slate-500">
                    — {d.kindLabel}
                    {d.dueDate ? ` (au plus tard le ${dateFr(d.dueDate)})` : ""}
                  </span>
                  {d.templateName ? <span className="text-xs text-indigo-600">modèle : {d.templateName}</span> : null}
                </li>
              ))}
            </ul>
          </div>
        ) : null}
        <div className="rounded-xl border border-amber-200 bg-amber-50 p-4">
          <h3 className="font-bold text-amber-900 mb-1">Participation financière</h3>
          {model.price ? (
            <div className="text-sm text-amber-950 space-y-1">
              <p className="font-bold">Prix par élève : {eur(model.price.pricePerStudent)}</p>
              {model.price.installments.length > 0 ? (
                <ul className="list-disc list-inside">
                  {model.price.installments.map((i) => (
                    <li key={`${i.dueDate}-${i.amount}`}>
                      {dateFr(i.dueDate)} : {eur(i.amount)}
                    </li>
                  ))}
                </ul>
              ) : null}
              {model.price.paymentText ? <p className="whitespace-pre-line">{model.price.paymentText}</p> : null}
            </div>
          ) : (
            <p className="text-sm text-amber-800 italic">À renseigner par la comptabilité.</p>
          )}
        </div>
        {model.deadline ? (
          <p className="text-sm font-bold text-orange-700">Date limite de réponse : {dateFr(model.deadline)}</p>
        ) : (
          <p className="text-sm italic text-slate-400">Date limite de réponse : à renseigner.</p>
        )}
      </div>
    </div>
  );
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

export function TripFamilyPagePanel({
  trip,
  canEditText,
  saveFamilyPage,
}: {
  trip: TravelsTrip;
  /** Organisateur (ou administratif / admin général / direction). */
  canEditText: boolean;
  saveFamilyPage: (page: TravelsFamilyPage) => Promise<SaveResult>;
}) {
  const stored: TravelsFamilyPage = trip.data.familyPage || { text: "", documents: [] };
  const opened = isFamilyPageOpened(stored);
  const editableStatus = isFamilyPageEditableStatus(String(trip.status));
  const editable = canEditText && editableStatus && !opened;
  const departureIso = tripDepartureIso(trip.data);
  const todayIso = todayIsoParis();

  const [text, setText] = useState(stored.text || "");
  const [deadline, setDeadline] = useState(stored.deadline || "");
  const [documents, setDocuments] = useState<TravelsFamilyDocRequest[]>(stored.documents || []);
  const [busy, setBusy] = useState<null | "save" | "draft" | "pdf">(null);
  const [message, setMessage] = useState<{ tone: "ok" | "error"; text: string } | null>(null);

  const dirty =
    text !== (stored.text || "") ||
    deadline !== (stored.deadline || "") ||
    JSON.stringify(documents) !== JSON.stringify(stored.documents || []);

  const model = useMemo(
    () =>
      buildFamilyPagePreviewModel(trip, {
        familyPage: opened ? undefined : { text, deadline: deadline || undefined, documents },
      }),
    [trip, text, deadline, documents, opened],
  );

  const deadlineMin = addDaysIso(todayIso, REGISTRATION_MIN_OPEN_DAYS);
  const deadlineMax = departureIso ? addDaysIso(departureIso, -REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE) : undefined;

  const save = async () => {
    setBusy("save");
    setMessage(null);
    try {
      const res = await saveFamilyPage({
        ...stored,
        text,
        deadline: deadline || undefined,
        documents,
        authorName: stored.authorName,
        updatedAt: new Date().toISOString(),
      });
      setMessage(res.ok ? { tone: "ok", text: "Page enregistrée." } : { tone: "error", text: res.error || "Enregistrement impossible — réessayez." });
    } finally {
      setBusy(null);
    }
  };

  const generateDraft = async () => {
    if (text.trim() && !confirm("Remplacer le texte actuel par un brouillon généré par l’IA ?")) return;
    setBusy("draft");
    setMessage(null);
    try {
      const res = await fetch("/api/travels/family-page-draft", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId: trip.id }),
      });
      const j = (await res.json().catch(() => ({}))) as { text?: string; error?: string; mentionsPrice?: boolean };
      if (!res.ok || !j.text) {
        setMessage({ tone: "error", text: j.error || "Le brouillon n’a pas pu être généré." });
        return;
      }
      setText(j.text);
      setMessage({
        tone: "ok",
        text: "Brouillon généré : relisez-le, corrigez-le puis enregistrez. Il n’est pas encore enregistré.",
      });
    } finally {
      setBusy(null);
    }
  };

  const exportPdf = async () => {
    setBusy("pdf");
    setMessage(null);
    try {
      const res = await fetch("/api/travels/family-page-pdf", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: trip.id,
          familyPage: opened ? undefined : { text, deadline: deadline || undefined, documents },
        }),
      });
      const j = (await res.json().catch(() => ({}))) as { pdf?: string; filename?: string; error?: string };
      if (!res.ok || !j.pdf) {
        setMessage({ tone: "error", text: j.error || "Export PDF impossible." });
        return;
      }
      const a = document.createElement("a");
      a.href = j.pdf;
      a.download = j.filename || "Page_familles.pdf";
      document.body.appendChild(a);
      a.click();
      a.remove();
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="space-y-6">
      {opened ? (
        <TripAlert tone="success" icon="🔒" title="Page ouverte — verrouillée en écriture">
          La page a été ouverte aux inscriptions le {stored.openedAt ? new Date(stored.openedAt).toLocaleDateString("fr-FR") : "—"}.
          Son texte ne peut plus être modifié.
        </TripAlert>
      ) : !editableStatus ? (
        <TripAlert tone="info" icon="ℹ️" title="Rédaction possible après la validation pédagogique">
          L’organisateur pourra rédiger la page des familles dès que la direction aura validé la pédagogie (en parallèle du transport).
        </TripAlert>
      ) : !canEditText ? (
        <TripAlert tone="muted" icon="👀" title="Lecture seule">
          Seul l’organisateur (ou l’administration) rédige la page. Le prix est saisi par la comptabilité.
        </TripAlert>
      ) : null}

      {editable ? (
        <TripSection title="Rédiger la page" subtitle="Aucun prix dans le texte : le prix vient de la comptabilité." icon="✍️">
          <div className="p-6 space-y-4">
            <div className="flex flex-wrap gap-2">
              <TripButton variant="secondary" size="sm" onClick={() => void generateDraft()} disabled={busy !== null}>
                {busy === "draft" ? "Génération…" : "✨ Brouillon IA à partir du dossier"}
              </TripButton>
            </div>
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              rows={14}
              placeholder="Présentez le séjour aux familles : objectifs, programme, rendez-vous, équipement à prévoir…"
              className="w-full p-3 bg-white border rounded-xl text-sm leading-relaxed"
            />
            {textMentionsPrice(text) ? (
              <p className="text-sm font-semibold text-amber-700" role="alert">
                ⚠️ Le texte semble contenir un prix. Ne l’écrivez pas ici : le prix par élève est saisi par la comptabilité et s’affiche automatiquement.
              </p>
            ) : null}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-xs font-bold mb-1">Date limite de réponse des familles</label>
                <input
                  type="date"
                  value={deadline}
                  min={deadlineMin}
                  max={deadlineMax}
                  onChange={(e) => setDeadline(e.target.value)}
                  className="w-full p-2.5 bg-white border rounded-xl"
                />
                <p className="text-[11px] text-slate-500 mt-1">
                  Au moins 7 jours après l’ouverture et au plus tard 14 jours avant le départ
                  {deadlineMax ? ` (${dateFr(deadlineMax)})` : ""}. Contrôlé par la direction à la validation finale.
                </p>
              </div>
            </div>
            <div>
              <p className="text-sm font-bold mb-2">Pièces demandées aux familles</p>
              <FamilyDocumentsEditor value={documents} onChange={setDocuments} departureIso={departureIso} />
            </div>
            {departureIso ? <RetroplanningCard departureIso={departureIso} todayIso={todayIso} /> : null}
            <div className="flex flex-wrap items-center gap-3">
              <TripButton variant="primary" onClick={() => void save()} disabled={busy !== null || !dirty}>
                {busy === "save" ? "Enregistrement…" : "Enregistrer la page"}
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

      <TripSection
        title="Aperçu — tel que les parents le verront"
        subtitle={opened ? "Page ouverte." : "Brouillon : à ne pas diffuser tant que la direction n’a pas ouvert les inscriptions."}
        icon="👁️"
        action={
          <TripButton variant="secondary" size="sm" onClick={() => void exportPdf()} disabled={busy !== null}>
            {busy === "pdf" ? "Export…" : "Exporter en PDF"}
          </TripButton>
        }
      >
        <div className="p-4 sm:p-6">
          <FamilyPagePreview model={model} />
        </div>
      </TripSection>
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
        <FamilyPagePreview model={model} />
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
