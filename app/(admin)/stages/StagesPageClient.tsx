"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import { useOneDriveConnection } from "@/app/hooks/useOneDriveConnection";
import { getOneDriveProfileForClerkUser } from "@/app/lib/onedrive-user-profiles";
import type { StageConvention, StageOffer } from "@/app/lib/stage-types";
import { STAGE_CONVENTION_STATUS_LABELS, STAGE_OFFER_KIND_LABELS } from "@/app/lib/stage-types";
import StageReferentsEditor from "@/app/components/stages/StageReferentsEditor";
import StageClassRosterPanel from "@/app/components/stages/StageClassRosterPanel";
import StagePendingSignaturesPanel from "@/app/components/stages/StagePendingSignaturesPanel";
import StageMySignatureBlock from "@/app/components/stages/StageMySignatureBlock";
import type { PendingStageSignature } from "@/app/lib/stage-pending-signatures";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";

type StageTab = "board" | "classe" | "offers" | "conventions";

type Board = {
  viewer: string;
  permissions: {
    canModerateOffers: boolean;
    canReviewPreconvention: boolean;
    canViewAllConventions: boolean;
    canViewReferentConventions: boolean;
    canDepositOffer: boolean;
    canFileToOneDrive: boolean;
    canPurge: boolean;
    canManageReferents: boolean;
    referentOnly: boolean;
    canViewClassRoster: boolean;
  };
  counts: Record<string, number>;
  myPendingSignatures?: PendingStageSignature[];
  pendingOffers: Array<{ id: string; companyName: string; kind: string; targetLevels: string[] }>;
  adminQueue: Array<{
    id: string;
    student?: { firstName: string; lastName: string };
    company?: { name: string };
    studentName?: string;
    companyName?: string;
    status: string;
  }>;
  conventions: Array<{
    id: string;
    studentName: string;
    className: string;
    companyName: string;
    status: string;
    periodStart: string;
    periodEnd: string;
  }>;
};

const LEVELS = ["3e", "2de", "1re", "Tle", "CAP", "BTS"];

function currentSchoolYearLabel() {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 8) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}

function candidatureHref(token: string) {
  return `/stages/candidater?token=${encodeURIComponent(token)}`;
}

function CandidatureLinkBlock({ token }: { token: string }) {
  const path = candidatureHref(token);
  const [copied, setCopied] = useState(false);

  async function copy() {
    const full =
      typeof window !== "undefined" ? `${window.location.origin}${path}` : path;
    try {
      await navigator.clipboard.writeText(full);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      /* ignore */
    }
  }

  return (
    <div className="mt-3 rounded-lg border border-[#2F6B4A]/20 bg-white/80 p-3 text-xs">
      <p className="font-semibold text-[#1F3D2B]">Lien candidature élève</p>
      <a href={path} className="mt-1 block break-all text-[#2F6B4A] underline" target="_blank" rel="noreferrer">
        {path}
      </a>
      <button
        type="button"
        onClick={() => void copy()}
        className="mt-2 rounded-md border border-stone-300 px-2 py-1 font-semibold text-stone-700 hover:bg-stone-50"
      >
        {copied ? "Copié !" : "Copier le lien complet"}
      </button>
    </div>
  );
}

function emptyOfferForm() {
  return {
    kind: "pfmp",
    companyName: "",
    companyAddress: "",
    description: "",
    positionsCount: 1,
    targetLevels: ["3e"],
    periodStart: "",
    periodEnd: "",
    contactName: "",
    contactEmail: "",
    contactPhone: "",
    sector: "",
  };
}

function StagesContent() {
  const searchParams = useSearchParams();
  const { user: clerkUser } = useUser();
  const oneDriveProfile = useMemo(
    () => (clerkUser ? getOneDriveProfileForClerkUser(clerkUser) : null),
    [clerkUser],
  );
  const od = useOneDriveConnection();
  const [board, setBoard] = useState<Board | null>(null);
  const [offers, setOffers] = useState<StageOffer[]>([]);
  const [approvedOffers, setApprovedOffers] = useState<StageOffer[]>([]);
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
  const [attachIne, setAttachIne] = useState("");
  const [tab, setTab] = useState<StageTab>(
    (searchParams.get("tab") as StageTab) || "board",
  );
  const [offerForm, setOfferForm] = useState(emptyOfferForm);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [purgeYear, setPurgeYear] = useState(currentSchoolYearLabel);
  const [purgePreview, setPurgePreview] = useState<{
    offersArchived: number;
    conventionsArchived: number;
  } | null>(null);
  const [hasStoredSignature, setHasStoredSignature] = useState<boolean | undefined>(undefined);
  const [oneDrivePreview, setOneDrivePreview] = useState<{
    totalPending: number;
    forMySecteur: number;
    secteurLabel: string | null;
    bySecteur: Partial<Record<string, number>>;
  } | null>(null);
  const [filingConventionId, setFilingConventionId] = useState<string | null>(null);

  const loadOneDrivePreview = useCallback(async () => {
    try {
      const res = await fetch("/api/stages/conventions/file-onedrive-batch", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) return;
      setOneDrivePreview(data);
    } catch {
      /* ignore */
    }
  }, []);

  const load = useCallback(async () => {
    setError(null);
    try {
      const [bRes, oRes, cRes] = await Promise.all([
        fetch("/api/stages", { cache: "no-store" }),
        fetch("/api/stages/offers", { cache: "no-store" }),
        fetch("/api/stages/conventions", { cache: "no-store" }),
      ]);
      const b = await bRes.json();
      const o = await oRes.json();
      const c = await cRes.json();
      if (!bRes.ok) throw new Error(b?.error || "Erreur");
      setBoard(b);
      setOffers(o.myOffers || o.offers || []);
      setApprovedOffers(o.approvedOffers || []);
      setConventions(c.conventions || []);
      await loadOneDrivePreview();
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
  }, [loadOneDrivePreview]);

  const loadDetail = useCallback(async (id: string) => {
    const res = await fetch(`/api/stages/conventions/${id}`, { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Erreur");
    setDetail(data);
    setAttachIne(data.convention?.ocrMeta?.matchedEleveIne ?? "");
    setSelectedId(id);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (board?.permissions.referentOnly && tab === "board") {
      setTab("classe");
    }
  }, [board, tab]);

  useEffect(() => {
    const id = searchParams.get("convention");
    if (id) void loadDetail(id).catch(() => undefined);
  }, [searchParams, loadDetail]);

  useEffect(() => {
    if (board?.permissions.referentOnly && tab === "board") {
      setTab("classe");
    }
  }, [board, tab]);

  const permissions = board?.permissions;

  async function submitOffer(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setMsg(null);
    try {
      const res = await fetch("/api/stages/offers", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(offerForm),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setOfferForm(emptyOfferForm());
      setMsg("Offre déposée — en attente de validation par la direction.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function moderateOffer(id: string, status: "approved" | "rejected") {
    setBusy(true);
    setMsg(null);
    try {
      const res = await fetch(`/api/stages/offers/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ status }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      if (status === "approved" && data.candidatureLink) {
        const full =
          typeof window !== "undefined"
            ? `${window.location.origin}${data.candidatureLink}`
            : data.candidatureLink;
        setMsg(`Offre validée — lien candidature élève : ${full}`);
        try {
          await navigator.clipboard.writeText(full);
        } catch {
          /* ignore */
        }
      } else if (status === "rejected") {
        setMsg("Offre refusée.");
      }
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function runPurge(dryRun: boolean) {
    if (!dryRun) {
      const ok = window.confirm(
        `Archiver toutes les offres et conventions de ${purgeYear} ? Cette action est irréversible.`,
      );
      if (!ok) return;
    }
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/stages/purge", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ schoolYear: purgeYear, dryRun }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      const r = data.result as { offersArchived: number; conventionsArchived: number };
      setPurgePreview(r);
      setMsg(
        dryRun
          ? `Simulation ${purgeYear} : ${r.offersArchived} offre(s), ${r.conventionsArchived} convention(s) à archiver.`
          : `Archivage terminé (${purgeYear}) : ${r.offersArchived} offre(s), ${r.conventionsArchived} convention(s).`,
      );
      if (!dryRun) await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function adminReview(approved: boolean) {
    if (!detail) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "admin_review", approved }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setDetail(data);
      setMsg(
        approved
          ? detail.convention.status === "convention_deposited"
            ? "Dépôt validé — e-mails de signature envoyés à l'élève, l'entreprise, le prof référent et la direction."
            : "Convention validée — e-mails de signature envoyés aux signataires (si SMTP configuré)."
          : detail.convention.status === "convention_deposited"
            ? "Dépôt refusé."
            : "Renvoyé à l'élève — e-mail de correction envoyé si possible.",
      );
      await load();
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

  async function attachEleveIne() {
    if (!detail) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${detail.convention.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "attach_eleve", matchedEleveIne: attachIne.trim() }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setMsg(attachIne.trim() ? "Élève rattaché par INE." : "Rattachement INE retiré.");
      await loadDetail(detail.convention.id);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function batchFileToOneDrive() {
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const token = await od.ensureToken();
      if (!token) {
        setError(od.error || "Connectez-vous à OneDrive avant d'envoyer les conventions.");
        return;
      }
      const res = await fetch("/api/stages/conventions/file-onedrive-batch", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ accessToken: token }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur envoi OneDrive");

      const parts = [
        `${data.filed ?? 0} convention(s) déposée(s) dans les dossiers élèves.`,
      ];
      if (data.skippedOtherSecteur > 0) {
        parts.push(
          `${data.skippedOtherSecteur} ignorée(s) (autre secteur — reconnectez-vous avec le bon compte Microsoft).`,
        );
      }
      if (data.failed?.length) {
        parts.push(`${data.failed.length} échec(s) — voir le détail ci-dessous.`);
      }
      setMsg(parts.join(" "));

      if (data.failed?.length) {
        setError(
          data.failed
            .slice(0, 5)
            .map((f: { studentName: string; error: string }) => `${f.studentName} : ${f.error}`)
            .join(" · "),
        );
      }

      await load();
      if (detail) await loadDetail(detail.convention.id);
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
      await loadOneDrivePreview();
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

  const canShowOneDriveFiling =
    permissions?.canFileToOneDrive && detail?.convention.status === "signed";

  const dossiers = useMemo(() => {
    const map = new Map<string, typeof conventions>();
    for (const c of conventions) {
      const key = `${c.student.lastName}|${c.student.firstName}|${c.student.className}`;
      const list = map.get(key) || [];
      list.push(c);
      map.set(key, list);
    }
    return [...map.entries()];
  }, [conventions]);

  return (
    <main className="mx-auto max-w-6xl px-4 py-8">
      <header className="mb-8">
        <h1 className="text-3xl font-black text-[#1F3D2B]">Stages & conventions</h1>
        <p className="mt-2 text-stone-600 max-w-2xl">
          Les élèves déposent leur convention signée en PDF sur une page publique. L&apos;IA extrait
          entreprise, SIRET et classe — vous validez dans la file d&apos;attente.
        </p>
      </header>

      {permissions?.canFileToOneDrive && od.oneDriveEnabled && (
        <section className="mb-6 rounded-2xl border border-slate-200 bg-slate-50 p-5 shadow-sm">
          <h2 className="text-lg font-bold text-[#1F3D2B]">Dépôt OneDrive — conventions signées</h2>
          <p className="mt-1 text-sm text-stone-600 max-w-3xl">
            S3 sert de transit jusqu&apos;au dépôt OneDrive. Connectez-vous à Microsoft puis envoyez
            en une fois les conventions finalisées — le PDF est alors rangé chez l&apos;élève et
            retiré de S3.
          </p>

          {oneDrivePreview && oneDrivePreview.totalPending > 0 && (
            <p className="mt-3 text-sm text-stone-800">
              <strong>{oneDrivePreview.totalPending}</strong> convention
              {oneDrivePreview.totalPending > 1 ? "s" : ""} signée
              {oneDrivePreview.totalPending > 1 ? "s" : ""} en attente de dépôt OneDrive
              {oneDrivePreview.secteurLabel && oneDrivePreview.forMySecteur > 0 ? (
                <>
                  {" "}
                  — <strong>{oneDrivePreview.forMySecteur}</strong> pour le{" "}
                  {oneDrivePreview.secteurLabel}
                </>
              ) : null}
              .
            </p>
          )}

          {oneDrivePreview && oneDrivePreview.totalPending === 0 && (
            <p className="mt-3 text-sm text-emerald-800">Aucune convention signée en attente de dépôt.</p>
          )}

          <div className="mt-4 flex flex-wrap items-center gap-2">
            {od.connected ? (
              <span className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-1.5 text-xs font-semibold text-emerald-800">
                OneDrive connecté{od.accountLabel ? ` (${od.accountLabel})` : ""}
              </span>
            ) : (
              <button
                type="button"
                onClick={() => void od.login()}
                disabled={!od.msalReady || od.checking}
                className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-semibold text-white hover:bg-blue-700 disabled:opacity-50"
              >
                Se connecter à OneDrive
              </button>
            )}
            <button
              type="button"
              onClick={() => void batchFileToOneDrive()}
              disabled={
                busy ||
                od.checking ||
                !od.msalReady ||
                !oneDrivePreview?.totalPending
              }
              className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
            >
              {busy || od.checking
                ? "Envoi en cours…"
                : "Envoyer les conventions signées vers OneDrive"}
            </button>
          </div>

          {oneDriveProfile && (
            <p className="mt-2 text-xs text-stone-600">
              Votre secteur : <strong>{oneDriveProfile.label}</strong> — les conventions des autres
              secteurs seront ignorées (reconnectez-vous avec le compte adapté).
            </p>
          )}
          {od.error && <p className="mt-2 text-xs text-rose-700">{od.error}</p>}
        </section>
      )}

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

      {permissions?.canViewClassRoster && permissions.referentOnly && <StageMySignatureBlock />}

      <nav className="mb-6 flex flex-wrap gap-2">
        {(
          [
            ...(permissions?.referentOnly ? [] : (["board"] as const)),
            ...(permissions?.canViewClassRoster ? (["classe"] as const) : []),
            ...(permissions?.referentOnly ? [] : (["offers"] as const)),
            ["conventions"],
          ] as StageTab[]
        ).map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={`rounded-full px-4 py-2 text-sm font-semibold ${
              tab === t ? "bg-[#2F6B4A] text-white" : "bg-white border border-stone-200 text-stone-700"
            }`}
          >
            {t === "board"
              ? "Tableau de bord"
              : t === "classe"
                ? "Suivi classe"
                : t === "offers"
                  ? "Offres"
                  : `Conventions${
                      board?.counts?.myPendingSignatures
                        ? ` (${board.counts.myPendingSignatures})`
                        : ""
                    }`}
          </button>
        ))}
      </nav>

      {tab === "classe" && permissions?.canViewClassRoster && (
        <section data-tour="stages-classe" className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
          <h2 className="text-lg font-bold text-[#1F3D2B]">Suivi des stages par classe</h2>
          <p className="mt-2 text-sm text-stone-600 max-w-3xl">
            Liste de la classe : élèves avec stage validé, en cours de traitement, sans stage, ou avec
            plusieurs conventions. Cliquez sur un dossier pour ouvrir le détail.
          </p>
          <div className="mt-6">
            <StageClassRosterPanel
              defaultSchoolYear={purgeYear}
              onOpenConvention={(id) => {
                void loadDetail(id);
                setTab("conventions");
              }}
              canFileOneDrive={Boolean(permissions?.canFileToOneDrive && od.oneDriveEnabled)}
              oneDriveConnected={od.connected}
              onFileOneDrive={(id) => void fileConventionToOneDrive(id)}
              filingConventionId={filingConventionId}
            />
          </div>
        </section>
      )}

      {tab === "board" && board && (
        <div data-tour="stages-board" className="space-y-8">
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {[
              ...(permissions?.referentOnly
                ? [
                    ["Mes conventions", board.counts.conventions],
                    ...(board.counts.myPendingSignatures
                      ? [["À signer", board.counts.myPendingSignatures]]
                      : []),
                  ]
                : [
                    ["Offres en attente", board.counts.pendingOffers],
                    ["Dépôts à valider", board.counts.adminQueue],
                    ["Signatures en cours", board.counts.signaturesPending],
                    ["Conventions totales", board.counts.conventions],
                  ]),
            ].map(([label, n]) => (
              <div key={String(label)} className="rounded-2xl border border-stone-200 bg-white p-5 shadow-sm">
                <p className="text-sm text-stone-500">{label}</p>
                <p className="text-3xl font-black text-[#2F6B4A] mt-1">{n}</p>
              </div>
            ))}
          </div>

          {!permissions?.referentOnly && (
            <section className="rounded-2xl border border-emerald-200 bg-emerald-50/60 p-5">
              <h2 className="text-sm font-bold text-emerald-900">Lien public — dépôt convention PDF</h2>
              <p className="mt-2 text-sm text-emerald-800">
                Communiquez cette adresse aux élèves (mail, ENT, affiche…) :
              </p>
              <p className="mt-2 rounded-lg bg-white border border-emerald-100 px-3 py-2 text-sm font-mono break-all text-[#1F3D2B]">
                {typeof window !== "undefined" ? window.location.origin : ""}/stages/deposer
              </p>
              <p className="mt-2 text-xs text-emerald-700">
                Le PDF est lu par OCR et IA (entreprise, SIRET, élève, classe). Aucun compte requis.
              </p>
            </section>
          )}

          {!permissions?.referentOnly && board.adminQueue.length > 0 && permissions?.canReviewPreconvention && (
            <section className="rounded-2xl border border-amber-200 bg-amber-50/50 p-5">
              <h2 className="text-sm font-bold text-amber-900">File d&apos;attente administrative</h2>
              <ul className="mt-3 space-y-2">
                {board.adminQueue.map((c) => {
                  const studentName =
                    c.studentName ||
                    (c.student ? `${c.student.firstName} ${c.student.lastName}`.trim() : "Élève");
                  const companyName = c.companyName || c.company?.name || "—";
                  return (
                    <li key={c.id}>
                      <button
                        type="button"
                        className="text-sm font-medium text-[#2F6B4A] underline"
                        onClick={() => void loadDetail(c.id)}
                      >
                        {studentName} → {companyName} ·{" "}
                        {STAGE_CONVENTION_STATUS_LABELS[c.status as keyof typeof STAGE_CONVENTION_STATUS_LABELS] ||
                          c.status}
                      </button>
                    </li>
                  );
                })}
              </ul>
            </section>
          )}

          {permissions?.canManageReferents && (
            <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
              <h2 className="text-lg font-bold text-[#1F3D2B]">Professeurs principaux / référents par classe</h2>
              <p className="mt-2 text-sm text-stone-600 max-w-2xl">
                Assignez le professeur principal (ou référent stage) de chaque classe. Il verra l&apos;onglet
                <strong> Suivi classe</strong> avec tous les élèves et l&apos;état de leurs conventions.
              </p>
              <div className="mt-4">
                <StageReferentsEditor
                  initialYear={purgeYear}
                  onSaved={(m) => setMsg(m)}
                />
              </div>
            </section>
          )}

          {permissions?.canPurge && (
            <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
              <h2 className="text-lg font-bold text-[#1F3D2B]">Purge fin d&apos;année</h2>
              <p className="mt-2 text-sm text-stone-600 max-w-xl">
                Archive les offres et conventions d&apos;une année scolaire (statut « archivé »). Les données restent
                stockées mais disparaissent des listes actives.
              </p>
              <label className="mt-4 block text-sm">
                Année scolaire
                <input
                  className="mt-1 w-full max-w-xs rounded-lg border border-stone-300 px-3 py-2"
                  value={purgeYear}
                  onChange={(e) => setPurgeYear(e.target.value)}
                  placeholder="2024-2025"
                />
              </label>
              {purgePreview && (
                <p className="mt-3 text-xs text-stone-500">
                  Dernière simulation : {purgePreview.offersArchived} offre(s), {purgePreview.conventionsArchived}{" "}
                  convention(s).
                </p>
              )}
              <div className="mt-4 flex flex-wrap gap-2">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void runPurge(true)}
                  className="rounded-lg border border-stone-300 px-4 py-2 text-sm font-semibold text-stone-800 disabled:opacity-50"
                >
                  Simuler
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void runPurge(false)}
                  className="rounded-lg bg-rose-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
                >
                  Archiver l&apos;année
                </button>
              </div>
            </section>
          )}
        </div>
      )}

      {tab === "offers" && (
        <div className="grid gap-8 lg:grid-cols-2">
          {permissions?.canDepositOffer && (
            <form onSubmit={submitOffer} className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm space-y-4">
              <h2 className="text-lg font-bold text-[#1F3D2B]">Déposer une offre</h2>
              <label className="block text-sm">
                Type
                <select
                  className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2"
                  value={offerForm.kind}
                  onChange={(e) => setOfferForm({ ...offerForm, kind: e.target.value })}
                >
                  {Object.entries(STAGE_OFFER_KIND_LABELS).map(([k, v]) => (
                    <option key={k} value={k}>{v}</option>
                  ))}
                </select>
              </label>
              <input
                required
                placeholder="Nom de l'entreprise *"
                className="w-full rounded-lg border border-stone-300 px-3 py-2"
                value={offerForm.companyName}
                onChange={(e) => setOfferForm({ ...offerForm, companyName: e.target.value })}
              />
              <textarea
                required
                placeholder="Description du poste / activité *"
                className="w-full rounded-lg border border-stone-300 px-3 py-2 min-h-[80px]"
                value={offerForm.description}
                onChange={(e) => setOfferForm({ ...offerForm, description: e.target.value })}
              />
              <div className="flex flex-wrap gap-2">
                {LEVELS.map((lv) => (
                  <label key={lv} className="text-sm flex items-center gap-1">
                    <input
                      type="checkbox"
                      checked={offerForm.targetLevels.includes(lv)}
                      onChange={(e) => {
                        const targetLevels = e.target.checked
                          ? [...offerForm.targetLevels, lv]
                          : offerForm.targetLevels.filter((x) => x !== lv);
                        setOfferForm({ ...offerForm, targetLevels });
                      }}
                    />
                    {lv}
                  </label>
                ))}
              </div>
              <input
                required
                placeholder="Contact (nom) *"
                className="w-full rounded-lg border border-stone-300 px-3 py-2"
                value={offerForm.contactName}
                onChange={(e) => setOfferForm({ ...offerForm, contactName: e.target.value })}
              />
              <input
                required
                type="email"
                placeholder="Contact (e-mail) *"
                className="w-full rounded-lg border border-stone-300 px-3 py-2"
                value={offerForm.contactEmail}
                onChange={(e) => setOfferForm({ ...offerForm, contactEmail: e.target.value })}
              />
              <button
                type="submit"
                disabled={busy}
                className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
              >
                Soumettre à la direction
              </button>
            </form>
          )}

          <div className="space-y-4">
            <h2 className="text-lg font-bold text-[#1F3D2B]">Offres</h2>
            {offers.map((o) => (
              <div key={o.id} className="rounded-xl border border-stone-200 bg-white p-4">
                <p className="font-semibold">{o.companyName}</p>
                <p className="text-sm text-stone-600">{STAGE_OFFER_KIND_LABELS[o.kind]} · {o.status}</p>
                {permissions?.canModerateOffers && o.status === "pending" && (
                  <div className="mt-3 flex gap-2">
                    <button
                      type="button"
                      onClick={() => void moderateOffer(o.id, "approved")}
                      className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold text-white"
                    >
                      Valider
                    </button>
                    <button
                      type="button"
                      onClick={() => void moderateOffer(o.id, "rejected")}
                      className="rounded-lg bg-rose-600 px-3 py-1.5 text-xs font-semibold text-white"
                    >
                      Refuser
                    </button>
                  </div>
                )}
                {o.status === "approved" && o.candidatureToken && (
                  <CandidatureLinkBlock token={o.candidatureToken} />
                )}
              </div>
            ))}
            {approvedOffers.length > 0 && (
              <>
                <h3 className="text-sm font-bold text-stone-500 mt-6">Offres validées (réseau)</h3>
                {approvedOffers.map((o) => (
                  <div key={o.id} className="rounded-xl border border-emerald-100 bg-emerald-50/50 p-4">
                    <p className="font-semibold">{o.companyName}</p>
                    <p className="text-sm text-stone-600">{o.description.slice(0, 120)}…</p>
                    {o.candidatureToken && <CandidatureLinkBlock token={o.candidatureToken} />}
                    <button
                      type="button"
                      className="mt-2 text-xs font-semibold text-[#2F6B4A] underline"
                      onClick={() => setTab("conventions")}
                    >
                      Voir les conventions déposées →
                    </button>
                  </div>
                ))}
              </>
            )}
          </div>
        </div>
      )}

      {tab === "conventions" && (
        <div data-tour="stages-conventions" className="grid gap-8 lg:grid-cols-2">
          <div className="rounded-2xl border border-emerald-100 bg-emerald-50/40 p-6 shadow-sm space-y-3 text-sm" data-tour="stages-deposer-link">
            <h2 className="text-lg font-bold text-[#1F3D2B]">Dépôt élève (PDF)</h2>
            <p className="text-stone-600">
              Les élèves envoient leur convention remplie et signée sur la page publique. Plus besoin de
              remplir une préconvention en ligne.
            </p>
            <a
              href="/stages/deposer"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-block rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white"
            >
              Ouvrir /stages/deposer →
            </a>
          </div>

          <div className="space-y-6 lg:col-span-2">
            <h2 className="text-lg font-bold text-[#1F3D2B]">Dossiers élèves</h2>
            {dossiers.map(([key, list]) => {
              const first = list[0]!;
              return (
                <div key={key} className="rounded-xl border border-stone-200 bg-white p-4">
                  <p className="font-semibold">
                    {first.student.firstName} {first.student.lastName} — {first.student.className}
                  </p>
                  <p className="text-xs text-stone-500">{list.length} convention(s)</p>
                  <ul className="mt-2 space-y-2">
                    {list.map((c) => (
                      <li key={c.id} className="flex flex-wrap items-center gap-2">
                        <button
                          type="button"
                          className="text-sm text-[#2F6B4A] font-medium underline"
                          onClick={() => void loadDetail(c.id)}
                        >
                          {c.company.name} · {STAGE_CONVENTION_STATUS_LABELS[c.status]}
                        </button>
                        {permissions?.canFileToOneDrive && od.oneDriveEnabled && (
                          <>
                            {c.oneDriveFiling?.filedAt ? (
                              <span className="text-xs font-semibold text-emerald-700">OneDrive ✓</span>
                            ) : c.status === "signed" ? (
                              <button
                                type="button"
                                disabled={!od.connected || filingConventionId === c.id || busy}
                                onClick={() => void fileConventionToOneDrive(c.id)}
                                className="rounded border border-[#2F6B4A]/40 px-2 py-0.5 text-xs font-semibold text-[#2F6B4A] disabled:opacity-50"
                              >
                                {filingConventionId === c.id ? "Envoi…" : "→ OneDrive"}
                              </button>
                            ) : null}
                          </>
                        )}
                      </li>
                    ))}
                  </ul>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {detail && (
        <section className="mt-10 rounded-2xl border border-[#2F6B4A]/20 bg-white p-6 shadow-sm">
          <h2 className="text-xl font-bold text-[#1F3D2B]">Détail convention</h2>
          <p className="text-sm text-stone-600 mt-1">
            {detail.convention.student.firstName} {detail.convention.student.lastName} →{" "}
            {detail.convention.company.name} ·{" "}
            {STAGE_CONVENTION_STATUS_LABELS[detail.convention.status]}
          </p>
          {detail.studentLink && (
            <p className="mt-3 text-sm break-all">
              <span className="font-semibold">Lien élève :</span>{" "}
              <a className="text-[#2F6B4A] underline" href={detail.studentLink}>
                {typeof window !== "undefined" ? window.location.origin : ""}
                {detail.studentLink}
              </a>
            </p>
          )}
          <div className="mt-4 flex flex-wrap gap-2">
            {detail.convention.uploadedPdf && (
              <a
                href={`/api/stages/conventions/${detail.convention.id}/uploaded-pdf`}
                target="_blank"
                rel="noopener noreferrer"
                className="rounded-lg border border-emerald-300 bg-emerald-50 px-4 py-2 text-sm font-semibold text-emerald-900 hover:bg-emerald-100"
              >
                Voir le PDF déposé
              </a>
            )}
            <a
              href={`/api/stages/conventions/${detail.convention.id}/pdf`}
              className="rounded-lg border border-stone-300 px-4 py-2 text-sm font-semibold text-stone-800 hover:bg-stone-50"
            >
              Télécharger PDF généré
            </a>
            {permissions?.canReviewPreconvention && detail.convention.status === "signatures_pending" && (
              <button
                type="button"
                onClick={() => void resendSignatures()}
                disabled={busy}
                className="rounded-lg border border-[#2F6B4A] px-4 py-2 text-sm font-semibold text-[#2F6B4A] disabled:opacity-50"
              >
                Renvoyer les e-mails de signature
              </button>
            )}
          </div>

          {detail.convention.ocrMeta && permissions?.canReviewPreconvention && (
            <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50/80 p-4">
              <h3 className="text-sm font-bold text-[#1F3D2B]">Rattachement élève (eleves.json)</h3>
              <p className="mt-1 text-xs text-stone-600">
                L&apos;élève n&apos;a pas besoin de connaître son INE — le nom/prénom suffit en général. Si le
                matching est ambigu, vous pouvez préciser l&apos;INE ici (lu sur le PDF ou dans Pronote).
              </p>
              {detail.eleveMatch?.matchedEleve ? (
                <p className="mt-2 text-sm text-emerald-900">
                  Correspondance :{" "}
                  <strong>
                    {detail.eleveMatch.matchedEleve.prenom} {detail.eleveMatch.matchedEleve.nom}
                  </strong>
                  {detail.eleveMatch.matchedEleve.ine
                    ? ` (INE ${detail.eleveMatch.matchedEleve.ine})`
                    : ""}
                  {detail.eleveMatch.matchedEleve.folderName
                    ? ` — dossier « ${detail.eleveMatch.matchedEleve.folderName} »`
                    : ""}
                </p>
              ) : (
                <p className="mt-2 text-sm text-amber-900">Aucune correspondance fiable pour l&apos;instant.</p>
              )}
              <div className="mt-3 flex flex-wrap items-end gap-2">
                <label className="text-xs font-semibold text-stone-700">
                  INE (optionnel)
                  <input
                    type="text"
                    value={attachIne}
                    onChange={(e) => setAttachIne(e.target.value)}
                    placeholder="ex. 180123456AB"
                    className="mt-1 block w-48 rounded-lg border border-stone-300 px-3 py-2 text-sm"
                  />
                </label>
                <button
                  type="button"
                  onClick={() => void attachEleveIne()}
                  disabled={busy}
                  className="rounded-lg border border-[#2F6B4A] px-4 py-2 text-sm font-semibold text-[#2F6B4A] disabled:opacity-50"
                >
                  Enregistrer le rattachement
                </button>
              </div>
            </div>
          )}

          {canShowOneDriveFiling && (
            <div className="mt-6 rounded-xl border border-slate-200 bg-slate-50 p-4">
              <h3 className="text-sm font-bold text-[#1F3D2B]">Dépôt dossier élève (OneDrive)</h3>
              <p className="mt-1 text-xs text-stone-600">
                Après signature complète, dépôt automatique dans le dossier OneDrive de l&apos;élève (si configuré).
                Le PDF de transition est retiré de S3 une fois le dépôt réussi.
              </p>

              {detail.convention.oneDriveFilingPending && !detail.convention.oneDriveFiling && (
                <div className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
                  <p className="font-semibold">Dépôt automatique en attente</p>
                  {detail.convention.oneDriveFilingError && (
                    <p className="mt-1 text-xs">{detail.convention.oneDriveFilingError}</p>
                  )}
                </div>
              )}

              {detail.eleveMatch?.targetOneDriveLabel && (
                <p className="mt-2 text-xs text-stone-600">
                  Arborescence cible : <strong>{detail.eleveMatch.targetOneDriveLabel}</strong>
                  {detail.eleveMatch.folderPath ? (
                    <>
                      {" "}
                      — <span className="font-mono">{detail.eleveMatch.folderPath}</span>
                    </>
                  ) : null}
                </p>
              )}

              {detail.convention.oneDriveFiling ? (
                <div className="mt-3 rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
                  <p className="font-semibold">Déjà déposée</p>
                  <p className="mt-1">
                    {detail.convention.oneDriveFiling.folderPath}/{detail.convention.oneDriveFiling.fileName}
                  </p>
                  <p className="mt-1 text-xs text-emerald-800">
                    Par {detail.convention.oneDriveFiling.filedBy} le{" "}
                    {new Date(detail.convention.oneDriveFiling.filedAt).toLocaleString("fr-FR")}
                    {detail.convention.oneDriveFiling.matchedFolderName
                      ? ` — dossier ${detail.convention.oneDriveFiling.matchedFolderName}`
                      : ""}
                  </p>
                </div>
              ) : (
                <>
                  {clerkUser && oneDriveProfile && detail.eleveMatch?.secteur && oneDriveProfile.secteur !== detail.eleveMatch.secteur && (
                    <p className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
                      Cette convention concerne le secteur « {detail.eleveMatch.targetOneDriveLabel ?? detail.eleveMatch.secteur} » — connectez-vous avec le compte Microsoft correspondant.
                    </p>
                  )}
                  {clerkUser && !oneDriveProfile && (
                    <p className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
                      Profil OneDrive non reconnu pour votre compte Clerk — le dépôt utilisera l&apos;arborescence du secteur élève si configurée.
                    </p>
                  )}
                  {oneDriveProfile && (
                    <p className="mt-2 text-xs text-stone-600">
                      Dossier configuré : <strong>{oneDriveProfile.label}</strong> —{" "}
                      <span className="font-mono">{oneDriveProfile.basePath}</span>
                    </p>
                  )}
                  {!od.oneDriveEnabled && od.msalReady && (
                    <p className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
                      OneDrive n&apos;est pas activé pour cet établissement (Paramètres → Intégrations).
                    </p>
                  )}
                  {od.oneDriveEnabled && (
                    <div className="mt-3 flex flex-wrap items-center gap-2">
                      {od.connected ? (
                        <span className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-1.5 text-xs font-semibold text-emerald-800">
                          OneDrive connecté{od.accountLabel ? ` (${od.accountLabel})` : ""}
                        </span>
                      ) : (
                        <button
                          type="button"
                          onClick={() => void od.login()}
                          disabled={!od.msalReady || od.checking}
                          className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-semibold text-white hover:bg-blue-700 disabled:opacity-50"
                        >
                          Se connecter à OneDrive
                        </button>
                      )}
                      <button
                        type="button"
                        onClick={() => void fileToOneDrive()}
                        disabled={
                          busy ||
                          od.checking ||
                          !od.msalReady ||
                          !od.oneDriveEnabled
                        }
                        className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
                      >
                        {busy || od.checking ? "Envoi…" : "Envoyer vers dossier élève"}
                      </button>
                    </div>
                  )}
                  {od.error && (
                    <p className="mt-2 text-xs text-rose-700">{od.error}</p>
                  )}
                </>
              )}
            </div>
          )}
          {permissions?.canReviewPreconvention && detail.convention.status === "convention_deposited" && (
            <div className="mt-4 flex gap-2">
              <button
                type="button"
                onClick={() => void adminReview(true)}
                className="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white"
              >
                Valider → lancer les signatures
              </button>
              <button
                type="button"
                onClick={() => void adminReview(false)}
                className="rounded-lg bg-amber-600 px-4 py-2 text-sm font-semibold text-white"
              >
                Refuser
              </button>
            </div>
          )}
          {permissions?.canReviewPreconvention && detail.convention.status === "admin_review" && (
            <div className="mt-4 flex gap-2">
              <button
                type="button"
                onClick={() => void adminReview(true)}
                className="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white"
              >
                Valider → lancer signatures
              </button>
              <button
                type="button"
                onClick={() => void adminReview(false)}
                className="rounded-lg bg-amber-600 px-4 py-2 text-sm font-semibold text-white"
              >
                Renvoyer pour correction
              </button>
            </div>
          )}
          {detail.signLinks?.length > 0 && (
            <div className="mt-6">
              <h3 className="text-sm font-bold text-stone-700">Liens de signature</h3>
              <ul className="mt-2 space-y-2 text-sm">
                {detail.signLinks.map((s) => (
                  <li key={s.link}>
                    <span className="font-medium">{s.label}</span>
                    {s.email ? ` (${s.email})` : ""} —{" "}
                    <a href={s.link} className="text-[#2F6B4A] underline break-all">
                      {s.link}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </section>
      )}
      <ReplayModuleTourButton moduleId="stages" />
    </main>
  );
}

export default function StagesPage() {
  return (
    <Suspense fallback={<main className="p-8">Chargement…</main>}>
      <StagesContent />
    </Suspense>
  );
}
