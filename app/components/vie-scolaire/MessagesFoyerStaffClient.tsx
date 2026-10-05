"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";
import { ACCEPT_FILE_INPUT } from "@/app/lib/messaging/constants";
import {
  FAMILLE_MESSAGING_ROLE_OPTIONS,
  type FamilleMessagingSettingsDto,
} from "@/app/lib/famille-messaging-matrix";

type FoyerLight = {
  id: string;
  label: string;
  eleves: Array<{ id: string; nom: string; prenom: string; classe: string | null }>;
};

type ThreadRow = {
  id: string;
  foyerId: string;
  eleveId: string | null;
  sujet: string;
  isBroadcast?: boolean;
  createdByNom: string | null;
  lastMessageAt: string;
  foyerLabel: string | null;
  eleveNom: string | null;
  elevePrenom: string | null;
};

type MessageRow = {
  id: string;
  auteurCote: string;
  auteurNom: string | null;
  corps: string;
  createdAt: string;
};

type AttachmentRow = {
  id: string;
  messageId: string;
  fileName: string;
  mime: string;
  size: number;
};

type PendingAtt = {
  fileName: string;
  mime: string;
  size: number;
  s3Key?: string | null;
  contentBase64?: string | null;
};

export default function MessagesFoyerStaffClient() {
  const [foyers, setFoyers] = useState<FoyerLight[]>([]);
  const [threads, setThreads] = useState<ThreadRow[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageRow[]>([]);
  const [attachments, setAttachments] = useState<AttachmentRow[]>([]);
  const [foyerId, setFoyerId] = useState("");
  const [eleveId, setEleveId] = useState("");
  const [sujet, setSujet] = useState("");
  const [corps, setCorps] = useState("");
  const [reply, setReply] = useState("");
  const [broadcast, setBroadcast] = useState(false);
  const [canBroadcast, setCanBroadcast] = useState(false);
  const [pendingAtts, setPendingAtts] = useState<PendingAtt[]>([]);
  const [replyAtts, setReplyAtts] = useState<PendingAtt[]>([]);
  const [settings, setSettings] = useState<FamilleMessagingSettingsDto | null>(null);
  const [canEditMatrix, setCanEditMatrix] = useState(false);
  const [matrixDraft, setMatrixDraft] = useState<FamilleMessagingSettingsDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);

  const selectedFoyer = useMemo(
    () => foyers.find((f) => f.id === foyerId) || null,
    [foyers, foyerId],
  );

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const [res, setRes] = await Promise.all([
        fetch("/api/vie-scolaire/messages-foyer", { cache: "no-store" }),
        fetch("/api/vie-scolaire/messages-foyer/settings", { cache: "no-store" }),
      ]);
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      const nextFoyers = (data.foyers || []) as FoyerLight[];
      setFoyers(nextFoyers);
      setThreads(data.threads || []);
      setCanBroadcast(!!data.canBroadcast);
      setFoyerId((prev) => prev || nextFoyers[0]?.id || "");
      if (setRes.ok) {
        const sj = await setRes.json();
        setSettings(sj.settings);
        setMatrixDraft(sj.settings);
        setCanEditMatrix(!!sj.canEdit);
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!selectedFoyer) {
      setEleveId("");
      return;
    }
    if (!selectedFoyer.eleves.some((e) => e.id === eleveId)) {
      setEleveId(selectedFoyer.eleves[0]?.id || "");
    }
  }, [selectedFoyer, eleveId]);

  const uploadFiles = async (files: FileList | null, threadId?: string) => {
    if (!files?.length) return [] as PendingAtt[];
    const out: PendingAtt[] = [];
    for (const file of Array.from(files)) {
      const fd = new FormData();
      fd.set("file", file);
      if (threadId) fd.set("threadId", threadId);
      const res = await fetch("/api/vie-scolaire/messages-foyer/upload", {
        method: "POST",
        body: fd,
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || `Upload ${file.name} impossible`);
      out.push(data.attachment as PendingAtt);
    }
    return out;
  };

  const openThread = async (id: string) => {
    setSelectedId(id);
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/vie-scolaire/messages-foyer/${id}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setMessages(data.messages || []);
      setAttachments(data.attachments || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const createThread = async () => {
    setBusy(true);
    setError(null);
    setOk(null);
    try {
      const res = await fetch("/api/vie-scolaire/messages-foyer", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          foyerId: broadcast ? undefined : foyerId,
          eleveId: broadcast ? null : eleveId || null,
          sujet,
          corps,
          broadcast,
          attachments: pendingAtts,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Envoi impossible");
      setSujet("");
      setCorps("");
      setPendingAtts([]);
      setBroadcast(false);
      setOk(
        data.broadcast
          ? `Diffusion envoyée à ${data.count} foyer(s).`
          : "Message envoyé à la famille.",
      );
      await load();
      if (data.thread?.id) await openThread(data.thread.id);
      else if (data.threads?.[0]?.id) await openThread(data.threads[0].id);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const sendReply = async () => {
    if (!selectedId) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/vie-scolaire/messages-foyer", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "reply",
          threadId: selectedId,
          corps: reply,
          attachments: replyAtts,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Envoi impossible");
      setReply("");
      setReplyAtts([]);
      await openThread(selectedId);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const saveMatrix = async () => {
    if (!matrixDraft) return;
    setBusy(true);
    setError(null);
    setOk(null);
    try {
      const res = await fetch("/api/vie-scolaire/messages-foyer/settings", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(matrixDraft),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Enregistrement impossible");
      setSettings(data.settings);
      setMatrixDraft(data.settings);
      setOk("Matrice enregistrée.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const attsFor = (messageId: string) => attachments.filter((a) => a.messageId === messageId);

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Vie scolaire"
        title="Messages familles"
        description="Canal dédié établissement ↔ foyer : texte, pièces, diffusion, matrice de rôles. Distinct du carnet et du Messenger staff."
        actions={
          <Link href="/vie-scolaire/carnet" className="text-sm font-bold text-indigo-600 hover:underline">
            Carnet de liaison
          </Link>
        }
      />

      {error ? <p className="mb-3 text-sm font-semibold text-rose-700">{error}</p> : null}
      {ok ? <p className="mb-3 text-sm font-semibold text-emerald-700">{ok}</p> : null}

      {canEditMatrix && matrixDraft ? (
        <section className="mb-4 rounded-2xl border border-amber-200 bg-amber-50/60 p-4 space-y-3 text-sm">
          <h2 className="font-bold text-slate-900">Matrice — qui peut écrire aux familles</h2>
          <div className="flex flex-wrap gap-3">
            {FAMILLE_MESSAGING_ROLE_OPTIONS.map((opt) => {
              const checked = matrixDraft.rolesCanInitiate.some(
                (r) => r.toLowerCase() === opt.id || (opt.id === "direction" && r.includes("direct")),
              );
              return (
                <label key={opt.id} className="inline-flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={(e) => {
                      const next = new Set(matrixDraft.rolesCanInitiate.map((r) => r.toLowerCase()));
                      if (e.target.checked) next.add(opt.id);
                      else next.delete(opt.id);
                      if (opt.id === "direction") {
                        next.delete("directeur");
                        next.delete("directrice");
                      }
                      setMatrixDraft({ ...matrixDraft, rolesCanInitiate: [...next] });
                    }}
                  />
                  {opt.label}
                </label>
              );
            })}
          </div>
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={matrixDraft.profOwnClassesOnly}
              onChange={(e) =>
                setMatrixDraft({ ...matrixDraft, profOwnClassesOnly: e.target.checked })
              }
            />
            Professeur limité à ses classes
          </label>
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={matrixDraft.allowBroadcast}
              onChange={(e) => setMatrixDraft({ ...matrixDraft, allowBroadcast: e.target.checked })}
            />
            Autoriser la diffusion multi-foyers
          </label>
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={matrixDraft.allowParentAttachments}
              onChange={(e) =>
                setMatrixDraft({ ...matrixDraft, allowParentAttachments: e.target.checked })
              }
            />
            Autoriser les pièces jointes parents
          </label>
          <button
            type="button"
            disabled={busy}
            onClick={() => void saveMatrix()}
            className="rounded-xl bg-amber-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-40"
          >
            Enregistrer la matrice
          </button>
          {settings ? (
            <p className={`text-xs ${dash.textMid}`}>
              Active : {settings.rolesCanInitiate.join(", ")}
              {settings.allowBroadcast ? " · diffusion OK" : " · pas de diffusion"}
            </p>
          ) : null}
        </section>
      ) : null}

      <div className="grid gap-4 lg:grid-cols-2">
        <section className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3 text-sm">
          <h2 className="font-bold text-slate-900">Nouveau message</h2>
          {canBroadcast ? (
            <label className="flex items-center gap-2 font-semibold text-slate-700">
              <input
                type="checkbox"
                checked={broadcast}
                onChange={(e) => setBroadcast(e.target.checked)}
              />
              Diffuser à tous les foyers visibles
            </label>
          ) : null}
          {!broadcast ? (
            <>
              <label className={`block ${dash.textMid}`}>
                Foyer
                <select
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                  value={foyerId}
                  onChange={(e) => setFoyerId(e.target.value)}
                >
                  {foyers.map((f) => (
                    <option key={f.id} value={f.id}>
                      {f.label}
                      {f.eleves.length
                        ? ` — ${f.eleves.map((e) => `${e.prenom} ${e.nom}`).join(", ")}`
                        : ""}
                    </option>
                  ))}
                </select>
              </label>
              {selectedFoyer?.eleves.length ? (
                <label className={`block ${dash.textMid}`}>
                  Élève (contexte)
                  <select
                    className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                    value={eleveId}
                    onChange={(e) => setEleveId(e.target.value)}
                  >
                    {selectedFoyer.eleves.map((e) => (
                      <option key={e.id} value={e.id}>
                        {e.prenom} {e.nom}
                        {e.classe ? ` (${e.classe})` : ""}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
            </>
          ) : null}
          <label className={`block ${dash.textMid}`}>
            Sujet
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={sujet}
              onChange={(e) => setSujet(e.target.value)}
              placeholder="Ex. Rendez-vous CPE"
            />
          </label>
          <label className={`block ${dash.textMid}`}>
            Message
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 min-h-[100px]"
              value={corps}
              onChange={(e) => setCorps(e.target.value)}
              placeholder="Texte destiné au foyer…"
            />
          </label>
          <label className={`block ${dash.textMid}`}>
            Pièces jointes
            <input
              type="file"
              multiple
              accept={ACCEPT_FILE_INPUT}
              className="mt-1 block w-full text-xs"
              onChange={(e) => {
                void (async () => {
                  try {
                    const uploaded = await uploadFiles(e.target.files);
                    setPendingAtts((prev) => [...prev, ...uploaded]);
                  } catch (err) {
                    setError(err instanceof Error ? err.message : "Upload impossible");
                  } finally {
                    e.target.value = "";
                  }
                })();
              }}
            />
          </label>
          {pendingAtts.length ? (
            <ul className="text-xs text-slate-600 list-disc pl-4">
              {pendingAtts.map((a, i) => (
                <li key={`${a.fileName}-${i}`}>{a.fileName}</li>
              ))}
            </ul>
          ) : null}
          <button
            type="button"
            disabled={
              busy ||
              !sujet.trim() ||
              (!corps.trim() && !pendingAtts.length) ||
              (!broadcast && !foyerId)
            }
            onClick={() => void createThread()}
            className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-40"
          >
            {broadcast ? "Diffuser" : "Envoyer au foyer"}
          </button>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-4">
          <h2 className="font-bold text-slate-900 mb-3">Conversations</h2>
          <ul className="space-y-2 text-sm max-h-[420px] overflow-auto">
            {threads.length === 0 ? (
              <li className={dash.textMid}>Aucune conversation pour l’instant.</li>
            ) : (
              threads.map((t) => (
                <li key={t.id}>
                  <button
                    type="button"
                    onClick={() => void openThread(t.id)}
                    className={`w-full text-left rounded-xl border px-3 py-2 ${
                      selectedId === t.id
                        ? "border-indigo-400 bg-indigo-50"
                        : "border-slate-200 bg-white hover:bg-slate-50"
                    }`}
                  >
                    <div className="font-bold text-slate-900">
                      {t.isBroadcast ? "📢 " : ""}
                      {t.sujet}
                    </div>
                    <div className={`text-xs ${dash.textMid}`}>
                      {t.foyerLabel || "Foyer"}
                      {t.elevePrenom || t.eleveNom
                        ? ` · ${t.elevePrenom || ""} ${t.eleveNom || ""}`.trim()
                        : ""}
                    </div>
                  </button>
                </li>
              ))
            )}
          </ul>
        </section>
      </div>

      {selectedId ? (
        <section className="mt-4 rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
          <h2 className="font-bold text-slate-900">Fil de discussion</h2>
          <div className="max-h-72 overflow-auto space-y-2">
            {messages.map((m) => (
              <div
                key={m.id}
                className={`rounded-xl px-3 py-2 text-sm ${
                  m.auteurCote === "staff"
                    ? "bg-indigo-50 border border-indigo-100"
                    : "bg-emerald-50 border border-emerald-100"
                }`}
              >
                <div className="text-xs font-bold text-slate-600 mb-1">
                  {m.auteurCote === "staff" ? "Établissement" : "Famille"}
                  {m.auteurNom ? ` — ${m.auteurNom}` : ""}
                </div>
                <p className="text-slate-900 whitespace-pre-wrap">{m.corps}</p>
                {attsFor(m.id).length ? (
                  <ul className="mt-2 space-y-1">
                    {attsFor(m.id).map((a) => (
                      <li key={a.id}>
                        <a
                          href={`/api/famille/messages/attachment/${a.id}`}
                          target="_blank"
                          rel="noreferrer"
                          className="text-xs font-bold text-indigo-700 underline"
                        >
                          📎 {a.fileName}
                        </a>
                      </li>
                    ))}
                  </ul>
                ) : null}
              </div>
            ))}
          </div>
          <div className="space-y-2">
            <input
              type="file"
              multiple
              accept={ACCEPT_FILE_INPUT}
              className="block w-full text-xs"
              onChange={(e) => {
                void (async () => {
                  try {
                    const uploaded = await uploadFiles(e.target.files, selectedId || undefined);
                    setReplyAtts((prev) => [...prev, ...uploaded]);
                  } catch (err) {
                    setError(err instanceof Error ? err.message : "Upload impossible");
                  } finally {
                    e.target.value = "";
                  }
                })();
              }}
            />
            {replyAtts.length ? (
              <ul className="text-xs text-slate-600 list-disc pl-4">
                {replyAtts.map((a, i) => (
                  <li key={`${a.fileName}-${i}`}>{a.fileName}</li>
                ))}
              </ul>
            ) : null}
            <div className="flex gap-2">
              <input
                className="flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={reply}
                onChange={(e) => setReply(e.target.value)}
                placeholder="Répondre…"
              />
              <button
                type="button"
                disabled={busy || (!reply.trim() && !replyAtts.length)}
                onClick={() => void sendReply()}
                className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-40"
              >
                Envoyer
              </button>
            </div>
          </div>
        </section>
      ) : null}
    </ModulePageShell>
  );
}
