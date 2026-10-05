"use client";

import { useState } from "react";
import Link from "next/link";
import PartenariatSignaturePad from "@/app/components/partenariats/PartenariatSignaturePad";
import {
  PARTENARIAT_CYCLE_LABELS,
  formatTarifLine,
  niveauLabel,
  type PartenariatOffrePublicDetail,
} from "@/app/lib/partenariats-types";

type Props = { offre: PartenariatOffrePublicDetail };

export default function PartenariatPublicDetailClient({ offre }: Props) {
  const [eleveFirstName, setEleveFirstName] = useState("");
  const [eleveLastName, setEleveLastName] = useState("");
  const [eleveNiveau, setEleveNiveau] = useState(offre.niveaux[0] || "");
  const [eleveClasse, setEleveClasse] = useState("");
  const [eleveBirthDate, setEleveBirthDate] = useState("");
  const [parentFirstName, setParentFirstName] = useState("");
  const [parentLastName, setParentLastName] = useState("");
  const [parentEmail, setParentEmail] = useState("");
  const [parentPhone, setParentPhone] = useState("");
  const [engagementAccepted, setEngagementAccepted] = useState(false);
  const [signatureDataUrl, setSignatureDataUrl] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const [schoolLogoFailed, setSchoolLogoFailed] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      const res = await fetch(`/api/partenariats/public/${offre.slug}/inscrire`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eleveFirstName,
          eleveLastName,
          eleveNiveau,
          eleveClasse,
          eleveBirthDate: eleveBirthDate || null,
          parentFirstName,
          parentLastName,
          parentEmail,
          parentPhone,
          engagementAccepted,
          signatureDataUrl,
        }),
      });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Inscription impossible.");
      setDone(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  }

  const hasSchoolContact =
    offre.contactName || offre.contactEmail || offre.contactPhone || offre.contactRole;
  const hasPartnerContact =
    offre.partnerContactName ||
    offre.partnerContactEmail ||
    offre.partnerContactPhone ||
    offre.partnerContactRole;

  return (
    <div className="partenariats-public min-h-screen bg-[radial-gradient(1000px_500px_at_0%_0%,#dbeafe_0%,transparent_50%),linear-gradient(180deg,#f8fafc,#eef2ff)]">
      <div className="mx-auto max-w-3xl px-4 py-8 sm:py-12">
        <Link
          href="/partenariats"
          className="text-sm font-bold text-sky-800 hover:underline"
        >
          ← Tous les partenariats
        </Link>

        {/* eslint-disable-next-line @next/next/no-img-element */}
        {!schoolLogoFailed ? (
          <img
            src="/api/site/header-logo"
            alt={offre.schoolName || "Logo de l’établissement"}
            className="mt-6 h-14 w-auto max-w-[200px] object-contain object-left sm:h-16"
            onError={() => setSchoolLogoFailed(true)}
          />
        ) : null}

        <header className={`${schoolLogoFailed ? "mt-6" : "mt-4"} flex flex-col gap-4 sm:flex-row sm:items-start`}>
          {offre.logoUrl ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={offre.logoUrl}
              alt=""
              className="h-20 w-20 rounded-2xl object-contain bg-white p-2 shadow-sm ring-1 ring-slate-200"
            />
          ) : null}
          <div className="min-w-0 space-y-2">
            <p className="text-xs font-bold uppercase tracking-[0.16em] text-sky-800/80">
              {offre.schoolName}
              {offre.categoryLabel ? ` · ${offre.categoryLabel}` : ""}
            </p>
            <h1 className="font-[family-name:var(--font-partenariats-display)] text-3xl font-semibold text-slate-900 sm:text-4xl">
              {offre.title}
            </h1>
            {offre.shortDescription ? (
              <p className="text-base text-slate-600">{offre.shortDescription}</p>
            ) : null}
            <div className="flex flex-wrap gap-1.5 pt-1">
              {offre.cycles.map((c) => (
                <span
                  key={c}
                  className="rounded-md bg-white px-2 py-0.5 text-[11px] font-bold text-slate-600 ring-1 ring-slate-200"
                >
                  {PARTENARIAT_CYCLE_LABELS[c]}
                </span>
              ))}
              {offre.niveaux.map((n) => (
                <span
                  key={n}
                  className="rounded-md bg-sky-50 px-2 py-0.5 text-[11px] font-bold text-sky-800"
                >
                  {niveauLabel(n)}
                </span>
              ))}
            </div>
          </div>
        </header>

        {offre.body ? (
          <section className="mt-8 whitespace-pre-wrap rounded-2xl bg-white/90 p-5 text-sm leading-relaxed text-slate-700 shadow-sm ring-1 ring-slate-200/80">
            {offre.body}
          </section>
        ) : null}

        {(hasSchoolContact || hasPartnerContact) && (
          <section className="mt-6 grid gap-4 sm:grid-cols-2">
            {hasSchoolContact ? (
              <div className="rounded-2xl bg-white/90 p-5 shadow-sm ring-1 ring-slate-200/80">
                <h2 className="text-xs font-black uppercase tracking-wide text-slate-500">
                  Interlocuteur établissement
                </h2>
                <p className="mt-2 text-base font-bold text-slate-900">
                  {offre.contactName || "—"}
                </p>
                {offre.contactRole ? (
                  <p className="text-sm text-slate-600">{offre.contactRole}</p>
                ) : null}
                {offre.contactEmail ? (
                  <a
                    href={`mailto:${offre.contactEmail}`}
                    className="mt-2 block text-sm font-semibold text-sky-800 hover:underline"
                  >
                    {offre.contactEmail}
                  </a>
                ) : null}
                {offre.contactPhone ? (
                  <a
                    href={`tel:${offre.contactPhone}`}
                    className="block text-sm font-semibold text-slate-700"
                  >
                    {offre.contactPhone}
                  </a>
                ) : null}
              </div>
            ) : null}
            {hasPartnerContact ? (
              <div className="rounded-2xl bg-white/90 p-5 shadow-sm ring-1 ring-slate-200/80">
                <h2 className="text-xs font-black uppercase tracking-wide text-slate-500">
                  Interlocuteur partenaire
                </h2>
                <p className="mt-2 text-base font-bold text-slate-900">
                  {offre.partnerContactName || "—"}
                </p>
                {offre.partnerContactRole ? (
                  <p className="text-sm text-slate-600">{offre.partnerContactRole}</p>
                ) : null}
                {offre.partnerContactEmail ? (
                  <a
                    href={`mailto:${offre.partnerContactEmail}`}
                    className="mt-2 block text-sm font-semibold text-sky-800 hover:underline"
                  >
                    {offre.partnerContactEmail}
                  </a>
                ) : null}
                {offre.partnerContactPhone ? (
                  <a
                    href={`tel:${offre.partnerContactPhone}`}
                    className="block text-sm font-semibold text-slate-700"
                  >
                    {offre.partnerContactPhone}
                  </a>
                ) : null}
              </div>
            ) : null}
          </section>
        )}

        {offre.ctaLinks.length > 0 ? (
          <section className="mt-6 flex flex-wrap gap-2">
            {offre.ctaLinks.map((l, i) => (
              <a
                key={`${l.url}-${i}`}
                href={l.url}
                target={l.url.startsWith("mailto:") ? undefined : "_blank"}
                rel="noreferrer"
                className="rounded-xl bg-slate-900 px-4 py-2.5 text-sm font-bold text-white hover:bg-slate-800"
              >
                {l.label}
              </a>
            ))}
          </section>
        ) : null}

        {offre.tarifs.length > 0 ? (
          <section className="mt-6 rounded-2xl bg-white/90 p-5 shadow-sm ring-1 ring-slate-200/80">
            <h2 className="text-sm font-black text-slate-900">Tarifs</h2>
            <ul className="mt-3 space-y-2">
              {offre.tarifs.map((t, i) => (
                <li key={i} className="text-sm text-slate-700">
                  <span className="font-semibold">{formatTarifLine(t)}</span>
                  {t.note ? <span className="block text-xs text-slate-500">{t.note}</span> : null}
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        {offre.demarche ? (
          <section className="mt-6 rounded-2xl bg-white/90 p-5 shadow-sm ring-1 ring-slate-200/80">
            <h2 className="text-sm font-black text-slate-900">Démarche</h2>
            <p className="mt-2 whitespace-pre-wrap text-sm leading-relaxed text-slate-700">
              {offre.demarche}
            </p>
          </section>
        ) : null}

        {offre.evenements.length > 0 ? (
          <section className="mt-6 rounded-2xl bg-white/90 p-5 shadow-sm ring-1 ring-slate-200/80">
            <h2 className="text-sm font-black text-slate-900">Prochaines réunions</h2>
            <ul className="mt-3 space-y-3">
              {offre.evenements.map((ev) => (
                <li key={ev.id} className="rounded-xl bg-slate-50 px-3 py-2">
                  <p className="text-sm font-bold text-slate-900">{ev.title}</p>
                  <p className="text-xs text-slate-600">
                    {new Date(ev.startsAt).toLocaleString("fr-FR", {
                      dateStyle: "full",
                      timeStyle: "short",
                    })}
                    {ev.location ? ` — ${ev.location}` : ""}
                  </p>
                  {ev.notes ? <p className="mt-1 text-xs text-slate-500">{ev.notes}</p> : null}
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        {offre.kind === "inscription" ? (
          <section className="mt-8 rounded-2xl bg-white p-5 shadow-md ring-1 ring-slate-200">
            <h2 className="text-lg font-black text-slate-900">Inscription en ligne</h2>
            {typeof offre.placesRemaining === "number" ? (
              <p className="mt-1 text-xs font-semibold text-slate-500">
                Places restantes : {offre.placesRemaining}
                {offre.maxPlaces != null ? ` / ${offre.maxPlaces}` : ""}
              </p>
            ) : null}

            {!offre.inscriptionOpen ? (
              <p className="mt-4 rounded-xl bg-amber-50 px-4 py-3 text-sm text-amber-900">
                Les inscriptions sont actuellement fermées pour cette offre.
              </p>
            ) : done ? (
              <p className="mt-4 rounded-xl bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
                Votre inscription a bien été enregistrée. Un e-mail de confirmation peut vous être
                envoyé. L’établissement vous recontactera si besoin.
              </p>
            ) : (
              <form onSubmit={(e) => void submit(e)} className="mt-4 space-y-4">
                <div className="grid gap-3 sm:grid-cols-2">
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    Prénom de l’élève *
                    <input
                      required
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={eleveFirstName}
                      onChange={(e) => setEleveFirstName(e.target.value)}
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    Nom de l’élève *
                    <input
                      required
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={eleveLastName}
                      onChange={(e) => setEleveLastName(e.target.value)}
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    Niveau
                    <select
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={eleveNiveau}
                      onChange={(e) => setEleveNiveau(e.target.value)}
                    >
                      <option value="">—</option>
                      {(offre.niveaux.length
                        ? offre.niveaux
                        : ["6eme", "5eme", "4eme", "3eme", "2nde", "1ere", "terminale"]
                      ).map((n) => (
                        <option key={n} value={n}>
                          {niveauLabel(n)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    Classe
                    <input
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={eleveClasse}
                      onChange={(e) => setEleveClasse(e.target.value)}
                      placeholder="ex. 3B"
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    Date de naissance
                    <input
                      type="date"
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={eleveBirthDate}
                      onChange={(e) => setEleveBirthDate(e.target.value)}
                    />
                  </label>
                </div>

                <div className="grid gap-3 sm:grid-cols-2">
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    Prénom du parent *
                    <input
                      required
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={parentFirstName}
                      onChange={(e) => setParentFirstName(e.target.value)}
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    Nom du parent *
                    <input
                      required
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={parentLastName}
                      onChange={(e) => setParentLastName(e.target.value)}
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    E-mail *
                    <input
                      required
                      type="email"
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={parentEmail}
                      onChange={(e) => setParentEmail(e.target.value)}
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
                    Téléphone
                    <input
                      className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
                      value={parentPhone}
                      onChange={(e) => setParentPhone(e.target.value)}
                    />
                  </label>
                </div>

                <label className="flex items-start gap-2 text-sm text-slate-700">
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={engagementAccepted}
                    onChange={(e) => setEngagementAccepted(e.target.checked)}
                    required
                  />
                  <span>{offre.engagementText}</span>
                </label>

                {offre.requireSignature ? (
                  <div>
                    <p className="mb-2 text-xs font-semibold text-slate-600">
                      Signature du responsable légal *
                    </p>
                    <PartenariatSignaturePad onChange={setSignatureDataUrl} />
                  </div>
                ) : null}

                {error ? (
                  <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
                    {error}
                  </p>
                ) : null}

                <button
                  type="submit"
                  disabled={submitting}
                  className="rounded-xl bg-sky-700 px-5 py-2.5 text-sm font-bold text-white hover:bg-sky-800 disabled:opacity-60"
                >
                  {submitting ? "Envoi…" : "Valider mon inscription"}
                </button>
              </form>
            )}
          </section>
        ) : null}
      </div>
    </div>
  );
}
