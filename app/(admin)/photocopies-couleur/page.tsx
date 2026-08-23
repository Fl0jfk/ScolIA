"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { useCallback, useEffect, useMemo, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";
import EstablishmentSelect from "@/app/components/establishments/EstablishmentSelect";
import { useAppContext } from "@/app/hooks/useAppContext";
import {
  canCreatePhotocopiesDemand,
  canManagePhotocopiesDemand,
  getPhotocopiesRoleFlags,
} from "@/app/lib/photocopies-couleur-access";

type Etablissement = string;
type PhotoCopieStatus = "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE";

type PhotoCopieItem = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: PhotoCopieStatus;
  createdBy: { userId: string; name: string; email: string };
  etablissement: Etablissement;
  motif: string;
  classesOuMatiere: string;
  nombrePhotocopies: number;
  documentFileName?: string;
  decidedBy?: { userId: string; name: string };
  decidedAt?: string;
  directionNote?: string;
};

function statusBadgeClass(s: PhotoCopieStatus) {
  if (s === "ACCEPTEE") return "bg-emerald-50 text-emerald-800 border-emerald-200";
  if (s === "REFUSEE") return "bg-rose-50 text-rose-800 border-rose-200";
  return "bg-amber-50 text-amber-800 border-amber-200";
}

function statusLabel(s: PhotoCopieStatus) {
  if (s === "ACCEPTEE") return "Acceptée";
  if (s === "REFUSEE") return "Refusée";
  return "En attente";
}

export default function PhotocopiesCouleurPage() {
  const { user, isLoaded } = useSessionUser();
  const { data: appCtx } = useAppContext();
  const establishments = appCtx?.establishments ?? [];
  const [items, setItems] = useState<PhotoCopieItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [etablissement, setEtablissement] = useState<Etablissement>("");
  const [motif, setMotif] = useState("");
  const [classesOuMatiere, setClassesOuMatiere] = useState("");
  const [nombrePhotocopies, setNombrePhotocopies] = useState("");
  const [documentFile, setDocumentFile] = useState<File | null>(null);
  const [directionNotes, setDirectionNotes] = useState<Record<string, string>>({});
  const [patchingId, setPatchingId] = useState<string | null>(null);

  const roles = rolesFromUserLike(user);
  const creator = canCreatePhotocopiesDemand(roles);
  const dirFlags = getPhotocopiesRoleFlags(roles);
  const directionAny = dirFlags.isDirection;
  const userEmail = user?.primaryEmailAddress?.emailAddress?.trim() ?? "";

  const fetchItems = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const res = await fetch("/api/photocopies-couleur");
      if (res.status === 403) {
        setItems([]);
        setError("Accès réservé à l’administratif, la vie scolaire, aux enseignants et aux directions.");
        return;
      }
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(data?.error || "Chargement impossible.");
      }
      const list = Array.isArray(data?.items) ? (data.items as PhotoCopieItem[]) : [];
      setItems(list);
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

  const mine = useMemo(
    () =>
      [...items].filter((i) => i.createdBy.userId === user?.id).sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt)),
    [items, user?.id],
  );

  const directionPending = useMemo(
    () =>
      [...items]
        .filter((i) => i.status === "EN_ATTENTE" && canManagePhotocopiesDemand(i, roles, establishments, user?.id))
        .sort((a, b) => +new Date(a.createdAt) - +new Date(b.createdAt)),
    [items, roles],
  );

  const directionHistory = useMemo(
    () =>
      [...items]
        .filter((i) => i.status !== "EN_ATTENTE" && canManagePhotocopiesDemand(i, roles, establishments, user?.id))
        .sort((a, b) => +new Date(b.decidedAt || b.updatedAt) - +new Date(a.decidedAt || a.updatedAt)),
    [items, roles],
  );

  const submit = async () => {
    setError(null);
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
    if (!userEmail) {
      setError("Votre compte ne comporte pas d’adresse e-mail : impossible de recevoir la décision de la direction.");
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
      await fetchItems();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur lors de l’envoi.");
    } finally {
      setSaving(false);
    }
  };

  const patchStatus = async (id: string, status: "ACCEPTEE" | "REFUSEE") => {
    const confirmed = window.confirm(
      status === "ACCEPTEE"
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
        body: JSON.stringify({ id, status, directionNote: note || undefined }),
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

  if (!isLoaded || !user) return null;

  return (
    <ModulePageShell maxWidthClass="max-w-[1500px]" tourModuleId="photocopies-couleur">
      <ModulePageHeader
        eyebrow="Services"
        title="Photocopies couleur"
        description="Demande destinée au service impressions : la direction de l’établissement choisi valide avant traitement opérationnel."
      />

      {creator && !userEmail && (
        <div className="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-900 mb-6">
          Compte sans e-mail principal : vous ne pouvez pas déposer une demande tant qu’aucune adresse n’est joignable pour vous (réception de la décision).
        </div>
      )}

      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
        {creator ? (
          <ModuleCard data-tour="photocopies-new" className="xl:col-span-1 h-fit" bodyClassName="p-6">
            <h2 className={`mb-4 text-xl font-semibold ${dash.ink}`}>Nouvelle demande</h2>
            <div className="space-y-4">
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
                <label className={`mb-2 block ${dash.fieldLabel}`}>
                  Nombre de photocopies
                </label>
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
                <label className={`mb-2 block ${dash.fieldLabel}`}>
                  Document à imprimer (PDF)
                </label>
                <input
                  type="file"
                  accept="application/pdf"
                  onChange={(e) => setDocumentFile(e.target.files?.[0] ?? null)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-1 file:font-semibold file:text-indigo-700"
                />
                <p className="text-[11px] text-slate-500 mt-1">
                  Optionnel : joint à l&apos;e-mail de la direction et, si acceptée, à Madame Périé pour impression directe.
                </p>
              </div>
              {error && <div className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">{error}</div>}
              <ModuleButton onClick={() => void submit()} disabled={saving || !userEmail} className="w-full py-3">
                {saving ? "Envoi..." : "Envoyer la demande"}
              </ModuleButton>
            </div>
          </ModuleCard>
        ) : (
          error &&
          !directionAny && (
            <div className="xl:col-span-3 text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-3">{error}</div>
          )
        )}

        <div className={`space-y-6 ${creator ? "xl:col-span-2" : "xl:col-span-3"}`}>
          {creator && (
            <>
              <div data-tour="photocopies-mine" className="relative rounded-[1.5rem] border border-white/55 bg-white/50 p-4 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-xl">
                <h3 className="font-semibold text-[var(--dash-ink)]">Mes demandes</h3>
                <p className="text-xs text-slate-500 mt-1">Historique personnel (y compris en attente de traitement).</p>
              </div>
              {loading ? (
                <div className="relative rounded-[1.5rem] border border-white/55 bg-white/50 p-8 text-[var(--dash-mid)] shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-xl">Chargement…</div>
              ) : mine.length === 0 ? (
                <div className="relative rounded-[1.5rem] border border-dashed border-white/70 bg-white/40 p-8 text-[var(--dash-mid)]">Aucune demande encore.</div>
              ) : (
                mine.map((item) => (
                  <div key={item.id} className="relative rounded-[1.5rem] border border-white/55 bg-white/50 p-5 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-xl">
                    <div className="flex flex-wrap gap-3 items-center justify-between mb-3">
                      <div>
                        <p className="font-semibold text-[var(--dash-ink)]">
                          {item.etablissement} · {item.nombrePhotocopies} copie(s)
                        </p>
                        <p className="text-xs text-slate-500">
                          Créée le {new Date(item.createdAt).toLocaleString("fr-FR")} — {item.createdBy.email}
                        </p>
                      </div>
                      <span className={`text-xs font-black px-3 py-1.5 rounded-xl border ${statusBadgeClass(item.status)}`}>
                        {statusLabel(item.status)}
                      </span>
                    </div>
                    <p className="text-sm text-slate-700 mb-1">
                      <span className="font-bold">Motif :</span> {item.motif}
                    </p>
                    <p className="text-sm text-slate-700 mb-1">
                      <span className="font-bold">Classes / matière :</span> {item.classesOuMatiere}
                    </p>
                    {item.documentFileName ? (
                      <p className="text-xs text-indigo-700 mb-1">PDF joint : {item.documentFileName}</p>
                    ) : null}
                    {item.directionNote && (
                      <p className="text-sm text-indigo-800 mt-2">
                        <span className="font-bold">Message direction :</span> {item.directionNote}
                      </p>
                    )}
                    {item.decidedBy && (
                      <p className="text-xs text-slate-500 mt-2">
                        Décision par {item.decidedBy.name} le {item.decidedAt ? new Date(item.decidedAt).toLocaleString("fr-FR") : "—"}
                      </p>
                    )}
                  </div>
                ))
              )}
            </>
          )}

          {directionAny && (
            <div data-tour="photocopies-queue">
            <>
              <div className="relative rounded-[1.5rem] border border-white/55 bg-white/50 p-4 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-xl">
                <h3 className="font-semibold text-[var(--dash-ink)]">File de votre pôle</h3>
                <p className="text-xs text-slate-500 mt-1">
                  Demandes pour l’établissement dont vous assurez la direction — en attente ou déjà traitées sur votre périmètre.
                </p>
                {error && !creator && (
                  <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2 mt-3">{error}</p>
                )}
              </div>

              {loading ? (
                <div className="relative rounded-[1.5rem] border border-white/55 bg-white/50 p-8 text-[var(--dash-mid)] shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-xl">Chargement…</div>
              ) : (
                <>
                  <h4 className={`px-1 text-sm font-semibold uppercase tracking-wide ${dash.ink}`}>À traiter</h4>
                  {directionPending.length === 0 ? (
                    <div className="rounded-[1.5rem] border border-dashed border-white/70 bg-white/40 p-6 text-sm text-[var(--dash-mid)]">
                      Aucune demande en attente pour votre périmètre.
                    </div>
                  ) : (
                    directionPending.map((item) => (
                      <div key={item.id} className="relative rounded-[1.5rem] border border-white/55 bg-white/50 p-5 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-xl">
                        <div className="flex flex-wrap gap-3 items-center justify-between mb-3">
                          <div>
                            <p className="font-semibold text-[var(--dash-ink)]">
                              {item.createdBy.name} — {item.etablissement}
                            </p>
                            <p className="text-xs text-slate-500">
                              {item.createdBy.email} · {item.nombrePhotocopies} copie(s) · le{" "}
                              {new Date(item.createdAt).toLocaleString("fr-FR")}
                            </p>
                          </div>
                          <span className={`text-xs font-black px-3 py-1.5 rounded-xl border ${statusBadgeClass(item.status)}`}>
                            {statusLabel(item.status)}
                          </span>
                        </div>
                        <p className="text-sm text-slate-700 mb-1">
                          <span className="font-bold">Motif :</span> {item.motif}
                        </p>
                        <p className="text-sm text-slate-700 mb-3">
                          <span className="font-bold">Classes / matière :</span> {item.classesOuMatiere}
                        </p>
                        {item.documentFileName ? (
                          <p className="text-xs text-indigo-700 mb-3">PDF joint : {item.documentFileName}</p>
                        ) : null}
                        <label className={`mb-2 block ${dash.fieldLabel}`}>
                          Note pour le demandeur (optionnel)
                        </label>
                        <textarea
                          rows={2}
                          value={directionNotes[item.id] ?? ""}
                          onChange={(e) => setDirectionNotes((p) => ({ ...p, [item.id]: e.target.value }))}
                          className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm mb-3"
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
                            className="px-4 py-2 rounded-xl bg-rose-600 hover:bg-rose-700 text-white font-bold text-sm disabled:opacity-60"
                          >
                            Refuser
                          </button>
                        </div>
                      </div>
                    ))
                  )}

                  <h4 className={`px-1 pt-4 text-sm font-semibold uppercase tracking-wide ${dash.ink}`}>Traitées (pôle)</h4>
                  {directionHistory.length === 0 ? (
                    <div className="rounded-[1.5rem] border border-dashed border-white/70 bg-white/40 p-6 text-sm text-[var(--dash-mid)]">
                      Pas encore d’historique de décision sur votre périmètre.
                    </div>
                  ) : (
                    directionHistory.map((item) => (
                      <div key={item.id} className="relative rounded-[1.5rem] border border-white/55 bg-white/50 p-5 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-xl opacity-95">
                        <div className="flex flex-wrap gap-3 items-center justify-between mb-2">
                          <p className="font-semibold text-[var(--dash-ink)]">
                            {item.createdBy.name} — {item.etablissement}
                          </p>
                          <span className={`text-xs font-black px-3 py-1.5 rounded-xl border ${statusBadgeClass(item.status)}`}>
                            {statusLabel(item.status)}
                          </span>
                        </div>
                        <p className="text-xs text-slate-500 mb-2">
                          {item.nombrePhotocopies} copie(s) · {new Date(item.createdAt).toLocaleDateString("fr-FR")}
                        </p>
                        <p className="text-sm text-slate-700">{item.motif}</p>
                        {item.directionNote && (
                          <p className="text-sm text-indigo-800 mt-2">
                            <span className="font-bold">Note :</span> {item.directionNote}
                          </p>
                        )}
                      </div>
                    ))
                  )}
                </>
              )}
            </>
          </div>
          )}

          {!creator && !directionAny && !loading && (
            <div className="rounded-[1.5rem] border border-white/55 bg-white/50 p-8 text-[var(--dash-mid)] backdrop-blur-xl">
              Votre profil ne permet pas d’accéder à cette page. Contactez l’administrateur si vous pensez qu’il s’agit d’une erreur.
            </div>
          )}
        </div>
      </div>
    </ModulePageShell>
  );
}
