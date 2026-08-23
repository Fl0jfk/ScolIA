"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import MarketingShell from "@/app/components/landing/MarketingShell";
import RequirePlatformMaster from "@/app/components/RequirePlatformMaster";
import { slugifyEstablishmentName, SIGNUP_STATUS_LABELS, type TenantSignupRequest } from "@/app/lib/platform-signup-types";

export default function PlateformeDemandeDetailPage() {
  const params = useParams();
  const id = String(params.id || "");
  const [request, setRequest] = useState<TenantSignupRequest | null>(null);
  const [notes, setNotes] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [provision, setProvision] = useState({
    slug: "",
    hostname: "",
    dataBucket: "",
    publishableKey: "",
    secretKey: "",
  });

  const reload = useCallback(async () => {
    const res = await fetch(`/api/platform/signup-requests/${id}`, { cache: "no-store" });
    const j = await res.json();
    if (res.ok) {
      setRequest(j.request);
      setNotes(j.request.masterNotes || "");
      const slug = j.request.provisionedTenantSlug || slugifyEstablishmentName(j.request.establishment.legalName);
      setProvision((p) => ({
        ...p,
        slug,
        hostname: p.hostname || `${slug}.scola.fr`,
      }));
    }
  }, [id]);

  useEffect(() => {
    if (id) void reload();
  }, [id, reload]);

  async function action(path: string, body?: unknown) {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/platform/signup-requests/${id}/${path}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: body ? JSON.stringify(body) : undefined,
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function saveNotes() {
    setBusy(true);
    try {
      await fetch(`/api/platform/signup-requests/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ masterNotes: notes }),
      });
      await reload();
    } finally {
      setBusy(false);
    }
  }

  if (!request) {
    return (
      <RequirePlatformMaster redirectTo="/">
        <MarketingShell>
          <p className="p-10 text-center text-stone-500">Chargement…</p>
        </MarketingShell>
      </RequirePlatformMaster>
    );
  }

  const e = request.establishment;
  const a = request.adminContact;

  return (
    <RequirePlatformMaster redirectTo="/">
      <MarketingShell>
        <main className="mx-auto max-w-3xl px-6 py-10 space-y-6">
          <p className="text-sm text-stone-500">
            <Link href="/plateforme/demandes" className="text-[#2F6B4A] hover:underline">
              ← Dossiers
            </Link>
          </p>
          <div>
            <h1 className="text-2xl font-black text-[#14231A]">{e.legalName}</h1>
            <p className="text-sm text-stone-600 mt-1">
              {SIGNUP_STATUS_LABELS[request.status]} · RNE {e.rne}
            </p>
          </div>

          {error && (
            <p className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</p>
          )}

          <section className="rounded-2xl border border-stone-200 bg-white p-5 text-sm space-y-2">
            <p>
              <strong>Adresse :</strong> {e.postalAddress.street}, {e.postalAddress.zip} {e.postalAddress.city}
            </p>
            <p>
              <strong>Effectif :</strong> {e.estimatedStudentCount} élèves ·{" "}
              <strong>Microsoft :</strong> {e.wantsMicrosoftLicenses ? "oui" : "non"}
            </p>
            <p>
              <strong>Référent :</strong> {a.firstName} {a.lastName} — {a.email}
              {a.phone ? ` — ${a.phone}` : ""}
            </p>
            <p>
              <strong>Gouvernance Microsoft :</strong>{" "}
              {e.microsoftCurrentManagement === "external_provider"
                ? "Prestataire externe"
                : e.microsoftCurrentManagement === "internal_establishment"
                  ? "Interne établissement"
                  : "Aucun tenant existant"}{" "}
              →{" "}
              {e.microsoftTargetMode === "scola_takeover"
                ? "Reprise d'administration par Scola"
                : "Déploiement Scola indépendant"}
            </p>
            <p>
              <strong>Décisionnaire Microsoft :</strong>{" "}
              {e.microsoftDecisionContact?.fullName || "—"}
              {e.microsoftDecisionContact?.email
                ? ` — ${e.microsoftDecisionContact.email}`
                : ""}
              {e.microsoftDecisionContact?.role
                ? ` (${e.microsoftDecisionContact.role})`
                : ""}
            </p>
          </section>

          <section className="rounded-2xl border border-stone-200 bg-white p-5 space-y-3">
            <h2 className="font-bold text-stone-900">Notes internes</h2>
            <textarea
              value={notes}
              onChange={(ev) => setNotes(ev.target.value)}
              rows={3}
              className="w-full rounded-xl border border-stone-200 px-3 py-2 text-sm"
            />
            <button
              type="button"
              disabled={busy}
              onClick={() => void saveNotes()}
              className="rounded-lg bg-stone-800 px-4 py-2 text-xs font-bold text-white"
            >
              Enregistrer les notes
            </button>
          </section>

          <section className="flex flex-wrap gap-2">
            {request.status === "pending_microsoft" && (
              <>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void action("approve-microsoft")}
                  className="rounded-xl bg-emerald-700 px-4 py-2 text-sm font-bold text-white"
                >
                  Valider Microsoft Education
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => {
                    const reason = prompt("Motif du refus :");
                    if (reason) void action("reject", { reason });
                  }}
                  className="rounded-xl border border-red-300 px-4 py-2 text-sm font-bold text-red-700"
                >
                  Refuser
                </button>
              </>
            )}
            {(request.status === "microsoft_approved" || request.status === "pending_payment") && (
              <button
                type="button"
                disabled={busy}
                onClick={() => void action("mark-paid", { billingMode: "monthly" })}
                className="rounded-xl border border-stone-300 px-4 py-2 text-sm font-bold text-stone-700"
              >
                Marquer payé (manuel)
              </button>
            )}
          </section>

          {(request.status === "payment_completed" || request.status === "provisioning") && (
            <section className="rounded-2xl border border-violet-200 bg-violet-50/50 p-5 space-y-4">
              <h2 className="font-bold text-violet-950">Provisioning tenant</h2>
              <p className="text-xs text-violet-800">
                Créez l&apos;instance auth dans le dashboard, puis saisissez les clés ci-dessous.
              </p>
              <div className="grid gap-3 sm:grid-cols-2">
                <label className="block space-y-1 text-sm">
                  <span className="font-semibold">Slug</span>
                  <input
                    className="w-full rounded-lg border px-3 py-2 font-mono text-sm"
                    value={provision.slug}
                    onChange={(ev) => setProvision({ ...provision, slug: ev.target.value })}
                  />
                </label>
                <label className="block space-y-1 text-sm">
                  <span className="font-semibold">Domaine</span>
                  <input
                    className="w-full rounded-lg border px-3 py-2 font-mono text-sm"
                    value={provision.hostname}
                    onChange={(ev) => setProvision({ ...provision, hostname: ev.target.value })}
                  />
                </label>
                <label className="block space-y-1 text-sm sm:col-span-2">
                  <span className="font-semibold">Bucket S3 données</span>
                  <input
                    className="w-full rounded-lg border px-3 py-2 font-mono text-sm"
                    value={provision.dataBucket}
                    onChange={(ev) => setProvision({ ...provision, dataBucket: ev.target.value })}
                    placeholder={process.env.NEXT_PUBLIC_DEFAULT_TENANT_BUCKET || "scola-tenant-…"}
                  />
                </label>
                <label className="block space-y-1 text-sm sm:col-span-2">
                  <span className="font-semibold">Publishable key (legacy) key</span>
                  <input
                    className="w-full rounded-lg border px-3 py-2 font-mono text-sm"
                    value={provision.publishableKey}
                    onChange={(ev) => setProvision({ ...provision, publishableKey: ev.target.value })}
                  />
                </label>
                <label className="block space-y-1 text-sm sm:col-span-2">
                  <span className="font-semibold">Secret key (legacy) key</span>
                  <input
                    type="password"
                    className="w-full rounded-lg border px-3 py-2 font-mono text-sm"
                    value={provision.secretKey}
                    onChange={(ev) => setProvision({ ...provision, secretKey: ev.target.value })}
                  />
                </label>
              </div>
              <button
                type="button"
                disabled={busy}
                onClick={() => void action("provision", provision)}
                className="rounded-xl bg-violet-700 px-4 py-2 text-sm font-bold text-white"
              >
                Provisionner l&apos;établissement
              </button>
            </section>
          )}

          {request.auditLog?.length > 0 && (
            <section className="rounded-2xl border border-stone-200 bg-stone-50 p-5">
              <h2 className="font-bold text-stone-800 text-sm mb-3">Historique</h2>
              <ul className="text-xs text-stone-600 space-y-1 max-h-48 overflow-y-auto">
                {[...request.auditLog].reverse().map((entry, i) => (
                  <li key={i}>
                    {new Date(entry.at).toLocaleString("fr-FR")} — {entry.action}
                    {entry.detail ? ` (${entry.detail})` : ""}
                  </li>
                ))}
              </ul>
            </section>
          )}
        </main>
      </MarketingShell>
    </RequirePlatformMaster>
  );
}
