"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import dynamic from "next/dynamic";
import { useUser } from "@clerk/nextjs";
import MesDemandesSuivi from "@/app/(admin)/requests/MesDemandesSuivi";
import CompleteRequestModal, {
  type CompleteRequestTarget,
} from "@/app/components/requests/CompleteRequestModal";
import CreateRequestModal from "@/app/components/requests/CreateRequestModal";
import FaireUneDemandeForm from "@/app/components/requests/FaireUneDemandeForm";
import CorbeilleInbox, { type PileKey } from "@/app/components/requests/CorbeilleInbox";
import RequestBoardMoveSelect from "@/app/components/requests/RequestBoardMoveSelect";
import { useBoardPointerDnd } from "@/app/lib/requests-board-dnd";
import { useMobileBoardUi } from "@/app/hooks/useMobileBoardUi";
import type { VisualColumnKey } from "@/app/lib/request-board-move";
import { getViewerServiceLabel } from "@/app/lib/requests-view-utils";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import type { RequestsRoutingConfig } from "@/app/lib/app-config-schemas";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";

type RequestsMainTab = "board" | "tags" | "routing";

const RequestPersonnelTagsPanel = dynamic(
  () => import("@/app/components/requests/RequestPersonnelTagsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const RequestsRoutingPanel = dynamic(
  () => import("@/app/components/requests/RequestsRoutingPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

type RequestStatus = "NOUVELLE" | "EN_COURS" | "EN_ATTENTE" | "TERMINEE";

type BoardColumnKey = "CORBEILLE" | "NOUVELLES" | "EN_COURS" | "EN_ATTENTE" | "TERMINEE";

type RequestRecord = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: RequestStatus;
  category: string;
  subject: string;
  description: string;
  requester: {
    fullName: string;
    email: string;
    phone: string;
  };
  assignedTo: {
    routeId?: string;
    unit: string;
    roleLabel: string;
    email: string;
    ccEmails?: string[];
    poolEmails?: string[];
    claimedBy?: { email: string; name: string; userId?: string | null; at: string } | null;
  };
  routing?: {
    source?: string;
    confidence?: number;
    reason?: string;
    suggestedRouteId?: string;
    directionHint?: { suggestedQueueId: string; label: string; confidence?: number; reason?: string };
  };
  parentContext?: {
    source?: string;
    matched?: boolean;
    children?: Array<{ ine: string; nom: string; prenom: string; classe?: string }>;
  };
  attachments?: Array<{ id: string; fileName: string; size: number }>;
  comments: Array<{
    id: string;
    at: string;
    by: string;
    toRequester: boolean;
    content: string;
    attachments?: Array<{ id: string; fileName: string; size: number }>;
  }>;
  boardColumn?: BoardColumnKey;
  boardCanReassign?: boolean;
  boardCanDelegate?: boolean;
  delegateTargets?: string[];
  purgeAt?: string | null;
};

type SubmittedRequest = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: RequestStatus;
  category: string;
  subject: string;
  description: string;
  assignedTo: {
    routeId?: string;
    unit: string;
    roleLabel: string;
  };
};

const PILE_BOARD_KEYS: BoardColumnKey[] = ["CORBEILLE", "NOUVELLES"];

function isPileColumn(col?: BoardColumnKey | null): boolean {
  return Boolean(col && PILE_BOARD_KEYS.includes(col));
}

function buildVisualColumns(serviceLabel: string) {
  return [
    {
      key: "A_TRAITER" as const,
      title: "À traiter",
      hint: `Non démarrées ou en attente (document, pièce manquante…) — ${serviceLabel}. Glissez depuis « En cours » pour marquer en attente.`,
      acceptDrop: true,
      boardKeys: ["EN_ATTENTE"] as BoardColumnKey[],
      shell: "border-red-200/60 bg-gradient-to-b from-red-50 to-rose-50/50",
      body: "",
      titleClass: "text-red-900",
      hintClass: "text-red-800/80",
      dropRing: "ring-2 ring-red-300 border-red-300",
      badgeClass: "bg-red-100 text-red-900 border-red-200",
    },
    {
      key: "EN_COURS" as const,
      title: "En cours",
      hint: "Demandes que vous traitez activement. Déposer ici pour prendre en charge.",
      acceptDrop: true,
      boardKeys: ["EN_COURS"] as BoardColumnKey[],
      shell: "border-orange-200/60 bg-gradient-to-b from-orange-50 to-amber-50/50",
      body: "",
      titleClass: "text-orange-900",
      hintClass: "text-orange-800/80",
      dropRing: "ring-2 ring-orange-300 border-orange-300",
      badgeClass: "bg-orange-100 text-orange-900 border-orange-200",
    },
    {
      key: "TERMINEE" as const,
      title: "Terminée",
      hint: "Glisser ici pour clôturer — un message au demandeur (avec PJ) vous sera proposé.",
      acceptDrop: true,
      boardKeys: ["TERMINEE"] as BoardColumnKey[],
      shell: "border-emerald-200/60 bg-gradient-to-b from-emerald-50 to-green-50/50",
      body: "",
      titleClass: "text-emerald-900",
      hintClass: "text-emerald-800/80",
      dropRing: "ring-2 ring-emerald-300 border-emerald-300",
      badgeClass: "bg-emerald-100 text-emerald-900 border-emerald-200",
    },
  ];
}

const CARD_SURFACE: Record<VisualColumnKey, string> = {
  A_TRAITER:
    "border-red-300 bg-[#fdfcfb] hover:border-red-400 motion-safe:hover:translate-y-[-1px] motion-reduce:hover:translate-y-0",
  EN_COURS:
    "border-orange-300 bg-[#fdfcfb] hover:border-orange-400 motion-safe:hover:translate-y-[-1px] motion-reduce:hover:translate-y-0",
  TERMINEE:
    "border-emerald-300 bg-[#fdfcfb] hover:border-emerald-400 motion-safe:hover:translate-y-[-1px] motion-reduce:hover:translate-y-0",
};

function normEmail(e: string) { return e.trim().toLowerCase()}

function requesterShort(name: string) {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0]!.slice(0, 12);
  return `${parts[0]!.charAt(0)}. ${parts[parts.length - 1]!.slice(0, 10)}`;
}

function RequestAttachmentLinks({
  requestId,
  items,
}: {
  requestId: string;
  items?: Array<{ id: string; fileName: string; size: number }>;
}) {
  if (!items?.length) return null;
  const open = async (attachmentId: string) => {
    const res = await fetch(`/api/requests/attachment-url?requestId=${encodeURIComponent(requestId)}&attachmentId=${encodeURIComponent(attachmentId)}`);
    const data = (await res.json()) as { url?: string };
    if (data.url) window.open(data.url, "_blank", "noopener,noreferrer");
  };
  return (
    <div className="mt-1 rounded-md bg-white/80 border border-slate-200/80 px-1.5 py-1">
      <p className="text-[8px] font-bold uppercase tracking-wide text-slate-400 mb-0.5">PJ</p>
      <ul className="space-y-1">
        {items.map((a) => (
          <li key={a.id}>
            <button
              type="button"
              className="text-[10px] text-sky-800 hover:underline text-left font-medium"
              onClick={() => void open(a.id)}
            >
              {a.fileName}
            </button>
            <span className="text-slate-400 text-[9px]">
              {" "}
              {a.size >= 1024 ? `${Math.round(a.size / 1024)} Ko` : `${a.size} o`}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export default function RequestsPage() {
  const { isLoaded, user } = useUser();
  const isOrgAdmin = useIsOrgAdmin();
  const [mainTab, setMainTab] = useState<RequestsMainTab>("board");
  const [requestsRouting, setRequestsRouting] = useState<RequestsRoutingConfig | null>(null);
  const [routingMsg, setRoutingMsg] = useState<string | null>(null);
  const [routingBusy, setRoutingBusy] = useState(false);
  const [clerkMembers, setClerkMembers] = useState<
    Array<{ clerkUserId: string; email: string; displayName: string }>
  >([]);
  const [membersLoading, setMembersLoading] = useState(false);
  const [items, setItems] = useState<RequestRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [internalNoteById, setInternalNoteById] = useState<Record<string, string>>({});
  const [requesterNoteById, setRequesterNoteById] = useState<Record<string, string>>({});
  const [submittingId, setSubmittingId] = useState<string | null>(null);
  const [routeOptions, setRouteOptions] = useState<Array<{ id: string; label: string; category: string }>>([]);
  const [submittedItems, setSubmittedItems] = useState<SubmittedRequest[]>([]);
  const didScrollToMesDemandes = useRef(false);
  const [dropTarget, setDropTarget] = useState<VisualColumnKey | null>(null);
  const [commentFilesInternalById, setCommentFilesInternalById] = useState<Record<string, File[]>>({});
  const [commentFilesRequesterById, setCommentFilesRequesterById] = useState<Record<string, File[]>>({});
  const [pinnedCardId, setPinnedCardId] = useState<string | null>(null);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [completeTarget, setCompleteTarget] = useState<CompleteRequestTarget | null>(null);
  const [activePile, setActivePile] = useState<PileKey | null>(null);
  const [dropPileTarget, setDropPileTarget] = useState<PileKey | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const initialLoadDone = useRef(false);
  const draggedRequestIdRef = useRef<string | null>(null);
  const [hasStaffBoard, setHasStaffBoard] = useState(false);
  const [delegateEmailById, setDelegateEmailById] = useState<Record<string, string>>({});
  const mobileMoveMode = useMobileBoardUi();
  const userEmail = user?.primaryEmailAddress?.emailAddress ?? "";
  const userRoles = useMemo(() => rolesFromUserLike(user), [user]);
  const serviceLabel = useMemo(() => getViewerServiceLabel(userRoles), [userRoles]);
  const visualColumns = useMemo(() => buildVisualColumns(serviceLabel), [serviceLabel]);
  const kanbanItems = useMemo(
    () => items.filter((r) => r.boardColumn && !isPileColumn(r.boardColumn)),
    [items],
  );
  const { isSubmitOnlyUser } = useMemo(() => {
    if (!user) return { isSubmitOnlyUser: false };
    const norm = (v: string) => v.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/[\s-]+/g, "_");
    const normalized = userRoles.map(norm);
    const isProfesseur = normalized.includes("professeur");
    const isInfirmerie = normalized.includes("infirmerie");
    return { isSubmitOnlyUser: isProfesseur || isInfirmerie };
  }, [user, userRoles]);
  const refreshBoard = useCallback(async () => {
    try {
      const resSubmitted = await fetch("/api/requests/list?scope=submitted", { cache: "no-store" });
      const dataSubmitted = await resSubmitted.json();
      if (resSubmitted.ok) setSubmittedItems(Array.isArray(dataSubmitted) ? dataSubmitted : []);
      const resTeam = await fetch("/api/requests/list?scope=board", { cache: "no-store" });
      const dataTeam = await resTeam.json();
      if (resTeam.ok) {
        setHasStaffBoard(true);
        setItems(Array.isArray(dataTeam) ? dataTeam : []);
      } else {
        setHasStaffBoard(false);
        setItems([]);
      }
    } catch {
      /* ignore */
    }
  }, []);

  const load = useCallback(async () => {
    const showFullPageLoader = !initialLoadDone.current;
    if (showFullPageLoader) setLoading(true);
    try {
      await refreshBoard();
      initialLoadDone.current = true;
    } finally {
      if (showFullPageLoader) setLoading(false);
    }
  }, [refreshBoard]);
  useEffect(() => {
    if (!isLoaded || !user) return;
    void load();
  }, [isLoaded, user, load]);
  useEffect(() => {
    if (!isLoaded || loading || didScrollToMesDemandes.current) return;
    if (typeof window === "undefined") return;
    if (window.location.hash !== "#mes-demandes") return;
    didScrollToMesDemandes.current = true;
    requestAnimationFrame(() => {
      document.getElementById("mes-demandes")?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }, [isLoaded, loading]);

  useEffect(() => {
    if (!isLoaded || loading || typeof window === "undefined") return;
    const params = new URLSearchParams(window.location.search);
    if (params.get("nouvelle") !== "1") return;

    if (hasStaffBoard) {
      setCreateModalOpen(true);
    } else if (isSubmitOnlyUser) {
      requestAnimationFrame(() => {
        document.getElementById("faire-demande")?.scrollIntoView({ behavior: "smooth", block: "start" });
      });
    }

    params.delete("nouvelle");
    const qs = params.toString();
    const hash = window.location.hash;
    window.history.replaceState(null, "", `${window.location.pathname}${qs ? `?${qs}` : ""}${hash}`);
  }, [isLoaded, loading, hasStaffBoard, isSubmitOnlyUser]);

  useEffect(() => {
    if (!isLoaded || !user || !hasStaffBoard || loading) return;
    const loadRoutes = async () => {
      try {
        const res = await fetch("/api/requests/routes", { cache: "no-store" });
        const data = await res.json();
        if (res.ok && Array.isArray(data)) setRouteOptions(data);
      } catch {
        /* ignore */
      }
    };
    loadRoutes();
  }, [isLoaded, user, hasStaffBoard, loading]);
  const BOARD_MOVE_MIN_VISIBLE_MS = 280;
  const waitBoardMutationMinVisible = async (startedAt: number) => {
    const elapsed = Date.now() - startedAt;
    const rest = BOARD_MOVE_MIN_VISIBLE_MS - elapsed;
    if (rest > 0) await new Promise((r) => setTimeout(r, rest));
  };

  const patchRequest = async (
    id: string,
    body: Record<string, unknown> | FormData,
    isFormData = false,
  ): Promise<boolean> => {
    const res = await fetch("/api/requests/update", {
      method: "PATCH",
      ...(isFormData ? { body: body as FormData } : { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }),
    });
    const data = (await res.json()) as { request?: RequestRecord; error?: string };
    if (!res.ok) throw new Error(data.error || "Mise à jour impossible");
    await refreshBoard();
    return true;
  };

  const moveStatus = async (id: string, status: RequestStatus) => {
    const t0 = Date.now();
    setSubmittingId(id);
    try {
      await patchRequest(id, { id, status });
    } catch {
      /* refreshBoard déjà tenté dans patchRequest */
    } finally {
      await waitBoardMutationMinVisible(t0);
      setSubmittingId(null);
    }
  };

  const promptComplete = (requestId: string) => {
    const item = items.find((i) => i.id === requestId);
    if (!item) return;
    if (item.status === "TERMINEE" || item.boardColumn === "TERMINEE") {
      void moveStatus(requestId, "TERMINEE");
      return;
    }
    setCompleteTarget({
      id: item.id,
      subject: item.subject,
      requester: { fullName: item.requester.fullName, email: item.requester.email },
    });
  };

  const completeWithoutMessage = async (requestId: string) => {
    setCompleteTarget(null);
    await moveStatus(requestId, "TERMINEE");
  };

  const completeWithMessage = async (requestId: string, message: string, files: File[]) => {
    const t0 = Date.now();
    setSubmittingId(requestId);
    try {
      if (files.length > 0) {
        const fd = new FormData();
        fd.append("id", requestId);
        fd.append("status", "TERMINEE");
        fd.append("comment", message);
        fd.append("toRequester", "true");
        files.forEach((f) => fd.append("files", f));
        await patchRequest(requestId, fd, true);
      } else {
        await patchRequest(requestId, {
          id: requestId,
          status: "TERMINEE",
          comment: message,
          toRequester: true,
        });
      }
      setCompleteTarget(null);
    } catch {
      await refreshBoard();
    } finally {
      await waitBoardMutationMinVisible(t0);
      setSubmittingId(null);
    }
  };
  const reassign = async (id: string, assignRouteId: string) => {
    if (!assignRouteId) return;
    setSubmittingId(id);
    try {
      await patchRequest(id, { id, assignRouteId });
    } catch {
      /* noop */
    } finally {
      setSubmittingId(null);
    }
  };
  const claimAction = async (id: string, action: "claim" | "release_claim", toCorbeille?: boolean) => {
    const t0 = Date.now();
    setSubmittingId(id);
    try {
      await patchRequest(id, { id, action, ...(toCorbeille ? { toCorbeille: true } : {}) });
    } catch {
      /* noop */
    } finally {
      await waitBoardMutationMinVisible(t0);
      setSubmittingId(null);
    }
  };
  const delegateClaim = async (id: string) => {
    const targetEmail = (delegateEmailById[id] || "").trim();
    if (!targetEmail) return;
    const t0 = Date.now();
    setSubmittingId(id);
    try {
      await patchRequest(id, { id, action: "delegate_claim", targetEmail });
      setDelegateEmailById((prev) => ({ ...prev, [id]: "" }));
    } catch {
      /* noop */
    } finally {
      await waitBoardMutationMinVisible(t0);
      setSubmittingId(null);
    }
  };
  const claimSelf = async (id: string, status?: RequestStatus) => {
    const t0 = Date.now();
    setSubmittingId(id);
    try {
      await patchRequest(id, { id, action: "claim_self", ...(status ? { status } : {}) });
    } catch {
      /* noop */
    } finally {
      await waitBoardMutationMinVisible(t0);
      setSubmittingId(null);
    }
  };
  const transmitToDirection = async (id: string, assignRouteId: string) => {
    const t0 = Date.now();
    setSubmittingId(id);
    try {
      await patchRequest(id, { id, action: "transmit_to_direction", assignRouteId });
    } catch {
      /* noop */
    } finally {
      await waitBoardMutationMinVisible(t0);
      setSubmittingId(null);
    }
  };

  const onDropOnPile = (pile: PileKey, requestId: string) => {
    setDropPileTarget(null);
    setIsDragging(false);
    const item = items.find((i) => i.id === requestId);
    if (!item) return;
    void (async () => {
      if (pile === "etablissement") {
        if (item.boardColumn === "CORBEILLE") return;
        if (item.boardCanReassign) {
          await claimAction(requestId, "release_claim", true);
          return;
        }
        if (item.assignedTo.claimedBy?.email) {
          await claimAction(requestId, "release_claim");
        }
        return;
      }
      if (item.boardColumn === "CORBEILLE") {
        await claimAction(requestId, "claim");
        return;
      }
      if (item.assignedTo.claimedBy?.email) {
        await claimAction(requestId, "release_claim");
        if (item.status === "EN_ATTENTE" || item.status === "EN_COURS") {
          await moveStatus(requestId, "NOUVELLE");
        }
      }
    })();
  };

  const runColumnDrop = (targetCol: VisualColumnKey, requestId: string) => {
    const item = items.find((i) => i.id === requestId);
    if (!item) return;
    void (async () => {
      if (targetCol === "A_TRAITER") {
        if (item.boardColumn === "EN_COURS" || item.status === "EN_COURS") {
          await moveStatus(requestId, "EN_ATTENTE");
          return;
        }
        if (isPileColumn(item.boardColumn)) {
          if (item.boardColumn === "CORBEILLE") {
            await claimAction(requestId, "claim");
            await moveStatus(requestId, "EN_ATTENTE");
          } else {
            await claimSelf(requestId, "EN_ATTENTE");
          }
          return;
        }
        return;
      }
      if (targetCol === "EN_COURS") {
        if (isPileColumn(item.boardColumn)) {
          await claimSelf(requestId, "EN_COURS");
          return;
        }
        if (item.boardColumn === "EN_ATTENTE" || item.status === "EN_ATTENTE") {
          await moveStatus(requestId, "EN_COURS");
          return;
        }
        if (item.status === "EN_COURS" || item.status === "NOUVELLE") {
          const claimedByMe =
            Boolean(userEmail && item.assignedTo.claimedBy?.email && normEmail(item.assignedTo.claimedBy.email) === normEmail(userEmail));
          if (claimedByMe) return;
        }
        await claimSelf(requestId, "EN_COURS");
        return;
      }
      if (targetCol === "TERMINEE") promptComplete(requestId);
    })();
  };

  const { makeCardProps } = useBoardPointerDnd({
    draggedRequestIdRef,
    onDropColumn: (column, id) => {
      setDropTarget(null);
      setDropPileTarget(null);
      setIsDragging(false);
      runColumnDrop(column as VisualColumnKey, id);
    },
    onDropPile: (pile, id) => {
      setDropTarget(null);
      setDropPileTarget(null);
      setIsDragging(false);
      onDropOnPile(pile as PileKey, id);
    },
    onDragStateChange: setIsDragging,
    onHoverColumn: (column) => setDropTarget(column as VisualColumnKey | null),
    onHoverPile: (pile) => setDropPileTarget(pile as PileKey | null),
  });

  const sendComment = async (id: string, toRequester: boolean) => {
    const comment = toRequester ? (requesterNoteById[id] || "").trim() : (internalNoteById[id] || "").trim();
    const files = toRequester ? (commentFilesRequesterById[id] ?? []) : (commentFilesInternalById[id] ?? []);
    if (!comment && files.length === 0) return;
    setSubmittingId(id);
    try {
      if (files.length > 0) {
        const fd = new FormData();
        fd.append("id", id);
        fd.append("comment", comment);
        fd.append("toRequester", String(toRequester));
        files.forEach((f) => fd.append("files", f));
        await patchRequest(id, fd, true);
      } else {
        await patchRequest(id, { id, comment, toRequester });
      }
      if (toRequester) {
        setRequesterNoteById((prev) => ({ ...prev, [id]: "" }));
        setCommentFilesRequesterById((prev) => ({ ...prev, [id]: [] }));
      } else {
        setInternalNoteById((prev) => ({ ...prev, [id]: "" }));
        setCommentFilesInternalById((prev) => ({ ...prev, [id]: [] }));
      }
      await refreshBoard();
    } catch {
      await refreshBoard();
    } finally {
      setSubmittingId(null);
    }
  };

  const loadRoutingSettings = useCallback(async () => {
    if (!requestsRouting) {
      try {
        const res = await fetch("/api/settings/requests-routing");
        const j = await res.json();
        if (res.ok) setRequestsRouting(j.config as RequestsRoutingConfig);
      } catch {
        /* ignore */
      }
    }
    if (clerkMembers.length === 0) {
      setMembersLoading(true);
      try {
        const res = await fetch("/api/reservation-rooms/clerk-users");
        const j = await res.json();
        if (res.ok && Array.isArray(j.users)) {
          setClerkMembers(
            j.users.map(
              (u: {
                id?: string;
                clerkUserId?: string;
                email?: string;
                displayName?: string;
                name?: string;
              }) => ({
                clerkUserId: String(u.clerkUserId || u.id || ""),
                email: String(u.email || ""),
                displayName: String(u.displayName || u.name || u.email || ""),
              }),
            ),
          );
        }
      } catch {
        /* ignore */
      } finally {
        setMembersLoading(false);
      }
    }
  }, [requestsRouting, clerkMembers.length]);

  const saveRouting = async () => {
    if (!requestsRouting) return;
    setRoutingBusy(true);
    setRoutingMsg(null);
    try {
      const res = await fetch("/api/settings/requests-routing", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(requestsRouting),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec");
      setRequestsRouting(j.config as RequestsRoutingConfig);
      setRoutingMsg("Routage enregistré.");
    } catch (e) {
      setRoutingMsg(e instanceof Error ? e.message : "Erreur");
    } finally {
      setRoutingBusy(false);
    }
  };

  if (!isLoaded) {
    return (
      <ModulePageShell maxWidthClass="max-w-7xl">
        <p className="text-sm text-slate-600">Chargement...</p>
      </ModulePageShell>
    );
  }
  if (!user) {
    return (
      <ModulePageShell maxWidthClass="max-w-7xl">
        <p className="text-sm text-slate-600">Connexion requise.</p>
      </ModulePageShell>
    );
  }
  if (loading) {
    return (
      <ModulePageShell maxWidthClass="max-w-7xl">
        <p className="text-sm text-slate-600">Chargement des demandes…</p>
      </ModulePageShell>
    );
  }
  if (!isSubmitOnlyUser && !hasStaffBoard) {
    return (
      <ModulePageShell maxWidthClass="max-w-3xl">
        <p className="text-sm text-slate-700">
          Accès refusé. Le suivi des demandes est réservé aux enseignants (leurs dépôts) et au personnel figurant dans la table
          équipe des demandes ou disposant d’un rôle personnel adapté dans Clerk.
        </p>
      </ModulePageShell>
    );
  }
  if (!hasStaffBoard && isSubmitOnlyUser) {
    return (
      <ModulePageShell maxWidthClass="max-w-3xl" className="pb-24">
        <ModulePageHeader
          title="Demandes"
          description="Déposez une demande et suivez son avancement depuis cette page."
        />
        <div id="faire-demande" className="mt-2 scroll-mt-24">
          <h2 className="text-lg font-black text-slate-900 mb-3">Faire une demande</h2>
          <FaireUneDemandeForm variant="inline" onSuccess={() => void load()} mesDemandesHref="/requests#mes-demandes" />
        </div>
        <div className="mt-10">
          <MesDemandesSuivi
            items={submittedItems}
            loading={loading}
            title="Mes demandes"
            intro="État d’avancement et service en charge (libellé lisible, pas de code technique)."
          />
        </div>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1500px]" className="relative">
      <ModulePageHeader
        title="Demandes"
        description={
          mainTab === "tags"
            ? "Paramétrage des tags pour le routage IA"
            : mainTab === "routing"
              ? "Réglages du ticketing — files et routage des demandes"
              : `Tableau de traitement — ${serviceLabel}`
        }
        actions={
          <>
            {mainTab === "board" ? (
              <ModuleButton
                data-tour="requests-new"
                onClick={() => setCreateModalOpen(true)}
                className="inline-flex items-center gap-2 shadow-lg"
              >
                <span className="text-lg leading-none">+</span>
                Faire une demande
              </ModuleButton>
            ) : null}
            <ReplayModuleTourButton moduleId="requests-staff" />
          </>
        }
      />

      {isOrgAdmin ? (
        <ModuleTabNav
          className="mb-6"
          tabs={[
            { id: "board", label: "Tableau" },
            { id: "tags", label: "Tags équipe" },
            { id: "routing", label: "Réglages" },
          ]}
          active={mainTab}
          onChange={(id) => {
            setMainTab(id);
            if (id === "routing") void loadRoutingSettings();
          }}
        />
      ) : null}

      {mainTab === "tags" && isOrgAdmin ? (
        <div className="mt-6 max-w-5xl">
          <RequestPersonnelTagsPanel />
        </div>
      ) : mainTab === "routing" && isOrgAdmin ? (
        <RequestsRoutingPanel
          requestsRouting={requestsRouting}
          onChange={setRequestsRouting}
          members={clerkMembers}
          membersLoading={membersLoading}
          routingMsg={routingMsg}
          routingBusy={routingBusy}
          onSave={saveRouting}
        />
      ) : (
        <>
      <CreateRequestModal
        open={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        onCreated={() => {
          setCreateModalOpen(false);
          void refreshBoard();
        }}
      />
      <CompleteRequestModal
        target={completeTarget}
        busy={Boolean(completeTarget && submittingId === completeTarget.id)}
        onClose={() => {
          if (submittingId) return;
          setCompleteTarget(null);
        }}
        onCompleteWithoutMessage={(id) => void completeWithoutMessage(id)}
        onCompleteWithMessage={(id, message, files) => void completeWithMessage(id, message, files)}
      />
      <div data-tour="requests-inbox">
      <CorbeilleInbox
        items={items}
        serviceLabel={serviceLabel}
        activePile={activePile}
        onActivePileChange={setActivePile}
        pinnedCardId={pinnedCardId}
        submittingId={submittingId}
        dropPileTarget={dropPileTarget}
        isDragging={isDragging && !mobileMoveMode}
        makeCardProps={makeCardProps}
        mobileMoveMode={mobileMoveMode}
        onMoveToPile={onDropOnPile}
        onMoveToColumn={runColumnDrop}
        onCardClick={(id) => {
          setPinnedCardId((cur) => (cur === id ? null : id));
        }}
      />
      </div>
      <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-4 relative z-0">
        {visualColumns.map((col) => {
          const colCards = kanbanItems.filter((r) => r.boardColumn && col.boardKeys.includes(r.boardColumn));
          return (
          <section
            key={col.key}
            data-drop-column={col.key}
            className={`rounded-2xl border p-3 min-h-[260px] transition-all duration-200 ${col.shell} ${
              dropTarget === col.key ? col.dropRing : ""
            }`}
          >
            <div className="min-h-[220px]">
            <h2 className={`text-sm font-black mb-1 tracking-tight flex items-center gap-2 ${col.titleClass}`}>
              <span className={`inline-block w-2.5 h-2.5 rounded-full border ${col.badgeClass}`} aria-hidden />
              {col.title}
              <span className="text-[10px] font-bold opacity-60">({colCards.length})</span>
            </h2>
            <p className={`text-[10px] mb-3 leading-snug ${col.hintClass}`}>{col.hint}</p>
            <div className="space-y-2 min-h-[160px]">
              {colCards.length === 0 ? (
                <p className="text-[10px] text-slate-400/90 italic text-center py-8 px-2 border border-dashed border-slate-200/80 rounded-xl bg-white/40 pointer-events-none">
                  {mobileMoveMode ? "Aucune demande ici" : "Glissez une demande ici"}
                </p>
              ) : null}
              {colCards.map((r) => {
                const pool = r.assignedTo.poolEmails && r.assignedTo.poolEmails.length > 0 ? r.assignedTo.poolEmails : [r.assignedTo.email];
                const isCorbeilleCard = r.assignedTo.routeId === "corbeille" || r.assignedTo.unit === "corbeille" || r.assignedTo.unit === "tri.inconnu";
                const sharedPool = isCorbeilleCard || (r.assignedTo.poolEmails?.length ?? 0) > 1;
                const inPool = Boolean(userEmail && pool.some((p) => normEmail(p) === normEmail(userEmail)));
                const claimed = r.assignedTo.claimedBy;
                const claimedByOther = claimed?.email && userEmail && normEmail(claimed.email) !== normEmail(userEmail);
                const claimedByMe = claimed?.email && userEmail && normEmail(claimed.email) === normEmail(userEmail);
                const showPoolClaim = sharedPool && inPool && !claimed;
                const showSelfClaim = !claimedByMe && (!claimed || Boolean(claimedByOther)) && !(sharedPool && inPool && !claimed);
                const isPinned = pinnedCardId === r.id;
                const isWaiting = r.status === "EN_ATTENTE" || r.boardColumn === "EN_ATTENTE";
                const isUnstarted = r.boardColumn === "NOUVELLES" || (r.status === "NOUVELLE" && !r.assignedTo.claimedBy?.email);
                return (
                <article
                  key={r.id}
                  id={`request-card-${r.id}`}
                  {...makeCardProps(r.id, {
                    enabled: !mobileMoveMode,
                    disabled: submittingId === r.id,
                    onActivate: () => setPinnedCardId((id) => (id === r.id ? null : r.id)),
                  })}
                  title={
                    mobileMoveMode
                      ? "Clic pour ouvrir ou fermer le détail"
                      : "Glisser la fiche vers une colonne ou une corbeille · Clic pour ouvrir ou fermer le détail"
                  }
                  className={`group relative rounded-xl border p-2 shadow-sm select-none transition-shadow duration-300 ease-out hover:shadow-md ${CARD_SURFACE[col.key]} ${
                    mobileMoveMode ? "cursor-pointer" : "cursor-grab active:cursor-grabbing"
                  } ${isPinned ? "z-10 shadow-md ring-1 ring-slate-300/80" : "z-0"}`}
                >
                  {submittingId === r.id ? (
                    <div
                      className="absolute inset-0 z-30 flex flex-col items-center justify-center gap-2 rounded-[11px] bg-white/80 backdrop-blur-[2px] motion-reduce:bg-white/92 motion-reduce:backdrop-blur-none"
                      aria-live="polite"
                      aria-busy="true"
                    >
                      <span
                        className="h-5 w-5 rounded-full border-2 border-slate-200 border-t-sky-600 motion-safe:animate-spin motion-reduce:border-slate-400 motion-reduce:border-t-slate-600 motion-reduce:animate-none"
                        aria-hidden
                      />
                      <span className="text-[9px] font-semibold text-slate-600">Enregistrement…</span>
                    </div>
                  ) : null}
                  <div className="flex gap-1 items-start rounded-lg -mx-0.5 px-0.5 pt-0.5 pb-1">
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-x-1.5 gap-y-0.5">
                        <span className="text-[9px] font-bold uppercase tracking-wide text-slate-400 truncate max-w-[min(100%,7rem)]">
                          {r.category}
                        </span>
                        {r.attachments?.length ? (
                          <span
                            className="text-[8px] font-bold tabular-nums text-sky-800 bg-sky-100/90 px-1 py-px rounded"
                            title={`${r.attachments.length} pièce(s) jointe(s)`}
                          >
                            {r.attachments.length} PJ
                          </span>
                        ) : null}
                        {r.routing?.directionHint ? (
                          <span className="text-[8px] font-bold uppercase tracking-wide text-amber-800 bg-amber-50 border border-amber-100 px-1 py-px rounded" title={r.routing.directionHint.label}>
                            Direction ?
                          </span>
                        ) : null}
                        {isWaiting ? (
                          <span className="text-[8px] font-bold uppercase tracking-wide text-orange-900 bg-orange-100 border border-orange-300 px-1 py-px rounded" title="En attente de document ou d'information">
                            En attente
                          </span>
                        ) : isUnstarted ? (
                          <span className="text-[8px] font-bold uppercase tracking-wide text-red-800 bg-red-100 border border-red-200 px-1 py-px rounded">
                            Non démarrée
                          </span>
                        ) : null}
                        <span
                          className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                            claimedByMe
                              ? "bg-emerald-500"
                              : claimedByOther
                                ? "bg-amber-500"
                                : sharedPool
                                  ? "bg-indigo-400 ring-2 ring-indigo-200"
                                  : "bg-slate-300"
                          }`}
                          title={
                            claimedByMe
                              ? "Vous traitez"
                              : claimedByOther
                                ? "Collègue sur la fiche"
                                : sharedPool
                                  ? "File équipe"
                                  : "Libre"
                          }
                        />
                      </div>
                      <h3 className="text-[11px] font-bold text-slate-900 leading-snug mt-0.5 line-clamp-2">
                        {r.subject}
                      </h3>
                      <p className="text-[9px] text-slate-500 leading-snug mt-0.5 line-clamp-3 opacity-95">
                        {r.description}
                      </p>
                      <p className="text-[9px] text-slate-400 mt-0.5 truncate">
                        {requesterShort(r.requester.fullName)}
                      </p>
                    </div>
                    <button
                      type="button"
                      draggable={false}
                      onClick={(e) => {
                        e.stopPropagation();
                        if (draggedRequestIdRef.current === r.id) return;
                        setPinnedCardId((id) => (id === r.id ? null : r.id));
                      }}
                      className="shrink-0 -mr-0.5 -mt-0.5 rounded-lg px-1.5 py-1 text-slate-400 hover:bg-slate-200/70 hover:text-slate-800 transition-colors"
                      aria-expanded={isPinned}
                      aria-label={isPinned ? "Réduire la fiche" : "Épingler le détail"}
                      title="Épingler / réduire"
                    >
                      <span className="text-sm font-bold leading-none">{isPinned ? "▴" : "⋯"}</span>
                    </button>
                  </div>

                  <RequestBoardMoveSelect
                    requestId={r.id}
                    item={r}
                    serviceLabel={serviceLabel}
                    disabled={submittingId === r.id}
                    onMoveToPile={onDropOnPile}
                    onMoveToColumn={runColumnDrop}
                  />

                  <div
                    className={`grid transition-[grid-template-rows,opacity] ease-out motion-reduce:transition-opacity motion-reduce:duration-200 ${
                      isPinned
                        ? "grid-rows-[1fr] opacity-100 duration-[950ms] delay-[120ms] motion-reduce:delay-0 motion-reduce:duration-200"
                        : "grid-rows-[0fr] opacity-0 duration-[650ms] delay-0"
                    }`}
                  >
                    <div className="min-h-0 overflow-hidden">
                      <div className="mt-2 space-y-2 border-t border-slate-200/50 pt-2">
                    <p className="text-[10px] text-slate-600 leading-relaxed">{r.description}</p>
                    <RequestAttachmentLinks requestId={r.id} items={r.attachments} />
                    <p className="text-[10px] text-slate-700">
                      <span className="font-semibold text-slate-800">{r.requester.fullName}</span>
                      <span className="text-slate-400"> · </span>
                      <span className="break-all">{r.requester.email}</span>
                    </p>
                    {r.parentContext?.source === "parent_portal" ? (
                      <div
                        className={`rounded-lg px-1.5 py-1 text-[9px] leading-snug ${
                          r.parentContext.matched
                            ? "bg-emerald-50 text-emerald-900"
                            : "bg-amber-50 text-amber-950"
                        }`}
                      >
                        {r.parentContext.matched ? (
                          <>
                            <span className="font-black uppercase tracking-wide">
                              Parent reconnu
                            </span>
                            {r.parentContext.children && r.parentContext.children.length > 0 ? (
                              <span>
                                {" "}
                                —{" "}
                                {r.parentContext.children
                                  .map(
                                    (c) =>
                                      `${c.prenom} ${c.nom}${c.classe ? ` (${c.classe})` : ""}`,
                                  )
                                  .join(", ")}
                              </span>
                            ) : null}
                          </>
                        ) : (
                          <span className="font-black uppercase tracking-wide">
                            Parent non reconnu
                          </span>
                        )}
                      </div>
                    ) : null}
                    <p className="text-[10px] text-slate-700">
                      <span className="font-semibold text-slate-800">Service :</span> {r.assignedTo.roleLabel}
                    </p>
                    {r.assignedTo.routeId ? (
                      <p className="text-[8px] text-slate-400" title="Référence interne (routage)">
                        Réf. {r.assignedTo.routeId.replace(/\./g, " · ")}
                      </p>
                    ) : null}
                    {r.routing?.directionHint ? (
                      <p className="text-[9px] text-amber-900 bg-amber-50/90 rounded px-1.5 py-1">
                        Probablement pour la direction ({r.routing.directionHint.label}). L&apos;administratif valide avant transmission.
                      </p>
                    ) : null}
                    {isWaiting ? (
                      <p className="text-[9px] text-orange-900 bg-orange-100/90 border border-orange-200 rounded-lg px-2 py-1.5 leading-snug">
                        <span className="font-black uppercase tracking-wide">En attente</span> — document, justificatif ou information manquant(e). Repassez en « En cours » quand vous pouvez continuer.
                      </p>
                    ) : null}
                    {isCorbeilleCard ? (
                      <p className="text-[9px] text-rose-900 bg-rose-50/90 rounded px-1.5 py-1">
                        Corbeille établissement — visible par tout le personnel.
                      </p>
                    ) : sharedPool ? (
                      <p className="text-[9px] text-indigo-800 bg-indigo-50/80 rounded px-1.5 py-1">
                        File d’attente partagée : {pool.length} membre(s) du service peuvent prendre la demande.
                      </p>
                    ) : null}
                    {claimed?.email ? (
                      <p className={`text-[10px] font-medium ${claimedByOther ? "text-amber-800" : "text-emerald-800"}`}>
                        {claimedByOther
                          ? `→ ${claimed.name || claimed.email}`
                          : "→ Vous"}
                      </p>
                    ) : null}
                    <div className="flex flex-wrap gap-1">
                      {showPoolClaim ? (
                        <button
                          type="button"
                          disabled={submittingId === r.id}
                          onClick={() => void claimAction(r.id, "claim")}
                          className="text-[9px] px-2 py-1 rounded-md bg-indigo-600 text-white disabled:opacity-50 font-bold"
                        >
                          Prendre
                        </button>
                      ) : null}
                      {showSelfClaim ? (
                        <button
                          type="button"
                          disabled={submittingId === r.id}
                          onClick={() => void claimSelf(r.id, "EN_COURS")}
                          className="text-[9px] px-2 py-1 rounded-md bg-emerald-600 text-white disabled:opacity-50 font-bold"
                        >
                          M’attribuer
                        </button>
                      ) : null}
                      {r.routing?.directionHint ? (
                        <button
                          type="button"
                          disabled={submittingId === r.id}
                          onClick={() => void transmitToDirection(r.id, r.routing!.directionHint!.suggestedQueueId)}
                          className="text-[9px] px-2 py-1 rounded-md bg-amber-600 text-white disabled:opacity-50 font-bold"
                        >
                          Transmettre direction
                        </button>
                      ) : null}
                      {claimedByMe ? (
                        <button
                          type="button"
                          disabled={submittingId === r.id}
                          onClick={() => void claimAction(r.id, "release_claim")}
                          className="text-[9px] px-2 py-1 rounded-md border border-slate-300 text-slate-800 disabled:opacity-50 font-bold"
                        >
                          Libérer
                        </button>
                      ) : null}
                      {r.boardCanReassign ? (
                        <button
                          type="button"
                          disabled={submittingId === r.id}
                          onClick={() => void claimAction(r.id, "release_claim", true)}
                          className="text-[9px] px-2 py-1 rounded-md bg-amber-600 text-white disabled:opacity-50 font-bold"
                          title="Renvoyer à la corbeille établissement (responsable du service)"
                        >
                          Corbeille
                        </button>
                      ) : null}
                    </div>
                    {r.boardCanDelegate && (r.delegateTargets?.length ?? 0) > 0 ? (
                      <div className="flex flex-wrap gap-1 items-center mt-1">
                        <label htmlFor={`delegate-${r.id}`} className="text-[8px] font-bold text-violet-900 shrink-0">
                          Déléguer
                        </label>
                        <select
                          id={`delegate-${r.id}`}
                          disabled={submittingId === r.id}
                          value={delegateEmailById[r.id] ?? ""}
                          onChange={(e) =>
                            setDelegateEmailById((prev) => ({ ...prev, [r.id]: e.target.value }))
                          }
                          className="min-w-0 flex-1 max-w-[14rem] rounded-md border border-violet-200 p-1 text-[9px] bg-white"
                        >
                          <option value="">Choisir…</option>
                          {(r.delegateTargets ?? []).map((em) => (
                            <option key={em} value={em}>
                              {em}
                            </option>
                          ))}
                        </select>
                        <button
                          type="button"
                          disabled={
                            submittingId === r.id || !(delegateEmailById[r.id] || "").trim()
                          }
                          onClick={() => void delegateClaim(r.id)}
                          className="text-[9px] px-2 py-1 rounded-md bg-violet-600 text-white disabled:opacity-50 font-bold shrink-0"
                        >
                          OK
                        </button>
                      </div>
                    ) : null}
                    {r.routing?.suggestedRouteId ? (
                      <p className="text-[9px] text-amber-800 bg-amber-50/80 rounded px-1.5 py-0.5">
                        Suggestion IA :{" "}
                        {(() => {
                          const opt = routeOptions.find((o) => o.id === r.routing?.suggestedRouteId);
                          return opt ? `${opt.category} — ${opt.label}` : r.routing.suggestedRouteId.replace(/\./g, " · ");
                        })()}
                      </p>
                    ) : null}
                    {r.routing?.reason ? (
                      <p className="text-[9px] text-slate-500 leading-snug" title={r.routing.reason}>
                        {r.routing.reason}
                      </p>
                    ) : null}
                    {r.boardCanReassign ? (
                      <>
                        <label className="block text-[9px] font-bold text-slate-500">
                          Renvoyer vers un autre service (réaffectation — responsables uniquement)
                        </label>
                        <select
                          defaultValue=""
                          disabled={submittingId === r.id}
                          onChange={(e) => {
                            const v = e.target.value;
                            e.target.value = "";
                            if (v) void reassign(r.id, v);
                          }}
                          className="w-full rounded-md border border-slate-200 p-1.5 text-[9px] bg-white"
                        >
                          <option value="">Choisir un service…</option>
                          {routeOptions.map((opt) => (
                            <option key={opt.id} value={opt.id}>
                              {opt.category} — {opt.label}
                            </option>
                          ))}
                        </select>
                      </>
                    ) : null}
                    <div className="mt-2 space-y-2 rounded-xl border border-stone-200/80 bg-gradient-to-b from-stone-50 to-amber-50/30 p-2.5 shadow-sm shadow-stone-100/40">
                      <div>
                        <p className="text-[9px] font-black text-stone-700 uppercase tracking-wide">1 — Note équipe</p>
                        <p className="text-[8px] text-stone-500 mt-0.5">
                          Réservé au personnel : reste sur la fiche, <span className="font-semibold">aucun e-mail</span> au demandeur.
                        </p>
                      </div>
                      <textarea
                        value={internalNoteById[r.id] || ""}
                        onChange={(e) => setInternalNoteById((prev) => ({ ...prev, [r.id]: e.target.value }))}
                        rows={2}
                        placeholder="Votre note interne…"
                        className="w-full rounded-md border border-stone-200 p-1.5 text-[10px] bg-white/90"
                      />
                      <label className="block text-[8px] font-bold text-stone-500">Fichiers (optionnel, interne)</label>
                      <input
                        type="file"
                        multiple
                        accept="image/*,.pdf,.doc,.docx,.xls,.xlsx,application/pdf"
                        disabled={submittingId === r.id}
                        className="w-full text-[9px] text-stone-600 file:mr-1 file:rounded file:border-0 file:bg-stone-200 file:px-1.5 file:py-0.5"
                        onChange={(e) => {
                          const list = e.target.files ? Array.from(e.target.files) : [];
                          setCommentFilesInternalById((prev) => ({
                            ...prev,
                            [r.id]: [...(prev[r.id] ?? []), ...list].slice(0, 12),
                          }));
                          e.target.value = "";
                        }}
                      />
                      {(commentFilesInternalById[r.id]?.length ?? 0) > 0 ? (
                        <ul className="space-y-0.5">
                          {(commentFilesInternalById[r.id] ?? []).map((f, i) => (
                            <li
                              key={`${r.id}-int-${f.name}-${i}`}
                              className="flex justify-between gap-2 text-[8px] text-stone-600 bg-white rounded px-1.5 py-0.5 border border-stone-100"
                            >
                              <span className="truncate">{f.name}</span>
                              <button
                                type="button"
                                className="shrink-0 text-red-700 font-bold"
                                onClick={() =>
                                  setCommentFilesInternalById((prev) => ({
                                    ...prev,
                                    [r.id]: (prev[r.id] ?? []).filter((_, j) => j !== i),
                                  }))
                                }
                              >
                                ×
                              </button>
                            </li>
                          ))}
                        </ul>
                      ) : null}
                      <button
                        type="button"
                        onClick={() => void sendComment(r.id, false)}
                        disabled={submittingId === r.id}
                        className="w-full text-[9px] px-2 py-1.5 rounded-md bg-stone-700 hover:bg-stone-800 text-white disabled:opacity-50 font-bold transition-colors"
                      >
                        Enregistrer la note équipe
                      </button>
                    </div>
                    <div className="mt-2 space-y-2 rounded-lg border border-sky-200 bg-sky-50/60 p-2">
                      <div>
                        <p className="text-[9px] font-black text-sky-900 uppercase tracking-wide">2 — Message au demandeur</p>
                        <p className="text-[8px] text-slate-600 mt-0.5">
                          Part en <span className="font-semibold">e-mail</span> à {r.requester.fullName} ({r.requester.email}), avec les fichiers ci-dessous
                          si vous en ajoutez.
                        </p>
                      </div>
                      <textarea
                        value={requesterNoteById[r.id] || ""}
                        onChange={(e) => setRequesterNoteById((prev) => ({ ...prev, [r.id]: e.target.value }))}
                        rows={2}
                        placeholder="Message que recevra le demandeur…"
                        className="w-full rounded-md border border-sky-200 p-1.5 text-[10px] bg-white"
                      />
                      <label className="block text-[8px] font-bold text-sky-800">Fichiers pour le demandeur (optionnel)</label>
                      <input
                        type="file"
                        multiple
                        accept="image/*,.pdf,.doc,.docx,.xls,.xlsx,application/pdf"
                        disabled={submittingId === r.id}
                        className="w-full text-[9px] text-slate-600 file:mr-1 file:rounded file:border-0 file:bg-sky-200 file:px-1.5 file:py-0.5"
                        onChange={(e) => {
                          const list = e.target.files ? Array.from(e.target.files) : [];
                          setCommentFilesRequesterById((prev) => ({
                            ...prev,
                            [r.id]: [...(prev[r.id] ?? []), ...list].slice(0, 12),
                          }));
                          e.target.value = "";
                        }}
                      />
                      {(commentFilesRequesterById[r.id]?.length ?? 0) > 0 ? (
                        <ul className="space-y-0.5">
                          {(commentFilesRequesterById[r.id] ?? []).map((f, i) => (
                            <li
                              key={`${r.id}-req-${f.name}-${i}`}
                              className="flex justify-between gap-2 text-[8px] text-slate-600 bg-white rounded px-1.5 py-0.5 border border-sky-100"
                            >
                              <span className="truncate">{f.name}</span>
                              <button
                                type="button"
                                className="shrink-0 text-red-700 font-bold"
                                onClick={() =>
                                  setCommentFilesRequesterById((prev) => ({
                                    ...prev,
                                    [r.id]: (prev[r.id] ?? []).filter((_, j) => j !== i),
                                  }))
                                }
                              >
                                ×
                              </button>
                            </li>
                          ))}
                        </ul>
                      ) : null}
                      <button
                        type="button"
                        onClick={() => void sendComment(r.id, true)}
                        disabled={submittingId === r.id}
                        className="w-full text-[9px] px-2 py-1.5 rounded-md bg-sky-600 text-white disabled:opacity-50 font-bold"
                      >
                        Envoyer par e-mail au demandeur
                      </button>
                    </div>
                    {r.comments.length > 0 ? (
                      <div className="space-y-1.5 max-h-40 overflow-y-auto pr-0.5">
                        {r.comments.slice(-3).map((c) => (
                          <div key={c.id} className="rounded-md border border-slate-100 bg-white/90 p-1.5">
                            <p className="text-[9px] text-slate-600">
                              <span className="font-semibold text-slate-800">{c.by}</span> {c.content}
                            </p>
                            <RequestAttachmentLinks requestId={r.id} items={c.attachments} />
                          </div>
                        ))}
                      </div>
                    ) : null}
                      </div>
                    </div>
                  </div>
                </article>
              );
              })}
            </div>
            </div>
          </section>
          );
        })}
      </div>
      <div className="mt-14">
        <MesDemandesSuivi
          items={submittedItems}
          loading={loading}
          title="Mes demandes"
          intro="Demandes que vous avez déposées : statut et service qui les traite."
        />
      </div>
        </>
      )}
    </ModulePageShell>
  );
}