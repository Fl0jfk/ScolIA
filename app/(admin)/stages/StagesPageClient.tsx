"use client";

import { Suspense, useCallback, useEffect, useState } from "react";
import dynamic from "next/dynamic";
import { useSearchParams } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useOneDriveConnection } from "@/app/hooks/useOneDriveConnection";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";
import StagePendingSignaturesPanel from "@/app/components/stages/StagePendingSignaturesPanel";
import StageConventionDetail from "@/app/components/stages/StageConventionDetail";
import StagesBoardPanel from "@/app/components/stages/StagesBoardPanel";
import type {
  StageTab,
  StagesHubBoard,
} from "@/app/components/stages/stages-hub-types";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import { MODULE_TOUR_STEP_EVENT } from "@/app/lib/module-tour-actions";
import { resolveStagesTourTab } from "@/app/lib/module-tours";
import type { StageConvention } from "@/app/lib/stage-types";

const StagesClassePanel = dynamic(() => import("@/app/components/stages/StagesClassePanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const StagesConventionsPanel = dynamic(
  () => import("@/app/components/stages/StagesConventionsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const StagesSettingsPanel = dynamic(() => import("@/app/components/stages/StagesSettingsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const StageRepasAbsencesPanel = dynamic(
  () => import("@/app/components/stages/StageRepasAbsencesPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

function StagesContent() {
  const searchParams = useSearchParams();
  const { user: sessionUser } = useSessionUser();
  const [oneDriveProfile, setOneDriveProfile] = useState<OneDriveUserProfile | null>(null);
  const [board, setBoard] = useState<StagesHubBoard | null>(null);
  const odEnabled = Boolean(board?.permissions?.canFileToOneDrive);
  useEffect(() => {
    if (!sessionUser || !odEnabled) {
      setOneDriveProfile(null);
      return;
    }
    let cancelled = false;
    fetch("/api/onedrive/profile", { credentials: "include", cache: "no-store" })
      .then(async (r) => {
        if (!r.ok) return { profile: null };
        return r.json();
      })
      .then((j) => {
        if (!cancelled) setOneDriveProfile(j.profile || null);
      })
      .catch(() => {
        if (!cancelled) setOneDriveProfile(null);
      });
    return () => {
      cancelled = true;
    };
  }, [sessionUser, odEnabled]);
  const od = useOneDriveConnection({ enabled: odEnabled, restoreOnMount: false });
  const [conventions, setConventions] = useState<StageConvention[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(searchParams.get("convention"));
  const [detail, setDetail] = useState<{
    convention: StageConvention;
    studentLink: string | null;
    signLinks: Array<{ role: string; label: string; link: string; email?: string }>;
    eleveMatch?: {
      matchedEleve: {
        ine?: string;
        nom: string;
        prenom: string;
        folderName: string;
      } | null;
      folderPath: string | null;
      secteur: string | null;
      targetOneDriveLabel: string | null;
    };
  } | null>(null);
  const [tab, setTab] = useState<StageTab>(
    (searchParams.get("tab") as StageTab) || "board",
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [hasStoredSignature, setHasStoredSignature] = useState<boolean | undefined>(undefined);
  const [filingConventionId, setFilingConventionId] = useState<string | null>(null);
  const [adminReviewNote, setAdminReviewNote] = useState("");
  const [adminEditing, setAdminEditing] = useState(false);

  const load = useCallback(async () => {
    setError(null);
    try {
      const [bRes, cRes] = await Promise.all([
        fetch("/api/stages", { cache: "no-store" }),
        fetch("/api/stages/conventions", { cache: "no-store" }),
      ]);
      const b = await bRes.json();
      const c = await cRes.json();
      if (!bRes.ok) throw new Error(b?.error || "Erreur");
      setBoard(b);
      setConventions(c.conventions || []);
      if ((b.myPendingSignatures?.length ?? 0) > 0) {
        try {
          const sigRes = await fetch("/api/stages/my-signature", { cache: "no-store" });
          const sigData = await sigRes.json();
          if (sigRes.ok) setHasStoredSignature(Boolean(sigData.hasSignature));
        } catch {
          /* ignore */
        }
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  const loadDetail = useCallback(async (id: string) => {
    const res = await fetch(`/api/stages/conventions/${id}`, { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Erreur");
    setDetail(data);
    setSelectedId(id);
    setTab("conventions");
  }, []);

  const closeDetail = useCallback(() => {
    setDetail(null);
    setSelectedId(null);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (searchParams.get("tab") === "offers") {
      setTab("board");
    }
  }, [searchParams]);

  useEffect(() => {
    if (board?.permissions.referentOnly && tab === "board") {
      setTab("classe");
    }
  }, [board, tab]);

  useEffect(() => {
    const onStep = (e: Event) => {
      const target = (e as CustomEvent<{ target?: string }>).detail?.target;
      const nextTab = resolveStagesTourTab(target);
      if (nextTab) setTab(nextTab);
    };
    window.addEventListener(MODULE_TOUR_STEP_EVENT, onStep);
    return () => window.removeEventListener(MODULE_TOUR_STEP_EVENT, onStep);
  }, []);

  useEffect(() => {
    const id = searchParams.get("convention");
    if (id) void loadDetail(id).catch(() => undefined);
  }, [searchParams, loadDetail]);

  const permissions = board?.permissions;

  async function adminReview(approved: boolean) {
    if (!detail) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "admin_review",
          approved,
          note: adminReviewNote.trim() || undefined,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail(data);
      setAdminEditing(false);
      setAdminReviewNote("");
      setMsg(
        approved
          ? detail.convention.status === "convention_deposited"
            ? "Dépôt validé — e-mails de signature envoyés à l'élève, l'entreprise, le prof référent et la direction."
            : "Convention validée — e-mails de signature envoyés aux signataires (si SMTP configuré)."
          : detail.convention.status === "convention_deposited"
            ? "Dépôt refusé."
            : "Renvoyé pour correction — e-mails envoyés aux responsables légaux si possible.",
      );
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function adminSavePreconvention() {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "save", convention: detail.convention }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail({ ...detail, convention: data.convention });
      setMsg("Préconvention enregistrée (modification administrative).");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function resendSignature(signatureId: string) {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "resend_signature", signatureId }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setMsg(
        data.mail?.sent
          ? `Relance envoyée à ${data.email || "le signataire"}.`
          : `Relance non envoyée (${data.mail?.reason || "erreur"}).`,
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function resendSignatures() {
    if (!detail) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "resend_signatures" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setMsg(
        `Relance envoyée : ${data.mail?.sentCount ?? 0} e-mail(s) sur ${data.mail?.total ?? 0} signataire(s).`,
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function addSignatory() {
    if (!detail) return;
    const role = window.prompt(
      "Rôle (professeur_referent | professeur_principal | direction | parent | tuteur_entreprise) :",
      "professeur_referent",
    );
    if (!role) return;
    const email = window.prompt("E-mail du signataire :");
    if (!email) return;
    const name = window.prompt("Nom affiché (optionnel) :") || undefined;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "add_signatory", role, email, name }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail({ ...detail, convention: data.convention });
      setMsg("Signataire ajouté — e-mail de signature envoyé.");
      await loadDetail(detail.convention.id);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function removeSignatory(signatureId: string) {
    if (!detail) return;
    if (!window.confirm("Retirer ce signataire et invalider son lien ?")) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "remove_signatory", signatureId }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail({ ...detail, convention: data.convention });
      setMsg("Signataire retiré — lien invalidé.");
      await loadDetail(detail.convention.id);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function markSignatureManual(signatureId: string) {
    if (!detail) return;
    const note =
      window.prompt(
        "Valider manuellement (papier / hors plateforme). Note optionnelle :",
        "Signé hors plateforme",
      ) ?? undefined;
    if (note === undefined) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "mark_signature_manual",
          signatureId,
          note: note.trim() || undefined,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail({ ...detail, convention: data.convention });
      setMsg("Signature validée manuellement — lien invalidé.");
      await loadDetail(detail.convention.id);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function fileConventionToOneDrive(conventionId: string) {
    setFilingConventionId(conventionId);
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const token = await od.ensureToken();
      if (!token) {
        setError(od.error || "Connectez-vous à OneDrive avant d'envoyer la convention.");
        return;
      }
      const res = await fetch(`/api/stages/conventions/${conventionId}/file-onedrive`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ accessToken: token }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur envoi OneDrive");
      if (detail?.convention.id === conventionId) {
        setDetail({ ...detail, convention: data.convention });
      }
      setMsg(
        `Convention déposée dans le dossier élève : ${data.oneDrive?.fullPath ?? data.oneDrive?.folderPath ?? "OneDrive"}.`,
      );
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
      setFilingConventionId(null);
    }
  }

  async function fileToOneDrive() {
    if (!detail) return;
    await fileConventionToOneDrive(detail.convention.id);
  }

  async function fileToEleveDossier() {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "file_eleve_dossier" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail({ ...detail, convention: data.convention });
      setMsg(
        `Convention enregistrée dans le dossier élève (tiroir scolaire).${
          data.eleveDossier?.dossierUrl ? ` Voir : ${data.eleveDossier.dossierUrl}` : ""
        }`,
      );
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }


  return (
    <ModulePageShell maxWidthClass="max-w-[1400px]" tourModuleId="stages">
      <ModulePageHeader
        title="Stages & conventions"
        description="Les élèves remplissent leur préconvention en ligne (entreprise, horaires, contacts). Après validation, chaque signataire reçoit un code sécurisé par e-mail."
      />

      {error && (
        <p className="mb-4 rounded-lg border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">{error}</p>
      )}
      {msg && (
        <p className="mb-4 rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800 break-all">{msg}</p>
      )}

      {board?.myPendingSignatures && board.myPendingSignatures.length > 0 && (
        <StagePendingSignaturesPanel
          items={board.myPendingSignatures}
          hasStoredSignature={hasStoredSignature}
        />
      )}

      {permissions?.referentOnly && (
        <div className="mb-6 rounded-xl border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-900">
          <p className="font-semibold">Vue professeur principal / référent</p>
          <p className="mt-1 text-blue-800">
            Consultez l&apos;onglet <strong>Suivi classe</strong>, enregistrez votre signature ci-dessous, puis
            signez les conventions reçues par e-mail — votre paraphe sera ajouté directement sur le PDF.
          </p>
        </div>
      )}


      <ModuleTabNav
        className="mb-6"
        tabs={[
          {
            id: "board",
            label: "Tableau de bord",
            hidden: Boolean(permissions?.referentOnly),
            dataAttrs: { "data-stages-tab": "board" },
          },
          {
            id: "classe",
            label: "Suivi classe",
            hidden: !permissions?.canViewClassRoster,
            dataAttrs: { "data-stages-tab": "classe" },
          },
          {
            id: "conventions",
            label: "Conventions",
            dataAttrs: { "data-stages-tab": "conventions" },
          },
          {
            id: "repas",
            label: "Absences repas",
            hidden: !permissions?.canViewRepasAbsences,
            dataAttrs: { "data-stages-tab": "repas" },
          },
          {
            id: "settings",
            label: "Réglages",
            hidden: !permissions?.canManageStageSettings,
            dataAttrs: { "data-stages-tab": "settings" },
          },
        ]}
        active={tab}
        onChange={setTab}
        badges={{ conventions: board?.counts?.myPendingSignatures }}
      />

      {tab === "classe" && permissions?.canViewClassRoster && (
        <StagesClassePanel
          onOpenConvention={(id) => {
            void loadDetail(id);
            setTab("conventions");
          }}
          canFileOneDrive={Boolean(permissions?.canFileToOneDrive && od.oneDriveEnabled)}
          oneDriveConnected={od.connected}
          onFileOneDrive={(id) => void fileConventionToOneDrive(id)}
          filingConventionId={filingConventionId}
        />
      )}

      {tab === "board" && board && (
        <StagesBoardPanel
          board={board}
          permissions={permissions}
          onLoadDetail={(id) => void loadDetail(id)}
        />
      )}

      {tab === "repas" && permissions?.canViewRepasAbsences && (
        <section className="mb-8 rounded-2xl border border-amber-200 bg-white p-6 shadow-sm">
          <h2 className="text-lg font-bold text-[#1F3D2B]">Absences repas (stages)</h2>
          <p className="mt-1 text-sm text-stone-600">
            Élèves en stage sur la période — utiles pour la restauration et le suivi CPE.
          </p>
          <div className="mt-4">
            <StageRepasAbsencesPanel />
          </div>
        </section>
      )}

      {tab === "settings" && permissions?.canManageStageSettings && (
        <StagesSettingsPanel onSavedMsg={setMsg} />
      )}


      {tab === "conventions" && (
        <StagesConventionsPanel
          conventions={conventions}
          permissions={permissions}
          oneDriveEnabled={Boolean(od.oneDriveEnabled)}
          oneDriveConnected={od.connected}
          filingConventionId={filingConventionId}
          busy={busy}
          selectedId={selectedId}
          onLoadDetail={(id) => void loadDetail(id)}
          onCloseDetail={closeDetail}
          onFileOneDrive={(id) => void fileConventionToOneDrive(id)}
          detailPanel={
            detail && detail.convention.id === selectedId ? (
              <StageConventionDetail
                detail={detail}
                permissions={permissions}
                busy={busy}
                adminReviewNote={adminReviewNote}
                adminEditing={adminEditing}
                sessionUser={sessionUser}
                oneDriveProfile={oneDriveProfile}
                od={{
                  oneDriveEnabled: Boolean(od.oneDriveEnabled),
                  connected: od.connected,
                  msalReady: od.msalReady,
                  checking: od.checking,
                  accountLabel: od.accountLabel,
                  error: od.error,
                  login: () => od.login(),
                }}
                onClose={closeDetail}
                onAdminReviewNote={setAdminReviewNote}
                onAdminEditing={setAdminEditing}
                onConventionChange={(next) => setDetail({ ...detail, convention: next })}
                onAdminReview={(approved) => void adminReview(approved)}
                onAdminSave={() => void adminSavePreconvention()}
                onResendSignatures={() => void resendSignatures()}
                onResendSignature={(id) => void resendSignature(id)}
                onMarkSignatureManual={(id) => void markSignatureManual(id)}
                onRemoveSignatory={(id) => void removeSignatory(id)}
                onAddSignatory={() => void addSignatory()}
                onFileToEleveDossier={() => void fileToEleveDossier()}
                onFileToOneDrive={() => void fileToOneDrive()}
              />
            ) : null
          }
        />
      )}
    </ModulePageShell>
  );
}

export default function StagesPage() {
  return (
    <Suspense
      fallback={
        <ModulePageShell maxWidthClass="max-w-[1400px]">
          <p>Chargement…</p>
        </ModulePageShell>
      }
    >
      <StagesContent />
    </Suspense>
  );
}
