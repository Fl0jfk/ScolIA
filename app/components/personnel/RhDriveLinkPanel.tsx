"use client";

import { useCallback, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";

type DriveStatus = {
  enabled: boolean;
  linked: boolean;
  healthy: boolean;
  linkedUpn: string | null;
  linkedDisplayName: string | null;
  linkedAt: string | null;
  basePath: string;
  error: string | null;
  oauthRedirectUri?: string | null;
};

export default function RhDriveLinkPanel() {
  const searchParams = useSearchParams();
  const [status, setStatus] = useState<DriveStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [banner, setBanner] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/rh/drive/status", { cache: "no-store" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Statut indisponible");
      setStatus(json);
    } catch (e) {
      setStatus(null);
      setBanner(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const flag = searchParams.get("rhDrive");
    if (!flag) return;
    if (flag === "linked") setBanner("OneDrive RH connecté.");
    else if (flag === "forbidden") setBanner("Accès refusé pour lier le OneDrive RH.");
    else if (flag === "azure_redirect") {
      const detail = searchParams.get("detail");
      setBanner(
        detail ||
          "URI de redirection Azure manquante — ajoutez /api/rh/drive/oauth/callback dans Microsoft.",
      );
    } else if (flag === "error") {
      const detail = searchParams.get("detail");
      setBanner(detail ? `Échec liaison OneDrive RH : ${detail}` : "Échec liaison OneDrive RH.");
    }
  }, [searchParams]);

  const disconnect = async () => {
    if (!confirm("Déconnecter le OneDrive RH ? Les dépôts serveur échoueront jusqu'à reconnexion.")) {
      return;
    }
    setBusy(true);
    try {
      const res = await fetch("/api/rh/drive/disconnect", { method: "POST" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Déconnexion impossible");
      setBanner("OneDrive RH déconnecté.");
      await load();
    } catch (e) {
      setBanner(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="bg-white rounded-2xl border border-slate-200 p-5 space-y-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="font-black text-slate-900">OneDrive RH</h3>
          <p className="text-sm text-slate-600 mt-1 max-w-2xl">
            Liez une fois le compte Microsoft de l&apos;attachée de gestion. Tous les dossiers
            personnels et <code className="text-xs bg-slate-100 px-1 rounded">meta-rh.json</code>{" "}
            seront écrits sur <strong>son</strong> OneDrive (pas celui du déposant).
          </p>
        </div>
        {status?.healthy ? (
          <span className="text-xs font-bold uppercase tracking-wide text-emerald-700 bg-emerald-50 border border-emerald-100 px-2.5 py-1 rounded-lg">
            Connecté
          </span>
        ) : status?.linked ? (
          <span className="text-xs font-bold uppercase tracking-wide text-amber-800 bg-amber-50 border border-amber-100 px-2.5 py-1 rounded-lg">
            À reconnecter
          </span>
        ) : (
          <span className="text-xs font-bold uppercase tracking-wide text-slate-600 bg-slate-50 border border-slate-200 px-2.5 py-1 rounded-lg">
            Non lié
          </span>
        )}
      </div>

      {banner && (
        <p
          className={`text-sm rounded-xl border px-3 py-2 whitespace-pre-wrap ${
            searchParams.get("rhDrive") === "azure_redirect"
              ? "border-amber-200 bg-amber-50 text-amber-950"
              : "border-slate-200 bg-slate-50 text-slate-800"
          }`}
        >
          {banner}
        </p>
      )}

      {loading ? (
        <p className="text-sm text-slate-500">Vérification du lien…</p>
      ) : status ? (
        <dl className="grid sm:grid-cols-2 gap-2 text-sm">
          <div>
            <dt className="text-slate-500 text-xs font-semibold uppercase">Compte</dt>
            <dd className="font-medium text-slate-900">
              {status.linkedDisplayName || status.linkedUpn || "—"}
            </dd>
          </div>
          <div>
            <dt className="text-slate-500 text-xs font-semibold uppercase">UPN</dt>
            <dd className="font-medium text-slate-900 break-all">{status.linkedUpn || "—"}</dd>
          </div>
          <div>
            <dt className="text-slate-500 text-xs font-semibold uppercase">Dossier racine</dt>
            <dd className="font-medium text-slate-900">{status.basePath}</dd>
          </div>
          <div>
            <dt className="text-slate-500 text-xs font-semibold uppercase">Lié le</dt>
            <dd className="font-medium text-slate-900">
              {status.linkedAt ? new Date(status.linkedAt).toLocaleString("fr-FR") : "—"}
            </dd>
          </div>
          {status.error && (
            <div className="sm:col-span-2">
              <dt className="text-rose-600 text-xs font-semibold uppercase">Erreur</dt>
              <dd className="text-rose-700 text-sm">{status.error}</dd>
            </div>
          )}
        </dl>
      ) : null}

      <div className="flex flex-wrap gap-2 pt-1">
        <a
          href="/api/rh/drive/oauth/start"
          className="inline-flex items-center justify-center rounded-xl bg-slate-900 text-white text-sm font-bold px-4 py-2.5 hover:bg-slate-800"
        >
          {status?.linked ? "Reconnecter le OneDrive RH" : "Connecter le OneDrive RH"}
        </a>
        {status?.linked && (
          <button
            type="button"
            disabled={busy}
            onClick={() => void disconnect()}
            className="inline-flex items-center justify-center rounded-xl border border-slate-200 text-slate-700 text-sm font-bold px-4 py-2.5 hover:bg-slate-50 disabled:opacity-50"
          >
            Déconnecter
          </button>
        )}
        <button
          type="button"
          disabled={loading || busy}
          onClick={() => void load()}
          className="inline-flex items-center justify-center rounded-xl border border-slate-200 text-slate-700 text-sm font-bold px-4 py-2.5 hover:bg-slate-50 disabled:opacity-50"
        >
          Rafraîchir
        </button>
      </div>

      <p className="text-xs text-slate-500 leading-relaxed">
        <strong>Rappel Azure (obligatoire avant la 1ère connexion) :</strong> enregistrez l&apos;URI
        Web de redirection ci-dessous dans Microsoft Entra → App Registration → Authentification →
        URI de redirection (plateforme <strong>Web</strong>).
        {status?.oauthRedirectUri ? (
          <>
            {" "}
            URI à copier :{" "}
            <code className="bg-slate-100 px-1 rounded break-all">{status.oauthRedirectUri}</code>
          </>
        ) : (
          <>
            {" "}
            Chemin :{" "}
            <code className="bg-slate-100 px-1 rounded">/api/rh/drive/oauth/callback</code>
          </>
        )}
        . Le <code className="bg-slate-100 px-1 rounded">clientSecret</code> tenant doit être
        renseigné. Pas besoin de permission Application{" "}
        <code className="bg-slate-100 px-1 rounded">Files.ReadWrite.All</code>.
      </p>
    </section>
  );
}
