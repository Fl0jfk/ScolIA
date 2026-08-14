"use client";

import { useUser } from "@clerk/nextjs";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { useCallback, useEffect, useMemo, useState } from "react";
import EstablishmentSelect from "@/app/components/establishments/EstablishmentSelect";
import { useAppContext } from "@/app/hooks/useAppContext";
import {
  canCreateHseDemand,
  canManageHseDemand,
  getHseRoleFlags,
} from "@/app/lib/demandes-hse-access";

type Etablissement = string;
type HseStatus = "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE" | "ANNULEE";

type HseItem = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: HseStatus;
  createdBy: { userId: string; name: string; email: string };
  etablissement: Etablissement;
  resumeDemande: string;
  motif: string;
  nombreHeures?: number;
  classe: string;
  details: string;
  decidedBy?: { userId: string; name: string };
  decidedAt?: string;
  directionNote?: string;
};


function canManageItem(item: HseItem, roles: string[], establishments: import("@/app/lib/app-config-schemas").Establishment[], userId?: string | null) {
  return canManageHseDemand(item, roles, establishments, userId);
}

function statusBadgeClass(s: HseStatus) {
  if (s === "ACCEPTEE") return "bg-emerald-50 text-emerald-800 border-emerald-200";
  if (s === "REFUSEE") return "bg-rose-50 text-rose-800 border-rose-200";
  if (s === "ANNULEE") return "bg-slate-100 text-slate-600 border-slate-200";
  return "bg-amber-50 text-amber-800 border-amber-200";
}

function statusLabel(s: HseStatus) {
  if (s === "ACCEPTEE") return "Acceptée";
  if (s === "REFUSEE") return "Refusée";
  if (s === "ANNULEE") return "Annulée";
  return "En attente";
}

function formatNombreHeures(h: number): string {
  const text = Number.isInteger(h) ? String(h) : h.toFixed(2).replace(/\.?0+$/, "").replace(".", ",");
  return `${text} h`;
}

function AttestationPdfLink({ id }: { id: string }) {
  return (
    <a
      href={`/api/demandes-hse/${id}/pdf`}
      className="inline-flex items-center gap-1.5 px-3 py-2 rounded-xl border border-indigo-200 bg-indigo-50 text-indigo-800 font-bold text-sm hover:bg-indigo-100"
      download
    >
      Télécharger l&apos;attestation PDF
    </a>
  );
}

export default function DemandesHsePanel({
  embeddedInRh = false,
}: {
  embeddedInRh?: boolean;
} = {}) {
  const { user, isLoaded } = useUser();
  const { data: appCtx } = useAppContext();
  const establishments = appCtx?.establishments ?? [];
  const [items, setItems] = useState<HseItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [etablissement, setEtablissement] = useState<Etablissement>("");
  const [resumeDemande, setResumeDemande] = useState("");
  const [nombreHeures, setNombreHeures] = useState("");
  const [classe, setClasse] = useState("");
  const [details, setDetails] = useState("");
  const [directionNotes, setDirectionNotes] = useState<Record<string, string>>({});
  const [patchingId, setPatchingId] = useState<string | null>(null);
  const [cancellingId, setCancellingId] = useState<string | null>(null);

  const roles = rolesFromUserLike(user);
  const creator = canCreateHseDemand(roles);
  const dirFlags = getHseRoleFlags(roles);
  const directionAny = dirFlags.isDirection;
  const userEmail = user?.primaryEmailAddress?.emailAddress?.trim() ?? "";

  const fetchItems = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const res = await fetch("/api/demandes-hse");
      if (res.status === 403) {
        setItems([]);
        setError("Accès réservé aux enseignants et aux directions d’établissement.");
        return;
      }
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(data?.error || "Chargement impossible.");
      }
      const list = Array.isArray(data?.items) ? (data.items as HseItem[]) : [];
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
        .filter((i) => i.status === "EN_ATTENTE" && canManageItem(i, roles, establishments, user?.id))
        .sort((a, b) => +new Date(a.createdAt) - +new Date(b.createdAt)),
    [items, roles],
  );

  const directionHistory = useMemo(
    () =>
      [...items]
        .filter(
          (i) =>
            (i.status === "ACCEPTEE" || i.status === "REFUSEE") && canManageItem(i, roles, establishments, user?.id),
        )
        .sort((a, b) => +new Date(b.decidedAt || b.updatedAt) - +new Date(a.decidedAt || a.updatedAt)),
    [items, roles],
  );

  const cancelDemand = async (id: string) => {
    const confirmed = window.confirm(
      "Annuler cette demande HSE ? Elle disparaîtra de la file de la direction.",
    );
    if (!confirmed) return;
    try {
      setCancellingId(id);
      setError(null);
      const res = await fetch("/api/demandes-hse", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, status: "ANNULEE" }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data?.error || "Annulation impossible.");
      await fetchItems();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur.");
    } finally {
      setCancellingId(null);
    }
  };

  const submit = async () => {
    setError(null);
    if (!resumeDemande.trim()) {
      setError("Décrivez votre demande (objet et motivation).");
      return;
    }
    const heures = Number(String(nombreHeures).trim().replace(",", "."));
    if (!Number.isFinite(heures) || heures <= 0) {
      setError("Indiquez le nombre d’heures demandé.");
      return;
    }
    if (Math.abs(heures * 4 - Math.round(heures * 4)) > 1e-6) {
      setError("Le nombre d’heures doit être un multiple de 0,25 (ex. 1, 1,25, 2,5…).");
      return;
    }
    if (!classe.trim()) {
      setError("Indiquez la classe ou le contexte.");
      return;
    }
    if (!userEmail) {
      setError("Votre compte ne comporte pas d’adresse e-mail : impossible de recevoir la décision de la direction.");
      return;
    }
    try {
      setSaving(true);
      const res = await fetch("/api/demandes-hse", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          etablissement,
          resumeDemande: resumeDemande.trim(),
          nombreHeures: Math.round(heures * 4) / 4,
          classe: classe.trim(),
          details: details.trim(),
        }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(data?.error || "Enregistrement impossible.");
      }
      setResumeDemande("");
      setNombreHeures("");
      setClasse("");
      setDetails("");
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
      const res = await fetch("/api/demandes-hse", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, status, directionNote: note || undefined }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data?.error || "Décision impossible.");
      setDirectionNotes((p) => {
        const next = { ...p };
        delete next[id];
        return next;
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
    <div className={embeddedInRh ? "space-y-4" : "max-w-7xl mx-auto p-6"}>
      <div className={embeddedInRh ? "mb-4" : "mb-8"}>
        {embeddedInRh ? (
          <>
            <h2 className="text-xl font-black text-slate-900">Demandes HSE</h2>
            <p className="text-sm text-slate-500 mt-1">
              Heures supplémentaires exceptionnelles — vos demandes uniquement (profs) ou votre
              établissement (direction).
            </p>
          </>
        ) : (
          <>
            <h1 className="text-4xl font-black text-slate-900">HSE — heures supplémentaires exceptionnelles</h1>
            <p className="text-slate-500 font-medium mt-2">
              Déposez votre demande à la validation de votre direction. Chaque enseignant ne voit que
              ses demandes ; la direction voit celles de son établissement.
            </p>
          </>
        )}
      </div>

      {creator && !userEmail && (
        <div className="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-900 mb-6">
          Compte sans e-mail principal : vous ne pouvez pas déposer une demande tant que Clerk n’a pas d’adresse joignable pour vous (réception de la décision).
        </div>
      )}

      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
        {creator ? (
          <div className="xl:col-span-1 bg-white border border-slate-200 rounded-3xl p-6 h-fit">
            <h2 className="text-xl font-black text-slate-900 mb-4">Nouvelle demande</h2>
            <div className="space-y-4">
              <div>
                <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">Établissement</label>
                <EstablishmentSelect
                  value={etablissement}
                  onChange={setEtablissement}
                  establishments={establishments}
                  includeGroupe={false}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"
                />
              </div>
              <div>
                <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                  Votre demande
                </label>
                <textarea
                  value={resumeDemande}
                  onChange={(e) => setResumeDemande(e.target.value)}
                  rows={5}
                  placeholder="Décrivez ce que vous demandez et pourquoi (remplacement, activité, circonstances…)"
                  className="w-full rounded-xl border border-slate-200 px-3 py-2"
                />
              </div>
              <div>
                <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                  Nombre d&apos;heures
                </label>
                <input
                  type="number"
                  min={0.25}
                  step={0.25}
                  value={nombreHeures}
                  onChange={(e) => setNombreHeures(e.target.value)}
                  placeholder="Ex. 2 ou 2,5"
                  className="w-full rounded-xl border border-slate-200 px-3 py-2"
                />
                <p className="mt-1 text-[11px] text-slate-500">Pas de 0,25 h (ex. 1 · 1,25 · 1,5 · 2,75).</p>
              </div>
              <div>
                <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                  Classe / groupe / contexte
                </label>
                <textarea
                  value={classe}
                  onChange={(e) => setClasse(e.target.value)}
                  rows={2}
                  placeholder="Texte libre : classe, niveau, matière…"
                  className="w-full rounded-xl border border-slate-200 px-3 py-2"
                />
              </div>
              <div>
                <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                  Précisions <span className="font-bold normal-case text-slate-400">(optionnel)</span>
                </label>
                <textarea
                  value={details}
                  onChange={(e) => setDetails(e.target.value)}
                  rows={3}
                  placeholder="Horaires, modalités, remplacement… si besoin"
                  className="w-full rounded-xl border border-slate-200 px-3 py-2"
                />
              </div>
              {error && <div className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">{error}</div>}
              <button
                type="button"
                onClick={() => void submit()}
                disabled={saving || !userEmail}
                className="w-full bg-indigo-600 hover:bg-indigo-700 text-white font-bold py-3 rounded-xl disabled:opacity-60"
              >
                {saving ? "Envoi..." : "Envoyer la demande"}
              </button>
            </div>
          </div>
        ) : (
          error &&
          !directionAny && (
            <div className="xl:col-span-3 text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-3">{error}</div>
          )
        )}

        <div className={`space-y-6 ${creator ? "xl:col-span-2" : "xl:col-span-3"}`}>
          {creator && (
            <>
              <div className="bg-white border border-slate-200 rounded-3xl p-4">
                <h3 className="font-black text-slate-900">Mes demandes</h3>
                <p className="text-xs text-slate-500 mt-1">
                  Historique personnel. Une demande encore en attente peut être annulée.
                </p>
              </div>
              {loading ? (
                <div className="bg-white border border-slate-200 rounded-3xl p-8 text-slate-500">Chargement…</div>
              ) : mine.length === 0 ? (
                <div className="bg-white border border-dashed border-slate-300 rounded-3xl p-8 text-slate-500">Aucune demande encore.</div>
              ) : (
                mine.map((item) => (
                  <div key={item.id} className="bg-white border border-slate-200 rounded-3xl p-5">
                    <div className="flex flex-wrap gap-3 items-center justify-between mb-3">
                      <div>
                        <p className="font-black text-slate-900">
                          {item.etablissement}
                          {item.nombreHeures != null ? ` · ${formatNombreHeures(item.nombreHeures)}` : ""}
                          {" · "}
                          {item.resumeDemande.slice(0, 60)}
                          {item.resumeDemande.length > 60 ? "…" : ""}
                        </p>
                        <p className="text-xs text-slate-500">
                          Créée le {new Date(item.createdAt).toLocaleString("fr-FR")} — {item.createdBy.email}
                        </p>
                      </div>
                      <span className={`text-xs font-black px-3 py-1.5 rounded-xl border ${statusBadgeClass(item.status)}`}>
                        {statusLabel(item.status)}
                      </span>
                    </div>
                    <p className="text-sm text-slate-700 mb-1 whitespace-pre-wrap">
                      <span className="font-bold">Demande :</span> {item.resumeDemande}
                    </p>
                    {item.nombreHeures != null ? (
                      <p className="text-sm text-slate-700 mb-1">
                        <span className="font-bold">Heures :</span> {formatNombreHeures(item.nombreHeures)}
                      </p>
                    ) : null}
                    <p className="text-sm text-slate-700 mb-1">
                      <span className="font-bold">Classe / contexte :</span> {item.classe}
                    </p>
                    {item.details ? (
                      <p className="text-sm text-slate-600 whitespace-pre-wrap">
                        <span className="font-bold">Précisions :</span> {item.details}
                      </p>
                    ) : null}
                    {item.status === "EN_ATTENTE" ? (
                      <div className="mt-4 flex flex-wrap gap-2">
                        <button
                          type="button"
                          disabled={cancellingId === item.id}
                          onClick={() => void cancelDemand(item.id)}
                          className="px-4 py-2 rounded-xl border border-slate-300 bg-white text-slate-700 font-bold text-sm hover:bg-slate-50 disabled:opacity-60"
                        >
                          {cancellingId === item.id ? "Annulation…" : "Annuler la demande"}
                        </button>
                      </div>
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
                    {item.status === "ACCEPTEE" ? (
                      <div className="mt-3">
                        <AttestationPdfLink id={item.id} />
                      </div>
                    ) : null}
                  </div>
                ))
              )}
            </>
          )}

          {directionAny && (
            <>
              <div className="bg-white border border-slate-200 rounded-3xl p-4">
                <h3 className="font-black text-slate-900">File de votre pôle</h3>
                <p className="text-xs text-slate-500 mt-1">
                  Demandes HSE pour l’établissement dont vous assurez la direction — en attente ou déjà traitées sur votre périmètre.
                </p>
                {error && !creator && (
                  <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2 mt-3">{error}</p>
                )}
              </div>

              {loading ? (
                <div className="bg-white border border-slate-200 rounded-3xl p-8 text-slate-500">Chargement…</div>
              ) : (
                <>
                  <h4 className="text-sm font-black text-slate-700 uppercase tracking-wide px-1">À traiter</h4>
                  {directionPending.length === 0 ? (
                    <div className="bg-white border border-dashed border-slate-300 rounded-3xl p-6 text-slate-500 text-sm">
                      Aucune demande en attente pour votre périmètre.
                    </div>
                  ) : (
                    directionPending.map((item) => (
                      <div key={item.id} className="bg-white border border-slate-200 rounded-3xl p-5">
                        <div className="flex flex-wrap gap-3 items-center justify-between mb-3">
                          <div>
                            <p className="font-black text-slate-900">
                              {item.createdBy.name} — {item.etablissement}
                            </p>
                            <p className="text-xs text-slate-500">
                              {item.createdBy.email} · le {new Date(item.createdAt).toLocaleString("fr-FR")}
                            </p>
                          </div>
                          <span className={`text-xs font-black px-3 py-1.5 rounded-xl border ${statusBadgeClass(item.status)}`}>
                            {statusLabel(item.status)}
                          </span>
                        </div>
                        <p className="text-sm text-slate-800 font-semibold mb-1 whitespace-pre-wrap">{item.resumeDemande}</p>
                        {item.nombreHeures != null ? (
                          <p className="text-sm text-slate-700 mb-1">
                            <span className="font-bold">Heures :</span> {formatNombreHeures(item.nombreHeures)}
                          </p>
                        ) : null}
                        <p className="text-sm text-slate-700 mb-3">
                          <span className="font-bold">Classe / contexte :</span> {item.classe}
                        </p>
                        {item.details ? (
                          <p className="text-sm text-slate-600 whitespace-pre-wrap mb-3">{item.details}</p>
                        ) : null}
                        <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                          Note pour le demandeur (optionnel)
                        </label>
                        <textarea
                          rows={2}
                          value={directionNotes[item.id] ?? ""}
                          onChange={(e) => setDirectionNotes((p) => ({ ...p, [item.id]: e.target.value }))}
                          className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm mb-3"
                          placeholder="Ex. : précision RH, motif du refus…"
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

                  <h4 className="text-sm font-black text-slate-700 uppercase tracking-wide px-1 pt-4">Traitées (pôle)</h4>
                  {directionHistory.length === 0 ? (
                    <div className="bg-white border border-dashed border-slate-300 rounded-3xl p-6 text-slate-500 text-sm">
                      Pas encore d’historique de décision sur votre périmètre.
                    </div>
                  ) : (
                    directionHistory.map((item) => (
                      <div key={item.id} className="bg-white border border-slate-200 rounded-3xl p-5 opacity-95">
                        <div className="flex flex-wrap gap-3 items-center justify-between mb-2">
                          <p className="font-black text-slate-900">
                            {item.createdBy.name} — {item.etablissement}
                          </p>
                          <span className={`text-xs font-black px-3 py-1.5 rounded-xl border ${statusBadgeClass(item.status)}`}>
                            {statusLabel(item.status)}
                          </span>
                        </div>
                        <p className="text-xs text-slate-500 mb-2">{new Date(item.createdAt).toLocaleDateString("fr-FR")}</p>
                        <p className="text-sm font-semibold text-slate-800 mb-2 whitespace-pre-wrap">{item.resumeDemande}</p>
                        {item.nombreHeures != null ? (
                          <p className="text-sm text-slate-700 mb-1">{formatNombreHeures(item.nombreHeures)}</p>
                        ) : null}
                        {item.directionNote && (
                          <p className="text-sm text-indigo-800 mt-2">
                            <span className="font-bold">Note :</span> {item.directionNote}
                          </p>
                        )}
                        {item.status === "ACCEPTEE" ? (
                          <div className="mt-3">
                            <AttestationPdfLink id={item.id} />
                          </div>
                        ) : null}
                      </div>
                    ))
                  )}
                </>
              )}
            </>
          )}

          {!creator && !directionAny && !loading && (
            <div className="bg-white border border-slate-200 rounded-3xl p-8 text-slate-600">
              Votre profil ne permet pas d’accéder aux demandes HSE. Réservé aux enseignants (leurs
              propres demandes) et à la direction de leur établissement.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
