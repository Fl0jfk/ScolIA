"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";
import EstablishmentSelect from "@/app/components/establishments/EstablishmentSelect";
import { useAppContext } from "@/app/hooks/useAppContext";
import {
  canCreatePhotocopiesDemand,
  canDeclarePhotocopiesOnBehalf,
  canManagePhotocopiesDemand,
  getPhotocopiesRoleFlags,
} from "@/app/lib/photocopies-couleur-access";
import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";
import DirectoryPersonSelect, {
  directoryMemberLabel,
} from "@/app/components/settings/DirectoryPersonSelect";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import {
  photoCopieStatusBadgeClass,
  photoCopieStatusLabel,
  type PhotoCopieRecord,
  type PhotoCopieStatus,
} from "@/app/lib/photocopies-couleur-types";

async function openPhotocopieDocument(id: string): Promise<void> {
  const res = await fetch(`/api/photocopies-couleur/document?id=${encodeURIComponent(id)}`, {
    cache: "no-store",
  });
  const data = (await res.json().catch(() => ({}))) as { signedUrl?: string; error?: string };
  if (!res.ok || !data.signedUrl) {
    throw new Error(data.error || "Impossible d'ouvrir le PDF.");
  }
  window.open(data.signedUrl, "_blank", "noopener,noreferrer");
}

function PhotocopieDocumentButton({
  item,
  emphasize,
}: {
  item: PhotoCopieRecord;
  /** Mise en avant pour la file d'impression (ouvrir pour imprimer). */
  emphasize?: boolean;
}) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  if (!item.documentKey || !item.documentFileName) {
    return (
      <p className="text-xs text-amber-800 bg-amber-50 border border-amber-100 rounded-lg px-2.5 py-1.5 mb-3">
        Aucun PDF joint à cette demande.
      </p>
    );
  }
  return (
    <div className="mb-3">
      <button
        type="button"
        disabled={busy}
        onClick={() => {
          setErr(null);
          setBusy(true);
          void openPhotocopieDocument(item.id)
            .catch((e: unknown) => {
              setErr(e instanceof Error ? e.message : "Ouverture impossible.");
            })
            .finally(() => setBusy(false));
        }}
        className={
          emphasize
            ? "inline-flex items-center gap-2 rounded-xl border border-indigo-300 bg-indigo-600 px-3 py-2 text-sm font-bold text-white hover:bg-indigo-700 disabled:opacity-60"
            : "inline-flex items-center gap-2 rounded-xl border border-indigo-200 bg-indigo-50 px-3 py-1.5 text-xs font-bold text-indigo-800 hover:bg-indigo-100 disabled:opacity-60"
        }
      >
        {busy ? "Ouverture…" : emphasize ? "Ouvrir le PDF pour imprimer" : "Voir / télécharger le PDF"}
        <span className={emphasize ? "font-medium opacity-90" : "font-normal text-indigo-600"}>
          {item.documentFileName}
        </span>
      </button>
      {err ? <p className="mt-1 text-xs text-rose-700">{err}</p> : null}
    </div>
  );
}

export default function PhotocopiesCouleurPage() {
  const { user, isLoaded } = useSessionUser();
  const { data: appCtx } = useAppContext();
  const establishments = appCtx?.establishments ?? [];
  const [items, setItems] = useState<PhotoCopieRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [etablissement, setEtablissement] = useState("");
  const [motif, setMotif] = useState("");
  const [classesOuMatiere, setClassesOuMatiere] = useState("");
  const [nombrePhotocopies, setNombrePhotocopies] = useState("");
  const [documentFile, setDocumentFile] = useState<File | null>(null);
  const [directionNotes, setDirectionNotes] = useState<Record<string, string>>({});
  const [patchingId, setPatchingId] = useState<string | null>(null);
  const [forOther, setForOther] = useState(false);
  const [colleague, setColleague] = useState<DirectoryMemberOption | null>(null);
  const [directoryMembers, setDirectoryMembers] = useState<DirectoryMemberOption[]>([]);
  const [loadingDirectory, setLoadingDirectory] = useState(false);
  const [isOpsHandler, setIsOpsHandler] = useState(false);

  const roles = rolesFromUserLike(user);
  const creator = canCreatePhotocopiesDemand(roles);
  const canOnBehalf = canDeclarePhotocopiesOnBehalf(roles);
  const dirFlags = getPhotocopiesRoleFlags(roles);
  const directionAny =
    dirFlags.isDirection ||
    hasGlobalAdminRole(roles) ||
    hasMasterRole(roles) ||
    roles.includes("admin");
  const userEmail = user?.primaryEmailAddress?.emailAddress?.trim() ?? "";

  const fetchItems = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const res = await fetch("/api/photocopies-couleur");
      if (res.status === 403) {
        setItems([]);
        setIsOpsHandler(false);
        setError("Accès réservé à l'administratif, la vie scolaire, aux enseignants et aux directions.");
        return;
      }
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(data?.error || "Chargement impossible.");
      }
      const list = Array.isArray(data?.items) ? (data.items as PhotoCopieRecord[]) : [];
      setItems(list);
      setIsOpsHandler(Boolean(data?.isOpsHandler));
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : "Erreur de chargement.";
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (isLoaded && user) void fetchItems();
  }, [isLoaded, user, fetchItems]);

  useEffect(() => {
    if (!canOnBehalf) return;
    let cancelled = false;
    setLoadingDirectory(true);
    fetch("/api/photocopies-couleur/directory-users", { cache: "no-store" })
      .then(async (res) => {
        const data = await res.json().catch(() => ({}));
        if (!res.ok) throw new Error(data?.error || "Annuaire indisponible.");
        if (!cancelled) {
          setDirectoryMembers(Array.isArray(data?.users) ? data.users : []);
        }
      })
      .catch(() => {
        if (!cancelled) setDirectoryMembers([]);
      })
      .finally(() => {
        if (!cancelled) setLoadingDirectory(false);
      });
    return () => {
      cancelled = true;
    };
  }, [canOnBehalf]);

  const mine = useMemo(
    () =>
      [...items]
        .filter((i) => i.createdBy.userId === user?.id)
        .sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt)),
    [items, user?.id],
  );

  const readyMine = useMemo(() => mine.filter((i) => i.status === "PRETE"), [mine]);

  const opsPrintQueue = useMemo(
    () =>
      [...items]
        .filter((i) => i.status === "ACCEPTEE")
        .sort((a, b) => +new Date(a.createdAt) - +new Date(b.createdAt)),
    [items],
  );

  const opsDone = useMemo(
    () =>
      [...items]
        .filter((i) => i.status === "PRETE")
        .sort((a, b) => +new Date(b.readyAt || b.updatedAt) - +new Date(a.readyAt || a.updatedAt)),
    [items],
  );

  const directionPending = useMemo(
    () =>
      [...items]
        .filter((i) => i.status === "EN_ATTENTE" && canManagePhotocopiesDemand(i, roles, establishments, user?.id))
        .sort((a, b) => +new Date(a.createdAt) - +new Date(b.createdAt)),
    [items, roles, establishments, user?.id],
  );

  const directionHistory = useMemo(
    () =>
      [...items]
        .filter((i) => i.status !== "EN_ATTENTE" && canManagePhotocopiesDemand(i, roles, establishments, user?.id))
        .sort((a, b) => +new Date(b.decidedAt || b.updatedAt) - +new Date(a.decidedAt || a.updatedAt)),
    [items, roles, establishments, user?.id],
  );

  const submit = async () => {
    setError(null);
    if (forOther && !colleague) {
      setError("Choisissez l'enseignant concerné.");
      return;
    }
    if (!nombrePhotocopies.trim()) {
      setError("Indiquez un nombre de photocopies.");
      return;
    }
    const n = Number(nombrePhotocopies);
    if (!Number.isFinite(n) || n < 1) {
      setError("Le nombre doit être un entier strictement positif.");
      return;
    }
    if (!motif.trim()) {
      setError("Le motif est obligatoire.");
      return;
    }
    if (!classesOuMatiere.trim()) {
      setError("Indiquez les classes ou la matière concernée.");
      return;
    }
    if (!forOther && !userEmail) {
      setError("Votre compte ne comporte pas d'adresse e-mail : impossible de recevoir la décision de la direction.");
      return;
    }
    try {
      setSaving(true);
      let documentKey: string | undefined;
      let documentFileName: string | undefined;
      let documentContentType: string | undefined;

      if (documentFile) {
        if (documentFile.type !== "application/pdf") {
          setError("Le document à imprimer doit être un PDF.");
          setSaving(false);
          return;
        }
        const prep = await fetch("/api/photocopies-couleur/upload-url", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ fileName: documentFile.name, contentType: documentFile.type }),
        });
        const prepJson = await prep.json();
        if (!prep.ok) throw new Error(prepJson.error || "Préparation du fichier impossible.");
        const put = await fetch(prepJson.uploadUrl, {
          method: "PUT",
          headers: { "Content-Type": documentFile.type },
          body: documentFile,
        });
        if (!put.ok) throw new Error("Envoi du PDF échoué.");
        documentKey = prepJson.key;
        documentFileName = documentFile.name;
        documentContentType = documentFile.type;
      }

      const res = await fetch("/api/photocopies-couleur", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          etablissement,
          motif: motif.trim(),
          classesOuMatiere: classesOuMatiere.trim(),
          nombrePhotocopies: n,
          ...(forOther && colleague ? { onBehalfOf: { userId: colleague.externalUserId } } : {}),
          ...(documentKey ? { documentKey, documentFileName, documentContentType } : {}),
        }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(data?.error || "Enregistrement impossible.");
      }
      setMotif("");
      setClassesOuMatiere("");
      setNombrePhotocopies("");
      setDocumentFile(null);
      setForOther(false);
      setColleague(null);
      await fetchItems();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur lors de l'envoi.");
    } finally {
      setSaving(false);
    }
  };

  const patchStatus = async (id: string, status: "ACCEPTEE" | "REFUSEE" | "PRETE") => {
    const confirmed = window.confirm(
      status === "PRETE"
        ? "Marquer ces photocopies comme imprimées / prêtes et notifier le demandeur ?"
        : status === "ACCEPTEE"
          ? "Marquer cette demande comme acceptée et notifier le demandeur ?"
          : "Marquer cette demande comme refusée et notifier le demandeur ?",
    );
    if (!confirmed) return;
    try {
      setPatchingId(id);
      setError(null);
      const note = (directionNotes[id] ?? "").trim();
      const res = await fetch("/api/photocopies-couleur", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          id,
          status,
          ...(status !== "PRETE" ? { directionNote: note || undefined } : {}),
        }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data?.error || "Décision impossible.");
      setDirectionNotes((p) => {
        const n = { ...p };
        delete n[id];
        return n;
      });
      await fetchItems();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur.");
    } finally {
      setPatchingId(null);
    }
  };

  const renderItemCard = (item: PhotoCopieRecord, opts?: { showReadyBanner?: boolean }) => (
    <div
      key={item.id}
      className={`rounded-2xl border bg-white p-4 sm:p-5 ${
        item.status === "PRETE"
          ? "border-emerald-200 shadow-[0_0_0_1px_rgba(16,185,129,0.12)]"
          : "border-slate-200/90"
      }`}
    >
      {opts?.showReadyBanner && item.status === "PRETE" ? (
        <div className="mb-3 rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm font-semibold text-emerald-800">
          Vos photocopies sont prêtes — vous pouvez venir les retirer.
        </div>
      ) : null}
      <div className="flex flex-wrap items-start justify-between gap-3 mb-3">
        <div className="min-w-0">
          <p className={`font-semibold ${dash.ink}`}>
            {item.etablissement}
            <span className="font-medium text-slate-500"> · {item.nombrePhotocopies} copie(s)</span>
          </p>
          <p className="text-xs text-slate-500 mt-0.5">
            {new Date(item.createdAt).toLocaleString("fr-FR")}
            {item.createdBy.name ? ` — ${item.createdBy.name}` : ""}
          </p>
          {item.submittedBy ? (
            <p className="text-xs text-slate-600 mt-0.5">
              Déposée par {item.submittedBy.name} pour l&apos;enseignant
            </p>
          ) : null}
        </div>
        <span
          className={`shrink-0 text-[11px] font-bold px-2.5 py-1 rounded-lg border ${photoCopieStatusBadgeClass(item.status)}`}
        >
          {photoCopieStatusLabel(item.status)}
        </span>
      </div>
      <dl className="space-y-1.5 text-sm text-slate-700 mb-2">
        <div>
          <dt className="inline text-slate-500">Motif · </dt>
          <dd className="inline">{item.motif}</dd>
        </div>
        <div>
          <dt className="inline text-slate-500">Classes / matière · </dt>
          <dd className="inline">{item.classesOuMatiere}</dd>
        </div>
      </dl>
      {item.documentFileName || item.documentKey ? <PhotocopieDocumentButton item={item} /> : null}
      {item.directionNote ? (
        <p className="text-sm text-slate-700 mt-2 rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
          <span className="font-semibold text-slate-800">Message direction · </span>
          {item.directionNote}
        </p>
      ) : null}
      {item.readyAt ? (
        <p className="text-xs text-emerald-700 mt-2">
          Prêt le {new Date(item.readyAt).toLocaleString("fr-FR")}
          {item.readyBy ? ` — ${item.readyBy}` : ""}
        </p>
      ) : null}
      {item.decidedBy ? (
        <p className="text-xs text-slate-500 mt-2">
          Décision par {item.decidedBy.name}
          {item.decidedAt ? ` le ${new Date(item.decidedAt).toLocaleString("fr-FR")}` : ""}
        </p>
      ) : null}
    </div>
  );

  const EmptyHint = ({ children }: { children: ReactNode }) => (
    <div className="rounded-2xl border border-dashed border-slate-200 bg-slate-50/70 px-4 py-8 text-center text-sm text-slate-500">
      {children}
    </div>
  );

  const SectionLabel = ({
    title,
    count,
  }: {
    title: string;
    count?: number;
  }) => (
    <div className="flex items-center gap-2 mb-3">
      <h4 className={`text-sm font-semibold ${dash.ink}`}>{title}</h4>
      {typeof count === "number" ? (
        <span className="inline-flex min-w-[1.4rem] justify-center rounded-md bg-slate-100 px-1.5 py-0.5 text-[11px] font-bold text-slate-600">
          {count}
        </span>
      ) : null}
    </div>
  );

  if (!isLoaded || !user) return null;

  const submitDisabled = saving || (forOther ? !colleague : !userEmail);

  return (
    <ModulePageShell maxWidthClass="max-w-[1500px]" tourModuleId="photocopies-couleur">
      <ModulePageHeader
        eyebrow="Services"
        title="Photocopies couleur"
        description="Demande destinée au service impressions : la direction de l'établissement choisi valide avant traitement opérationnel."
      />

      {creator && !forOther && !userEmail && (
        <div className="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-900 mb-6">
          Compte sans e-mail principal : vous ne pouvez pas déposer une demande tant qu'aucune adresse n'est joignable
          pour vous (réception de la décision).
        </div>
      )}

      {readyMine.length > 0 && (
        <div className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900 mb-6">
          <span className="font-bold">
            {readyMine.length === 1
              ? "1 demande prête à retirer"
              : `${readyMine.length} demandes prêtes à retirer`}
          </span>
          <span className="block text-xs text-emerald-800 mt-1">
            Vos photocopies couleur ont été traitées par le service impressions.
          </span>
        </div>
      )}

      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6 items-start">
        {creator ? (
          <ModuleCard data-tour="photocopies-new" className="xl:col-span-1 h-fit" bodyClassName="p-6">
            <h2 className={`mb-4 text-xl font-semibold ${dash.ink}`}>Nouvelle demande</h2>
            <div className="space-y-4">
              {canOnBehalf ? (
                <label className="flex items-start gap-3 rounded-xl border border-slate-200 bg-slate-50 px-3 py-3 cursor-pointer">
                  <input
                    type="checkbox"
                    className="mt-1 h-4 w-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                    checked={forOther}
                    onChange={(e) => {
                      const checked = e.target.checked;
                      setForOther(checked);
                      setError(null);
                      if (!checked) setColleague(null);
                    }}
                  />
                  <span className="text-sm text-slate-700">
                    <span className="font-bold text-slate-900">Pour une autre personne</span>
                    <span className="block text-xs text-slate-500 mt-0.5">
                      Déposer une demande pour un enseignant — elle apparaîtra dans son espace, pas dans le vôtre.
                    </span>
                  </span>
                </label>
              ) : null}

              {canOnBehalf && forOther ? (
                <div>
                  <label className={`mb-2 block ${dash.fieldLabel}`}>Enseignant concerné</label>
                  <DirectoryPersonSelect
                    members={directoryMembers}
                    selectedId={colleague?.externalUserId}
                    selectedEmail={colleague?.email}
                    onChange={(m) => {
                      if (!m) {
                        setColleague(null);
                        return;
                      }
                      const full =
                        directoryMembers.find((x) => x.externalUserId === m.externalUserId) || m;
                      setColleague(full);
                    }}
                    loading={loadingDirectory}
                  />
                  {colleague ? (
                    <p className="text-xs text-slate-500 mt-1">
                      Demande pour <span className="font-semibold">{directoryMemberLabel(colleague)}</span>
                    </p>
                  ) : null}
                </div>
              ) : null}

              <div>
                <label className={`mb-2 block ${dash.fieldLabel}`}>Établissement</label>
                <EstablishmentSelect
                  value={etablissement}
                  onChange={setEtablissement}
                  establishments={establishments}
                  includeGroupe={false}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"
                />
              </div>
              <div>
                <label className={`mb-2 block ${dash.fieldLabel}`}>Motif</label>
                <textarea
                  value={motif}
                  onChange={(e) => setMotif(e.target.value)}
                  rows={4}
                  placeholder="Contexte ou usage prévu..."
                  className={dash.field}
                />
              </div>
              <div>
                <label className={`mb-2 block ${dash.fieldLabel}`}>Classes ou matière</label>
                <input
                  value={classesOuMatiere}
                  onChange={(e) => setClassesOuMatiere(e.target.value)}
                  type="text"
                  placeholder="Ex. : 4e B, latin, cours de…"
                  className={dash.field}
                />
              </div>
              <div>
                <label className={`mb-2 block ${dash.fieldLabel}`}>Nombre de photocopies</label>
                <input
                  value={nombrePhotocopies}
                  onChange={(e) => setNombrePhotocopies(e.target.value)}
                  type="number"
                  min={1}
                  placeholder="Entier strictement positif"
                  className={dash.field}
                />
              </div>
              <div>
                <label className={`mb-2 block ${dash.fieldLabel}`}>Document à imprimer (PDF)</label>
                <input
                  type="file"
                  accept="application/pdf"
                  onChange={(e) => setDocumentFile(e.target.files?.[0] ?? null)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-1 file:font-semibold file:text-indigo-700"
                />
                <p className="text-[11px] text-slate-500 mt-1">
                  Recommandé : visible par la direction à la validation, puis par le service impressions.
                </p>
              </div>
              {error && (
                <div className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">
                  {error}
                </div>
              )}
              <ModuleButton onClick={() => void submit()} disabled={submitDisabled} className="w-full py-3">
                {saving
                  ? "Envoi..."
                  : forOther
                    ? "Envoyer la demande pour cet enseignant"
                    : "Envoyer la demande"}
              </ModuleButton>
            </div>
          </ModuleCard>
        ) : (
          error &&
          !directionAny && (
            <div className="xl:col-span-3 text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-3">
              {error}
            </div>
          )
        )}

        <div className={`space-y-5 ${creator ? "xl:col-span-2" : "xl:col-span-3"}`}>
          {creator && (
            <ModuleCard data-tour="photocopies-mine" bodyClassName="p-5 sm:p-6">
              <div className="flex flex-wrap items-baseline justify-between gap-2 mb-1">
                <h3 className={`text-lg font-semibold ${dash.ink}`}>Mes demandes</h3>
                {!loading ? (
                  <span className="text-xs font-medium text-slate-500">{mine.length} au total</span>
                ) : null}
              </div>
              <p className="text-xs text-slate-500 mb-4">Suivi de vos demandes, y compris en attente.</p>
              {loading ? (
                <p className="text-sm text-slate-500 py-6 text-center">Chargement…</p>
              ) : mine.length === 0 ? (
                <EmptyHint>Aucune demande pour le moment.</EmptyHint>
              ) : (
                <div className="space-y-3">{mine.map((item) => renderItemCard(item, { showReadyBanner: true }))}</div>
              )}
            </ModuleCard>
          )}

          {isOpsHandler && (
            <ModuleCard data-tour="photocopies-ops-queue" bodyClassName="p-5 sm:p-6">
              <div className="flex flex-wrap items-baseline justify-between gap-2 mb-1">
                <h3 className={`text-lg font-semibold ${dash.ink}`}>File d&apos;impression</h3>
                {!loading ? (
                  <span className="text-xs font-medium text-slate-500">
                    {opsPrintQueue.length} à imprimer
                  </span>
                ) : null}
              </div>
              <p className="text-xs text-slate-500 mb-4">
                Demandes acceptées par la direction. Une validation « prête » notifie le demandeur pour tous les
                réceptionnaires.
              </p>

              {loading ? (
                <p className="text-sm text-slate-500 py-6 text-center">Chargement…</p>
              ) : (
                <div className="space-y-5">
                  <div>
                    <SectionLabel title="À imprimer" count={opsPrintQueue.length} />
                    {opsPrintQueue.length === 0 ? (
                      <EmptyHint>Rien en attente d&apos;impression.</EmptyHint>
                    ) : (
                      <div className="space-y-3">
                        {opsPrintQueue.map((item) => (
                          <div
                            key={item.id}
                            className="rounded-2xl border border-sky-200 bg-sky-50/40 p-4 sm:p-5"
                          >
                            <div className="flex flex-wrap items-start justify-between gap-3 mb-3">
                              <div className="min-w-0">
                                <p className={`font-semibold ${dash.ink}`}>
                                  {item.createdBy.name}
                                  <span className="font-medium text-slate-500">
                                    {" "}
                                    — {item.etablissement}
                                  </span>
                                </p>
                                <p className="text-xs text-slate-500 mt-0.5">
                                  {item.nombrePhotocopies} copie(s)
                                  {item.createdBy.email ? ` · ${item.createdBy.email}` : ""}
                                  {" · acceptée "}
                                  {item.decidedAt
                                    ? new Date(item.decidedAt).toLocaleString("fr-FR")
                                    : new Date(item.createdAt).toLocaleString("fr-FR")}
                                </p>
                              </div>
                              <span
                                className={`shrink-0 text-[11px] font-bold px-2.5 py-1 rounded-lg border ${photoCopieStatusBadgeClass("ACCEPTEE")}`}
                              >
                                À imprimer
                              </span>
                            </div>
                            <dl className="space-y-1.5 text-sm text-slate-700 mb-3">
                              <div>
                                <dt className="inline text-slate-500">Motif · </dt>
                                <dd className="inline">{item.motif}</dd>
                              </div>
                              <div>
                                <dt className="inline text-slate-500">Classes / matière · </dt>
                                <dd className="inline">{item.classesOuMatiere}</dd>
                              </div>
                            </dl>
                            <PhotocopieDocumentButton item={item} emphasize />
                            {item.directionNote ? (
                              <p className="text-sm text-slate-700 mb-3 rounded-xl bg-white border border-slate-100 px-3 py-2">
                                <span className="font-semibold">Note direction · </span>
                                {item.directionNote}
                              </p>
                            ) : null}
                            <button
                              type="button"
                              disabled={patchingId === item.id}
                              onClick={() => void patchStatus(item.id, "PRETE")}
                              className="px-4 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-sm disabled:opacity-60"
                            >
                              {patchingId === item.id ? "…" : "Marquer comme imprimée / prête"}
                            </button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {opsDone.length > 0 ? (
                    <div>
                      <SectionLabel title="Récemment prêtes" count={Math.min(opsDone.length, 8)} />
                      <div className="space-y-3">
                        {opsDone.slice(0, 8).map((item) => renderItemCard(item))}
                      </div>
                    </div>
                  ) : null}
                </div>
              )}
            </ModuleCard>
          )}

          {directionAny && (
            <ModuleCard data-tour="photocopies-queue" bodyClassName="p-5 sm:p-6">
              <div className="flex flex-wrap items-baseline justify-between gap-2 mb-1">
                <h3 className={`text-lg font-semibold ${dash.ink}`}>File de votre pôle</h3>
                {!loading ? (
                  <span className="text-xs font-medium text-slate-500">
                    {directionPending.length} en attente
                  </span>
                ) : null}
              </div>
              <p className="text-xs text-slate-500 mb-4">
                Demandes de l&apos;établissement dont vous assurez la direction.
              </p>
              {error && !creator ? (
                <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2 mb-4">
                  {error}
                </p>
              ) : null}

              {loading ? (
                <p className="text-sm text-slate-500 py-6 text-center">Chargement…</p>
              ) : (
                <div className="space-y-5">
                  <div>
                    <SectionLabel title="À traiter" count={directionPending.length} />
                    {directionPending.length === 0 ? (
                      <EmptyHint>Aucune demande en attente.</EmptyHint>
                    ) : (
                      <div className="space-y-3">
                        {directionPending.map((item) => (
                          <div
                            key={item.id}
                            className="rounded-2xl border border-amber-200/80 bg-amber-50/30 p-4 sm:p-5"
                          >
                            <div className="flex flex-wrap items-start justify-between gap-3 mb-3">
                              <div className="min-w-0">
                                <p className={`font-semibold ${dash.ink}`}>
                                  {item.createdBy.name}
                                  <span className="font-medium text-slate-500">
                                    {" "}
                                    — {item.etablissement}
                                  </span>
                                </p>
                                <p className="text-xs text-slate-500 mt-0.5">
                                  {item.nombrePhotocopies} copie(s)
                                  {item.createdBy.email ? ` · ${item.createdBy.email}` : ""}
                                  {" · "}
                                  {new Date(item.createdAt).toLocaleString("fr-FR")}
                                </p>
                                {item.submittedBy ? (
                                  <p className="text-xs text-slate-600 mt-0.5">
                                    Déposée par {item.submittedBy.name}
                                  </p>
                                ) : null}
                              </div>
                              <span
                                className={`shrink-0 text-[11px] font-bold px-2.5 py-1 rounded-lg border ${photoCopieStatusBadgeClass(item.status as PhotoCopieStatus)}`}
                              >
                                {photoCopieStatusLabel(item.status as PhotoCopieStatus)}
                              </span>
                            </div>
                            <dl className="space-y-1.5 text-sm text-slate-700 mb-3">
                              <div>
                                <dt className="inline text-slate-500">Motif · </dt>
                                <dd className="inline">{item.motif}</dd>
                              </div>
                              <div>
                                <dt className="inline text-slate-500">Classes / matière · </dt>
                                <dd className="inline">{item.classesOuMatiere}</dd>
                              </div>
                            </dl>
                            <PhotocopieDocumentButton item={item} emphasize />
                            <label className={`mb-2 block ${dash.fieldLabel}`}>
                              Note pour le demandeur (optionnel)
                            </label>
                            <textarea
                              rows={2}
                              value={directionNotes[item.id] ?? ""}
                              onChange={(e) =>
                                setDirectionNotes((p) => ({ ...p, [item.id]: e.target.value }))
                              }
                              className="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm mb-3"
                              placeholder="Ex. : à retirer au secrétariat, délai, motif du refus…"
                            />
                            <div className="flex flex-wrap gap-2">
                              <button
                                type="button"
                                disabled={patchingId === item.id}
                                onClick={() => void patchStatus(item.id, "ACCEPTEE")}
                                className="px-4 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-sm disabled:opacity-60"
                              >
                                {patchingId === item.id ? "…" : "Accepter"}
                              </button>
                              <button
                                type="button"
                                disabled={patchingId === item.id}
                                onClick={() => void patchStatus(item.id, "REFUSEE")}
                                className="px-4 py-2 rounded-xl border border-rose-200 bg-white text-rose-700 hover:bg-rose-50 font-bold text-sm disabled:opacity-60"
                              >
                                Refuser
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  <div>
                    <SectionLabel title="Traitées" count={directionHistory.length} />
                    {directionHistory.length === 0 ? (
                      <EmptyHint>Pas encore d&apos;historique sur votre périmètre.</EmptyHint>
                    ) : (
                      <div className="space-y-3">
                        {directionHistory.map((item) => renderItemCard(item))}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </ModuleCard>
          )}

          {!creator && !directionAny && !isOpsHandler && !loading && (
            <ModuleCard bodyClassName="p-6">
              <p className="text-sm text-slate-600 text-center py-4">
                Votre profil ne permet pas d&apos;accéder à cette page. Contactez l&apos;administrateur si besoin.
              </p>
            </ModuleCard>
          )}
        </div>
      </div>
    </ModulePageShell>
  );
}
