"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import RequireModuleAccess from "@/app/components/RequireModuleAccess";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import {
  PARTENARIAT_CYCLES,
  PARTENARIAT_CYCLE_LABELS,
  PARTENARIAT_NIVEAUX_BY_CYCLE,
  type PartenariatCtaLink,
  type PartenariatCycle,
  type PartenariatEvenementRecord,
  type PartenariatInscriptionRecord,
  type PartenariatInscriptionStatus,
  type PartenariatKind,
  type PartenariatOffreRecord,
  type PartenariatTarif,
} from "@/app/lib/partenariats-types";

type Props = { offreId: string };

function fieldClass() {
  return "w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-medium text-slate-900";
}

function labelClass() {
  return "flex flex-col gap-1 text-xs font-semibold text-slate-600";
}

export default function PartenariatOffreAdminClient({ offreId }: Props) {
  const [offre, setOffre] = useState<PartenariatOffreRecord | null>(null);
  const [evenements, setEvenements] = useState<PartenariatEvenementRecord[]>([]);
  const [inscriptions, setInscriptions] = useState<PartenariatInscriptionRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);
  const [uploading, setUploading] = useState(false);

  const [title, setTitle] = useState("");
  const [slug, setSlug] = useState("");
  const [shortDescription, setShortDescription] = useState("");
  const [body, setBody] = useState("");
  const [kind, setKind] = useState<PartenariatKind>("info");
  const [cycles, setCycles] = useState<PartenariatCycle[]>([]);
  const [niveaux, setNiveaux] = useState<string[]>([]);
  const [categoryLabel, setCategoryLabel] = useState("");
  const [contactName, setContactName] = useState("");
  const [contactRole, setContactRole] = useState("");
  const [contactEmail, setContactEmail] = useState("");
  const [contactPhone, setContactPhone] = useState("");
  const [partnerContactName, setPartnerContactName] = useState("");
  const [partnerContactRole, setPartnerContactRole] = useState("");
  const [partnerContactEmail, setPartnerContactEmail] = useState("");
  const [partnerContactPhone, setPartnerContactPhone] = useState("");
  const [ctaLinks, setCtaLinks] = useState<PartenariatCtaLink[]>([]);
  const [tarifs, setTarifs] = useState<PartenariatTarif[]>([]);
  const [demarche, setDemarche] = useState("");
  const [engagementText, setEngagementText] = useState("");
  const [requireSignature, setRequireSignature] = useState(true);
  const [notifyEmail, setNotifyEmail] = useState("");
  const [maxPlaces, setMaxPlaces] = useState("");
  const [inscriptionOpensAt, setInscriptionOpensAt] = useState("");
  const [inscriptionClosesAt, setInscriptionClosesAt] = useState("");
  const [enabled, setEnabled] = useState(false);
  const [sortOrder, setSortOrder] = useState("0");
  const [logoUrl, setLogoUrl] = useState<string | null>(null);

  const [evTitle, setEvTitle] = useState("Réunion d’information");
  const [evStarts, setEvStarts] = useState("");
  const [evLocation, setEvLocation] = useState("");
  const [evNotes, setEvNotes] = useState("");
  const [addingEvent, setAddingEvent] = useState(false);

  const availableNiveaux = useMemo(() => {
    const selected = cycles.length ? cycles : [...PARTENARIAT_CYCLES];
    return selected.flatMap((c) => PARTENARIAT_NIVEAUX_BY_CYCLE[c]);
  }, [cycles]);

  function applyOffre(o: PartenariatOffreRecord) {
    setOffre(o);
    setTitle(o.title);
    setSlug(o.slug);
    setShortDescription(o.shortDescription);
    setBody(o.body);
    setKind(o.kind);
    setCycles(o.cycles);
    setNiveaux(o.niveaux);
    setCategoryLabel(o.categoryLabel);
    setContactName(o.contactName);
    setContactRole(o.contactRole);
    setContactEmail(o.contactEmail);
    setContactPhone(o.contactPhone);
    setPartnerContactName(o.partnerContactName);
    setPartnerContactRole(o.partnerContactRole);
    setPartnerContactEmail(o.partnerContactEmail);
    setPartnerContactPhone(o.partnerContactPhone);
    setCtaLinks(o.ctaLinks.length ? o.ctaLinks : []);
    setTarifs(o.tarifs.length ? o.tarifs : []);
    setDemarche(o.demarche);
    setEngagementText(o.engagementText);
    setRequireSignature(o.requireSignature);
    setNotifyEmail(o.notifyEmail || "");
    setMaxPlaces(o.maxPlaces != null ? String(o.maxPlaces) : "");
    setInscriptionOpensAt(o.inscriptionOpensAt ? o.inscriptionOpensAt.slice(0, 16) : "");
    setInscriptionClosesAt(o.inscriptionClosesAt ? o.inscriptionClosesAt.slice(0, 16) : "");
    setEnabled(o.enabled);
    setSortOrder(String(o.sortOrder));
    setLogoUrl(o.logoUrl);
  }

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/partenariats/offres/${offreId}`, { cache: "no-store" });
      const data = (await res.json()) as {
        offre?: PartenariatOffreRecord;
        evenements?: PartenariatEvenementRecord[];
        inscriptions?: PartenariatInscriptionRecord[];
        error?: string;
      };
      if (!res.ok) throw new Error(data.error || "Chargement impossible.");
      if (!data.offre) throw new Error("Offre introuvable.");
      applyOffre(data.offre);
      setEvenements(data.evenements || []);
      setInscriptions(data.inscriptions || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [offreId]);

  useEffect(() => {
    void load();
  }, [load]);

  function toggleCycle(c: PartenariatCycle) {
    setCycles((prev) => (prev.includes(c) ? prev.filter((x) => x !== c) : [...prev, c]));
  }

  function toggleNiveau(id: string) {
    setNiveaux((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
  }

  async function save() {
    setSaving(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch(`/api/partenariats/offres/${offreId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title,
          slug,
          shortDescription,
          body,
          kind,
          cycles,
          niveaux,
          categoryLabel,
          contactName,
          contactRole,
          contactEmail,
          contactPhone,
          partnerContactName,
          partnerContactRole,
          partnerContactEmail,
          partnerContactPhone,
          ctaLinks: ctaLinks.filter((l) => l.label.trim() && l.url.trim()),
          tarifs: tarifs.filter((t) => t.label.trim()),
          demarche,
          engagementText,
          requireSignature,
          notifyEmail: notifyEmail.trim() || null,
          maxPlaces: maxPlaces.trim() ? Number(maxPlaces) : null,
          inscriptionOpensAt: inscriptionOpensAt
            ? new Date(inscriptionOpensAt).toISOString()
            : null,
          inscriptionClosesAt: inscriptionClosesAt
            ? new Date(inscriptionClosesAt).toISOString()
            : null,
          enabled,
          sortOrder: Number(sortOrder) || 0,
        }),
      });
      const data = (await res.json()) as { offre?: PartenariatOffreRecord; error?: string };
      if (!res.ok) throw new Error(data.error || "Enregistrement impossible.");
      if (data.offre) applyOffre(data.offre);
      setOkMsg("Enregistré.");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }

  async function onLogoChange(file: File | null) {
    if (!file) return;
    setUploading(true);
    setError(null);
    try {
      const fd = new FormData();
      fd.set("file", file);
      const res = await fetch(`/api/partenariats/offres/${offreId}/logo`, {
        method: "POST",
        body: fd,
      });
      const data = (await res.json()) as { url?: string; error?: string };
      if (!res.ok) throw new Error(data.error || "Upload impossible.");
      setLogoUrl(data.url || null);
      setOkMsg("Logo mis à jour.");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setUploading(false);
    }
  }

  async function addEvent() {
    if (!evStarts.trim()) {
      setError("Indiquez la date de la réunion.");
      return;
    }
    setAddingEvent(true);
    setError(null);
    try {
      const res = await fetch(`/api/partenariats/offres/${offreId}/evenements`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title: evTitle.trim() || "Réunion",
          startsAt: new Date(evStarts).toISOString(),
          location: evLocation,
          notes: evNotes,
        }),
      });
      const data = (await res.json()) as {
        evenement?: PartenariatEvenementRecord;
        error?: string;
      };
      if (!res.ok) throw new Error(data.error || "Ajout impossible.");
      if (data.evenement) setEvenements((prev) => [...prev, data.evenement!]);
      setEvNotes("");
      setOkMsg("Réunion ajoutée.");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setAddingEvent(false);
    }
  }

  async function removeEvent(eventId: string) {
    if (!window.confirm("Supprimer cette réunion ?")) return;
    setError(null);
    try {
      const res = await fetch(
        `/api/partenariats/offres/${offreId}/evenements/${eventId}`,
        { method: "DELETE" },
      );
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Suppression impossible.");
      setEvenements((prev) => prev.filter((e) => e.id !== eventId));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function setInscriptionStatus(
    inscriptionId: string,
    status: PartenariatInscriptionStatus,
  ) {
    setError(null);
    try {
      const res = await fetch(`/api/partenariats/offres/${offreId}/inscriptions`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ inscriptionId, status }),
      });
      const data = (await res.json()) as {
        inscription?: PartenariatInscriptionRecord;
        error?: string;
      };
      if (!res.ok) throw new Error(data.error || "Mise à jour impossible.");
      if (data.inscription) {
        if (status === "annulee") {
          setInscriptions((prev) => prev.filter((i) => i.id !== inscriptionId));
        } else {
          setInscriptions((prev) =>
            prev.map((i) => (i.id === inscriptionId ? data.inscription! : i)),
          );
        }
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function deleteOffre() {
    if (!window.confirm("Supprimer définitivement cette fiche et ses inscriptions ?")) return;
    setError(null);
    try {
      const res = await fetch(`/api/partenariats/offres/${offreId}`, { method: "DELETE" });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Suppression impossible.");
      window.location.href = "/etablissement/partenariats";
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  if (loading) {
    return (
      <RequireModuleAccess moduleId="partenariats">
        <ModulePageShell>
          <p className="text-sm text-slate-500">Chargement…</p>
        </ModulePageShell>
      </RequireModuleAccess>
    );
  }

  if (!offre) {
    return (
      <RequireModuleAccess moduleId="partenariats">
        <ModulePageShell>
          <p className="text-sm text-rose-700">{error || "Offre introuvable."}</p>
          <Link href="/etablissement/partenariats" className="text-sm font-bold text-sky-700">
            Retour
          </Link>
        </ModulePageShell>
      </RequireModuleAccess>
    );
  }

  return (
    <RequireModuleAccess moduleId="partenariats">
      <ModulePageShell>
        <ModulePageHeader
          eyebrow="Partenariats & offres"
          title={offre.title}
          description={`Lien public : /partenariats/${offre.slug}`}
          actions={
            <div className="flex flex-wrap gap-2">
              <Link
                href="/etablissement/partenariats"
                className="rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Liste
              </Link>
              <a
                href={`/partenariats/${offre.slug}`}
                target="_blank"
                rel="noreferrer"
                className="rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Aperçu public
              </a>
              <ModuleButton type="button" onClick={() => void save()} disabled={saving}>
                {saving ? "Enregistrement…" : "Enregistrer"}
              </ModuleButton>
            </div>
          }
        />

        {error ? (
          <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
            {error}
          </p>
        ) : null}
        {okMsg ? (
          <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
            {okMsg}
          </p>
        ) : null}

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="text-sm font-black text-slate-900">Identité</h2>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className={labelClass()}>
              Titre
              <input className={fieldClass()} value={title} onChange={(e) => setTitle(e.target.value)} />
            </label>
            <label className={labelClass()}>
              Slug URL
              <input className={fieldClass()} value={slug} onChange={(e) => setSlug(e.target.value)} />
            </label>
            <label className={labelClass()}>
              Catégorie
              <input
                className={fieldClass()}
                value={categoryLabel}
                onChange={(e) => setCategoryLabel(e.target.value)}
                placeholder="Sport, Certification, International…"
              />
            </label>
            <label className={labelClass()}>
              Type
              <select
                className={fieldClass()}
                value={kind}
                onChange={(e) =>
                  setKind(e.target.value === "inscription" ? "inscription" : "info")
                }
              >
                <option value="info">Information / partenaire</option>
                <option value="inscription">Inscription en ligne (coupon)</option>
              </select>
            </label>
            <label className={`${labelClass()} sm:col-span-2`}>
              Accroche courte
              <textarea
                className={fieldClass()}
                rows={2}
                value={shortDescription}
                onChange={(e) => setShortDescription(e.target.value)}
              />
            </label>
            <label className={`${labelClass()} sm:col-span-2`}>
              Description détaillée
              <textarea
                className={fieldClass()}
                rows={6}
                value={body}
                onChange={(e) => setBody(e.target.value)}
                placeholder="Ce qu’est le partenariat, ce qu’il apporte à l’élève…"
              />
            </label>
          </div>

          <div className="flex flex-wrap items-center gap-4">
            <label className="inline-flex items-center gap-2 text-sm font-semibold text-slate-700">
              <input
                type="checkbox"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
              />
              Publié sur la page publique
            </label>
            <label className={labelClass()}>
              Ordre d’affichage
              <input
                className={`${fieldClass()} w-24`}
                value={sortOrder}
                onChange={(e) => setSortOrder(e.target.value)}
              />
            </label>
          </div>

          <div className="flex flex-wrap items-center gap-4">
            {logoUrl ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                src={logoUrl}
                alt="Logo"
                className="h-16 w-16 rounded-xl border border-slate-200 object-contain bg-white"
              />
            ) : (
              <div className="flex h-16 w-16 items-center justify-center rounded-xl border border-dashed border-slate-300 text-[10px] text-slate-400">
                Logo
              </div>
            )}
            <label className="text-xs font-semibold text-slate-600">
              {uploading ? "Upload…" : "Changer le logo"}
              <input
                type="file"
                accept="image/png,image/jpeg,image/webp,image/svg+xml"
                className="mt-1 block text-xs"
                disabled={uploading}
                onChange={(e) => void onLogoChange(e.target.files?.[0] || null)}
              />
            </label>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="text-sm font-black text-slate-900">Ciblage cycle / niveau</h2>
          <p className="text-xs text-slate-500">
            Les familles pourront filtrer le catalogue. Si aucun niveau n’est coché, tous les
            niveaux des cycles sélectionnés sont concernés.
          </p>
          <div className="flex flex-wrap gap-3">
            {PARTENARIAT_CYCLES.map((c) => (
              <label
                key={c}
                className="inline-flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-sm font-semibold text-slate-700"
              >
                <input
                  type="checkbox"
                  checked={cycles.includes(c)}
                  onChange={() => toggleCycle(c)}
                />
                {PARTENARIAT_CYCLE_LABELS[c]}
              </label>
            ))}
          </div>
          <div className="flex flex-wrap gap-2">
            {availableNiveaux.map((n) => (
              <label
                key={n.id}
                className={`inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs font-semibold ${
                  niveaux.includes(n.id)
                    ? "bg-sky-50 text-sky-800 ring-1 ring-sky-200"
                    : "bg-slate-50 text-slate-600"
                }`}
              >
                <input
                  type="checkbox"
                  checked={niveaux.includes(n.id)}
                  onChange={() => toggleNiveau(n.id)}
                />
                {n.label}
              </label>
            ))}
          </div>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="text-sm font-black text-slate-900">Interlocuteurs</h2>
          <div className="grid gap-4 lg:grid-cols-2">
            <div className="space-y-3 rounded-xl bg-slate-50 p-4">
              <h3 className="text-xs font-black uppercase tracking-wide text-slate-500">
                Côté établissement
              </h3>
              <label className={labelClass()}>
                Nom
                <input className={fieldClass()} value={contactName} onChange={(e) => setContactName(e.target.value)} />
              </label>
              <label className={labelClass()}>
                Fonction
                <input className={fieldClass()} value={contactRole} onChange={(e) => setContactRole(e.target.value)} />
              </label>
              <label className={labelClass()}>
                E-mail
                <input className={fieldClass()} value={contactEmail} onChange={(e) => setContactEmail(e.target.value)} />
              </label>
              <label className={labelClass()}>
                Téléphone
                <input className={fieldClass()} value={contactPhone} onChange={(e) => setContactPhone(e.target.value)} />
              </label>
            </div>
            <div className="space-y-3 rounded-xl bg-slate-50 p-4">
              <h3 className="text-xs font-black uppercase tracking-wide text-slate-500">
                Côté partenaire (optionnel)
              </h3>
              <label className={labelClass()}>
                Nom
                <input
                  className={fieldClass()}
                  value={partnerContactName}
                  onChange={(e) => setPartnerContactName(e.target.value)}
                />
              </label>
              <label className={labelClass()}>
                Fonction
                <input
                  className={fieldClass()}
                  value={partnerContactRole}
                  onChange={(e) => setPartnerContactRole(e.target.value)}
                />
              </label>
              <label className={labelClass()}>
                E-mail
                <input
                  className={fieldClass()}
                  value={partnerContactEmail}
                  onChange={(e) => setPartnerContactEmail(e.target.value)}
                />
              </label>
              <label className={labelClass()}>
                Téléphone
                <input
                  className={fieldClass()}
                  value={partnerContactPhone}
                  onChange={(e) => setPartnerContactPhone(e.target.value)}
                />
              </label>
            </div>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <div className="flex items-center justify-between gap-2">
            <h2 className="text-sm font-black text-slate-900">Liens utiles (CTA)</h2>
            <button
              type="button"
              className="text-xs font-bold text-sky-700"
              onClick={() => setCtaLinks((prev) => [...prev, { label: "", url: "" }])}
            >
              + Lien
            </button>
          </div>
          {ctaLinks.length === 0 ? (
            <p className="text-xs text-slate-500">Aucun lien — préinscription, site club, mailto…</p>
          ) : (
            <div className="space-y-2">
              {ctaLinks.map((link, idx) => (
                <div key={idx} className="flex flex-wrap gap-2">
                  <input
                    className={`${fieldClass()} min-w-[140px] flex-1`}
                    placeholder="Libellé"
                    value={link.label}
                    onChange={(e) =>
                      setCtaLinks((prev) =>
                        prev.map((l, i) => (i === idx ? { ...l, label: e.target.value } : l)),
                      )
                    }
                  />
                  <input
                    className={`${fieldClass()} min-w-[200px] flex-[2]`}
                    placeholder="https://… ou mailto:…"
                    value={link.url}
                    onChange={(e) =>
                      setCtaLinks((prev) =>
                        prev.map((l, i) => (i === idx ? { ...l, url: e.target.value } : l)),
                      )
                    }
                  />
                  <button
                    type="button"
                    className="text-xs font-bold text-rose-600"
                    onClick={() => setCtaLinks((prev) => prev.filter((_, i) => i !== idx))}
                  >
                    Retirer
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>

        {kind === "inscription" ? (
          <>
            <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
              <h2 className="text-sm font-black text-slate-900">Inscription (coupon numérique)</h2>
              <div className="grid gap-3 sm:grid-cols-2">
                <label className={labelClass()}>
                  E-mail de notification
                  <input
                    className={fieldClass()}
                    value={notifyEmail}
                    onChange={(e) => setNotifyEmail(e.target.value)}
                    placeholder="referent@etablissement.fr"
                  />
                </label>
                <label className={labelClass()}>
                  Places max (vide = illimité)
                  <input
                    className={fieldClass()}
                    value={maxPlaces}
                    onChange={(e) => setMaxPlaces(e.target.value)}
                  />
                </label>
                <label className={labelClass()}>
                  Ouverture inscriptions
                  <input
                    type="datetime-local"
                    className={fieldClass()}
                    value={inscriptionOpensAt}
                    onChange={(e) => setInscriptionOpensAt(e.target.value)}
                  />
                </label>
                <label className={labelClass()}>
                  Fermeture inscriptions
                  <input
                    type="datetime-local"
                    className={fieldClass()}
                    value={inscriptionClosesAt}
                    onChange={(e) => setInscriptionClosesAt(e.target.value)}
                  />
                </label>
                <label className={`${labelClass()} sm:col-span-2`}>
                  Démarche à suivre
                  <textarea
                    className={fieldClass()}
                    rows={4}
                    value={demarche}
                    onChange={(e) => setDemarche(e.target.value)}
                  />
                </label>
                <label className={`${labelClass()} sm:col-span-2`}>
                  Texte d’engagement
                  <textarea
                    className={fieldClass()}
                    rows={3}
                    value={engagementText}
                    onChange={(e) => setEngagementText(e.target.value)}
                  />
                </label>
                <label className="inline-flex items-center gap-2 text-sm font-semibold text-slate-700 sm:col-span-2">
                  <input
                    type="checkbox"
                    checked={requireSignature}
                    onChange={(e) => setRequireSignature(e.target.checked)}
                  />
                  Exiger une signature manuscrite (pad)
                </label>
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <h3 className="text-xs font-black uppercase tracking-wide text-slate-500">
                    Tarifs
                  </h3>
                  <button
                    type="button"
                    className="text-xs font-bold text-sky-700"
                    onClick={() =>
                      setTarifs((prev) => [...prev, { label: "", amountLabel: "" }])
                    }
                  >
                    + Tarif
                  </button>
                </div>
                {tarifs.map((t, idx) => (
                  <div key={idx} className="flex flex-wrap gap-2">
                    <input
                      className={`${fieldClass()} min-w-[160px] flex-1`}
                      placeholder="Libellé"
                      value={t.label}
                      onChange={(e) =>
                        setTarifs((prev) =>
                          prev.map((row, i) =>
                            i === idx ? { ...row, label: e.target.value } : row,
                          ),
                        )
                      }
                    />
                    <input
                      className={`${fieldClass()} w-36`}
                      placeholder="ex. 180 €"
                      value={t.amountLabel || ""}
                      onChange={(e) =>
                        setTarifs((prev) =>
                          prev.map((row, i) =>
                            i === idx ? { ...row, amountLabel: e.target.value } : row,
                          ),
                        )
                      }
                    />
                    <input
                      className={`${fieldClass()} min-w-[160px] flex-1`}
                      placeholder="Note"
                      value={t.note || ""}
                      onChange={(e) =>
                        setTarifs((prev) =>
                          prev.map((row, i) =>
                            i === idx ? { ...row, note: e.target.value } : row,
                          ),
                        )
                      }
                    />
                    <button
                      type="button"
                      className="text-xs font-bold text-rose-600"
                      onClick={() => setTarifs((prev) => prev.filter((_, i) => i !== idx))}
                    >
                      Retirer
                    </button>
                  </div>
                ))}
              </div>
            </section>

            <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
              <h2 className="text-sm font-black text-slate-900">Réunions / événements</h2>
              <div className="grid gap-3 sm:grid-cols-2">
                <label className={labelClass()}>
                  Titre
                  <input className={fieldClass()} value={evTitle} onChange={(e) => setEvTitle(e.target.value)} />
                </label>
                <label className={labelClass()}>
                  Date et heure
                  <input
                    type="datetime-local"
                    className={fieldClass()}
                    value={evStarts}
                    onChange={(e) => setEvStarts(e.target.value)}
                  />
                </label>
                <label className={labelClass()}>
                  Lieu
                  <input
                    className={fieldClass()}
                    value={evLocation}
                    onChange={(e) => setEvLocation(e.target.value)}
                  />
                </label>
                <label className={labelClass()}>
                  Notes
                  <input className={fieldClass()} value={evNotes} onChange={(e) => setEvNotes(e.target.value)} />
                </label>
              </div>
              <ModuleButton type="button" onClick={() => void addEvent()} disabled={addingEvent}>
                {addingEvent ? "Ajout…" : "Ajouter la réunion"}
              </ModuleButton>
              <ul className="space-y-2">
                {evenements.map((ev) => (
                  <li
                    key={ev.id}
                    className="flex flex-wrap items-start justify-between gap-2 rounded-xl border border-slate-100 bg-slate-50 px-3 py-2"
                  >
                    <div>
                      <p className="text-sm font-bold text-slate-900">{ev.title}</p>
                      <p className="text-xs text-slate-600">
                        {new Date(ev.startsAt).toLocaleString("fr-FR")}
                        {ev.location ? ` — ${ev.location}` : ""}
                      </p>
                      {ev.notes ? <p className="text-xs text-slate-500">{ev.notes}</p> : null}
                    </div>
                    <button
                      type="button"
                      className="text-xs font-bold text-rose-600"
                      onClick={() => void removeEvent(ev.id)}
                    >
                      Supprimer
                    </button>
                  </li>
                ))}
              </ul>
            </section>

            <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h2 className="text-sm font-black text-slate-900">
                  Inscriptions ({inscriptions.length})
                </h2>
                <a
                  href={`/api/partenariats/offres/${offreId}/export`}
                  className="text-xs font-bold text-sky-700 hover:underline"
                >
                  Export CSV
                </a>
              </div>
              {inscriptions.length === 0 ? (
                <p className="text-sm text-slate-500">Aucune inscription pour l’instant.</p>
              ) : (
                <div className="overflow-x-auto">
                  <table className="min-w-full text-left text-xs">
                    <thead className="text-slate-500">
                      <tr>
                        <th className="px-2 py-1.5 font-semibold">Élève</th>
                        <th className="px-2 py-1.5 font-semibold">Parent</th>
                        <th className="px-2 py-1.5 font-semibold">Statut</th>
                        <th className="px-2 py-1.5 font-semibold">Actions</th>
                      </tr>
                    </thead>
                    <tbody>
                      {inscriptions.map((ins) => (
                        <tr key={ins.id} className="border-t border-slate-100">
                          <td className="px-2 py-2 align-top">
                            <p className="font-semibold text-slate-900">
                              {ins.eleveFirstName} {ins.eleveLastName}
                            </p>
                            <p className="text-slate-500">
                              {ins.eleveNiveau || "—"} {ins.eleveClasse}
                            </p>
                          </td>
                          <td className="px-2 py-2 align-top">
                            <p className="font-semibold text-slate-900">
                              {ins.parentFirstName} {ins.parentLastName}
                            </p>
                            <p className="text-slate-500">{ins.parentEmail}</p>
                            <p className="text-slate-500">{ins.parentPhone}</p>
                          </td>
                          <td className="px-2 py-2 align-top capitalize">{ins.status}</td>
                          <td className="px-2 py-2 align-top space-x-2 whitespace-nowrap">
                            {ins.status !== "validee" ? (
                              <button
                                type="button"
                                className="font-bold text-emerald-700"
                                onClick={() => void setInscriptionStatus(ins.id, "validee")}
                              >
                                Valider
                              </button>
                            ) : null}
                            {ins.status !== "refusee" ? (
                              <button
                                type="button"
                                className="font-bold text-amber-700"
                                onClick={() => void setInscriptionStatus(ins.id, "refusee")}
                              >
                                Refuser
                              </button>
                            ) : null}
                            <button
                              type="button"
                              className="font-bold text-rose-600"
                              onClick={() => void setInscriptionStatus(ins.id, "annulee")}
                            >
                              Annuler
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </section>
          </>
        ) : null}

        <div className="flex flex-wrap items-center justify-between gap-3 pt-2">
          <ModuleButton type="button" onClick={() => void save()} disabled={saving}>
            {saving ? "Enregistrement…" : "Enregistrer"}
          </ModuleButton>
          <button
            type="button"
            className="text-xs font-bold text-rose-600 hover:underline"
            onClick={() => void deleteOffre()}
          >
            Supprimer la fiche
          </button>
        </div>
      </ModulePageShell>
    </RequireModuleAccess>
  );
}
