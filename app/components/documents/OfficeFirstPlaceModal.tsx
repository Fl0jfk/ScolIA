"use client";

import { useEffect, useMemo, useState } from "react";
import type { ShareInfo } from "@/app/lib/documents-page-model";

type Props = {
  kind: "writer" | "calc" | "impress";
  fromScope: string;
  fromShareId?: string | null;
  fromRelPath: string;
  currentName: string;
  onDone: (result: { editUrl?: string; keptDraft?: boolean }) => void;
  onCancelKeepDraft: () => void;
};

export default function OfficeFirstPlaceModal({
  kind,
  fromScope,
  fromShareId,
  fromRelPath,
  currentName,
  onDone,
  onCancelKeepDraft,
}: Props) {
  const [name, setName] = useState(currentName.replace(/\.[^.]+$/, "") || "Sans titre");
  const [shares, setShares] = useState<ShareInfo[]>([]);
  const [target, setTarget] = useState<"personal" | string>("personal");
  const [folders, setFolders] = useState<{ name: string; relPath: string }[]>([]);
  const [parentRel, setParentRel] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      const res = await fetch("/api/documents/shares");
      const data = await res.json();
      if (res.ok) setShares(data.shares || []);
    })();
  }, []);

  const scope = target === "personal" ? "personal" : "shared";
  const shareId = target === "personal" ? null : target;

  useEffect(() => {
    void (async () => {
      const params = new URLSearchParams({ scope, path: parentRel });
      if (shareId) params.set("shareId", shareId);
      const res = await fetch(`/api/documents/browse?${params}`);
      const data = await res.json();
      if (!res.ok) return;
      setFolders(
        (data.items || [])
          .filter((i: { type: string }) => i.type === "folder")
          .map((i: { name: string; relPath: string }) => ({
            name: i.name,
            relPath: i.relPath.endsWith("/") ? i.relPath : `${i.relPath}/`,
          })),
      );
    })();
  }, [scope, shareId, parentRel]);

  const breadcrumb = useMemo(() => {
    if (!parentRel) return "Racine";
    return parentRel.replace(/\/$/, "");
  }, [parentRel]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/documents/office/place", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind,
          fromScope,
          fromShareId,
          fromRelPath,
          newName: name,
          toScope: scope,
          toShareId: shareId,
          toParentRelPath: parentRel,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Enregistrement impossible.");
        return;
      }
      onDone({ editUrl: data.editUrl });
    } catch {
      setError("Erreur réseau.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[80] flex items-end justify-center bg-black/40 p-4 sm:items-center">
      <div className="w-full max-w-lg rounded-2xl bg-white p-5 shadow-xl">
        <h2 className="text-lg font-semibold text-slate-900">Enregistrer le document</h2>
        <p className="mt-2 text-sm text-slate-600">
          Pour l’instant il s’appelle <strong>{currentName}</strong> et il est à la racine de votre
          cloud. Voulez-vous le renommer et le ranger dans un dossier ?
        </p>

        <label className="mt-4 block text-xs font-semibold uppercase text-slate-500">Nom</label>
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
        />

        <label className="mt-4 block text-xs font-semibold uppercase text-slate-500">Emplacement</label>
        <select
          value={target}
          onChange={(e) => {
            setTarget(e.target.value);
            setParentRel("");
          }}
          className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
        >
          <option value="personal">Mon cloud personnel</option>
          {shares.map((s) => (
            <option key={s.id} value={s.id}>
              Partagé — {s.name}
            </option>
          ))}
        </select>

        <div className="mt-3 rounded-lg border border-slate-100 bg-slate-50 p-3">
          <div className="flex items-center justify-between gap-2">
            <p className="text-xs text-slate-500">Dossier : {breadcrumb || "Racine"}</p>
            {parentRel ? (
              <button
                type="button"
                className="text-xs font-semibold text-slate-700 underline"
                onClick={() => {
                  const parts = parentRel.split("/").filter(Boolean);
                  parts.pop();
                  setParentRel(parts.length ? `${parts.join("/")}/` : "");
                }}
              >
                Remonter
              </button>
            ) : null}
          </div>
          <ul className="mt-2 max-h-40 space-y-1 overflow-auto">
            {folders.length === 0 ? (
              <li className="text-xs text-slate-400">Aucun sous-dossier</li>
            ) : (
              folders.map((f) => (
                <li key={f.relPath}>
                  <button
                    type="button"
                    className="text-left text-sm text-slate-800 hover:underline"
                    onClick={() => setParentRel(f.relPath)}
                  >
                    📁 {f.name}
                  </button>
                </li>
              ))
            )}
          </ul>
        </div>

        {error ? <p className="mt-3 text-sm text-rose-700">{error}</p> : null}

        <div className="mt-5 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <button
            type="button"
            onClick={onCancelKeepDraft}
            className="rounded-lg px-4 py-2 text-sm font-semibold text-slate-600 hover:bg-slate-100"
          >
            Garder le brouillon à la racine
          </button>
          <button
            type="button"
            disabled={saving || !name.trim()}
            onClick={() => void save()}
            className="rounded-lg bg-[var(--dash-primary)] px-4 py-2 text-sm font-semibold text-white disabled:opacity-60"
          >
            {saving ? "Enregistrement…" : "Renommer et ranger"}
          </button>
        </div>
      </div>
    </div>
  );
}
