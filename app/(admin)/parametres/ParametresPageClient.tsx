"use client";

import { useEffect, useMemo, useState } from "react";
import dynamic from "next/dynamic";
import { useSearchParams } from "next/navigation";
import RequireOrgAdmin from "@/app/components/RequireOrgAdmin";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import SettingsSitePanel from "@/app/components/settings/SettingsSitePanel";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav, { type ModuleTabItem } from "@/app/components/module-chrome/ModuleTabNav";
import type { RequestsRoutingConfig } from "@/app/lib/app-config-schemas";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { useIsPlatformMaster } from "@/app/hooks/useIsPlatformMaster";
import {
  SettingsAtmosphere,
  SettingsLoading,
  SettingsNotice,
  settingsPillClass,
} from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import {
  listToLines,
  linesToList,
  type SettingsEstablishmentForm,
  type SettingsTravelsConfig,
  type Tab,
} from "@/app/lib/settings-page-model";

type MefSecteursConfig = { lycee: string[]; college: string[]; ecole: string[] };

const SettingsEstablishmentsPanel = dynamic(
  () => import("@/app/components/settings/SettingsEstablishmentsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const SettingsNotificationsPanel = dynamic(
  () => import("@/app/components/settings/SettingsNotificationsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const SettingsTravelsPanel = dynamic(
  () => import("@/app/components/settings/SettingsTravelsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const SettingsIntegrationsPanel = dynamic(
  () => import("@/app/components/settings/SettingsIntegrationsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const SettingsMefPanel = dynamic(
  () => import("@/app/components/settings/SettingsMefPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const SettingsProfRoomPanel = dynamic(
  () => import("@/app/components/settings/SettingsProfRoomPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const SettingsRequestsRoutingPanel = dynamic(
  () => import("@/app/components/settings/SettingsRequestsRoutingPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const SchoolRosterPanel = dynamic(
  () => import("@/app/components/settings/SchoolRosterPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const DashboardQuickLinksPanel = dynamic(
  () => import("@/app/components/settings/DashboardQuickLinksPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const MembresPanel = dynamic(
  () => import("@/app/components/settings/MembresPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

const SETTINGS_NAV_TABS: ModuleTabItem<Tab>[] = [
  { id: "site", label: "Établissement", icon: "🏫" },
  { id: "establishments", label: "Sites / directions", icon: "🗺️" },
  { id: "utilisateurs", label: "Utilisateurs", icon: "👥" },
  { id: "referentiel", label: "Liste des élèves", icon: "🎒" },
  { id: "mef", label: "Formations MEF", icon: "📚" },
  { id: "notifications", label: "Notifications", icon: "✉️" },
  { id: "integrations", label: "Intégrations", icon: "🔌" },
  { id: "dashboard-links", label: "Raccourcis tableau de bord", icon: "🔗" },
];

export default function ParametresPage() {
  const searchParams = useSearchParams();
  const isOrgAdmin = useIsOrgAdmin();
  const isPlatformMaster = useIsPlatformMaster();
  const [tab, setTab] = useState<Tab>("site");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [identity, setIdentity] = useState<Record<string, unknown>>({});
  const [establishments, setEstablishments] = useState<SettingsEstablishmentForm[]>([]);
  const [notifications, setNotifications] = useState<Record<string, unknown>>({});
  const [mefLycee, setMefLycee] = useState("");
  const [mefCollege, setMefCollege] = useState("");
  const [mefEcole, setMefEcole] = useState("");
  const [mefMessage, setMefMessage] = useState<string | null>(null);
  const [uploadingLogo, setUploadingLogo] = useState(false);
  /** URL signée pour l’aperçu (headerLogoUrl en config = clé S3, non affichable telle quelle). */
  const [headerLogoPreviewUrl, setHeaderLogoPreviewUrl] = useState<string | null>(null);
  const [uploadingSignatureId, setUploadingSignatureId] = useState<string | null>(null);
  const [profRoomAdminIds, setProfRoomAdminIds] = useState<string[]>([]);
  const [clerkMembers, setClerkMembers] = useState<ClerkMemberOption[]>([]);
  const [membersLoading, setMembersLoading] = useState(false);
  const [requestsRouting, setRequestsRouting] = useState<RequestsRoutingConfig | null>(null);
  const [travelsCfg, setTravelsCfg] = useState<SettingsTravelsConfig>({ transportProviders: [] });
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
        setHeaderLogoPreviewUrl(
          typeof j.headerLogoPreviewUrl === "string" && j.headerLogoPreviewUrl
            ? j.headerLogoPreviewUrl
            : null,
        );
        setEstablishments(
          (j.config?.establishments || []).map((e: Record<string, unknown>) => ({
            id: String(e.id || ""),
            label: String(e.label || ""),
            kind: String(e.kind || e.id || ""),
            directorName: String(e.directorName || ""),
            directorEmail: String(e.directorEmail || ""),
            directorClerkUserId: String(e.directorClerkUserId || ""),
            colorHex: String(e.colorHex || ""),
            clerkRoleSlugs: Array.isArray(e.clerkRoleSlugs) ? (e.clerkRoleSlugs as string[]).join(", ") : "",
            active: e.active !== false,
            grades: typeof e.grades === "string" ? e.grades : undefined,
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
    if (!isOrgAdmin || (tab !== "prof-room" && tab !== "requests-routing" && tab !== "establishments" && tab !== "notifications")) return;
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
        .map((e) => inferEstablishmentKind(e))
        .filter((k): k is "ecole" | "college" | "lycee" => k === "ecole" || k === "college" || k === "lycee"),
    );
  }, [establishments]);

  const activeCycleLabels = useMemo(() => {
    const labels: Partial<Record<"ecole" | "college" | "lycee", string[]>> = {};
    for (const e of establishments) {
      if (!e.active) continue;
      const kind = inferEstablishmentKind(e);
      if (kind !== "ecole" && kind !== "college" && kind !== "lycee") continue;
      const list = labels[kind] ?? [];
      if (e.label.trim()) list.push(e.label.trim());
      labels[kind] = list;
    }
    return labels;
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
      setHeaderLogoPreviewUrl(
        typeof prepJson.previewUrl === "string" && prepJson.previewUrl
          ? prepJson.previewUrl
          : null,
      );

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
    setHeaderLogoPreviewUrl(null);
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

  const saveMef = async () => {
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
  };

  const importMefJson = async (file: File) => {
    try {
      const parsed = JSON.parse(await file.text()) as MefSecteursConfig;
      setMefLycee(listToLines(parsed.lycee || []));
      setMefCollege(listToLines(parsed.college || []));
      setMefEcole(listToLines(parsed.ecole || []));
      setMefMessage("Fichier chargé dans le formulaire — cliquez sur Enregistrer pour pousser sur S3.");
    } catch {
      setMefMessage("Erreur : JSON invalide.");
    }
  };

  const saveRequestsRouting = async () => {
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
  };

  if (loading) {
    return (
      <RequireOrgAdmin>
        <ModulePageShell maxWidthClass="max-w-5xl" className="relative space-y-6">
          <SettingsAtmosphere />
          <div className="relative">
            <SettingsLoading label="Chargement des paramètres…" />
          </div>
        </ModulePageShell>
      </RequireOrgAdmin>
    );
  }

  return (
    <RequireOrgAdmin>
      <ModulePageShell maxWidthClass="max-w-5xl" className="relative space-y-6">
        <SettingsAtmosphere />
        <div className="relative space-y-6">
        <ModulePageHeader
          eyebrow="Configuration"
          title={
            <>
              Paramètres{" "}
              <span className={dash.gradientText}>globaux</span>
            </>
          }
          description="Identité, sites, utilisateurs et référentiels. Les réglages métier (salles, ticketing, transporteurs…) restent dans chaque module."
          actions={
            <div className="flex flex-wrap items-center gap-2">
              <a href="/onboarding?review=1" className={settingsPillClass}>
                Assistant de configuration
              </a>
              {isPlatformMaster && (
                <a href="/platform/setup" className={settingsPillClass}>
                  Plateforme (Master)
                </a>
              )}
            </div>
          }
        />
        {error ? <SettingsNotice tone="error">{error}</SettingsNotice> : null}
        <ModuleTabNav tabs={SETTINGS_NAV_TABS} active={tab} onChange={setTab} scroll />

        {(tab === "travels" || tab === "prof-room" || tab === "requests-routing") && (
          <SettingsNotice tone="info">
            {tab === "travels" ? (
              <>
                Transporteurs : désormais dans{" "}
                <a href="/travels?tab=settings" className="font-semibold underline">
                  Sorties scolaires → Paramétrage
                </a>
                .
              </>
            ) : null}
            {tab === "prof-room" ? (
              <>
                Admins salles : désormais dans{" "}
                <a href="/prof-room" className="font-semibold underline">
                  Réservation de salle → Paramétrage
                </a>
                .
              </>
            ) : null}
            {tab === "requests-routing" ? (
              <>
                Routage ticketing : désormais dans{" "}
                <a href="/requests" className="font-semibold underline">
                  Demandes → Réglages
                </a>
                .
              </>
            ) : null}
          </SettingsNotice>
        )}

        {tab === "site" && (
          <SettingsSitePanel
            identity={identity}
            setIdentity={setIdentity}
            headerLogoPreviewUrl={headerLogoPreviewUrl}
            uploadingLogo={uploadingLogo}
            saving={saving}
            uploadHeaderLogo={uploadHeaderLogo}
            removeHeaderLogo={removeHeaderLogo}
            saveSection={saveSection}
          />
        )}

        {tab === "establishments" && (
          <SettingsEstablishmentsPanel
            establishments={establishments}
            setEstablishments={setEstablishments}
            clerkMembers={clerkMembers}
            membersLoading={membersLoading}
            uploadingSignatureId={uploadingSignatureId}
            saving={saving}
            uploadDirectionSignature={uploadDirectionSignature}
            removeDirectionSignature={removeDirectionSignature}
            saveSection={saveSection}
          />
        )}

        {tab === "notifications" && (
          <SettingsNotificationsPanel
            notifications={notifications}
            setNotifications={setNotifications}
            activeEstablishmentKinds={activeEstablishmentKinds}
            clerkMembers={clerkMembers}
            membersLoading={membersLoading}
            saving={saving}
            saveSection={saveSection}
          />
        )}

        {tab === "travels" && (
          <SettingsTravelsPanel
            travelsCfg={travelsCfg}
            setTravelsCfg={setTravelsCfg}
            saving={saving}
            saveSection={saveSection}
          />
        )}

        {tab === "integrations" && (
          <SettingsIntegrationsPanel
            integrations={integrations}
            setIntegrations={setIntegrations}
            saving={saving}
            saveSection={saveSection}
            activeEstablishmentKinds={activeEstablishmentKinds}
            activeCycleLabels={activeCycleLabels}
          />
        )}

        {tab === "mef" && (
          <SettingsMefPanel
            mefLycee={mefLycee}
            setMefLycee={setMefLycee}
            mefCollege={mefCollege}
            setMefCollege={setMefCollege}
            mefEcole={mefEcole}
            setMefEcole={setMefEcole}
            mefMessage={mefMessage}
            saving={saving}
            onSave={saveMef}
            onImportJson={importMefJson}
            activeEstablishmentKinds={activeEstablishmentKinds}
          />
        )}

        {tab === "prof-room" && (
          <SettingsProfRoomPanel
            clerkMembers={clerkMembers}
            profRoomAdminIds={profRoomAdminIds}
            setProfRoomAdminIds={setProfRoomAdminIds}
            membersLoading={membersLoading}
            saving={saving}
            saveSection={saveSection}
          />
        )}

        {tab === "requests-routing" && (
          <SettingsRequestsRoutingPanel
            requestsRouting={requestsRouting}
            setRequestsRouting={setRequestsRouting}
            clerkMembers={clerkMembers}
            membersLoading={membersLoading}
            saving={saving}
            onSave={saveRequestsRouting}
          />
        )}

        {tab === "referentiel" && <SchoolRosterPanel />}

        {tab === "dashboard-links" && <DashboardQuickLinksPanel />}

        {tab === "utilisateurs" && <MembresPanel />}
        </div>
      </ModulePageShell>
    </RequireOrgAdmin>
  );
}
