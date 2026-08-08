"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";

type PostRow = {
  id: string;
  slug: string;
  title: string;
  excerpt?: string;
  status: "draft" | "published";
  updatedAt: string;
  publishedAt?: string;
};

type Props = {
  enabled: boolean;
  onRefreshFlag?: () => void;
};

export default function CommunicationSitePostsPanel({ enabled, onRefreshFlag }: Props) {
  const [items, setItems] = useState<PostRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [excerpt, setExcerpt] = useState("");
  const [body, setBody] = useState("");
  const [coverUrl, setCoverUrl] = useState("");
  const [status, setStatus] = useState<"draft" | "published">("draft");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    if (!enabled) return;
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/site-cms/posts", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setItems(Array.isArray(j.items) ? j.items : []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [enabled]);

  useEffect(() => {
    void load();
  }, [load]);

  const resetForm = () => {
    setEditingId(null);
    setTitle("");
    setExcerpt("");
    setBody("");
    setCoverUrl("");
    setStatus("draft");
  };

  const openEdit = async (id: string) => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/site-cms/posts/${id}`, { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Introuvable");
      const p = j.post;
      setEditingId(p.id);
      setTitle(p.title || "");
      setExcerpt(p.excerpt || "");
      setBody(p.body || "");
      setCoverUrl(p.coverUrl || "");
      setStatus(p.status === "published" ? "published" : "draft");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    setBusy(true);
    setMsg(null);
    setError(null);
    try {
      const payload = { title, excerpt, body, coverUrl, status };
      const res = await fetch(
        editingId ? `/api/site-cms/posts/${editingId}` : "/api/site-cms/posts",
        {
          method: editingId ? "PATCH" : "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        },
      );
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      setMsg(editingId ? "Article mis à jour." : "Article créé.");
      resetForm();
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const remove = async (id: string) => {
    if (!confirm("Supprimer cet article ?")) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/site-cms/posts/${id}`, { method: "DELETE" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Suppression impossible");
      if (editingId === id) resetForm();
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  if (!enabled) {
    return (
      <div className="rounded-2xl border border-amber-200 bg-amber-50/70 p-6 space-y-3">
        <h2 className="text-lg font-black text-amber-950">Actus site — option inactive</h2>
        <p className="text-sm text-amber-900/90">
          Activez <strong>Site vitrine Scola</strong> dans Paramètres → Établissement lorsque le
          site Next.js packagé a été livré. Sans cela, pas de CMS actus (évite un faux back-office
          pour les établissements sans site).
        </p>
        <div className="flex flex-wrap gap-2">
          <Link
            href="/parametres?tab=site"
            className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white"
          >
            Ouvrir les paramètres →
          </Link>
          <button
            type="button"
            onClick={() => onRefreshFlag?.()}
            className="rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-bold text-slate-700"
          >
            Actualiser
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-sky-200 bg-sky-50/50 p-5">
        <h2 className="text-lg font-black text-sky-950">Actus du site vitrine</h2>
        <p className="mt-1 text-sm text-sky-900/80">
          Articles publiés via l&apos;API publique{" "}
          <code className="text-xs bg-white/80 px-1 rounded">/api/public/site/posts</code> pour le
          site Next.js du tenant.
        </p>
      </div>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      ) : null}
      {msg ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
          {msg}
        </p>
      ) : null}

      <div className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h3 className="text-sm font-black text-slate-800">
          {editingId ? "Modifier l’article" : "Nouvel article"}
        </h3>
        <label className="block text-xs font-bold uppercase text-slate-500">
          Titre
          <input
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal normal-case text-slate-800"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
        </label>
        <label className="block text-xs font-bold uppercase text-slate-500">
          Extrait
          <input
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal normal-case text-slate-800"
            value={excerpt}
            onChange={(e) => setExcerpt(e.target.value)}
          />
        </label>
        <label className="block text-xs font-bold uppercase text-slate-500">
          Corps (markdown simple)
          <textarea
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal normal-case text-slate-800"
            rows={6}
            value={body}
            onChange={(e) => setBody(e.target.value)}
          />
        </label>
        <label className="block text-xs font-bold uppercase text-slate-500">
          Image de couverture (URL, optionnel)
          <input
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal normal-case text-slate-800"
            value={coverUrl}
            placeholder="https://…"
            onChange={(e) => setCoverUrl(e.target.value)}
          />
        </label>
        <label className="block text-xs font-bold uppercase text-slate-500">
          Statut
          <select
            className="mt-1 w-full max-w-xs rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal normal-case"
            value={status}
            onChange={(e) => setStatus(e.target.value === "published" ? "published" : "draft")}
          >
            <option value="draft">Brouillon</option>
            <option value="published">Publié</option>
          </select>
        </label>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            disabled={busy || !title.trim()}
            onClick={() => void save()}
            className="rounded-xl bg-sky-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            {busy ? "…" : editingId ? "Enregistrer" : "Créer"}
          </button>
          {editingId ? (
            <button
              type="button"
              onClick={resetForm}
              className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-700"
            >
              Annuler
            </button>
          ) : null}
        </div>
      </div>

      <div className="rounded-2xl border border-slate-100 bg-slate-50/80 p-5">
        <h3 className="text-sm font-black text-slate-800">Articles</h3>
        {loading ? (
          <p className="mt-2 text-sm text-slate-500">Chargement…</p>
        ) : items.length === 0 ? (
          <p className="mt-2 text-sm text-slate-500">Aucun article.</p>
        ) : (
          <ul className="mt-3 space-y-2">
            {items.map((p) => (
              <li
                key={p.id}
                className="flex flex-wrap items-center justify-between gap-2 rounded-xl bg-white px-3 py-2 text-sm ring-1 ring-slate-100"
              >
                <div>
                  <p className="font-semibold text-slate-800">{p.title}</p>
                  <p className="text-xs text-slate-500">
                    {p.status === "published" ? "Publié" : "Brouillon"} · {p.slug} ·{" "}
                    {new Date(p.updatedAt).toLocaleString("fr-FR")}
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    type="button"
                    className="text-xs font-bold text-sky-700 underline"
                    onClick={() => void openEdit(p.id)}
                  >
                    Éditer
                  </button>
                  <button
                    type="button"
                    className="text-xs font-bold text-rose-600 underline"
                    onClick={() => void remove(p.id)}
                  >
                    Supprimer
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
