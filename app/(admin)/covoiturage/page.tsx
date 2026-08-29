"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { useCallback, useEffect, useMemo, useState } from "react";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";
import type { CovoiturageDirection, CovoiturageStatus } from "@/app/lib/covoiturage-types";
import { directionLabel } from "@/app/lib/covoiturage-types";

type Establishment = {
  id: string;
  label: string;
  grades?: string;
};

type MatchView = {
  id: string;
  status: "pending" | "revealed" | "declined" | "cancelled";
  matchedZone: string;
  matchedEstablishments: string[];
  createdAt: string;
  revealedAt?: string;
  myAccepted: boolean;
  otherAccepted: boolean;
  other: { displayName: string; email: string | null } | null;
};

type ProfileView = {
  externalUserId: string;
  displayName: string;
  email: string;
  status: CovoiturageStatus;
  establishments: string[];
  zones: string[];
  direction: CovoiturageDirection;
  note?: string;
  schoolYear: string;
  registeredAt: string;
  updatedAt: string;
};

type BoardData = {
  profile: ProfileView | null;
  matches: MatchView[];
  establishments: Establishment[];
  schoolYear: string;
  disclaimer: string;
};

function statusBadge(status: CovoiturageStatus) {
  if (status === "active") return "bg-emerald-50 text-emerald-800 border-emerald-200";
  if (status === "complete") return "bg-sky-50 text-sky-800 border-sky-200";
  return "bg-slate-100 text-slate-600 border-slate-200";
}

function statusLabel(status: CovoiturageStatus) {
  if (status === "active") return "Actif — recherche en cours";
  if (status === "complete") return "Complet — plus de nouveaux matchs";
  return "Désinscrit";
}

export default function CovoituragePage() {
  const { isLoaded } = useSessionUser();
  const [data, setData] = useState<BoardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [zoneInput, setZoneInput] = useState("");
  const [zones, setZones] = useState<string[]>([]);
  const [establishments, setEstablishments] = useState<string[]>([]);
  const [direction, setDirection] = useState<CovoiturageDirection>("both");
  const [note, setNote] = useState("");

  const estById = useMemo(() => {
    const map = new Map<string, Establishment>();
    for (const e of data?.establishments ?? []) map.set(e.id, e);
    return map;
  }, [data?.establishments]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/covoiturage", { cache: "no-store" });
      const json = (await res.json()) as BoardData & { error?: string };
      if (!res.ok) throw new Error(json.error || "Chargement impossible");
      setData(json);
      if (json.profile) {
        setZones(json.profile.zones);
        setEstablishments(json.profile.establishments);
        setDirection(json.profile.direction);
        setNote(json.profile.note ?? "");
      } else if (json.establishments.length === 1) {
        setEstablishments([json.establishments[0]!.id]);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!isLoaded) return;
    void load();
  }, [isLoaded, load]);

  const addZone = () => {
    const raw = zoneInput.trim().replace(/\s+/g, "");
    if (!/^\d{5}$/.test(raw)) {
      setError("Code postal invalide (5 chiffres).");
      return;
    }
    if (!zones.includes(raw)) setZones((z) => [...z, raw]);
    setZoneInput("");
    setError(null);
  };

  const toggleEstablishment = (id: string) => {
    setEstablishments((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  };

  const postAction = async (body: Record<string, unknown>) => {
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/covoiturage", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const json = (await res.json()) as BoardData & { error?: string; ok?: boolean };
      if (!res.ok) throw new Error(json.error || "Action impossible");
      setData((prev) =>
        prev
          ? {
              ...prev,
              profile: json.profile ?? null,
              matches: json.matches ?? [],
            }
          : prev,
      );
      if (json.profile) {
        setZones(json.profile.zones);
        setEstablishments(json.profile.establishments);
        setDirection(json.profile.direction);
        setNote(json.profile.note ?? "");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const registerOrUpdate = () => {
    void postAction({
      action: data?.profile ? "update" : "register",
      zones,
      establishments,
      direction,
      note,
    });
  };

  const matchAction = async (matchId: string, action: "accept" | "decline") => {
    setSaving(true);
    setError(null);
    try {
      const res = await fetch(`/api/covoiturage/matches/${encodeURIComponent(matchId)}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action }),
      });
      const json = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(json.error || "Action impossible");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (!isLoaded || loading) {
    return (
      <ModulePageShell maxWidthClass="max-w-3xl">
        <p className="text-sm text-slate-600">Chargement du covoiturage…</p>
      </ModulePageShell>
    );
  }

  const toolDisabled =
    Boolean(error) &&
    /n.est pas activ|pas activé|désactiv/i.test(error || "");

  if (toolDisabled) {
    return (
      <ModulePageShell maxWidthClass="max-w-3xl">
        <ModulePageHeader
          eyebrow="Services"
          title="Covoiturage"
          description="Outil désactivé pour cet établissement."
        />
        <ModuleCard bodyClassName="p-6">
          <p className={`text-sm ${dash.textMid}`}>
            Le covoiturage n’est pas encore activé. Un administrateur peut l’activer dans{" "}
            <a href="/parametres?tab=modules" className={`font-semibold underline ${dash.textPrimary}`}>
              Paramètres → Droits modules
            </a>
            .
          </p>
        </ModuleCard>
      </ModulePageShell>
    );
  }

  const profile = data?.profile ?? null;
  const showMultiEst = (data?.establishments.length ?? 0) > 1;
  const pendingMatches = (data?.matches ?? []).filter((m) => m.status === "pending");
  const revealedMatches = (data?.matches ?? []).filter((m) => m.status === "revealed");

  return (
    <ModulePageShell maxWidthClass="max-w-3xl" className="pb-24">
      <ModulePageHeader
        eyebrow="Services"
        title="Covoiturage"
        description="Mise en relation entre familles par zone (code postal) et établissement. Aucune adresse exacte n'est stockée. Les coordonnées ne sont échangées qu'avec votre accord."
      />

      {error ? (
        <p className="mb-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      ) : null}

      {profile ? (
        <ModuleCard className="mb-6" bodyClassName="p-5">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p className={`text-xs font-semibold uppercase tracking-wide ${dash.textMid}`}>Mon profil</p>
              <p className={`mt-1 text-lg font-semibold ${dash.ink}`}>{profile.displayName}</p>
              <p className={`text-sm ${dash.textMid}`}>{profile.email}</p>
            </div>
            <span className={`text-[10px] font-bold uppercase px-2.5 py-1 rounded-full border ${statusBadge(profile.status)}`}>
              {statusLabel(profile.status)}
            </span>
          </div>
          <dl className="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
            <div>
              <dt className="text-[10px] font-bold text-slate-400 uppercase">Codes postaux</dt>
              <dd className="font-medium text-slate-800">{profile.zones.join(", ")}</dd>
            </div>
            <div>
              <dt className="text-[10px] font-bold text-slate-400 uppercase">Établissement(s)</dt>
              <dd className="font-medium text-slate-800">
                {profile.establishments.map((id) => estById.get(id)?.label ?? id).join(", ")}
              </dd>
            </div>
            <div>
              <dt className="text-[10px] font-bold text-slate-400 uppercase">Trajet</dt>
              <dd className="font-medium text-slate-800">{directionLabel(profile.direction)}</dd>
            </div>
            {profile.note ? (
              <div className="sm:col-span-2">
                <dt className="text-[10px] font-bold text-slate-400 uppercase">Note</dt>
                <dd className="text-slate-700">{profile.note}</dd>
              </div>
            ) : null}
          </dl>
          <div className="mt-4 flex flex-wrap gap-2">
            {profile.status === "active" ? (
              <button
                type="button"
                disabled={saving}
                onClick={() => void postAction({ action: "complete" })}
                className={`rounded-xl px-3 py-2 text-xs font-semibold text-white disabled:opacity-50 ${dash.bgPrimary}`}
              >
                Mon covoiturage est complet
              </button>
            ) : profile.status === "complete" ? (
              <button
                type="button"
                disabled={saving}
                onClick={() => void postAction({ action: "reactivate" })}
                className={`rounded-xl px-3 py-2 text-xs font-semibold text-white disabled:opacity-50 ${dash.bgPrimary}`}
              >
                Réactiver ma recherche
              </button>
            ) : null}
            <button
              type="button"
              disabled={saving}
              onClick={() => {
                if (!confirm("Se désinscrire du covoiturage ? Vos matchs en attente seront annulés.")) return;
                void postAction({ action: "unregister" });
              }}
              className="text-xs font-bold px-3 py-2 rounded-xl border border-slate-300 text-slate-700 disabled:opacity-50"
            >
              Me désinscrire
            </button>
          </div>
        </ModuleCard>
      ) : null}

      {(profile?.status === "active" || !profile) && (
        <ModuleCard className="mb-8" bodyClassName="p-5">
          <h2 className={`text-lg font-semibold ${dash.ink}`}>
            {profile ? "Modifier mon inscription" : "S'inscrire au covoiturage"}
          </h2>
          <p className="text-xs text-slate-600 mt-1">
            Inscription distincte de votre compte : vous choisissez les zones et établissements concernés.
          </p>

          <div className="mt-4 space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-700 mb-1">Codes postaux recherchés</label>
              <div className="flex gap-2">
                <input
                  value={zoneInput}
                  onChange={(e) => setZoneInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addZone();
                    }
                  }}
                  placeholder="Ex. 76240"
                  className="flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  maxLength={5}
                />
                <button
                  type="button"
                  onClick={addZone}
                  className="shrink-0 px-4 py-2 rounded-xl bg-slate-800 text-white text-sm font-bold"
                >
                  Ajouter
                </button>
              </div>
              {zones.length > 0 ? (
                <div className="flex flex-wrap gap-2 mt-2">
                  {zones.map((z) => (
                    <span
                      key={z}
                      className="inline-flex items-center gap-1 text-xs font-bold bg-white border border-slate-200 rounded-full px-2.5 py-1"
                    >
                      {z}
                      <button
                        type="button"
                        className="text-slate-400 hover:text-rose-600"
                        onClick={() => setZones((arr) => arr.filter((x) => x !== z))}
                      >
                        ×
                      </button>
                    </span>
                  ))}
                </div>
              ) : (
                <p className="text-[11px] text-slate-500 mt-1">Ajoutez un ou plusieurs codes postaux.</p>
              )}
            </div>

            {showMultiEst ? (
              <div>
                <label className="block text-xs font-bold text-slate-700 mb-2">Établissement(s)</label>
                <div className="flex flex-wrap gap-2">
                  {(data?.establishments ?? []).map((est) => {
                    const on = establishments.includes(est.id);
                    return (
                      <button
                        key={est.id}
                        type="button"
                        onClick={() => toggleEstablishment(est.id)}
                        className={`text-xs font-bold px-3 py-2 rounded-xl border transition ${
                          on
                            ? "bg-teal-600 text-white border-teal-600"
                            : "bg-white text-slate-700 border-slate-200 hover:border-teal-300"
                        }`}
                      >
                        {est.label}
                      </button>
                    );
                  })}
                </div>
              </div>
            ) : (
              <p className="text-xs text-slate-600">
                Établissement : <span className="font-bold">{data?.establishments[0]?.label}</span>
              </p>
            )}

            <div>
              <label className="block text-xs font-bold text-slate-700 mb-1">Trajet</label>
              <select
                value={direction}
                onChange={(e) => setDirection(e.target.value as CovoiturageDirection)}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm bg-white"
              >
                <option value="morning">Matin</option>
                <option value="evening">Soir</option>
                <option value="both">Matin et soir</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-bold text-slate-700 mb-1">Note courte (optionnel)</label>
              <textarea
                value={note}
                onChange={(e) => setNote(e.target.value)}
                rows={2}
                maxLength={280}
                placeholder="Ex. 1 place disponible le matin"
                className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
              />
            </div>

            <button
              type="button"
              disabled={saving || zones.length === 0}
              onClick={registerOrUpdate}
              className={`w-full rounded-2xl py-3 text-sm font-semibold text-white disabled:opacity-50 ${dash.bgPrimary}`}
            >
              {profile ? "Enregistrer les modifications" : "M'inscrire au covoiturage"}
            </button>
          </div>
        </ModuleCard>
      )}

      {profile && profile.status === "active" ? (
        <section className="space-y-4">
          <h2 className={`text-lg font-semibold ${dash.ink}`}>Mes mises en relation</h2>

          {pendingMatches.length === 0 && revealedMatches.length === 0 ? (
            <p className="text-sm text-slate-500 rounded-xl border border-dashed border-slate-200 p-6 text-center">
              Aucun match pour l&apos;instant. Dès qu&apos;une famille partage une zone et un établissement, vous serez notifié(e) par e-mail.
            </p>
          ) : null}

          {pendingMatches.map((m) => (
            <article key={m.id} className="rounded-2xl border border-amber-200 bg-amber-50/50 p-4">
              <p className="text-xs font-black uppercase text-amber-900">Match potentiel</p>
              <p className="text-sm text-slate-800 mt-2">
                Zone <span className="font-bold">{m.matchedZone}</span>
                {" · "}
                {m.matchedEstablishments.map((id) => estById.get(id)?.label ?? id).join(", ")}
              </p>
              <p className="text-xs text-slate-600 mt-1">
                {m.otherAccepted
                  ? "L'autre famille a déjà accepté. Acceptez pour dévoiler les coordonnées."
                  : "Aucune identité n'est visible tant que vous n'acceptez pas."}
              </p>
              {!m.myAccepted ? (
                <div className="mt-3 flex flex-wrap gap-2">
                  <button
                    type="button"
                    disabled={saving}
                    onClick={() => void matchAction(m.id, "accept")}
                    className={`rounded-xl px-3 py-2 text-xs font-semibold text-white disabled:opacity-50 ${dash.bgPrimary}`}
                  >
                    Oui, échanger mes coordonnées
                  </button>
                  <button
                    type="button"
                    disabled={saving}
                    onClick={() => void matchAction(m.id, "decline")}
                    className="text-xs font-bold px-3 py-2 rounded-xl border border-slate-300 text-slate-700 disabled:opacity-50"
                  >
                    Non merci
                  </button>
                </div>
              ) : (
                <p className="mt-3 text-xs font-bold text-emerald-800">
                  Vous avez accepté — en attente de l&apos;autre famille.
                </p>
              )}
            </article>
          ))}

          {revealedMatches.map((m) => (
            <article key={m.id} className="rounded-2xl border border-emerald-200 bg-emerald-50/40 p-4">
              <p className="text-xs font-black uppercase text-emerald-900">Mise en relation confirmée</p>
              <p className="text-sm text-slate-800 mt-2">
                Zone {m.matchedZone} ·{" "}
                {m.matchedEstablishments.map((id) => estById.get(id)?.label ?? id).join(", ")}
              </p>
              {m.other?.email ? (
                <div className="mt-3 rounded-xl bg-white border border-emerald-100 p-3 text-sm">
                  <p className="font-bold text-slate-900">{m.other.displayName}</p>
                  <a href={`mailto:${m.other.email}`} className="text-teal-700 font-medium break-all">
                    {m.other.email}
                  </a>
                </div>
              ) : null}
            </article>
          ))}
        </section>
      ) : null}

      {data?.disclaimer ? (
        <p className="mt-10 text-[11px] text-slate-500 leading-relaxed border-t border-slate-100 pt-6">
          {data.disclaimer}
        </p>
      ) : null}
    </ModulePageShell>
  );
}
