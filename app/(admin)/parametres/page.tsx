"use client";

import { useEffect, useMemo, useState } from "react";
import RequireOrgAdmin from "@/app/components/RequireOrgAdmin";
import ProfRoomAdminPicker, { type ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import RequestsRoutingEditor from "@/app/components/settings/RequestsRoutingEditor";
import type { RequestsRoutingConfig } from "@/app/lib/app-config-schemas";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { useIsPlatformMaster } from "@/app/hooks/useIsPlatformMaster";
import { DASHBOARD_ACCENT_OPTIONS } from "@/app/lib/dashboard-brand-presets";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";
import SchoolRosterPanel from "@/app/components/settings/SchoolRosterPanel";
import DashboardQuickLinksPanel from "@/app/components/settings/DashboardQuickLinksPanel";
import MembresPanel from "@/app/components/settings/MembresPanel";
import { useSearchParams } from "next/navigation";

type Tab = "site" | "establishments" | "notifications" | "mef" | "prof-room" | "requests-routing" | "travels" | "integrations" | "toolbox" | "referentiel" | "dashboard-links" | "utilisateurs";

type MefSecteursConfig = { lycee: string[]; college: string[]; ecole: string[] };

function linesToList(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function listToLines(arr: string[]): string {
  return (arr || []).join("\n");
}

export default function ParametresPage() {
  const searchParams = useSearchParams();
  const isOrgAdmin = useIsOrgAdmin();
  const isPlatformMaster = useIsPlatformMaster();
  const [tab, setTab] = useState<Tab>("site");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [identity, setIdentity] = useState<Record<string, unknown>>({});
  const [establishments, setEstablishments] = useState<
    Array<{
      id: string;
      label: string;
      kind?: string;
      directorName: string;
      directorEmail: string;
      clerkRoleSlugs: string;
      active: boolean;
      signatureS3Key?: string;
      signaturePreviewUrl?: string | null;
    }>
  >([]);
  const [notifications, setNotifications] = useState<Record<string, unknown>>({});
  const [mefLycee, setMefLycee] = useState("");
  const [mefCollege, setMefCollege] = useState("");
  const [mefEcole, setMefEcole] = useState("");
  const [mefMessage, setMefMessage] = useState<string | null>(null);
  const [uploadingLogo, setUploadingLogo] = useState(false);
  const [uploadingSignatureId, setUploadingSignatureId] = useState<string | null>(null);
  const [profRoomAdminIds, setProfRoomAdminIds] = useState<string[]>([]);
  const [clerkMembers, setClerkMembers] = useState<ClerkMemberOption[]>([]);
  const [membersLoading, setMembersLoading] = useState(false);
  const [requestsRouting, setRequestsRouting] = useState<RequestsRoutingConfig | null>(null);
  const [travelsCfg, setTravelsCfg] = useState<{ transportProviders: { name: string; email: string }[]; pdfFooterText?: string }>({ transportProviders: [] });
  const [integrations, setIntegrations] = useState<Record<string, unknown>>({});

  useEffect(() => {
    const t = searchParams.get("tab");
    if (
      t === "referentiel" ||
      t === "site" ||
      t === "establishments" ||
      t === "notifications" ||
      t === "mef" ||
      t === "prof-room" ||
      t === "requests-routing" ||
      t === "travels" ||
      t === "integrations" ||
      t === "toolbox" ||
      t === "dashboard-links" ||
      t === "utilisateurs" ||
      t === "membres"
    ) {
      setTab(t === "membres" ? "utilisateurs" : t);
    }
  }, [searchParams]);

  useEffect(() => {
    if (!isOrgAdmin) {
      setLoading(false);
      return;
    }
    (async () => {
      try {
        const res = await fetch("/api/settings");
        const j = await res.json();
        if (!res.ok) throw new Error(j.error || "Chargement impossible");
        setIdentity(j.config?.identity || {});
        setEstablishments(
          (j.config?.establishments || []).map((e: Record<string, unknown>) => ({
            id: String(e.id || ""),
            label: String(e.label || ""),
            kind: String(e.kind || e.id || ""),
            directorName: String(e.directorName || ""),
            directorEmail: String(e.directorEmail || ""),
            clerkRoleSlugs: Array.isArray(e.clerkRoleSlugs) ? (e.clerkRoleSlugs as string[]).join(", ") : "",
            active: e.active !== false,
            signatureS3Key: typeof e.signatureS3Key === "string" ? e.signatureS3Key : undefined,
            signaturePreviewUrl: null,
          })),
        );
        // Aperçus signatures (URLs signées dataBucket)
        void (async () => {
          const list = (j.config?.establishments || []) as Record<string, unknown>[];
          const withSig = list.filter((e) => e.id && e.signatureS3Key);
          if (!withSig.length) return;
          const previews = await Promise.all(
            withSig.map(async (e) => {
              const id = String(e.id);
              try {
                const pr = await fetch(
                  `/api/settings/upload-direction-signature?establishmentId=${encodeURIComponent(id)}`,
                );
                const pj = await pr.json();
                return { id, url: pr.ok ? (pj.previewUrl as string | null) : null };
              } catch {
                return { id, url: null };
              }
            }),
          );
          setEstablishments((prev) =>
            prev.map((est) => {
              const hit = previews.find((p) => p.id === est.id);
              return hit ? { ...est, signaturePreviewUrl: hit.url } : est;
            }),
          );
        })();
        setNotifications(j.config?.notifications || {});
        const profRoomCfg = j.config?.profRoom || {};
        const savedIds = Array.isArray(profRoomCfg.adminClerkUserIds) ? profRoomCfg.adminClerkUserIds : [];
        setProfRoomAdminIds(savedIds);
        const mRes = await fetch("/api/mef-secteurs");
        const mj = await mRes.json();
        if (mRes.ok && mj.config) {
          const c = mj.config as MefSecteursConfig;
          setMefLycee(listToLines(c.lycee));
          setMefCollege(listToLines(c.college));
          setMefEcole(listToLines(c.ecole));
        }
        const rrRes = await fetch("/api/settings/requests-routing");
        const rrJson = await rrRes.json();
        if (rrRes.ok && rrJson.config) {
          setRequestsRouting(rrJson.config as RequestsRoutingConfig);
        }
        const [trRes, intRes] = await Promise.all([
          fetch("/api/settings/travels"),
          fetch("/api/settings/integrations"),
        ]);
        const trJson = await trRes.json();
        const intJson = await intRes.json();
        if (trRes.ok && trJson.travels) setTravelsCfg(trJson.travels);
        if (intRes.ok && intJson.integrations) setIntegrations(intJson.integrations);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Erreur");
      } finally {
        setLoading(false);
      }
    })();
  }, [isOrgAdmin]);

  useEffect(() => {
    if (!isOrgAdmin || (tab !== "prof-room" && tab !== "requests-routing")) return;
    let cancelled = false;
    (async () => {
      setMembersLoading(true);
      try {
        const res = await fetch("/api/members");
        const j = await res.json();
        if (!res.ok) throw new Error(j.error || "Impossible de charger les membres Clerk");
        if (cancelled) return;
        const users = (j.users || []) as ClerkMemberOption[];
        setClerkMembers(users);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Erreur chargement membres");
      } finally {
        if (!cancelled) setMembersLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [isOrgAdmin, tab]);

  const activeEstablishmentKinds = useMemo(() => {
    return new Set(
      establishments
        .filter((e) => e.active)
        .map((e) => (e.kind || e.id).toLowerCase())
        .filter((k) => k === "ecole" || k === "college" || k === "lycee"),
    );
  }, [establishments]);

  const uploadHeaderLogo = async (file: File) => {
    setUploadingLogo(true);
    setError(null);
    try {
      const prep = await fetch("/api/settings/upload-logo", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fileName: file.name, fileType: file.type }),
      });
      const prepJson = await prep.json();
      if (!prep.ok) throw new Error(prepJson.error || "Préparation upload impossible");

      const putRes = await fetch(prepJson.uploadUrl, {
        method: "PUT",
        body: file,
        headers: { "Content-Type": file.type },
      });
      if (!putRes.ok) throw new Error("Envoi du fichier sur S3 impossible.");

      const nextIdentity = { ...identity, headerLogoUrl: prepJson.fileUrl as string };
      setIdentity(nextIdentity);

      const saveRes = await fetch("/api/settings/site", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(nextIdentity),
      });
      const saveJson = await saveRes.json();
      if (!saveRes.ok) throw new Error(saveJson.error || "Enregistrement impossible");

      alert("Logo mis à jour.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur upload logo");
    } finally {
      setUploadingLogo(false);
    }
  };

  const uploadDirectionSignature = async (establishmentId: string, file: File) => {
    if (!establishmentId.trim()) {
      setError("Enregistrez d’abord l’établissement (id) avant d’ajouter une signature.");
      return;
    }
    setUploadingSignatureId(establishmentId);
    setError(null);
    try {
      const prep = await fetch("/api/settings/upload-direction-signature", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ establishmentId, fileType: file.type }),
      });
      const prepJson = await prep.json();
      if (!prep.ok) throw new Error(prepJson.error || "Préparation upload impossible");

      const putRes = await fetch(prepJson.uploadUrl, {
        method: "PUT",
        body: file,
        headers: { "Content-Type": file.type },
      });
      if (!putRes.ok) throw new Error("Envoi de la signature sur S3 impossible.");

      setEstablishments((prev) =>
        prev.map((e) =>
          e.id === establishmentId
            ? {
                ...e,
                signatureS3Key: prepJson.fileKey as string,
                signaturePreviewUrl: (prepJson.previewUrl as string) || null,
              }
            : e,
        ),
      );
      alert("Signature direction enregistrée (bucket privé du tenant).");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur upload signature");
    } finally {
      setUploadingSignatureId(null);
    }
  };

  const removeDirectionSignature = async (establishmentId: string) => {
    if (!confirm("Supprimer la signature de cet établissement ?")) return;
    setUploadingSignatureId(establishmentId);
    setError(null);
    try {
      const res = await fetch("/api/settings/upload-direction-signature", {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ establishmentId }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Suppression impossible");
      setEstablishments((prev) =>
        prev.map((e) =>
          e.id === establishmentId
            ? { ...e, signatureS3Key: undefined, signaturePreviewUrl: null }
            : e,
        ),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur suppression signature");
    } finally {
      setUploadingSignatureId(null);
    }
  };

  const removeHeaderLogo = async () => {
    if (!confirm("Supprimer le logo personnalisé et revenir au logo par défaut ?")) return;
    const nextIdentity = { ...identity };
    delete nextIdentity.headerLogoUrl;
    setIdentity(nextIdentity);
    await saveSection("site", nextIdentity);
  };

  const saveSection = async (section: string, body: unknown) => {
    setSaving(true);
    setError(null);
    try {
      const res = await fetch(`/api/settings/${section}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec enregistrement");
      alert("Enregistré.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) return <p className="p-10 text-center text-slate-500">Chargement des paramètres…</p>;

  return (
    <RequireOrgAdmin>
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <h1 className="text-2xl font-bold text-slate-900">Paramètres</h1>
      <p className="text-sm text-slate-600 -mt-2">
        Réglages globaux du SaaS : établissement, utilisateurs, liste des élèves. Les réglages
        métier (salles, ticketing, transporteurs…) restent dans chaque module.
      </p>
      <div className="flex flex-wrap gap-x-4 gap-y-1">
        <a href="/onboarding?review=1" className="text-sm text-indigo-600 font-medium hover:underline">
          Relancer l&apos;assistant de configuration
        </a>
        {isPlatformMaster && (
          <a href="/platform/setup" className="text-sm text-violet-700 font-medium hover:underline">
            Configuration plateforme (Master)
          </a>
        )}
      </div>
      {error && <p className="text-red-600 text-sm">{error}</p>}
      <div className="flex flex-wrap gap-2">
        {(
          [
            ["site", "Établissement"],
            ["establishments", "Sites / directions"],
            ["utilisateurs", "Utilisateurs"],
            ["referentiel", "Liste des élèves"],
            ["mef", "Formations MEF"],
            ["notifications", "Notifications"],
            ["integrations", "Intégrations"],
            ["dashboard-links", "Raccourcis tableau de bord"],
            ["toolbox", "Boîte à outils"],
          ] as const
        ).map(([k, label]) => (
          <button
            key={k}
            type="button"
            onClick={() => setTab(k)}
            className={`px-4 py-2 rounded-xl text-sm font-bold ${tab === k ? "bg-indigo-600 text-white" : "bg-slate-100 text-slate-700"}`}
          >
            {label}
          </button>
        ))}
      </div>

      {(tab === "travels" || tab === "prof-room" || tab === "requests-routing") && (
        <div className="rounded-2xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm text-sky-950">
          {tab === "travels" ? (
            <>
              Transporteurs : désormais dans{" "}
              <a href="/travels?tab=settings" className="font-bold underline">
                Sorties scolaires → Paramétrage
              </a>
              .
            </>
          ) : null}
          {tab === "prof-room" ? (
            <>
              Admins salles : désormais dans{" "}
              <a href="/prof-room" className="font-bold underline">
                Réservation de salle → Paramétrage
              </a>
              .
            </>
          ) : null}
          {tab === "requests-routing" ? (
            <>
              Routage ticketing : désormais dans{" "}
              <a href="/requests" className="font-bold underline">
                Demandes → Réglages
              </a>
              .
            </>
          ) : null}
        </div>
      )}

      {tab === "site" && (
        <div className="bg-white rounded-2xl border p-6 space-y-4">
          <div className="mb-2">
            <h2 className="text-lg font-black text-slate-900">Établissement</h2>
            <p className="text-sm text-slate-500 mt-1">
              Nom, logo, couleurs, adresse et zone de vacances — identité globale de l&apos;intranet.
            </p>
          </div>
          <label className="block text-sm font-bold text-slate-600">Nom du groupe</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String(identity.name || "")}
            onChange={(e) => setIdentity({ ...identity, name: e.target.value })}
          />
          <label className="block text-sm font-bold text-slate-600">Nom court</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String(identity.shortName || "")}
            onChange={(e) => setIdentity({ ...identity, shortName: e.target.value })}
          />
          <label className="block text-sm font-bold text-slate-600">
            Zone de vacances scolaires
          </label>
          <p className="text-xs text-slate-500 -mt-2">
            Calendrier officiel MEN (zones A, B ou C). Exemple : Normandie = zone B. Requis pour
            afficher les vacances sur les plannings.
          </p>
          <select
            className="w-full border rounded-xl p-3 bg-white"
            value={String(identity.schoolHolidayZone || "")}
            onChange={(e) => {
              const v = e.target.value;
              setIdentity({
                ...identity,
                schoolHolidayZone: v === "A" || v === "B" || v === "C" ? v : undefined,
              });
            }}
          >
            <option value="">— Choisir la zone —</option>
            <option value="A">Zone A (ex. Lyon, Clermont, Montpellier…)</option>
            <option value="B">Zone B (ex. Normandie, Lille, Nantes, Rennes…)</option>
            <option value="C">Zone C (ex. Paris, Versailles, Créteil…)</option>
          </select>
          {!identity.schoolHolidayZone ? (
            <p className="text-xs font-semibold text-amber-800 bg-amber-50 border border-amber-200 rounded-lg px-3 py-2">
              Zone non configurée — les vacances scolaires ne s&apos;afficheront pas sur les
              plannings tant qu&apos;une zone n&apos;est pas enregistrée.
            </p>
          ) : null}

          <div className="rounded-xl border border-sky-200 bg-sky-50/60 p-4 space-y-3">
            <div>
              <p className="text-sm font-black text-sky-950">Site vitrine Scola</p>
              <p className="text-xs text-sky-900/80 mt-1">
                Activez uniquement si un site Next.js packagé a été livré pour cet établissement.
                Cela débloque l&apos;onglet Actus dans Communication.
              </p>
            </div>
            <label className="flex items-center gap-2 text-sm font-semibold text-slate-800 cursor-pointer">
              <input
                type="checkbox"
                className="h-4 w-4"
                checked={Boolean((identity.customWebsite as { enabled?: boolean } | undefined)?.enabled)}
                onChange={(e) =>
                  setIdentity({
                    ...identity,
                    customWebsite: {
                      ...((identity.customWebsite as object) || {}),
                      enabled: e.target.checked,
                      primaryDomain:
                        (identity.customWebsite as { primaryDomain?: string } | undefined)
                          ?.primaryDomain || "",
                    },
                  })
                }
              />
              Site vitrine Scola activé
            </label>
            <label className="block text-sm font-bold text-slate-600">
              Domaine principal (optionnel)
              <input
                className="mt-1 w-full border rounded-xl p-3 bg-white font-normal"
                placeholder="www.ecole.fr"
                value={String(
                  (identity.customWebsite as { primaryDomain?: string } | undefined)
                    ?.primaryDomain || "",
                )}
                onChange={(e) =>
                  setIdentity({
                    ...identity,
                    customWebsite: {
                      enabled: Boolean(
                        (identity.customWebsite as { enabled?: boolean } | undefined)?.enabled,
                      ),
                      primaryDomain: e.target.value,
                    },
                  })
                }
              />
            </label>
          </div>

          <label className="block text-sm font-bold text-slate-600">E-mail assistance technique</label>
          <input
            className="w-full border rounded-xl p-3 bg-slate-50 text-slate-600"
            type="email"
            value={PLATFORM_ASSISTANCE_EMAIL}
            readOnly
          />
          <label className="block text-sm font-bold text-slate-600">Adresse (rue)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String((identity.address as { street?: string })?.street || "")}
            onChange={(e) =>
              setIdentity({
                ...identity,
                address: { ...(identity.address as object), street: e.target.value },
              })
            }
          />
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-bold text-slate-600">Code postal</label>
              <input
                className="w-full border rounded-xl p-3"
                value={String((identity.address as { zip?: string })?.zip || "")}
                onChange={(e) =>
                  setIdentity({
                    ...identity,
                    address: { ...(identity.address as object), zip: e.target.value },
                  })
                }
              />
            </div>
            <div>
              <label className="block text-sm font-bold text-slate-600">Ville</label>
              <input
                className="w-full border rounded-xl p-3"
                value={String((identity.address as { city?: string })?.city || "")}
                onChange={(e) =>
                  setIdentity({
                    ...identity,
                    address: { ...(identity.address as object), city: e.target.value },
                  })
                }
              />
            </div>
          </div>

          <p className="text-xs text-slate-500">
            Le widget météo du tableau de bord utilise cette adresse. Les coordonnées GPS sont calculées
            automatiquement à l&apos;enregistrement.
          </p>
          {(identity.address as { latitude?: number; longitude?: number })?.latitude != null &&
          (identity.address as { latitude?: number; longitude?: number })?.longitude != null ? (
            <p className="text-xs font-semibold text-emerald-700 bg-emerald-50 border border-emerald-200 rounded-lg px-3 py-2">
              Coordonnées GPS :{" "}
              {(identity.address as { latitude: number }).latitude.toFixed(4)},{" "}
              {(identity.address as { longitude: number }).longitude.toFixed(4)} — météo active
            </p>
          ) : (
            <p className="text-xs font-semibold text-amber-800 bg-amber-50 border border-amber-200 rounded-lg px-3 py-2">
              Coordonnées GPS manquantes — enregistrez l&apos;identité du site pour activer la météo.
            </p>
          )}

          <div className="pt-2 border-t border-slate-100 space-y-3">
            <label className="block text-sm font-bold text-slate-600">Logo du header (haut gauche)</label>
            <p className="text-xs text-slate-500">
              PNG, JPEG, WebP ou SVG. Le fichier est enregistré sur S3 et affiché sur tout le site.
            </p>
            {identity.headerLogoUrl ? (
              <div className="flex items-center gap-4 p-4 bg-slate-50 rounded-xl border">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={String(identity.headerLogoUrl)}
                  alt="Aperçu logo"
                  className="max-h-16 w-auto object-contain"
                />
                <button
                  type="button"
                  onClick={removeHeaderLogo}
                  disabled={saving || uploadingLogo}
                  className="text-xs font-bold text-red-600 underline disabled:opacity-50"
                >
                  Supprimer le logo personnalisé
                </button>
              </div>
            ) : (
              <p className="text-xs text-slate-400 italic">Logo par défaut actuellement utilisé.</p>
            )}
            <input
              type="file"
              accept="image/png,image/jpeg,image/webp,image/svg+xml"
              disabled={uploadingLogo}
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) void uploadHeaderLogo(file);
                e.target.value = "";
              }}
              className="block w-full text-sm text-slate-600 file:mr-4 file:py-2 file:px-4 file:rounded-xl file:border-0 file:bg-indigo-50 file:text-indigo-700 file:font-bold hover:file:bg-indigo-100 disabled:opacity-50"
            />
            {uploadingLogo && <p className="text-xs text-indigo-600 font-medium">Envoi du logo en cours…</p>}
          </div>

          <div className="pt-4 border-t border-slate-100 space-y-3">
            <label className="block text-sm font-bold text-slate-600">Couleur du tableau de bord</label>
            <p className="text-xs text-slate-500">
              Teinte des boutons, titres et tuiles sur la page d&apos;accueil intranet (le dégradé s&apos;adapte
              automatiquement).
            </p>
            <div className="flex flex-wrap gap-2">
              {DASHBOARD_ACCENT_OPTIONS.map((opt) => {
                const selected = (String(identity.dashboardAccent || "green")) === opt.id;
                return (
                  <button
                    key={opt.id}
                    type="button"
                    onClick={() => setIdentity({ ...identity, dashboardAccent: opt.id })}
                    className={`flex items-center gap-2 rounded-xl border px-3 py-2 text-sm font-bold transition ${
                      selected
                        ? "border-indigo-600 bg-indigo-50 text-indigo-900 ring-2 ring-indigo-200"
                        : "border-slate-200 bg-white text-slate-700 hover:border-slate-300"
                    }`}
                  >
                    <span
                      className="h-5 w-5 shrink-0 rounded-full border border-black/10 shadow-sm"
                      style={{ backgroundColor: opt.swatch }}
                      aria-hidden
                    />
                    {opt.label}
                  </button>
                );
              })}
            </div>
          </div>

          <button
            type="button"
            disabled={saving}
            onClick={() => saveSection("site", identity)}
            className="bg-indigo-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
          >
            Enregistrer l&apos;identité
          </button>
        </div>
      )}

      {tab === "establishments" && (
        <div className="space-y-4">
          {establishments.map((est, idx) => (
            <div key={est.id || idx} className="bg-white rounded-2xl border p-5 space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <input
                  className="border rounded-lg p-2 text-sm"
                  placeholder="ID (ecole, college…)"
                  value={est.id}
                  onChange={(e) => {
                    const next = [...establishments];
                    next[idx] = { ...est, id: e.target.value };
                    setEstablishments(next);
                  }}
                />
                <input
                  className="border rounded-lg p-2 text-sm"
                  placeholder="Libellé"
                  value={est.label}
                  onChange={(e) => {
                    const next = [...establishments];
                    next[idx] = { ...est, label: e.target.value };
                    setEstablishments(next);
                  }}
                />
              </div>
              <input
                className="w-full border rounded-lg p-2 text-sm"
                placeholder="Nom direction"
                value={est.directorName}
                onChange={(e) => {
                  const next = [...establishments];
                  next[idx] = { ...est, directorName: e.target.value };
                  setEstablishments(next);
                }}
              />
              <input
                className="w-full border rounded-lg p-2 text-sm"
                placeholder="Email direction"
                value={est.directorEmail}
                onChange={(e) => {
                  const next = [...establishments];
                  next[idx] = { ...est, directorEmail: e.target.value };
                  setEstablishments(next);
                }}
              />
              <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 space-y-2">
                <p className="text-xs font-bold text-slate-700">Signature direction (PDF devis / stages / certificats)</p>
                <p className="text-[11px] text-slate-500">
                  Stockée dans le bucket privé du tenant — pas sur scolia-images.
                </p>
                {est.signaturePreviewUrl ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={est.signaturePreviewUrl}
                    alt={`Signature ${est.label || est.id}`}
                    className="h-14 max-w-[220px] object-contain bg-white rounded border"
                  />
                ) : est.signatureS3Key ? (
                  <p className="text-xs text-amber-700">Signature enregistrée (aperçu indisponible).</p>
                ) : (
                  <p className="text-xs text-slate-400">Aucune signature.</p>
                )}
                <div className="flex flex-wrap items-center gap-2">
                  <label className="cursor-pointer text-xs font-bold text-indigo-600 hover:underline">
                    {uploadingSignatureId === est.id ? "Envoi…" : "Ajouter / remplacer"}
                    <input
                      type="file"
                      accept="image/png,image/jpeg,image/webp"
                      className="hidden"
                      disabled={uploadingSignatureId === est.id || !est.id.trim()}
                      onChange={(e) => {
                        const f = e.target.files?.[0];
                        e.target.value = "";
                        if (f) void uploadDirectionSignature(est.id, f);
                      }}
                    />
                  </label>
                  {(est.signatureS3Key || est.signaturePreviewUrl) && (
                    <button
                      type="button"
                      className="text-xs font-bold text-rose-600"
                      disabled={uploadingSignatureId === est.id}
                      onClick={() => void removeDirectionSignature(est.id)}
                    >
                      Supprimer
                    </button>
                  )}
                </div>
              </div>
              <input
                className="w-full border rounded-lg p-2 text-sm"
                placeholder="Rôles Clerk (séparés par des virgules)"
                value={est.clerkRoleSlugs}
                onChange={(e) => {
                  const next = [...establishments];
                  next[idx] = { ...est, clerkRoleSlugs: e.target.value };
                  setEstablishments(next);
                }}
              />
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={est.active}
                  onChange={(e) => {
                    const next = [...establishments];
                    next[idx] = { ...est, active: e.target.checked };
                    setEstablishments(next);
                  }}
                />
                Actif
              </label>
            </div>
          ))}
          <button
            type="button"
            className="text-indigo-600 font-bold text-sm"
            onClick={() =>
              setEstablishments([
                ...establishments,
                { id: "", label: "", directorName: "", directorEmail: "", clerkRoleSlugs: "", active: true, signaturePreviewUrl: null },
              ])
            }
          >
            + Ajouter un établissement
          </button>
          <button
            type="button"
            disabled={saving}
            onClick={() =>
              saveSection("establishments", {
                establishments: establishments.map((e) => ({
                  id: e.id,
                  label: e.label,
                  kind: e.kind,
                  directorName: e.directorName,
                  directorEmail: e.directorEmail,
                  signatureS3Key: e.signatureS3Key,
                  active: e.active,
                  clerkRoleSlugs: e.clerkRoleSlugs.split(",").map((s) => s.trim()).filter(Boolean),
                })),
              })
            }
            className="bg-indigo-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
          >
            Enregistrer les établissements
          </button>
        </div>
      )}

      {tab === "notifications" && (
        <div className="bg-white rounded-2xl border p-6 space-y-4">
          <label className="block text-sm font-bold">Emails compta voyages (séparés par virgule)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={Array.isArray(notifications.travelsCompta) ? (notifications.travelsCompta as string[]).join(", ") : ""}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                travelsCompta: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
              })
            }
          />
          <label className="block text-sm font-bold">Email cuisine / restauration</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String(notifications.travelsCuisine || "")}
            onChange={(e) => setNotifications({ ...notifications, travelsCuisine: e.target.value })}
          />
          <label className="block text-sm font-bold">Gestionnaire HSE (e-mail)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String(notifications.hseOps || "")}
            onChange={(e) => setNotifications({ ...notifications, hseOps: e.target.value })}
          />
          <label className="block text-sm font-bold">Gestionnaire photocopies couleur (e-mail)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String(notifications.photocopiesOps || "")}
            onChange={(e) => setNotifications({ ...notifications, photocopiesOps: e.target.value })}
          />
          <label className="block text-sm font-bold">Email Zeendoc / envoi PDF voyages</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String(notifications.travelsZeendoc || "")}
            onChange={(e) => setNotifications({ ...notifications, travelsZeendoc: e.target.value })}
          />
          <hr className="border-slate-200" />
          <p className="text-sm font-black text-slate-800">Absences — notifications après validation direction</p>
          {activeEstablishmentKinds.has("ecole") && (
            <>
              <label className="block text-sm font-bold">Professeurs — école (nom)</label>
              <input
                className="w-full border rounded-xl p-3 mb-2"
                value={String((notifications.absencesNotifyProfEcole as { label?: string })?.label || "")}
                onChange={(e) =>
                  setNotifications({
                    ...notifications,
                    absencesNotifyProfEcole: {
                      ...((notifications.absencesNotifyProfEcole as object) || {}),
                      label: e.target.value,
                      email: String((notifications.absencesNotifyProfEcole as { email?: string })?.email || ""),
                    },
                  })
                }
              />
              <label className="block text-sm font-bold">Professeurs — école (e-mail)</label>
              <input
                className="w-full border rounded-xl p-3"
                type="email"
                value={String((notifications.absencesNotifyProfEcole as { email?: string })?.email || "")}
                onChange={(e) =>
                  setNotifications({
                    ...notifications,
                    absencesNotifyProfEcole: {
                      label: String((notifications.absencesNotifyProfEcole as { label?: string })?.label || ""),
                      email: e.target.value,
                    },
                  })
                }
              />
            </>
          )}
          {activeEstablishmentKinds.has("college") && (
            <>
              <label className="block text-sm font-bold">Professeurs — collège (nom)</label>
              <input
                className="w-full border rounded-xl p-3 mb-2"
                value={String((notifications.absencesNotifyProfCollege as { label?: string })?.label || (notifications.absencesNotifyProfCollegeLycee as { label?: string })?.label || "")}
                onChange={(e) =>
                  setNotifications({
                    ...notifications,
                    absencesNotifyProfCollege: {
                      label: e.target.value,
                      email: String((notifications.absencesNotifyProfCollege as { email?: string })?.email || (notifications.absencesNotifyProfCollegeLycee as { email?: string })?.email || ""),
                    },
                  })
                }
              />
              <label className="block text-sm font-bold">Professeurs — collège (e-mail)</label>
              <input
                className="w-full border rounded-xl p-3"
                type="email"
                value={String((notifications.absencesNotifyProfCollege as { email?: string })?.email || (notifications.absencesNotifyProfCollegeLycee as { email?: string })?.email || "")}
                onChange={(e) =>
                  setNotifications({
                    ...notifications,
                    absencesNotifyProfCollege: {
                      label: String((notifications.absencesNotifyProfCollege as { label?: string })?.label || (notifications.absencesNotifyProfCollegeLycee as { label?: string })?.label || ""),
                      email: e.target.value,
                    },
                  })
                }
              />
            </>
          )}
          {activeEstablishmentKinds.has("lycee") && (
            <>
              <label className="block text-sm font-bold">Professeurs — lycée (nom)</label>
              <input
                className="w-full border rounded-xl p-3 mb-2"
                value={String((notifications.absencesNotifyProfLycee as { label?: string })?.label || (notifications.absencesNotifyProfCollegeLycee as { label?: string })?.label || "")}
                onChange={(e) =>
                  setNotifications({
                    ...notifications,
                    absencesNotifyProfLycee: {
                      label: e.target.value,
                      email: String((notifications.absencesNotifyProfLycee as { email?: string })?.email || (notifications.absencesNotifyProfCollegeLycee as { email?: string })?.email || ""),
                    },
                  })
                }
              />
              <label className="block text-sm font-bold">Professeurs — lycée (e-mail)</label>
              <input
                className="w-full border rounded-xl p-3"
                type="email"
                value={String((notifications.absencesNotifyProfLycee as { email?: string })?.email || (notifications.absencesNotifyProfCollegeLycee as { email?: string })?.email || "")}
                onChange={(e) =>
                  setNotifications({
                    ...notifications,
                    absencesNotifyProfLycee: {
                      label: String((notifications.absencesNotifyProfLycee as { label?: string })?.label || (notifications.absencesNotifyProfCollegeLycee as { label?: string })?.label || ""),
                      email: e.target.value,
                    },
                  })
                }
              />
            </>
          )}
          <label className="block text-sm font-bold">Personnel OGEC, administratif & RH (e-mails séparés par virgule)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={
              Array.isArray(notifications.absencesNotifyOgecCompta)
                ? (notifications.absencesNotifyOgecCompta as string[]).join(", ")
                : ""
            }
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifyOgecCompta: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
              })
            }
          />
          <hr className="border-slate-200" />
          <p className="text-sm font-black text-slate-800">Internat — appel du soir (validation)</p>
          <label className="block text-sm font-bold">Qui reçoit l&apos;appel ? (e-mail)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String(
              (notifications.internatRollCallRecipients as { appelContact?: string; directionLycee?: string })?.appelContact ||
                (notifications.internatRollCallRecipients as { directionLycee?: string })?.directionLycee ||
                "",
            )}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                internatRollCallRecipients: {
                  ...((notifications.internatRollCallRecipients as object) || {}),
                  appelContact: e.target.value,
                },
              })
            }
          />
          <label className="block text-sm font-bold">CPE lycée (optionnel)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String((notifications.internatRollCallRecipients as { cpeLycee?: string })?.cpeLycee || "")}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                internatRollCallRecipients: {
                  ...((notifications.internatRollCallRecipients as object) || {}),
                  cpeLycee: e.target.value,
                },
              })
            }
          />
          <label className="block text-sm font-bold">CPE collège (optionnel)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String((notifications.internatRollCallRecipients as { cpeCollege?: string })?.cpeCollege || "")}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                internatRollCallRecipients: {
                  ...((notifications.internatRollCallRecipients as object) || {}),
                  cpeCollege: e.target.value,
                },
              })
            }
          />
          <label className="block text-sm font-bold">Internat — alertes urgence (emails séparés par virgule)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={
              Array.isArray(notifications.internatEmergencyRecipients)
                ? (notifications.internatEmergencyRecipients as string[]).join(", ")
                : ""
            }
            onChange={(e) =>
              setNotifications({
                ...notifications,
                internatEmergencyRecipients: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
              })
            }
          />
          <label className="block text-sm font-bold">Stages — e-mails administratif (séparés par virgule)</label>
          <input
            className="w-full border rounded-xl p-3"
            value={
              Array.isArray(notifications.stagesAdminEmails)
                ? (notifications.stagesAdminEmails as string[]).join(", ")
                : ""
            }
            onChange={(e) =>
              setNotifications({
                ...notifications,
                stagesAdminEmails: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
              })
            }
          />
          <label className="block text-sm font-bold">Stages — e-mail direction (signature)</label>
          <input
            className="w-full border rounded-xl p-3"
            type="email"
            value={String(notifications.stagesDirectionEmail || "")}
            onChange={(e) =>
              setNotifications({ ...notifications, stagesDirectionEmail: e.target.value.trim() })
            }
            placeholder="directeur@… (sinon e-mail directeur par établissement)"
          />
          <label className="block text-sm font-bold">Stages — modèle convention vierge (URL PDF)</label>
          <input
            className="w-full border rounded-xl p-3"
            type="url"
            value={String(notifications.stagesConventionTemplateUrl || "")}
            onChange={(e) =>
              setNotifications({ ...notifications, stagesConventionTemplateUrl: e.target.value.trim() })
            }
            placeholder="https://…/convention-stage-vierge.pdf"
          />
          <p className="text-xs text-slate-500">
            PDF remplissable (Adobe) hébergé sur S3 ou autre — lien affiché sur /stages/deposer.
          </p>
          <button
            type="button"
            disabled={saving}
            onClick={() => saveSection("notifications", notifications)}
            className="bg-indigo-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
          >
            Enregistrer les notifications
          </button>
        </div>
      )}

      {tab === "travels" && (
        <div className="bg-white rounded-2xl border p-6 space-y-4">
          <p className="text-sm text-slate-600">Transporteurs et pied de page des PDF de sorties scolaires.</p>
          <label className="block text-sm font-bold">Texte pied de page PDF</label>
          <input
            className="w-full border rounded-xl p-3"
            value={travelsCfg.pdfFooterText || ""}
            onChange={(e) => setTravelsCfg({ ...travelsCfg, pdfFooterText: e.target.value })}
          />
          <p className="text-sm font-bold">Transporteurs</p>
          {travelsCfg.transportProviders.map((p, idx) => (
            <div key={idx} className="grid grid-cols-2 gap-2">
              <input
                className="border rounded-lg p-2 text-sm"
                placeholder="Nom"
                value={p.name}
                onChange={(e) => {
                  const copy = [...travelsCfg.transportProviders];
                  copy[idx] = { ...copy[idx], name: e.target.value };
                  setTravelsCfg({ ...travelsCfg, transportProviders: copy });
                }}
              />
              <input
                className="border rounded-lg p-2 text-sm"
                placeholder="E-mail"
                type="email"
                value={p.email}
                onChange={(e) => {
                  const copy = [...travelsCfg.transportProviders];
                  copy[idx] = { ...copy[idx], email: e.target.value };
                  setTravelsCfg({ ...travelsCfg, transportProviders: copy });
                }}
              />
            </div>
          ))}
          <button
            type="button"
            className="text-indigo-600 text-sm font-bold"
            onClick={() =>
              setTravelsCfg({
                ...travelsCfg,
                transportProviders: [...travelsCfg.transportProviders, { name: "", email: "" }],
              })
            }
          >
            + Transporteur
          </button>
          <button
            type="button"
            disabled={saving}
            onClick={() => saveSection("travels", travelsCfg)}
            className="bg-indigo-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
          >
            Enregistrer sorties scolaires
          </button>
        </div>
      )}

      {tab === "integrations" && (
        <div className="bg-white rounded-2xl border p-6 space-y-4">
          <p className="text-sm text-slate-600">
            Paramètres <strong>métier</strong> (pas les raccourcis du tableau de bord — voir l&apos;onglet{" "}
            <strong>Raccourcis tableau de bord</strong>).
          </p>
          <label className="block text-sm font-bold">Zeendoc activé</label>
          <select
            className="w-full border rounded-xl p-3"
            value={(integrations.zeendoc as { enabled?: boolean })?.enabled ? "yes" : "no"}
            onChange={(e) =>
              setIntegrations({
                ...integrations,
                zeendoc: {
                  ...((integrations.zeendoc as object) || {}),
                  enabled: e.target.value === "yes",
                  buttonLabel: e.target.value === "yes" ? "Envoyer sur Zeendoc" : "Envoyer par mail",
                },
              })
            }
          >
            <option value="no">Non</option>
            <option value="yes">Oui</option>
          </select>
          <label className="block text-sm font-bold">E-mail destination Zeendoc / envoi PDF</label>
          <input
            className="w-full border rounded-xl p-3"
            value={String((integrations.zeendoc as { destinationEmail?: string })?.destinationEmail || "")}
            onChange={(e) =>
              setIntegrations({
                ...integrations,
                zeendoc: { ...((integrations.zeendoc as object) || {}), destinationEmail: e.target.value },
              })
            }
          />
          <label className="block text-sm font-bold">OneDrive / OCR activé</label>
          <select
            className="w-full border rounded-xl p-3"
            value={(integrations.microsoftOneDrive as { enabled?: boolean })?.enabled ? "yes" : "no"}
            onChange={(e) =>
              setIntegrations({
                ...integrations,
                microsoftOneDrive: {
                  ...((integrations.microsoftOneDrive as object) || {}),
                  enabled: e.target.value === "yes",
                },
              })
            }
          >
            <option value="no">Non</option>
            <option value="yes">Oui</option>
          </select>

          {(() => {
            const od = (integrations.microsoftOneDrive as {
              enabled?: boolean;
              basesBySecteur?: Partial<Record<"ecole" | "college" | "lycee", { basePath?: string; label?: string }>>;
              userSecteurs?: Array<{ match: string; secteur: "ecole" | "college" | "lycee" }>;
            }) || {};
            const bases = od.basesBySecteur || {};
            const userSecteurs = od.userSecteurs || [];
            const setOd = (patch: object) =>
              setIntegrations({
                ...integrations,
                microsoftOneDrive: { ...od, ...patch },
              });
            const setBase = (secteur: "ecole" | "college" | "lycee", basePath: string) =>
              setOd({ basesBySecteur: { ...bases, [secteur]: { ...(bases[secteur] || {}), basePath } } });
            const cycles: Array<{ key: "ecole" | "college" | "lycee"; label: string; placeholder: string }> = [
              { key: "ecole", label: "École", placeholder: "Dossier élèves/École" },
              { key: "college", label: "Collège", placeholder: "Dossier élèves/Collège" },
              { key: "lycee", label: "Lycée", placeholder: "Dossier élèves/Lycée" },
            ];
            return (
              <div className="space-y-4 border-t pt-4">
                <div>
                  <p className="text-sm font-bold">Dossiers racine OneDrive par cycle</p>
                  <p className="text-xs text-slate-500 mb-2">
                    Chemin du dossier (depuis la racine OneDrive) où l&apos;agent IA range les documents
                    élèves. Laissez vide pour utiliser la valeur par défaut.
                  </p>
                  <div className="space-y-2">
                    {cycles.map((c) => (
                      <div key={c.key} className="flex items-center gap-2">
                        <span className="w-20 text-sm">{c.label}</span>
                        <input
                          className="flex-1 border rounded-xl p-2 text-sm"
                          placeholder={c.placeholder}
                          value={bases[c.key]?.basePath || ""}
                          onChange={(e) => setBase(c.key, e.target.value)}
                        />
                      </div>
                    ))}
                  </div>
                </div>

                <div>
                  <p className="text-sm font-bold">Comptes → cycle (classement OCR)</p>
                  <p className="text-xs text-slate-500 mb-2">
                    Associe un e-mail (ou nom de famille) au cycle dont la personne gère le classement.
                    Utile pour les secrétariats non câblés en dur.
                  </p>
                  <div className="space-y-2">
                    {userSecteurs.map((row, i) => (
                      <div key={i} className="flex items-center gap-2">
                        <input
                          className="flex-1 border rounded-xl p-2 text-sm"
                          placeholder="email ou nom de famille"
                          value={row.match}
                          onChange={(e) => {
                            const next = [...userSecteurs];
                            next[i] = { ...row, match: e.target.value };
                            setOd({ userSecteurs: next });
                          }}
                        />
                        <select
                          className="border rounded-xl p-2 text-sm"
                          value={row.secteur}
                          onChange={(e) => {
                            const next = [...userSecteurs];
                            next[i] = { ...row, secteur: e.target.value as "ecole" | "college" | "lycee" };
                            setOd({ userSecteurs: next });
                          }}
                        >
                          <option value="ecole">École</option>
                          <option value="college">Collège</option>
                          <option value="lycee">Lycée</option>
                        </select>
                        <button
                          type="button"
                          onClick={() => setOd({ userSecteurs: userSecteurs.filter((_, j) => j !== i) })}
                          className="text-red-600 px-2 text-lg leading-none"
                          aria-label="Supprimer"
                        >
                          ×
                        </button>
                      </div>
                    ))}
                    <button
                      type="button"
                      onClick={() => setOd({ userSecteurs: [...userSecteurs, { match: "", secteur: "college" }] })}
                      className="text-sm text-indigo-600 font-semibold"
                    >
                      + Ajouter un mapping
                    </button>
                  </div>
                </div>
              </div>
            );
          })()}

          <button
            type="button"
            disabled={saving}
            onClick={() => saveSection("integrations", integrations)}
            className="bg-indigo-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
          >
            Enregistrer intégrations
          </button>
        </div>
      )}

      {tab === "mef" && (
        <div className="bg-white rounded-2xl border p-6 space-y-5">
          <p className="text-sm text-slate-600">
            Table des formations (libellés ou codes MEF) pour l&apos;agent IA OneDrive : chaque{" "}
            <strong>organisation</strong> a son propre fichier sur S3 (
            <code className="text-xs bg-slate-100 px-1 rounded">settings/mef-secteurs.json</code>
            ). Une ligne = une formation. Les élèves sont rattachés via le champ <code className="text-xs">mef</code>{" "}
            de la liste élèves.
          </p>
          <div>
            <label className="block text-sm font-bold text-slate-700 mb-1">Lycée</label>
            <textarea
              className="w-full border rounded-xl p-3 text-sm font-mono min-h-[140px]"
              placeholder={"2NDE GENERALE ET TECHNOLOGIQUE\nTERMINALE GENERALE\n…"}
              value={mefLycee}
              onChange={(e) => setMefLycee(e.target.value)}
            />
          </div>
          <div>
            <label className="block text-sm font-bold text-slate-700 mb-1">Collège</label>
            <textarea
              className="w-full border rounded-xl p-3 text-sm font-mono min-h-[120px]"
              placeholder={"6EME\n5EME\n3EME\n…"}
              value={mefCollege}
              onChange={(e) => setMefCollege(e.target.value)}
            />
          </div>
          <div>
            <label className="block text-sm font-bold text-slate-700 mb-1">École</label>
            <textarea
              className="w-full border rounded-xl p-3 text-sm font-mono min-h-[160px]"
              placeholder={"Cycle 2 - COURS PREPARATOIRE\nCycle 1 - GRANDE SECTION\n…"}
              value={mefEcole}
              onChange={(e) => setMefEcole(e.target.value)}
            />
          </div>
          {mefMessage && (
            <p className={`text-sm ${mefMessage.startsWith("Erreur") ? "text-red-600" : "text-green-700"}`}>
              {mefMessage}
            </p>
          )}
          <div className="flex flex-wrap gap-3">
            <button
              type="button"
              disabled={saving}
              onClick={async () => {
                setSaving(true);
                setMefMessage(null);
                setError(null);
                try {
                  const body: MefSecteursConfig = {
                    lycee: linesToList(mefLycee),
                    college: linesToList(mefCollege),
                    ecole: linesToList(mefEcole),
                  };
                  const res = await fetch("/api/mef-secteurs", {
                    method: "PUT",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify(body),
                  });
                  const j = await res.json();
                  if (!res.ok) throw new Error(j.error || "Échec enregistrement");
                  setMefMessage(j.message || "Table MEF enregistrée.");
                } catch (e) {
                  const msg = e instanceof Error ? e.message : "Erreur";
                  setMefMessage("Erreur : " + msg);
                } finally {
                  setSaving(false);
                }
              }}
              className="bg-indigo-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
            >
              Enregistrer les formations MEF
            </button>
            <label className="cursor-pointer text-sm font-bold text-indigo-600 hover:underline self-center">
              Importer un .json
              <input
                type="file"
                accept=".json,application/json"
                className="hidden"
                onChange={async (e) => {
                  const f = e.target.files?.[0];
                  if (!f) return;
                  try {
                    const parsed = JSON.parse(await f.text()) as MefSecteursConfig;
                    setMefLycee(listToLines(parsed.lycee || []));
                    setMefCollege(listToLines(parsed.college || []));
                    setMefEcole(listToLines(parsed.ecole || []));
                    setMefMessage("Fichier chargé dans le formulaire — cliquez sur Enregistrer pour pousser sur S3.");
                  } catch {
                    setMefMessage("Erreur : JSON invalide.");
                  }
                  e.target.value = "";
                }}
              />
            </label>
          </div>
        </div>
      )}

      {tab === "prof-room" && (
        <div className="bg-white rounded-2xl border p-6 space-y-4">
          <h2 className="text-lg font-bold text-slate-900">Administrateurs du module réservation de salles</h2>
          <p className="text-sm text-slate-500">
            Sélectionnez les personnes dans Clerk. Elles auront le mode administrateur dans l&apos;espace réservation
            de salles et pourront gérer le paramétrage (salles, matières, couleurs).
          </p>
          <ProfRoomAdminPicker
            members={clerkMembers}
            selectedIds={profRoomAdminIds}
            onChange={setProfRoomAdminIds}
            loading={membersLoading}
          />
          <button
            type="button"
            disabled={saving || membersLoading}
            onClick={() =>
              saveSection("prof-room", {
                adminClerkUserIds: profRoomAdminIds,
              })
            }
            className="bg-indigo-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
          >
            Enregistrer les administrateurs salles
          </button>
        </div>
      )}

      {tab === "requests-routing" && requestsRouting && (
        <div className="space-y-4">
          <RequestsRoutingEditor
            config={requestsRouting}
            onChange={setRequestsRouting}
            members={clerkMembers}
            membersLoading={membersLoading}
          />
          <button
            type="button"
            disabled={saving}
            onClick={async () => {
              setSaving(true);
              setError(null);
              try {
                const res = await fetch("/api/settings/requests-routing", {
                  method: "PUT",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify(requestsRouting),
                });
                const j = await res.json();
                if (!res.ok) throw new Error(j.error || "Échec enregistrement");
                setRequestsRouting(j.config as RequestsRoutingConfig);
                alert("Routage des demandes enregistré.");
              } catch (e) {
                setError(e instanceof Error ? e.message : "Erreur");
              } finally {
                setSaving(false);
              }
            }}
            className="bg-indigo-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
          >
            Enregistrer le routage des demandes
          </button>
        </div>
      )}

      {tab === "requests-routing" && !requestsRouting && (
        <p className="text-slate-500 text-sm">Chargement du catalogue de routage…</p>
      )}

      {tab === "referentiel" && (
        <div className="bg-white rounded-2xl border p-6">
          <SchoolRosterPanel />
        </div>
      )}

      {tab === "dashboard-links" && <DashboardQuickLinksPanel />}

      {tab === "utilisateurs" && <MembresPanel />}

      {tab === "toolbox" && (
        <div className="bg-white rounded-2xl border p-6 space-y-4">
          <h2 className="text-lg font-black text-slate-900">Boîte à outils saisonnière</h2>
          <p className="text-sm text-slate-600 max-w-xl">
            QR code et répartition des classes. Rentrée + fournitures →{" "}
            <a href="/etablissement/evenements" className="font-bold text-slate-900 underline">
              Événements
            </a>
            . Simulateur de tarifs →{" "}
            <a href="/etablissement/communication" className="font-bold text-slate-900 underline">
              Communication
            </a>
            .
          </p>
          <div className="flex flex-wrap gap-2">
            <a
              href="/toolbox"
              className="inline-flex rounded-xl bg-slate-900 px-5 py-2.5 text-sm font-bold text-white hover:bg-slate-800"
            >
              Ouvrir la boîte à outils →
            </a>
            <a
              href="/etablissement/evenements"
              className="inline-flex rounded-xl border border-slate-200 px-5 py-2.5 text-sm font-bold text-slate-800 hover:bg-slate-50"
            >
              Événements →
            </a>
            <a
              href="/etablissement/communication"
              className="inline-flex rounded-xl border border-slate-200 px-5 py-2.5 text-sm font-bold text-slate-800 hover:bg-slate-50"
            >
              Communication →
            </a>
          </div>
        </div>
      )}
    </div>
    </RequireOrgAdmin>
  );
}
