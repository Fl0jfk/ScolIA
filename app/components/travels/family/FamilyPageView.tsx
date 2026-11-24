"use client";

/**
 * Page des familles — composant unique, partagé entre l'aperçu (organisateur, direction) et la page publique (lot 2).
 * - mode "preview" : formulaire visible mais désactivé, aucune donnée envoyée, élève fictif ;
 * - mode "live"    : champs actifs ; le lot 2 branchera les callbacks (envoi du code, signature, dépôt).
 * Les valeurs pas encore connues sont signalées par une étiquette ambre à leur place.
 */

import { useState } from "react";
import {
  FAMILY_PAGE_SAMPLE_STUDENT,
  type FamilyPagePendingItem,
  type FamilyPagePreviewModel,
  dateFr,
} from "@/app/lib/travels-family-page";
import { FAMILY_PAGE_TEMPLATE } from "@/app/lib/travels-family-page-template";

export type FamilyPageViewMode = "preview" | "live";

function eur(n: number): string {
  return `${n.toFixed(2).replace(".", ",")} €`;
}

function Pending({ model, item, className = "" }: { model: FamilyPagePreviewModel; item: FamilyPagePendingItem["key"]; className?: string }) {
  const found = model.pending.find((p) => p.key === item);
  if (!found) return null;
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-lg border border-amber-300 bg-amber-100 px-2.5 py-1 text-xs font-bold text-amber-900 ${className}`}
      data-pending={item}
    >
      ⏳ {found.label}
    </span>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-2">
      <h3 className="text-base font-bold text-slate-900">{title}</h3>
      {children}
    </section>
  );
}

function MeetingRow({ model, label, point, pendingKey }: {
  model: FamilyPagePreviewModel;
  label: string;
  point: FamilyPagePreviewModel["outbound"];
  pendingKey: "outbound" | "inbound";
}) {
  const complete = point && point.place && point.time;
  return (
    <div className="text-sm">
      <span className="font-bold">{label} :</span>{" "}
      {complete ? (
        <span>
          {point.date ? `${dateFr(point.date)} à ` : "à "}
          {point.time} — {point.place}
        </span>
      ) : (
        <Pending model={model} item={pendingKey} />
      )}
    </div>
  );
}

const MEAL_CHOICES = [
  { value: "classique", label: "Panier classique" },
  { value: "sans_porc", label: "Panier sans porc" },
  { value: "vegetarien", label: "Panier végétarien" },
  { value: "famille", label: "Repas fourni par la famille" },
] as const;

export function FamilyPageView({
  model,
  mode,
  onFilesDropped,
}: {
  model: FamilyPagePreviewModel;
  mode: FamilyPageViewMode;
  /** Mode live : dépôt d'une pièce (lot 2). */
  onFilesDropped?: (docId: string, files: File[]) => void;
}) {
  const preview = mode === "preview";
  const h = FAMILY_PAGE_TEMPLATE.headings;
  const [participates, setParticipates] = useState<"oui" | "non" | "">("");
  const disabledInput = "disabled:bg-slate-100 disabled:text-slate-400 disabled:cursor-not-allowed";

  return (
    <article className="overflow-hidden rounded-2xl border-2 border-slate-200 bg-white shadow-sm">
      {preview ? (
        model.previewLabel ? (
          <div className="bg-slate-100 py-1 text-center text-xs font-bold uppercase tracking-wide text-slate-600">
            {model.previewLabel}
          </div>
        ) : (
          <div className="bg-emerald-600 py-1.5 text-center text-xs font-bold text-white">
            Page ouverte aux inscriptions{model.openedAt ? ` le ${new Date(model.openedAt).toLocaleDateString("fr-FR")}` : ""}
          </div>
        )
      ) : null}

      {/* En-tête */}
      {model.imageUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={model.imageUrl} alt="" className="h-44 w-full object-cover sm:h-56" />
      ) : null}
      <div className="space-y-6 p-4 leading-relaxed text-slate-700 sm:p-8">
        <header>
          <h2 className="text-xl font-black text-slate-900 sm:text-2xl">{model.title}</h2>
          {model.destination ? <p className="mt-1 text-sm text-slate-500">{model.destination}</p> : null}
          <p className="mt-2 font-bold text-indigo-700">{model.dateLine}</p>
          <p className="mt-1 text-xs text-slate-500">
            {model.classes ? `Classes : ${model.classes}` : ""}
            {model.classes && model.organizer ? " · " : ""}
            {model.organizer ? `Organisateur : ${model.organizer}` : ""}
          </p>
        </header>

        <div className="space-y-2">
          {model.intro.map((p, i) => (
            <p key={i}>{p}</p>
          ))}
        </div>

        {/* Pratique */}
        <Section title={h.meeting}>
          <div className="space-y-1 rounded-xl border bg-slate-50 p-3">
            <MeetingRow model={model} label={h.outbound} point={model.outbound} pendingKey="outbound" />
            <MeetingRow model={model} label={h.inbound} point={model.inbound} pendingKey="inbound" />
          </div>
        </Section>
        {model.program ? (
          <Section title={h.program}>
            <p className="whitespace-pre-line text-sm">{model.program}</p>
          </Section>
        ) : null}
        {model.equipment.length > 0 ? (
          <Section title={h.equipment}>
            <ul className="list-inside list-disc text-sm">
              {model.equipment.map((e) => (
                <li key={e}>{e}</li>
              ))}
            </ul>
          </Section>
        ) : null}
        {model.cash ? (
          <Section title={h.cash}>
            <p className="text-sm">
              <span className="font-bold">{eur(model.cash.amount)}</span> — {model.cash.reason}
            </p>
            <p className="text-sm font-bold text-orange-700">{FAMILY_PAGE_TEMPLATE.cashNotIncluded}</p>
          </Section>
        ) : null}

        {/* Prix */}
        <section className="space-y-2 rounded-xl border border-amber-200 bg-amber-50 p-4">
          <h3 className="text-base font-bold text-amber-900">{h.price}</h3>
          {model.price ? (
            <div className="space-y-1 text-sm text-amber-950">
              <p className="font-bold">Prix par élève : {eur(model.price.pricePerStudent)}</p>
              {model.price.installments.length > 0 ? (
                <ul className="list-inside list-disc">
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
            <Pending model={model} item="price" />
          )}
        </section>

        {/* Phrases fixes */}
        <Section title={h.help}>
          <p className="text-sm">{model.financialHelp}</p>
        </Section>
        <Section title={h.data}>
          <p className="text-sm">{model.dataRetention}</p>
        </Section>
        <Section title={h.meals}>
          {model.mealsRule.map((p, i) => (
            <p key={i} className="text-sm">
              {p}
            </p>
          ))}
        </Section>

        {/* Formulaire */}
        <section className="space-y-5 rounded-2xl border-2 border-indigo-100 bg-indigo-50/40 p-4">
          <h3 className="text-base font-bold text-indigo-900">Votre réponse</h3>
          {preview ? (
            <p className="text-xs text-indigo-700">
              Aperçu : le formulaire est désactivé. Exemple avec un élève fictif ({FAMILY_PAGE_SAMPLE_STUDENT}).
            </p>
          ) : null}

          <fieldset disabled={preview} className="space-y-2">
            <legend className="text-sm font-bold">
              Mon enfant {preview ? FAMILY_PAGE_SAMPLE_STUDENT : ""} participe au voyage
            </legend>
            <div className="flex gap-4 text-sm">
              {(["oui", "non"] as const).map((v) => (
                <label key={v} className="flex items-center gap-2">
                  <input type="radio" name="participe" value={v} checked={participates === v} onChange={() => setParticipates(v)} />
                  {v === "oui" ? "Oui" : "Non"}
                </label>
              ))}
            </div>
          </fieldset>

          {model.picnicDays.length > 0 ? (
            <fieldset disabled={preview} className="space-y-2">
              <legend className="text-sm font-bold">Repas de mon enfant</legend>
              <p className="text-xs text-slate-600">{model.mealsRule[model.mealsRule.length - 1]}</p>
              <div className="space-y-2">
                {model.picnicDays.map((d) => (
                  <label key={d.key} className="flex flex-col gap-1 text-sm sm:flex-row sm:items-center sm:gap-3">
                    <span className="w-28 font-semibold">{d.label}</span>
                    <select defaultValue="classique" className={`rounded-lg border bg-white p-2 ${disabledInput}`}>
                      {MEAL_CHOICES.map((c) => (
                        <option key={c.value} value={c.value}>
                          {c.label}
                        </option>
                      ))}
                    </select>
                  </label>
                ))}
              </div>
            </fieldset>
          ) : null}

          <fieldset disabled={preview} className="space-y-1">
            <legend className="text-sm font-bold">Téléphone joignable pendant le voyage</legend>
            <input type="tel" inputMode="tel" placeholder="06 00 00 00 00" className={`w-full max-w-xs rounded-lg border bg-white p-2 text-sm ${disabledInput}`} />
          </fieldset>

          {/* Pièces */}
          <div className="space-y-3">
            <h4 className="text-sm font-bold">{h.documents}</h4>
            {model.documents.length === 0 ? (
              <p className="text-xs text-slate-500">Aucune pièce demandée pour ce voyage.</p>
            ) : (
              model.documents.map((d) => (
                <div key={d.id} className="space-y-2 rounded-xl border bg-white p-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-sm font-semibold">{d.label}</span>
                    <span
                      className={`rounded-full px-2 py-0.5 text-[11px] font-bold ${
                        d.kind === "avant_signature"
                          ? "bg-rose-100 text-rose-800"
                          : d.kind === "avant_depart"
                            ? "bg-sky-100 text-sky-800"
                            : "bg-slate-100 text-slate-600"
                      }`}
                    >
                      {d.kindLabel}
                      {d.dueDate ? ` — avant le ${dateFr(d.dueDate)}` : ""}
                    </span>
                    {d.templateName ? <span className="text-xs text-indigo-600">modèle à télécharger : {d.templateName}</span> : null}
                  </div>
                  {preview ? (
                    <div className="rounded-lg border-2 border-dashed border-slate-300 bg-slate-50 px-3 py-4 text-center text-xs text-slate-400" aria-disabled>
                      Glisser-déposer ou prendre une photo
                    </div>
                  ) : (
                    <label className="block cursor-pointer rounded-lg border-2 border-dashed border-indigo-300 bg-indigo-50 px-3 py-4 text-center text-xs font-semibold text-indigo-700">
                      Glisser-déposer ou prendre une photo
                      <input
                        type="file"
                        accept="image/*,application/pdf"
                        className="sr-only"
                        onChange={(e) => onFilesDropped?.(d.id, Array.from(e.target.files || []))}
                      />
                    </label>
                  )}
                </div>
              ))
            )}
          </div>

          {/* Signature */}
          <fieldset disabled={preview} className="space-y-3 rounded-xl border bg-white p-3">
            <legend className="px-1 text-sm font-bold">Signature</legend>
            <input placeholder="Nom et prénom du responsable" className={`w-full rounded-lg border bg-white p-2 text-sm ${disabledInput}`} />
            <div className="flex flex-wrap items-center gap-2">
              <button type="button" className="rounded-lg bg-slate-200 px-3 py-2 text-xs font-bold text-slate-700 disabled:opacity-50">
                Recevoir un code par e-mail
              </button>
              <input inputMode="numeric" maxLength={6} placeholder="Code à 6 chiffres" className={`w-36 rounded-lg border bg-white p-2 text-sm ${disabledInput}`} />
            </div>
            <div className="flex h-28 items-center justify-center rounded-lg border-2 border-dashed border-slate-300 bg-slate-50 text-xs text-slate-400">
              Tracez votre signature ici
            </div>
            <button type="button" className="w-full rounded-xl bg-indigo-600 px-4 py-3 text-sm font-bold text-white disabled:opacity-40 sm:w-auto">
              Signer
            </button>
          </fieldset>
        </section>

        <Section title={h.deadline}>
          {model.deadline ? (
            <p className="text-sm font-bold text-orange-700">{dateFr(model.deadline)}</p>
          ) : (
            <Pending model={model} item="deadline" />
          )}
        </Section>
      </div>
    </article>
  );
}
