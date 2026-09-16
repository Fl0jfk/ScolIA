"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import OfficeFirstPlaceModal from "@/app/components/documents/OfficeFirstPlaceModal";
import PeerPicker from "@/app/components/documents/PeerPicker";
import DocumentModal from "@/app/components/documents/DocumentModal";
import type { Peer } from "@/app/lib/documents-page-model";
import type { OfficeKind } from "@/app/lib/office-types";

type Props = {
  kind: OfficeKind;
  scope: string;
  path: string;
  shareId?: string | null;
  fileShareId?: string | null;
  draft?: boolean;
};

type TokenPayload = {
  editorUrl: string | null;
  collaboraConfigured: boolean;
  fileName: string;
  kind: OfficeKind;
  isOwner: boolean;
  needsPlaceOnClose: boolean;
  sessionId: string;
  fileId: string;
  scope: string;
  shareId: string | null;
  fileShareId: string | null;
  relPath: string;
};

export default function OfficeEditClient(props: Props) {
  const router = useRouter();
  const sessionStorageKey = useMemo(
    () =>
      `office-edit-session:${props.scope}:${props.shareId || ""}:${props.fileShareId || ""}:${props.path}`,
    [props.scope, props.shareId, props.fileShareId, props.path],
  );
  const [sessionId] = useState(() => {
    try {
      const existing = sessionStorage.getItem(sessionStorageKey);
      if (existing) return existing;
    } catch {
      /* private mode */
    }
    const id = crypto.randomUUID();
    try {
      sessionStorage.setItem(sessionStorageKey, id);
    } catch {
      /* ignore */
    }
    return id;
  });
  const [token, setToken] = useState<TokenPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [alreadyOpen, setAlreadyOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [showPlace, setShowPlace] = useState(false);
  const [versions, setVersions] = useState<{ id: string; createdAt: string; size: number }[]>([]);
  const [showVersions, setShowVersions] = useState(false);
  const [showShare, setShowShare] = useState(false);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [shareMembers, setShareMembers] = useState<string[]>([]);
  const [shareBusy, setShareBusy] = useState(false);

  const returnHref = "/documents/office";
  const bootstrapping = useRef(false);
  const bootGen = useRef(0);
  const tokenRef = useRef<TokenPayload | null>(null);
  /** URL iframe figée — évite remount Collabora (coupure WS) si le token JWT est renouvelé. */
  const stableEditorUrl = useRef<string | null>(null);
  tokenRef.current = token;

  const releaseSession = useCallback(
    (fileId: string | undefined, sid: string, clearStorage = false) => {
      if (!fileId) return;
      const body = JSON.stringify({
        action: "release",
        sessionId: sid,
        scope: props.scope,
        path: props.path,
        shareId: props.shareId,
        fileShareId: props.fileShareId,
      });
      if (clearStorage) {
        try {
          sessionStorage.removeItem(sessionStorageKey);
        } catch {
          /* ignore */
        }
      }
      try {
        if (typeof navigator !== "undefined" && typeof navigator.sendBeacon === "function") {
          const blob = new Blob([body], { type: "application/json" });
          if (navigator.sendBeacon("/api/documents/office/session", blob)) return;
        }
      } catch {
        /* fallback fetch */
      }
      void fetch("/api/documents/office/session", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body,
        keepalive: true,
      }).catch(() => undefined);
    },
    [props.scope, props.path, props.shareId, props.fileShareId, sessionStorageKey],
  );

  const bootstrap = useCallback(
    async (takeover: boolean, opts?: { silent?: boolean }) => {
      const silent = opts?.silent === true && Boolean(tokenRef.current);
      if (bootstrapping.current && !takeover) return;
      bootstrapping.current = true;
      const gen = ++bootGen.current;
      if (!silent) {
        setLoading(true);
        setError(null);
        setAlreadyOpen(false);
      }
      try {
        const res = await fetch("/api/documents/office/token", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            scope: props.scope,
            path: props.path,
            shareId: props.shareId,
            fileShareId: props.fileShareId,
            sessionId,
            takeover,
            draft: props.draft ? true : undefined,
          }),
        });
        const data = await res.json();
        if (gen !== bootGen.current) return;
        if (res.status === 409 && data.error === "already_open") {
          if (!silent) {
            setAlreadyOpen(true);
            setToken(null);
            stableEditorUrl.current = null;
          }
          return;
        }
        if (!res.ok) {
          if (!silent) setError(data.error || data.message || "Ouverture impossible.");
          return;
        }
        const next = data as TokenPayload;
        // Conserve la même URL iframe tant que fileId inchangé (évite close WS anticipé).
        if (
          silent &&
          tokenRef.current?.fileId === next.fileId &&
          stableEditorUrl.current
        ) {
          setToken({ ...next, editorUrl: stableEditorUrl.current });
        } else {
          if (next.editorUrl) stableEditorUrl.current = next.editorUrl;
          setToken(next);
        }

        const sess = await fetch(
          `/api/documents/office/session?${new URLSearchParams({
            scope: props.scope,
            path: props.path,
            ...(props.shareId ? { shareId: props.shareId } : {}),
            ...(props.fileShareId ? { fileShareId: props.fileShareId } : {}),
          })}`,
        );
        if (gen !== bootGen.current) return;
        const sessData = await sess.json();
        if (sess.ok && Array.isArray(sessData.versions)) {
          setVersions(sessData.versions);
        }
      } catch {
        if (!silent && gen === bootGen.current) setError("Erreur réseau.");
      } finally {
        if (gen === bootGen.current) {
          bootstrapping.current = false;
          if (!silent) setLoading(false);
        }
      }
    },
    [props.scope, props.path, props.shareId, props.fileShareId, props.draft, sessionId],
  );

  useEffect(() => {
    void bootstrap(false);
  }, [bootstrap]);

  useEffect(() => {
    if (!token?.fileId) return;
    let cancelled = false;
    const beat = () => {
      void fetch("/api/documents/office/session", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "heartbeat",
          sessionId,
          scope: props.scope,
          path: props.path,
          shareId: props.shareId,
          fileShareId: props.fileShareId,
        }),
      })
        .then(async (res) => {
          if (cancelled) return;
          if (res.status === 409) {
            // Reclaim silencieux sans remount iframe.
            await bootstrap(true, { silent: true });
          }
        })
        .catch(() => undefined);
    };
    beat();
    const timer = window.setInterval(beat, 45_000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [
    token?.fileId,
    sessionId,
    props.scope,
    props.path,
    props.shareId,
    props.fileShareId,
    bootstrap,
  ]);

  // Libération uniquement à la fermeture réelle de l’onglet (pas au remount React).
  useEffect(() => {
    const onHide = () => {
      const t = tokenRef.current;
      if (t?.fileId) releaseSession(t.fileId, sessionId, true);
    };
    window.addEventListener("pagehide", onHide);
    return () => {
      window.removeEventListener("pagehide", onHide);
    };
  }, [releaseSession, sessionId]);

  const leave = () => {
    if (token?.fileId) releaseSession(token.fileId, sessionId, true);
    stableEditorUrl.current = null;
    if (token?.needsPlaceOnClose) {
      setShowPlace(true);
      return;
    }
    router.push(returnHref);
  };

  const openShare = async () => {
    setShowShare(true);
    if (peers.length === 0) {
      const res = await fetch("/api/documents/peers");
      const data = await res.json();
      if (res.ok) setPeers(data.peers || []);
    }
  };

  const submitShare = async () => {
    if (props.scope !== "personal" || props.fileShareId) {
      setError("Le partage fichier se fait depuis le cloud pour les dossiers partagés.");
      return;
    }
    setShareBusy(true);
    try {
      const res = await fetch("/api/documents/file-shares", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          sourcePath: props.path,
          memberIds: shareMembers,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Partage impossible.");
        return;
      }
      setShowShare(false);
      setShareMembers([]);
    } catch {
      setError("Erreur réseau.");
    } finally {
      setShareBusy(false);
    }
  };

  const restoreVersion = async (versionId: string) => {
    const res = await fetch("/api/documents/office/session", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        action: "restore",
        versionId,
        scope: props.scope,
        path: props.path,
        shareId: props.shareId,
        fileShareId: props.fileShareId,
      }),
    });
    const data = await res.json();
    if (!res.ok) {
      setError(data.error || "Restauration impossible.");
      return;
    }
    setShowVersions(false);
    stableEditorUrl.current = null;
    void bootstrap(true);
  };

  const title = token?.fileName || props.path.split("/").pop() || "Document";

  const iframeSrc = useMemo(() => {
    if (stableEditorUrl.current) return stableEditorUrl.current;
    return token?.editorUrl || null;
  }, [token?.editorUrl]);

  const showEditor = Boolean(iframeSrc && token?.collaboraConfigured);

  return (
    <div className="flex h-[100dvh] flex-col bg-slate-100">
      <header className="flex shrink-0 items-center gap-2 border-b border-slate-200 bg-white px-3 py-2">
        <button
          type="button"
          onClick={leave}
          className="rounded-lg px-3 py-1.5 text-sm font-semibold text-slate-700 hover:bg-slate-100"
        >
          ← Retour
        </button>
        <p className="min-w-0 flex-1 truncate text-sm font-semibold text-slate-900">{title}</p>
        {token?.isOwner && versions.length > 0 ? (
          <button
            type="button"
            onClick={() => setShowVersions(true)}
            className="rounded-lg px-3 py-1.5 text-sm font-semibold text-slate-700 hover:bg-slate-100"
          >
            Versions
          </button>
        ) : null}
        <button
          type="button"
          onClick={() => void openShare()}
          className="rounded-lg bg-[var(--dash-primary)] px-3 py-1.5 text-sm font-semibold text-white"
        >
          Partager
        </button>
      </header>

      <div className="relative min-h-0 flex-1">
        {showEditor ? (
          <iframe
            title={title}
            src={iframeSrc!}
            className="h-full w-full border-0"
            allow="clipboard-read; clipboard-write; fullscreen"
            referrerPolicy="no-referrer-when-downgrade"
          />
        ) : loading ? (
          <p className="p-6 text-sm text-slate-600">Ouverture de l’éditeur…</p>
        ) : alreadyOpen ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
            <p className="text-sm text-slate-700">
              Ce document est déjà ouvert dans un autre onglet.
            </p>
            <button
              type="button"
              className="rounded-lg bg-[var(--dash-primary)] px-4 py-2 text-sm font-semibold text-white"
              onClick={() => void bootstrap(true)}
            >
              Prendre la main
            </button>
            <button
              type="button"
              className="text-sm font-semibold text-slate-600 underline"
              onClick={() => router.push(returnHref)}
            >
              Annuler
            </button>
          </div>
        ) : error ? (
          <div className="p-6">
            <p className="text-sm text-rose-700">{error}</p>
            <button
              type="button"
              className="mt-3 text-sm font-semibold underline"
              onClick={() => void bootstrap(false)}
            >
              Réessayer
            </button>
          </div>
        ) : (
          <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
            <p className="max-w-md text-sm text-slate-700">
              L’éditeur Collabora CODE n’est pas joignable. Démarrez-le en local (
              <code className="text-xs">docker compose -f docker-compose.collabora.yml up -d</code>
              ) ou configurez <code className="text-xs">COLLABORA_URL</code>.
            </p>
            <p className="text-xs text-slate-500">
              Le fichier reste dans votre cloud : {title}
            </p>
            <button
              type="button"
              className="rounded-lg bg-[var(--dash-primary)] px-4 py-2 text-sm font-semibold text-white"
              onClick={() => void bootstrap(false)}
            >
              Réessayer
            </button>
          </div>
        )}
      </div>

      {showPlace && token ? (
        <OfficeFirstPlaceModal
          kind={props.kind}
          fromScope={token.scope}
          fromShareId={token.shareId}
          fromRelPath={token.relPath}
          currentName={token.fileName}
          onDone={() => {
            setShowPlace(false);
            router.push(returnHref);
          }}
          onCancelKeepDraft={() => {
            setShowPlace(false);
            router.push(returnHref);
          }}
        />
      ) : null}

      {showVersions ? (
        <DocumentModal title="Versions" onClose={() => setShowVersions(false)}>
          <ul className="divide-y divide-slate-100">
            {versions.map((v) => (
              <li key={v.id} className="flex items-center justify-between gap-2 py-2">
                <span className="text-sm text-slate-700">
                  {new Date(v.createdAt).toLocaleString("fr-FR")} —{" "}
                  {Math.round(v.size / 1024)} Ko
                </span>
                <button
                  type="button"
                  className="text-sm font-semibold text-[var(--dash-primary)]"
                  onClick={() => void restoreVersion(v.id)}
                >
                  Restaurer
                </button>
              </li>
            ))}
          </ul>
        </DocumentModal>
      ) : null}

      {showShare ? (
        <DocumentModal title={`Partager « ${title} »`} onClose={() => setShowShare(false)} wide>
          <PeerPicker peers={peers} selected={shareMembers} onChangeSelected={setShareMembers} />
          <div className="mt-4 flex justify-end gap-2">
            <button
              type="button"
              className="rounded-lg px-3 py-2 text-sm font-semibold text-slate-600"
              onClick={() => setShowShare(false)}
            >
              Annuler
            </button>
            <button
              type="button"
              disabled={shareBusy || shareMembers.length === 0}
              className="rounded-lg bg-[var(--dash-primary)] px-3 py-2 text-sm font-semibold text-white disabled:opacity-60"
              onClick={() => void submitShare()}
            >
              {shareBusy ? "…" : "Partager"}
            </button>
          </div>
        </DocumentModal>
      ) : null}
    </div>
  );
}
