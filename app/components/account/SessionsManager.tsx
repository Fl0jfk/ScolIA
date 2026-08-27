"use client";

import { useCallback, useEffect, useState } from "react";
import type { SessionDevicePublic } from "@/app/lib/session-device";

function formatWhen(iso: string): string {
  try {
    return new Date(iso).toLocaleString("fr-FR", {
      dateStyle: "short",
      timeStyle: "short",
    });
  } catch {
    return iso;
  }
}

type Props = {
  /** Si défini : sessions d’un autre utilisateur (admin). Sinon : compte courant. */
  externalUserId?: string;
  title?: string;
  onClose?: () => void;
  embedded?: boolean;
};

export default function SessionsManager({
  externalUserId,
  title,
  onClose,
  embedded = false,
}: Props) {
  const [sessions, setSessions] = useState<SessionDevicePublic[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const isAdminView = Boolean(externalUserId);

  const endpoint = isAdminView
    ? `/api/members/${encodeURIComponent(externalUserId!)}/sessions`
    : "/api/account/sessions";

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(endpoint, { cache: "no-store", credentials: "include" });
      const j = (await res.json()) as { sessions?: SessionDevicePublic[]; error?: string };
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setSessions(Array.isArray(j.sessions) ? j.sessions : []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      setSessions([]);
    } finally {
      setLoading(false);
    }
  }, [endpoint]);

  useEffect(() => {
    void load();
  }, [load]);

  async function revokeOne(sessionId: string, isCurrent: boolean) {
    if (isCurrent && !isAdminView) {
      const ok = window.confirm(
        "Révoquer cette session vous déconnectera immédiatement. Continuer ?",
      );
      if (!ok) return;
    } else {
      const ok = window.confirm("Révoquer cet appareil ? La personne devra se reconnecter (avec 2FA si activée).");
      if (!ok) return;
    }
    setBusyId(sessionId);
    setError(null);
    setInfo(null);
    try {
      const res = await fetch(endpoint, {
        method: "DELETE",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ sessionId }),
      });
      const j = (await res.json()) as { error?: string; revokedCurrent?: boolean };
      if (!res.ok) throw new Error(j.error || "Échec");
      if (j.revokedCurrent) {
        window.location.href = "/auth/sign-in";
        return;
      }
      setInfo("Session révoquée.");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusyId(null);
    }
  }

  async function revokeBulk() {
    const label = isAdminView
      ? "Révoquer toutes les sessions de cet utilisateur ? Il devra se reconnecter partout (avec 2FA si activée)."
      : "Révoquer tous les autres appareils ? Cet appareil restera connecté.";
    if (!window.confirm(label)) return;
    setBusyId("__bulk__");
    setError(null);
    setInfo(null);
    try {
      const res = await fetch(endpoint, {
        method: "DELETE",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(isAdminView ? { all: true } : { others: true }),
      });
      const j = (await res.json()) as { error?: string; revoked?: number };
      if (!res.ok) throw new Error(j.error || "Échec");
      setInfo(
        typeof j.revoked === "number"
          ? `${j.revoked} session(s) révoquée(s).`
          : "Sessions révoquées.",
      );
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusyId(null);
    }
  }

  const body = (
    <div className="space-y-4">
      {title ? <h3 className="text-sm font-bold text-slate-900">{title}</h3> : null}
      <p className="text-sm text-slate-600">
        {isAdminView
          ? "Appareils actuellement connectés pour ce compte. Une révocation force une nouvelle connexion (et la 2FA si elle est activée)."
          : "Voici les appareils où votre compte est connecté. Vous pouvez déconnecter un appareil à distance."}
      </p>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800" role="alert">
          {error}
        </p>
      ) : null}
      {info ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800" role="status">
          {info}
        </p>
      ) : null}

      {loading ? (
        <p className="text-sm text-slate-500">Chargement des sessions…</p>
      ) : sessions.length === 0 ? (
        <p className="text-sm italic text-slate-500">Aucune session active.</p>
      ) : (
        <ul className="max-h-72 space-y-2 overflow-y-auto">
          {sessions.map((s) => (
            <li
              key={s.id}
              className="flex flex-col gap-2 rounded-xl border border-slate-200 bg-slate-50/80 px-3 py-3 sm:flex-row sm:items-center sm:justify-between"
            >
              <div className="min-w-0">
                <p className="text-sm font-semibold text-slate-900">
                  {s.deviceLabel}
                  {s.current ? (
                    <span className="ml-2 rounded-full bg-emerald-100 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-emerald-800">
                      Cet appareil
                    </span>
                  ) : null}
                </p>
                <p className="mt-0.5 text-xs text-slate-500">
                  {s.ipAddress ? `IP ${s.ipAddress} · ` : ""}
                  Actif : {formatWhen(s.updatedAt)}
                </p>
                <p className="text-[11px] text-slate-400">Créée : {formatWhen(s.createdAt)}</p>
              </div>
              <button
                type="button"
                disabled={busyId !== null}
                onClick={() => void revokeOne(s.id, s.current)}
                className="shrink-0 rounded-lg border border-rose-200 bg-white px-3 py-1.5 text-xs font-bold text-rose-700 hover:bg-rose-50 disabled:opacity-50"
              >
                {busyId === s.id ? "…" : "Révoquer"}
              </button>
            </li>
          ))}
        </ul>
      )}

      <div className="flex flex-wrap gap-2 pt-1">
        <button
          type="button"
          disabled={busyId !== null || sessions.length === 0 || (!isAdminView && sessions.every((s) => s.current))}
          onClick={() => void revokeBulk()}
          className="rounded-xl border border-rose-300 bg-rose-50 px-3 py-2 text-xs font-bold text-rose-800 hover:bg-rose-100 disabled:opacity-50"
        >
          {busyId === "__bulk__"
            ? "Révocation…"
            : isAdminView
              ? "Révoquer toutes les sessions"
              : "Déconnecter les autres appareils"}
        </button>
        {onClose ? (
          <button
            type="button"
            onClick={onClose}
            className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-semibold text-slate-700 hover:bg-slate-50"
          >
            Fermer
          </button>
        ) : null}
      </div>
    </div>
  );

  if (embedded) return body;

  return (
    <div
      className="fixed inset-0 z-[90] flex items-center justify-center bg-slate-900/40 p-4 backdrop-blur-[2px]"
      onClick={onClose}
      role="presentation"
    >
      <div
        className="w-full max-w-lg overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-2xl"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="flex items-center justify-between border-b border-slate-100 px-5 py-4">
          <h2 className="text-base font-bold text-slate-900">
            {title || (isAdminView ? "Sessions utilisateur" : "Sécurité — sessions")}
          </h2>
          {onClose ? (
            <button
              type="button"
              onClick={onClose}
              className="rounded-lg px-2 py-1 text-sm font-semibold text-slate-500 hover:bg-slate-50"
            >
              Fermer
            </button>
          ) : null}
        </div>
        <div className="px-5 py-4">{body}</div>
      </div>
    </div>
  );
}
