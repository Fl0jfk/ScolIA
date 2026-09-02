"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import dynamic from "next/dynamic";
import { useState, useEffect, useRef, useCallback, useMemo, type Dispatch, type SetStateAction } from "react";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import { useTravelsPermissions } from "@/app/hooks/useTravelsPermissions";
import { useAppContext } from "@/app/hooks/useAppContext";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { mergeTripClassCatalogs } from "@/app/lib/travels-classes";
import {
  formFieldsToAccompagnateurs,
  type TravelsAccompagnateur,
} from "@/app/lib/travels-accompagnateurs";
import { emptyCuisineDetails } from "@/app/lib/travels-cuisine-form";
import {
  busLogisticsActive,
  complexNeedsBus,
  cuisineEffectifChanged,
  cuisineOrderWasSent,
  resolveCuisineOrderSentAt,
  datesChangedSinceSnapshot,
  effectifChangedSinceSnapshot,
  isValidEmailLoose,
  getModificationRequestNote,
  tripEffectifTotal,
} from "@/app/lib/travels-trip-helpers";
import type { TravelsHubTab, TravelsTrip } from "@/app/lib/travels-types";
import { uploadTravelDocument } from "@/app/lib/travels-upload-client";
import { TRAVELS_HUB_TABS, TRAVELS_STATUS_LABELS } from "@/app/lib/travels-types";
import { getTripNextGuidance } from "@/app/lib/travels-next-guidance";
import { orderEmailForQuote } from "@/app/lib/travels-transport-shared";
import { TripActionsPanel } from "@/app/components/travels/hub/TripActionsPanel";
import { TripAmendmentJournal } from "@/app/components/travels/hub/TripAmendmentJournal";
import { TripDecisionHubPanel } from "@/app/components/travels/hub/TripDecisionHubPanel";
import { TripDetailsModals } from "@/app/components/travels/hub/TripDetailsModals";
import { TripHubNav } from "@/app/components/travels/hub/TripHubNav";
import { TripInternalThreadPanel } from "@/app/components/travels/hub/TripInternalThreadPanel";
import { TripOverviewFieldsPanel } from "@/app/components/travels/hub/TripOverviewFieldsPanel";
import { TripRemindersBanner } from "@/app/components/travels/hub/TripRemindersBanner";
import { TripNextStepBanner } from "@/app/components/travels/hub/TripNextStepBanner";
import TravelsOwnerRepairSection from "@/app/components/travels/TravelsOwnerRepairSection";
import TravelsComptaSheetForm from "@/app/components/travels/TravelsComptaSheetForm";
import type { TravelsComptaSheet } from "@/app/lib/travels-compta-sheet";
import { comptaDocumentsFingerprint, comptaDefinitiveCostPerStudent, computeComptaSheetDerived } from "@/app/lib/travels-compta-sheet";
import {
  TripAlert,
  TripButton,
  TripHeroHeader,
  TripLoadingOverlay,
  TripPageShell,
  TripQuickStats,
  TripWorkflowStepper,
} from "@/app/components/travels/TripDetailUI";

const TripElevesListPanel = dynamic(
  () =>
    import("@/app/components/travels/hub/TripElevesListPanel").then((m) => ({
      default: m.TripElevesListPanel,
    })),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const TripParentComPanel = dynamic(
  () =>
    import("@/app/components/travels/hub/TripParentComPanel").then((m) => ({
      default: m.TripParentComPanel,
    })),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const TripTransportHubPanel = dynamic(
  () =>
    import("@/app/components/travels/hub/TripTransportHubPanel").then((m) => ({
      default: m.TripTransportHubPanel,
    })),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const TripCuisineHubPanel = dynamic(
  () =>
    import("@/app/components/travels/hub/TripCuisineHubPanel").then((m) => ({
      default: m.TripCuisineHubPanel,
    })),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const TripDocumentsHubPanel = dynamic(
  () =>
    import("@/app/components/travels/hub/TripDocumentsHubPanel").then((m) => ({
      default: m.TripDocumentsHubPanel,
    })),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

type TripDetailsLoadedProps = {
  trip: TravelsTrip;
  setTrip: Dispatch<SetStateAction<TravelsTrip | null>>;
};

export function TripDetailsLoaded({ trip, setTrip }: TripDetailsLoadedProps) {
  const { id } = useParams();
  const router = useRouter();
  const searchParams = useSearchParams();
  const remindersFocus = searchParams.get("focus") === "reminders";
  const highlightReminderId = searchParams.get("reminder");
  const tabFromUrl = searchParams.get("tab");
  const fileInputRef = useRef<HTMLInputElement>(null);
  const { user } = useSessionUser();
  const { data: appCtx } = useAppContext();
  const classOptions = useMemo(
    () =>
      mergeTripClassCatalogs(
        appCtx?.profRoom?.classesByPole,
        appCtx?.domainPlanning?.classesByPole,
      ),
    [appCtx?.profRoom?.classesByPole, appCtx?.domainPlanning?.classesByPole],
  );
  const [hubTab, setHubTab] = useState<TravelsHubTab>(() => {
    const t = tabFromUrl as TravelsHubTab | null;
    return t && TRAVELS_HUB_TABS.some((x) => x.id === t) ? t : "overview";
  });
  const [loadingAction, setLoadingAction] = useState<string | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [editedData, setEditedData] = useState<any>(() => ({
    ...trip.data,
    piqueNiqueDetails: trip.data?.piqueNiqueDetails || emptyCuisineDetails(),
  }));
  const [showCuisineModal, setShowCuisineModal] = useState(false);
  const [showEffectifModal, setShowEffectifModal] = useState(false);
  const [effectifFollowUp, setEffectifFollowUp] = useState<{
    sendTransport: boolean;
    sendCuisine: boolean;
    savedTrip: any;
  } | null>(null);
  const [draftNbEleves, setDraftNbEleves] = useState("");
  const [draftNbAccompagnateurs, setDraftNbAccompagnateurs] = useState("");
  const [draftNomsAccompagnateurs, setDraftNomsAccompagnateurs] = useState("");
  const [draftAccompagnateurs, setDraftAccompagnateurs] = useState<TravelsAccompagnateur[]>([]);
  const [showBudgetModal, setShowBudgetModal] = useState(false);
  const comptaTabAutoOpened = useRef<string | null>(null);
  const tripStatusRef = useRef(trip?.status);
  tripStatusRef.current = trip?.status;
  const [draftCoutTotal, setDraftCoutTotal] = useState("");
  const [cuisineModalStandalone, setCuisineModalStandalone] = useState(false);
  const [draftCuisineDetails, setDraftCuisineDetails] = useState<ReturnType<typeof emptyCuisineDetails> | null>(null);
  const [cuisineFollowUp, setCuisineFollowUp] = useState<{
    mode: "initial" | "amendment";
    savedTrip: TravelsTrip;
  } | null>(null);
  const [showDateModal, setShowDateModal] = useState(false);
  const [draftStartDate, setDraftStartDate] = useState("");
  const [draftEndDate, setDraftEndDate] = useState("");
  const [draftStartTime, setDraftStartTime] = useState("");
  const [draftEndTime, setDraftEndTime] = useState("");
  const [dateFollowUp, setDateFollowUp] = useState<{
    sendTransport: boolean;
    sendCuisine: boolean;
    savedTrip: TravelsTrip;
  } | null>(null);
  const [draftMessage, setDraftMessage] = useState("");
  const [transportReplyTo, setTransportReplyTo] = useState("");
  const [transportReplyBody, setTransportReplyBody] = useState("");
  const [transportReplyBusy, setTransportReplyBusy] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [manualDevisName, setManualDevisName] = useState("");
  const [manualDevisEmail, setManualDevisEmail] = useState("");
  const [manualDevisBusy, setManualDevisBusy] = useState(false);
  const [zeendocSendingUrl, setZeendocSendingUrl] = useState<string | null>(null);
  const [reopenStep, setReopenStep] = useState("");
  const perms = useTravelsPermissions(trip);
  const {
    isOwner,
    isDirection,
    canSign,
    isCompta,
    canAccessComptaTab,
    canSeeTravelDocHoverActions,
    canManageFiles,
    canAddDocuments,
    canUseInternalThread,
    canEditEffectif,
    isAdministratif,
    canReassignTripOwner,
    isGlobalAdmin,
  } = perms;
  useEffect(() => {
    const withBus = complexNeedsBus(trip);
    const hasCuisine = Boolean(trip.data?.piqueNiqueDetails?.active);
    const hasEleves = (trip.data?.participantEleves?.length || 0) > 0;
    const allowed = TRAVELS_HUB_TABS.filter((t) => {
      if (t.id === "transport" && !withBus) return false;
      if (t.id === "cuisine" && !hasCuisine) return false;
      if (t.id === "communication" && !hasEleves) return false;
      if (t.id === "compta" && !canAccessComptaTab) return false;
      return true;
    }).map((t) => t.id);
    if (!allowed.includes(hubTab)) setHubTab("overview");
  }, [trip, hubTab, canAccessComptaTab]);

  useEffect(() => {
    const t = tabFromUrl as TravelsHubTab | null;
    if (t && TRAVELS_HUB_TABS.some((x) => x.id === t)) setHubTab(t);
  }, [tabFromUrl]);

  useEffect(() => {
    if (!isCompta) return;
    if (trip.status === "EN_ATTENTE_COMPTA" && comptaTabAutoOpened.current !== trip.id) {
      comptaTabAutoOpened.current = trip.id;
      setHubTab("compta");
    }
  }, [trip, isCompta]);

  useEffect(() => {
    if (!remindersFocus) return;
    const t = window.setTimeout(() => {
      document.getElementById("trip-reminders")?.scrollIntoView({ behavior: "smooth", block: "start" });
    }, 400);
    return () => window.clearTimeout(t);
  }, [trip, remindersFocus]);

  const openSecureFile = async (fileUrl: string, s3Key?: string | null) => {
    const newWindow = window.open("", "_blank");
    try {
      const res = await fetch("/api/travels/download", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fileUrl, s3Key: s3Key || undefined }),
      });
      const data = await res.json();
      if (!res.ok || !data.signedUrl) {
        throw new Error(data?.error || "Impossible d'ouvrir le document.");
      }
      if (newWindow) {
        newWindow.location.href = data.signedUrl;
        newWindow.focus();
      } else {
        window.location.href = data.signedUrl;
      }
    } catch (err) {
      console.error(err);
      if (newWindow) newWindow.close();
      alert(err instanceof Error ? err.message : "Erreur lors de l'ouverture du fichier.");
    }
  };
  const prepareSendToZeendoc = async (file: { name?: string; url?: string }) => {
    if (!file?.url) {
      alert("Document invalide : URL manquante.");
      return;
    }
    try {
      setZeendocSendingUrl(file.url);
      const res = await fetch("/api/travels/send-zeendoc", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fileUrl: file.url, fileName: file.name || "document" }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data?.error || "Échec de l'envoi Zeendoc.");
      alert("Document envoyé sur Zeendoc.");
    } catch (err: any) {
      console.error("[travels] send-zeendoc:", err);
      alert(err?.message || "Impossible d'envoyer le document sur Zeendoc.");
    } finally {
      setZeendocSendingUrl(null);
    }
  };
  const saveUpdates = async (updatedTrip: any): Promise<boolean> => {
    try {
      const res = await fetch('/api/travels/update', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id: trip.id, data: updatedTrip })
      });
      if (res.ok) {
        setTrip(updatedTrip);
        setEditedData(updatedTrip.data);
        return true;
      }
      return false;
    } catch (err) {
      console.error("Erreur sauvegarde:", err);
      return false;
    }
  };
  const CIRCULAR_ATTACHMENT_NAME = "📄 Circulaire Parents";
  const isCircularAttachment = (file: { name?: string }) =>
    String(file.name || "").toLowerCase().includes("circulaire");
  const generateCircularAttachment = async (): Promise<{ name: string; url: string; s3Key?: string }> => {
    const circRes = await fetch("/api/travels/generate-circular", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tripData: trip }),
    });
    if (!circRes.ok) {
      const errPayload = await circRes.json().catch(() => ({}));
      throw new Error(errPayload?.error || "La circulaire n'a pas pu être générée.");
    }
    const { pdf } = await circRes.json();
    if (!pdf) throw new Error("La circulaire n'a pas été produite.");
    const safeTitle = String(trip.data.title || "Sortie").replace(/\s+/g, "_");
    const fileName = `Circulaire_${safeTitle}.pdf`;
    const base64Content = pdf.split(",")[1];
    const byteArray = new Uint8Array(atob(base64Content).split("").map((c) => c.charCodeAt(0)));
    const { fileUrl, s3Key: uploadedKey } = await uploadTravelDocument(
      new Blob([byteArray], { type: "application/pdf" }),
      fileName,
    );
    return { name: CIRCULAR_ATTACHMENT_NAME, url: fileUrl, s3Key: uploadedKey };
  };
  const mergeCircularIntoAttachments = (
    attachments: { name: string; url: string }[],
    circular: { name: string; url: string },
  ) => [...attachments.filter((f) => !isCircularAttachment(f)), circular];
  const handleRegenerateCircular = async () => {
    if (!canSign && !isOwner) return alert("Vous n'êtes pas autorisé(e) à régénérer la circulaire.");
    const hasExisting = (trip.data.attachments || []).some(isCircularAttachment);
    if (
      hasExisting &&
      !confirm("Une circulaire existe déjà dans les documents. La remplacer par une nouvelle version ?")
    ) {
      return;
    }
    setLoadingAction("regenerate-circular");
    try {
      const circular = await generateCircularAttachment();
      const updatedAttachments = mergeCircularIntoAttachments(trip.data.attachments || [], circular);
      const updatedTrip = {
        ...trip,
        data: { ...trip.data, attachments: updatedAttachments },
        history: [
          ...(trip.history || []),
          {
            date: new Date().toISOString(),
            user: user?.fullName ?? undefined,
            action: "CIRCULAIRE_REGENEREE",
            note: hasExisting ? "Circulaire remplacée." : "Circulaire régénérée.",
          },
        ],
      };
      const saved = await saveUpdates(updatedTrip);
      if (!saved) throw new Error("Impossible d'enregistrer la circulaire dans le dossier.");
      alert("Circulaire régénérée et ajoutée aux documents.");
    } catch (err) {
      console.error(err);
      alert(err instanceof Error ? err.message : "Erreur lors de la régénération de la circulaire.");
    } finally {
      setLoadingAction(null);
    }
  };
  const handleFinalValidation = async () => {
    if (!canSign) return alert("Vous n'êtes pas autorisé(e) à valider ce dossier.");
    setLoadingAction("final-validation");
    try {
      let tripBase = trip;
      let cuisineSent = false;
      const finalAttachments = [...(trip.data.attachments || [])];

      if (trip.data.piqueNiqueDetails?.active) {
        const cuisineRes = await fetch('/api/travels/send-cuisine', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            tripId: trip.id,
            userEmail: user?.primaryEmailAddress?.emailAddress,
            organizerEmail: trip.ownerEmail,
            userName: trip.ownerName,
            mode: 'initial',
          }),
        });
        if (cuisineRes.ok) {
          cuisineSent = true;
          const cuisinePayload = await cuisineRes.json().catch(() => ({}));
          if (cuisinePayload.trip) {
            tripBase = cuisinePayload.trip;
            setTrip(cuisinePayload.trip);
          }
        } else {
          const errPayload = await cuisineRes.json().catch(() => ({}));
          alert(`Attention : le mail cuisine n'a pas pu être envoyé (${errPayload?.error || "erreur inconnue"}).`);
        }
      }

      const historyNote = [
        "Dossier validé.",
        tripBase.data.piqueNiqueDetails?.active
          ? cuisineSent
            ? "Commande cuisine envoyée."
            : "Commande cuisine non envoyée."
          : null,
      ]
        .filter(Boolean)
        .join(" ");

      await handleAction("VALIDE", historyNote, { attachments: finalAttachments }, tripBase);

      const alertParts = ["Dossier validé !"];
      alertParts.push(
        "La circulaire n'est plus générée automatiquement — utilisez « Régénérer circulaire » dans Documents si besoin.",
      );
      if (tripBase.data.piqueNiqueDetails?.active && cuisineSent) {
        alertParts.push("Le bon de commande cuisine a été envoyé (chef + copies direction et organisateur).");
      }
      alert(alertParts.join("\n\n"));
    } catch (err) {
      console.error(err);
      alert(err instanceof Error ? err.message : "Erreur lors de la validation finale.");
    } finally {
      setLoadingAction(null);
    }
  };
  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!canAddDocuments) return;
    const file = e.target.files?.[0];
    if (!file) return;
    setUploading(true);
    try {
      const { fileUrl, s3Key: uploadedKey } = await uploadTravelDocument(file, file.name);
      const newAttachment = { name: file.name, url: fileUrl, s3Key: uploadedKey };
      const currentAttachments = isEditing ? (editedData.attachments || []) : (trip.data.attachments || []);
      const updatedAttachments = [...currentAttachments, newAttachment];
      if (isEditing) { setEditedData((prev: any) => ({ ...prev, attachments: updatedAttachments }));
      } else {
        const updatedTrip = { ...trip, data: { ...trip.data, attachments: updatedAttachments } };
        await saveUpdates(updatedTrip);
      }
    } catch (error) {
      console.error(error);
      alert("Erreur lors de l'envoi du fichier.");
    } finally {
      setUploading(false);
    }
  };
  const removeFile = async (index: number) => {
    if (!canManageFiles) return;
    const currentAttachments = isEditing ? (editedData.attachments || []) : (trip.data.attachments || []);
    const updatedFiles = currentAttachments.filter((_: any, i: number) => i !== index);
    if (isEditing) {
      setEditedData({ ...editedData, attachments: updatedFiles });
    } else {
      const updatedTrip = { ...trip, data: { ...trip.data, attachments: updatedFiles } };
      await saveUpdates(updatedTrip);
    }
  };
  const postInternalMessage = async () => {
    const text = draftMessage.trim();
    if (!text || !canUseInternalThread) return;
    const roleLabel = isDirection ? "Direction" : isCompta ? "Comptabilité" : "Créateur";
    const newMsg = {
      id: `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      user: user?.fullName || "Utilisateur",
      role: roleLabel,
      text,
      date: new Date().toISOString(),
    };
    const updatedTrip = {
      ...trip,
      messages: [...(trip.messages || []), newMsg],
    };
    await saveUpdates(updatedTrip);
    setDraftMessage("");
  };
  const handleAction = async (
    newStatus: string,
    note: string = "",
    extraData: any = null,
    baseTrip: TravelsTrip | null = null,
  ) => {
    if (!trip) return;
    if (!loadingAction) setLoadingAction("action");
    const source = baseTrip || trip;
    const baseData = isEditing ? { ...source.data, ...editedData } : source.data;
    const finalData = { ...(extraData ? { ...baseData, ...extraData } : baseData) };
    if (newStatus === "BESOIN_MODIFICATION" && note.trim()) {
      finalData.modificationRequestNote = note.trim();
    }
    if (source.status === "BESOIN_MODIFICATION" && newStatus !== "BESOIN_MODIFICATION") {
      delete finalData.modificationRequestNote;
      delete finalData.previousStatus;
    }
    const updatedTrip = {
      ...source,
      status: newStatus,
      data: finalData,
      history: [
        ...(source.history || []),
        { date: new Date().toISOString(), user: user?.fullName ?? undefined, action: newStatus, note: note },
      ],
    };
    const saved = await saveUpdates(updatedTrip);
    setIsEditing(false);
    setLoadingAction(null);
    if (!saved) alert("Impossible d'enregistrer la modification. Réessayez.");
  };

  const skipTransportToCompta = async () => {
    if (!canSign || !complexNeedsBus(trip)) return;
    if (trip.status !== "PROF_LOGISTICS" && trip.status !== "EN_ATTENTE_BUS_SIGNATURE") return;
    const note =
      prompt(
        "Motif du passage aux finances sans devis transport signé (obligatoire) :",
      )?.trim() ?? "";
    if (!note) return;
    if (
      !confirm(
        "Passer ce dossier à l'étape Finances (comptabilité) sans devis bus signé ?\n\nLa comptabilité pourra saisir le montant transport manuellement si besoin.",
      )
    ) {
      return;
    }
    await handleAction("EN_ATTENTE_COMPTA", `Passage aux finances sans devis signé — ${note}`, {
      pendingAmendedQuote: false,
      transportPhaseBypassedAt: new Date().toISOString(),
      transportPhaseBypassedBy: user?.fullName || "Direction",
      transportPhaseBypassNote: note,
    });
  };

  const onComptaSheetSaved = useCallback((sheet: TravelsComptaSheet) => {
    setTrip((prev) =>
      prev
        ? {
            ...prev,
            data: {
              ...prev.data,
              comptaSheet: sheet,
            },
          }
        : prev,
    );
  }, []);

  const onComptaValidateBudget = useCallback(
    async (sheet: TravelsComptaSheet) => {
      if (tripStatusRef.current !== "EN_ATTENTE_COMPTA") return;
      const finalSheet = computeComptaSheetDerived(sheet);
      const total = finalSheet.depensesTotal;
      const perStudent = comptaDefinitiveCostPerStudent(finalSheet);
      if (total == null || !finalSheet.nbEleves) {
        alert("Complétez le total des dépenses et le nombre d'élèves avant de valider.");
        return;
      }
      if (
        !confirm(
          `Valider le budget définitif et transmettre à la direction ?\n\n` +
            `Total dépenses : ${total} €\n` +
            `Prix par élève : ${perStudent != null ? `${perStudent} €` : "—"}`,
        )
      ) {
        return;
      }
      await handleAction("EN_ATTENTE_DIR_FINAL", "Budget validé par la comptabilité", {
        comptaSheet: {
          ...finalSheet,
          budgetValidatedAt: new Date().toISOString(),
        },
        finalTotalCost: total,
        costPerStudent: perStudent ?? "",
      });
    },
    // handleAction volontairement hors deps.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const handleCancelModificationRequest = async () => {
    if (trip.status !== "BESOIN_MODIFICATION" || loadingAction) return;
    if (!canSign && !isCompta) return;
    const restoreStatus = trip.data?.previousStatus || "EN_ATTENTE_DIR_INITIAL";
    const stepLabel = TRAVELS_STATUS_LABELS[restoreStatus] || restoreStatus;
    if (
      !confirm(
        `Annuler la demande de modification ?\n\nLe dossier reprendra à l'étape « ${stepLabel} » sans attendre de modification du créateur.`,
      )
    ) {
      return;
    }
    await handleAction(restoreStatus, "Demande de modification annulée");
  };
  const handleReopenDossier = async (targetStatus: string, stepLabel: string) => {
    if (!canSign || trip.status !== "VALIDE" || loadingAction) return;
    if (
      !confirm(
        `Réouvrir ce dossier à l'étape « ${stepLabel} » ?\n\nLe statut ne sera plus « Finalisé » et le circuit de validation reprendra à cette étape.`,
      )
    ) {
      return;
    }
    const note = prompt("Motif de réouverture (optionnel) :") ?? "";
    await handleAction(
      targetStatus,
      note.trim() ? `Dossier réouvert : ${note.trim()}` : `Dossier réouvert à l'étape « ${stepLabel} »`,
    );
  };
  const selectBusQuote = async (quote: any) => {
    if (!isOwner || canSign) return;
    if (trip.status !== "PROF_LOGISTICS") return;
    if (!confirm(`Confirmer le choix de ${quote.providerName} ? La direction devra ensuite signer la commande.`)) return;
    const updatedTrip = { ...trip, status: "EN_ATTENTE_BUS_SIGNATURE", data: { ...trip.data, selectedBusQuote: quote } };
    await saveUpdates(updatedTrip);
  };

  const selectAndSignBusQuote = async (quote: any) => {
    if (!canSign) return alert("Seule la direction de l'établissement concerné peut signer un devis.");
    if (trip.status !== "PROF_LOGISTICS" && trip.status !== "EN_ATTENTE_BUS_SIGNATURE") return;
    if (!confirm(`Choisir le devis de ${quote.providerName} et signer la commande au transporteur ?`)) return;
    await signBusQuote(quote);
  };

  const deleteBusQuote = async (quote: { id?: string; providerName?: string }) => {
    if (!canSign) return alert("Seule la direction de l'établissement concerné peut supprimer un devis.");
    const label = quote.providerName || "ce transporteur";
    if (!confirm(`Êtes-vous bien sûr de supprimer le devis de ${label} ?`)) return;
    if (!quote.id) return alert("Identifiant du devis manquant.");
    setLoadingAction(`delete-quote-${quote.id}`);
    try {
      const res = await fetch("/api/travels/delete-bus-quote", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId: trip.id, quoteId: quote.id }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Suppression impossible.");
      setTrip(payload.trip);
      setEditedData(payload.trip?.data);
    } catch (err) {
      console.error(err);
      alert(err instanceof Error ? err.message : "Erreur lors de la suppression du devis.");
    } finally {
      setLoadingAction(null);
    }
  };

  const requestAmendedBusQuote = async (opts?: { skipConfirm?: boolean; tripRef?: any }) => {
    if (!isOwner && !canSign) return alert("Vous n'êtes pas autorisé(e) à relancer une demande de devis.");
    const tripRef = opts?.tripRef || trip;
    if (isEditing && !opts?.skipConfirm) {
      return alert("Enregistrez d'abord les modifications d'effectif avant d'envoyer l'avenant.");
    }
    const snap = tripRef.data?.transportQuoteSnapshot;
    const nbEleves = Number(tripRef.data?.nbEleves) || 0;
    const nbAcc = Number(tripRef.data?.nbAccompagnateurs) || 0;
    const selected = tripRef.data?.selectedBusQuote;
    const signed = Boolean(tripRef.data?.signedQuoteUrl);
    const targetLabel =
      selected?.providerName && orderEmailForQuote(selected)
        ? `${selected.providerName} (${orderEmailForQuote(selected)})`
        : "tous les transporteurs";

    if (!opts?.skipConfirm) {
      let msg =
        `Envoyer une demande de devis rectifié (avenant effectif) à ${targetLabel} ?\n\n` +
        `Effectif actuel : ${nbEleves + nbAcc} personnes (${nbEleves} élèves, ${nbAcc} accomp.).`;
      if (snap) {
        const prev = Number(snap.nbEleves) + Number(snap.nbAccompagnateurs || 0);
        msg += `\nDernier devis demandé pour : ${prev} personnes.`;
      }
      if (signed) {
        msg += "\n\nLa commande avait déjà été signée : le transporteur recevra un avenant pour réviser son devis.";
      }
      if (!confirm(msg)) return;
    }

    setLoadingAction("amendment-quote");
    try {
      const res = await fetch("/api/travels/send-transport-amendment", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: tripRef.id,
          userName: user?.fullName || "La Providence",
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Envoi impossible.");
      setTrip(payload.trip);
      setEditedData(payload.trip?.data);
      if (!opts?.skipConfirm) {
        alert(
          payload.singleProvider
            ? "Avenant envoyé au transporteur sélectionné."
            : "Avenant envoyé à tous les transporteurs.",
        );
      }
      return payload;
    } catch (err) {
      console.error(err);
      if (!opts?.skipConfirm) {
        alert(err instanceof Error ? err.message : "Erreur lors de l'envoi de l'avenant.");
      }
      throw err;
    } finally {
      setLoadingAction(null);
    }
  };

  const sendInitialCuisine = async (opts?: { skipConfirm?: boolean; tripRef?: TravelsTrip }) => {
    const tripRef = opts?.tripRef || trip;
    if (!tripRef?.data?.piqueNiqueDetails?.active) {
      return alert("Aucune commande cuisine active sur ce dossier.");
    }
    if (tripRef.data?.cuisineOrderSentAt) {
      return sendCuisineAmendment(opts);
    }
    if (!opts?.skipConfirm) {
      const ok = confirm("Envoyer la commande cuisine au chef ?\n\nUn PDF sera joint au mail (chef + copies direction et organisateur).");
      if (!ok) return;
    }
    setLoadingAction("cuisine-initial");
    try {
      const res = await fetch("/api/travels/send-cuisine", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: tripRef.id,
          mode: "initial",
          userEmail: user?.primaryEmailAddress?.emailAddress,
          organizerEmail: tripRef.ownerEmail,
          userName: user?.fullName || tripRef.ownerName,
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Envoi impossible.");
      if (payload.trip) {
        setTrip(payload.trip);
        setEditedData(payload.trip.data);
      }
      if (!opts?.skipConfirm) alert("Commande cuisine envoyée au chef.");
      return payload;
    } catch (err) {
      console.error(err);
      if (!opts?.skipConfirm) {
        alert(err instanceof Error ? err.message : "Erreur lors de l'envoi cuisine.");
      }
      throw err;
    } finally {
      setLoadingAction(null);
    }
  };

  const sendCuisineAmendment = async (opts?: { skipConfirm?: boolean; tripRef?: any }) => {
    const tripRef = opts?.tripRef || trip;
    if (!tripRef.data?.piqueNiqueDetails?.active) {
      return alert("Aucune commande cuisine active sur ce dossier.");
    }
    if (!tripRef.data?.cuisineOrderSentAt) {
      return alert("Aucune commande cuisine n'a encore été envoyée au chef.");
    }
    if (!opts?.skipConfirm) {
      const ok = confirm(
        "Renvoyer la commande cuisine au chef (annule et remplace) ?\n\nLe mail précisera qu'il s'agit de la dernière commande en date.",
      );
      if (!ok) return;
    }
    setLoadingAction("cuisine-amendment");
    try {
      const res = await fetch("/api/travels/send-cuisine", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: tripRef.id,
          mode: "amendment",
          userEmail: user?.primaryEmailAddress?.emailAddress,
          organizerEmail: tripRef.ownerEmail,
          userName: user?.fullName || tripRef.ownerName,
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Envoi impossible.");
      if (payload.trip) {
        setTrip(payload.trip);
        setEditedData(payload.trip.data);
      }
      if (!opts?.skipConfirm) alert("Commande cuisine renvoyée au chef (annule et remplace).");
      return payload;
    } catch (err) {
      console.error(err);
      if (!opts?.skipConfirm) {
        alert(err instanceof Error ? err.message : "Erreur lors du renvoi cuisine.");
      }
      throw err;
    } finally {
      setLoadingAction(null);
    }
  };

  const openEffectifModal = () => {
    setDraftNbEleves(String(trip.data?.nbEleves ?? ""));
    const escorts = formFieldsToAccompagnateurs({
      nomsAccompagnateurs: trip.data?.nomsAccompagnateurs,
      accompagnateurs: trip.data?.accompagnateurs,
    });
    setDraftAccompagnateurs(escorts);
    setDraftNomsAccompagnateurs(escorts.map((a) => a.name).join(", "));
    setDraftNbAccompagnateurs(String(escorts.length || trip.data?.nbAccompagnateurs || 0));
    setShowEffectifModal(true);
  };

  const openBudgetModal = () => {
    setDraftCoutTotal(String(trip.data?.coutTotal ?? ""));
    setShowBudgetModal(true);
  };

  const cloneCuisineDetails = (src: TravelsTrip["data"]["piqueNiqueDetails"] | undefined) => {
    if (!src) return emptyCuisineDetails();
    return JSON.parse(JSON.stringify({ ...emptyCuisineDetails(), ...src })) as ReturnType<typeof emptyCuisineDetails>;
  };

  const openCuisineModalForOwner = () => {
    setDraftCuisineDetails(cloneCuisineDetails(trip.data?.piqueNiqueDetails));
    setCuisineModalStandalone(true);
    setShowCuisineModal(true);
  };

  const openCuisineModalFromEdit = () => {
    setCuisineModalStandalone(false);
    setShowCuisineModal(true);
  };

  const activeCuisineDetails = cuisineModalStandalone
    ? draftCuisineDetails
    : editedData?.piqueNiqueDetails;

  const patchCuisineDetails = (
    updater: (prev: ReturnType<typeof emptyCuisineDetails>) => ReturnType<typeof emptyCuisineDetails>,
  ) => {
    if (cuisineModalStandalone) {
      setDraftCuisineDetails((prev) => updater(prev || emptyCuisineDetails()));
    } else {
      setEditedData((prev: any) => ({
        ...prev,
        piqueNiqueDetails: updater({ ...(prev.piqueNiqueDetails || emptyCuisineDetails()) }),
      }));
    }
  };

  const tripAllowsCuisineSend = (t: TravelsTrip) =>
    !["BROUILLON", "EN_ATTENTE_VALIDATION", "REJETE", "ANNULE", "SEANCE_ANNULEE"].includes(t.status);

  const saveCuisineFromOwnerModal = async () => {
    const details = draftCuisineDetails;
    if (!details?.active) {
      return alert("Cochez « Commander une restauration » ou fermez sans enregistrer.");
    }
    const hasDay = Object.values(details.daysSelection || {}).some(Boolean);
    if (!hasDay) return alert("Sélectionnez au moins un jour de sortie.");

    const wasActive = Boolean(trip.data?.piqueNiqueDetails?.active);
    const updatedTrip: TravelsTrip = {
      ...trip,
      data: { ...trip.data, piqueNiqueDetails: details },
      history: [
        ...(trip.history || []),
        {
          date: new Date().toISOString(),
          user: user?.fullName ?? undefined,
          action: "CUISINE_MODIFIEE",
          note: wasActive ? "Commande cuisine modifiée." : "Commande cuisine ajoutée au dossier.",
        },
      ],
    };
    const saved = await saveUpdates(updatedTrip);
    if (!saved) return alert("Impossible d'enregistrer la commande cuisine.");

    setShowCuisineModal(false);
    setCuisineModalStandalone(false);
    setDraftCuisineDetails(null);

    if (tripAllowsCuisineSend(updatedTrip)) {
      setCuisineFollowUp({
        mode: updatedTrip.data.cuisineOrderSentAt ? "amendment" : "initial",
        savedTrip: updatedTrip,
      });
    } else {
      alert("Commande enregistrée — elle sera envoyée au chef lors de la validation du dossier.");
    }
  };

  const runCuisineFollowUp = async () => {
    if (!cuisineFollowUp) return;
    const { mode, savedTrip } = cuisineFollowUp;
    setCuisineFollowUp(null);
    try {
      if (mode === "initial") {
        await sendInitialCuisine({ skipConfirm: true, tripRef: savedTrip });
        alert("Commande cuisine enregistrée et envoyée au chef.");
      } else {
        await sendCuisineAmendment({ skipConfirm: true, tripRef: savedTrip });
        alert("Commande cuisine enregistrée et renvoyée au chef (annule et remplace).");
      }
    } catch {
      alert("Commande enregistrée, mais l'envoi au chef a échoué — réessayez depuis l'onglet Restauration.");
    }
  };

  const saveBudgetChange = async () => {
    const coutTotal = Number(draftCoutTotal);
    if (!Number.isFinite(coutTotal) || coutTotal < 0) {
      return alert("Indiquez un budget prévisionnel valide.");
    }
    const prev = Number(trip.data?.coutTotal) || 0;
    if (coutTotal === prev) {
      setShowBudgetModal(false);
      return alert("Aucun changement de budget.");
    }
    const updatedTrip: TravelsTrip = {
      ...trip,
      data: { ...trip.data, coutTotal },
      history: [
        ...(trip.history || []),
        {
          date: new Date().toISOString(),
          user: user?.fullName ?? undefined,
          action: "BUDGET_MODIFIE",
          note: `Budget prévisionnel : ${Math.round(prev)} € → ${Math.round(coutTotal)} €`,
        },
      ],
    };
    const saved = await saveUpdates(updatedTrip);
    if (!saved) return alert("Impossible d'enregistrer le budget.");
    setShowBudgetModal(false);
    alert("Budget prévisionnel enregistré.");
  };

  const saveEffectifChange = async () => {
    const nbEleves = Number(draftNbEleves);
    const nbAcc = draftAccompagnateurs.length;
    const nomsAccompagnateurs = draftNomsAccompagnateurs.trim() || draftAccompagnateurs.map((a) => a.name).join(", ");
    if (!Number.isFinite(nbEleves) || nbEleves < 0) {
      return alert("Indiquez un nombre d’élèves valide.");
    }
    const prevEleves = Number(trip.data?.nbEleves) || 0;
    const prevAcc = Number(trip.data?.nbAccompagnateurs) || 0;
    const prevNoms = String(trip.data?.nomsAccompagnateurs || "").trim();
    if (nbEleves === prevEleves && nbAcc === prevAcc && nomsAccompagnateurs === prevNoms) {
      setShowEffectifModal(false);
      return alert("Aucun changement.");
    }

    const updatedTrip: TravelsTrip = {
      ...trip,
      data: {
        ...trip.data,
        nbEleves,
        nbAccompagnateurs: nbAcc,
        nomsAccompagnateurs,
        accompagnateurs: draftAccompagnateurs,
      },
      history: [
        ...(trip.history || []),
        {
          date: new Date().toISOString(),
          user: user?.fullName ?? undefined,
          action: "EFFECTIF_MODIFIE",
          note: `Effectif : ${prevEleves}+${prevAcc} → ${nbEleves}+${nbAcc} (élèves + accomp.)${nomsAccompagnateurs !== prevNoms ? " · noms accomp. mis à jour" : ""}`,
        },
      ],
    };
    const saved = await saveUpdates(updatedTrip);
    if (!saved) return alert("Impossible d'enregistrer l'effectif.");

    setShowEffectifModal(false);

    const busActive = busLogisticsActive(updatedTrip);
    const cuisineActive =
      Boolean(updatedTrip.data?.piqueNiqueDetails?.active) && Boolean(updatedTrip.data?.cuisineOrderSentAt);

    if (busActive || cuisineActive) {
      setEffectifFollowUp({
        sendTransport: busActive,
        sendCuisine: cuisineActive,
        savedTrip: updatedTrip,
      });
    } else {
      alert("Effectif enregistré.");
    }
  };

  const runEffectifFollowUp = async () => {
    if (!effectifFollowUp) return;
    const { sendTransport, sendCuisine, savedTrip } = effectifFollowUp;
    setEffectifFollowUp(null);
    const results: string[] = [];
    const errors: string[] = [];

    if (sendTransport) {
      try {
        const payload = await requestAmendedBusQuote({ skipConfirm: true, tripRef: savedTrip });
        results.push(
          payload?.singleProvider
            ? "Avenant transport envoyé au transporteur sélectionné."
            : "Avenant transport envoyé à tous les transporteurs.",
        );
      } catch {
        errors.push("Échec de l'envoi de l'avenant transport.");
      }
    }
    if (sendCuisine) {
      try {
        await sendCuisineAmendment({ skipConfirm: true, tripRef: savedTrip });
        results.push("Commande cuisine renvoyée au chef (annule et remplace).");
      } catch {
        errors.push("Échec du renvoi de la commande cuisine.");
      }
    }

    const msg = ["Effectif enregistré.", ...results, ...errors].filter(Boolean).join("\n\n");
    alert(msg);
  };

  const openDateModal = () => {
    setDraftStartDate(String(trip.data?.startDate || trip.data?.date || ""));
    setDraftEndDate(String(trip.data?.endDate || ""));
    setDraftStartTime(String(trip.data?.startTime || ""));
    setDraftEndTime(String(trip.data?.endTime || ""));
    setShowDateModal(true);
  };

  const saveDateChange = async () => {
    const updatedTrip: TravelsTrip = {
      ...trip,
      data: {
        ...trip.data,
        startDate: draftStartDate,
        endDate: trip.type === "COMPLEX" ? draftEndDate : draftStartDate,
        date: draftStartDate,
        startTime: draftStartTime,
        endTime: draftEndTime,
      },
      history: [
        ...(trip.history || []),
        {
          date: new Date().toISOString(),
          user: user?.fullName ?? undefined,
          action: "DATES_MODIFIEES",
          note: `Horaires/dates mis à jour`,
        },
      ],
    };
    const saved = await saveUpdates(updatedTrip);
    if (!saved) return alert("Impossible d'enregistrer les dates.");
    setShowDateModal(false);

    const busActive = busLogisticsActive(updatedTrip);
    const cuisineActive =
      Boolean(updatedTrip.data?.piqueNiqueDetails?.active) && Boolean(updatedTrip.data?.cuisineOrderSentAt);
    if (busActive || cuisineActive) {
      setDateFollowUp({ sendTransport: busActive, sendCuisine: cuisineActive, savedTrip: updatedTrip });
    } else {
      alert("Dates enregistrées.");
    }
  };

  const runDateFollowUp = async () => {
    if (!dateFollowUp) return;
    const { sendTransport, sendCuisine, savedTrip } = dateFollowUp;
    setDateFollowUp(null);
    const results: string[] = [];
    if (sendTransport) {
      try {
        await requestAmendedBusQuote({ skipConfirm: true, tripRef: savedTrip });
        results.push("Relance transport envoyée (dates modifiées).");
      } catch {
        results.push("Échec relance transport.");
      }
    }
    if (sendCuisine) {
      try {
        await sendCuisineAmendment({ skipConfirm: true, tripRef: savedTrip });
        results.push("Commande cuisine renvoyée.");
      } catch {
        results.push("Échec relance cuisine.");
      }
    }
    alert(["Dates enregistrées.", ...results].join("\n\n"));
  };

  const addManualBusQuote = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!canAddDocuments) return;
    const name = manualDevisName.trim() || "Transporteur (saisie manuelle)";
    const email = manualDevisEmail.trim();
    if (!isValidEmailLoose(email)) {
      alert("Indiquez une adresse e-mail du transporteur valide (pour l’envoi de la commande après signature).");
      return;
    }
    const form = e.currentTarget;
    const fileInput = form.querySelector<HTMLInputElement>('input[name="manualDevisPdf"]');
    const file = fileInput?.files?.[0];
    if (!file || file.type !== "application/pdf") {
      alert("Choisissez un fichier PDF (devis du transporteur).");
      return;
    }
    setManualDevisBusy(true);
    try {
      const { fileUrl, s3Key: uploadedKey } = await uploadTravelDocument(file, file.name);
      const newQuote = {
        id: `${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
        providerName: name,
        providerEmail: email,
        fileUrl,
        s3KeyIncoming: uploadedKey,
        createdAt: new Date().toISOString(),
        source: "manual",
        originalFilename: file.name,
        matchConfidence: "high",
        matchReviewRequired: false,
        gmailMessageId: `manual_${Date.now()}`,
        extractedContactEmail: null,
        extractedPrice: null,
        extractedCompany: null,
      };
      const updatedTrip = {
        ...trip,
        receivedDevis: [...(trip.receivedDevis || []), newQuote],
      };
      await saveUpdates(updatedTrip);
      setTrip(updatedTrip);
      setManualDevisName("");
      setManualDevisEmail("");
      if (fileInput) fileInput.value = "";
      alert("Devis ajouté. Vous pouvez le sélectionner à l’étape « Choix du devis transport » comme un devis reçu par mail.");
    } catch (err) {
      console.error(err);
      alert("Impossible d’ajouter le devis. Réessayez ou vérifiez votre connexion.");
    } finally {
      setManualDevisBusy(false);
    }
  };

  const signBusQuote = async (quoteOverride?: any) => {
    if (!canSign) return alert("Seule la direction de l'établissement concerné peut signer un devis.");
    const quote = quoteOverride ?? trip.data.selectedBusQuote;
    if (!quote) return alert("Aucun devis sélectionné.");
    if (!quote.fileUrl) {
      return alert(
        "Le PDF du devis est introuvable (fileUrl manquant). Supprimez le devis et réimportez-le, ou relancez le transporteur.",
      );
    }
    if (!confirm("Voulez-vous signer le devis et envoyer la commande au transporteur ?")) return;
    const transporteurEmail = orderEmailForQuote(quote);
    if (!transporteurEmail) {
      return alert(
        "Erreur : aucun e-mail pour envoyer la commande (ni adresse lue sur le devis, ni expéditeur du mail, ni e-mail transporteur enregistré)."
      );
    }
    setLoadingAction("signing");
    const etab = trip.data?.etablissement || "";
    const matchedEst = matchEstablishment(appCtx?.establishments || [], etab);
    const sigType = matchedEst?.id || "";
    if (!sigType) {
      setLoadingAction(null);
      return alert(
        `Impossible d’identifier l’établissement « ${etab || "?" } » pour apposer la signature. Vérifiez le champ établissement du dossier (Paramètres → Établissements).`,
      );
    }
    const legacySig = appCtx?.travelsOptions?.signatureImageUrls?.[sigType]?.trim();
    if (!matchedEst?.signatureS3Key?.trim() && !legacySig) {
      setLoadingAction(null);
      return alert(
        "Signature direction non configurée pour cet établissement. Paramètres → Établissements → téléverser la signature PNG/JPEG, puis réessayer.",
      );
    }
    try {
      const signRes = await fetch('/api/travels/sign-pdf', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ quoteUrl: quote.fileUrl, signatureType: sigType })
      });
      const signPayload = (await signRes.json()) as { error?: string; signedPdfData?: string };
      if (!signRes.ok || !signPayload.signedPdfData) {
        throw new Error(signPayload.error || `Signature impossible (${signRes.status}).`);
      }
      const { signedPdfData } = signPayload;
      const fileName = `Devis_Signe_${String(quote.providerName || "transport").replace(/\s+/g, '_')}.pdf`;
      const byteArray = new Uint8Array(atob(signedPdfData.split(',')[1]).split("").map(c => c.charCodeAt(0)));
      const { fileUrl, s3Key: uploadedKey } = await uploadTravelDocument(
        new Blob([byteArray], { type: "application/pdf" }),
        fileName,
      );
      const orderRes = await fetch('/api/travels/send-order', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          tripId: trip.id,
          tripTitle: trip.data.title,
          tripData: trip.data,
          providerEmail: transporteurEmail,
          signedQuoteUrl: fileUrl,
          signedQuoteS3Key: uploadedKey,
          providerName: quote.providerName,
        })
      });
      const orderPayload = (await orderRes.json()) as { error?: string; details?: string };
      if (!orderRes.ok) {
        throw new Error(
          orderPayload.error
            ? `${orderPayload.error}${orderPayload.details ? ` — ${orderPayload.details}` : ""}`
            : `Envoi commande impossible (${orderRes.status}).`,
        );
      }
      const newAttachment = { name: `✅ ${fileName}`, url: fileUrl, s3Key: uploadedKey };
      handleAction("EN_ATTENTE_COMPTA", `Devis signé et commande envoyée`, {
        selectedBusQuote: quote,
        attachments: [...(trip.data.attachments || []), newAttachment],
        signedQuoteUrl: fileUrl,
        signedQuoteS3Key: uploadedKey,
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Erreur lors de la signature.";
      alert(message);
    } finally {
      setLoadingAction(null);
    }
  };
  const formatSafeDate = (dateStr: any) => {
    if (!dateStr) return "N/C";
    const d = new Date(dateStr);
    return isNaN(d.getTime()) ? "Date à préciser" : d.toLocaleDateString('fr-FR');
  };
  const nextGuidance = useMemo(
    () => getTripNextGuidance(trip, { isOwner, canSign, isCompta }),
    [trip, isOwner, canSign, isCompta],
  );
  const seriesId = trip.data?.recurrenceSeriesId as string | undefined;
  const seriesIndex = trip.data?.recurrenceIndex as number | undefined;
  const seriesTotal = trip.data?.recurrenceTotal as number | undefined;
  const isRecurrenceTrip = Boolean(seriesId && seriesTotal);
  const canCancelRecurrenceSession =
    isRecurrenceTrip &&
    trip.status !== "SEANCE_ANNULEE" &&
    trip.status !== "VALIDE" &&
    trip.status !== "REJETE" &&
    (isOwner || canSign);
  const validateSeriesPedagogy = async () => {
    if (loadingAction) return;
    if (!seriesId || !canSign) return;
    if (!confirm("Valider la pédagogie pour tous les dossiers de cette série encore en attente direction ?")) return;
    setLoadingAction("action");
    try {
      const res = await fetch("/api/travels/series-action", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "validate_pedagogy_series",
          seriesId,
          actorName: user?.fullName,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      alert(`${j.updated} dossier(s) passé(s) en attente comptabilité.`);
      const refresh = await fetch(`/api/travels/get?id=${id}`);
      if (refresh.ok) setTrip(await refresh.json());
    } catch (e) {
      console.error(e);
      alert("Impossible de valider la série. Réessayez ou vérifiez les droits.");
    } finally {
      setLoadingAction(null);
    }
  };
  const cancelRecurrenceSession = async () => {
    if (loadingAction) return;
    if (!trip?.id) return;
    if (!confirm("Annuler uniquement cette séance ? Les autres dates de la série ne sont pas modifiées.")) return;
    setLoadingAction("action");
    try {
      const res = await fetch("/api/travels/series-action", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "cancel_session",
          tripId: trip.id,
          actorName: user?.fullName,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      const refresh = await fetch(`/api/travels/get?id=${id}`);
      if (refresh.ok) setTrip(await refresh.json());
    } catch (e) {
      console.error(e);
      alert("Impossible d’annuler cette séance.");
    } finally {
      setLoadingAction(null);
    }
  };
  const withBusLogistics = complexNeedsBus(trip);
  const etabForSign = trip.data?.etablissement || "";
  const transportSnapshot = trip.data?.transportQuoteSnapshot;
  const currentEffectifTotal =
    Number(trip.data?.nbEleves) + Number(trip.data?.nbAccompagnateurs || 0);
  const snapshotEffectifTotal = transportSnapshot
    ? Number(transportSnapshot.nbEleves) + Number(transportSnapshot.nbAccompagnateurs || 0)
    : null;
  const effectifChanged = effectifChangedSinceSnapshot(trip.data, transportSnapshot);
  const datesChanged = datesChangedSinceSnapshot(trip.data, trip.data.transportDateSnapshot);
  const canRequestAmendedQuote =
    withBusLogistics &&
    (isOwner || canSign) &&
    Boolean(transportSnapshot || trip.data?.selectedBusQuote || trip.data?.signedQuoteUrl);
  const cuisineOrderSent = cuisineOrderWasSent(trip);
  const cuisineOrderSentAt = resolveCuisineOrderSentAt(trip);
  const cuisineChanged = cuisineEffectifChanged(trip.data);
  const canEditDates = canEditEffectif;
  const hasCuisineOrder = Boolean(trip.data.piqueNiqueDetails?.active);
  const participantCount = trip.data.participantEleves?.length || 0;
  const visibleHubTabs = TRAVELS_HUB_TABS.filter((t) => {
    if (t.id === "transport" && !withBusLogistics) return false;
    if (t.id === "cuisine" && !hasCuisineOrder) return false;
    if (t.id === "communication" && participantCount === 0) return false;
    if (t.id === "compta" && !canAccessComptaTab) return false;
    return true;
  });
  const documentCount =
    (trip.data.attachments?.length || 0) +
    (withBusLogistics ? trip.receivedDevis?.length || 0 : 0) +
    (trip.data.signedQuoteUrl ? 1 : 0);
  const modificationRequestNote = getModificationRequestNote(trip);
  const hubBadges: Partial<Record<TravelsHubTab, number>> = {
    journal: (trip.data.transportAmendments?.length || 0) + (trip.data.cuisineAmendments?.length || 0),
    transport: withBusLogistics ? trip.receivedDevis?.length || 0 : undefined,
    documents: documentCount,
    eleves: participantCount || undefined,
    communication: trip.data.parentComLogs?.length || undefined,
  };
  const currentSteps =
    trip.type === "COMPLEX"
      ? withBusLogistics
        ? [
            { n: "1", label: "Pédagogie", key: "EN_ATTENTE_DIR_INITIAL" },
            { n: "2", label: "Choix du devis", key: "PROF_LOGISTICS" },
            { n: "3", label: "Finances", key: "EN_ATTENTE_COMPTA" },
            { n: "4", label: "Validation", key: "EN_ATTENTE_DIR_FINAL" },
            { n: "5", label: "Finalisé", key: "VALIDE" },
          ]
        : [
            { n: "1", label: "Pédagogie", key: "EN_ATTENTE_DIR_INITIAL" },
            { n: "2", label: "Finances", key: "EN_ATTENTE_COMPTA" },
            { n: "3", label: "Validation", key: "EN_ATTENTE_DIR_FINAL" },
            { n: "4", label: "Finalisé", key: "VALIDE" },
          ]
      : [
          { n: "1", label: "Pédagogie", key: "EN_ATTENTE_DIR_INITIAL" },
          { n: "2", label: "Finances", key: "EN_ATTENTE_COMPTA" },
          { n: "3", label: "Validation", key: "EN_ATTENTE_DIR_FINAL" },
          { n: "4", label: "Finalisé", key: "VALIDE" },
        ];
  const reopenStepOptions: { value: string; label: string }[] = [];
  for (const s of currentSteps) {
    if (s.key === "VALIDE") continue;
    reopenStepOptions.push({ value: s.key, label: s.label });
  }
  if (withBusLogistics && !reopenStepOptions.some((o) => o.value === "EN_ATTENTE_BUS_SIGNATURE")) {
    const logIdx = reopenStepOptions.findIndex((o) => o.value === "PROF_LOGISTICS");
    const busStep = { value: "EN_ATTENTE_BUS_SIGNATURE", label: "Signature devis bus" };
    if (logIdx >= 0) reopenStepOptions.splice(logIdx + 1, 0, busStep);
    else reopenStepOptions.push(busStep);
  }
  const selectedReopenStep =
    reopenStepOptions.find((o) => o.value === reopenStep)?.value || reopenStepOptions[0]?.value || "";
  const dateLabel =
    trip.type === "COMPLEX"
      ? `${formatSafeDate(trip.data.startDate)} → ${formatSafeDate(trip.data.endDate)}`
      : formatSafeDate(trip.data.date);
  const loadingOverlayMode =
    loadingAction === "circular" ? "circular" : loadingAction === "signing" ? "signing" : "default";

  return (
    <TripPageShell>
      {loadingAction && <TripLoadingOverlay mode={loadingOverlayMode as "circular" | "signing" | "default"} />}

      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <TripButton variant="ghost" size="sm" onClick={() => router.push("/travels")}>
          ← Retour aux voyages
        </TripButton>
        {isEditing && (
          <div className="flex gap-2">
            <TripButton variant="secondary" onClick={() => setIsEditing(false)}>
              Annuler
            </TripButton>
            <TripButton
              variant="success"
              onClick={() => handleAction(trip.data.previousStatus || "EN_ATTENTE_DIR_INITIAL", "Modifications effectuées")}
              disabled={uploading}
            >
              {uploading ? "Envoi…" : "Enregistrer et renvoyer"}
            </TripButton>
          </div>
        )}
      </div>

      {trip.status === "ANNULE" && (
        <TripAlert tone="warning" icon="🚫" title="Sortie annulée">
          {trip.data.cancelReason ? String(trip.data.cancelReason) : "Ce dossier a été annulé."}
        </TripAlert>
      )}

      {trip.status === "SEANCE_ANNULEE" && (
        <TripAlert tone="muted" icon="🚫" title="Séance annulée">
          Ce créneau ne fait plus partie du circuit de validation. Les autres dossiers de la série restent inchangés.
        </TripAlert>
      )}

      {trip.status === "BESOIN_MODIFICATION" && !isEditing && (
        <TripAlert
          tone="warning"
          icon="⚠️"
          title="Modifications demandées"
          action={
            isOwner || canSign || isCompta ? (
              <div className="flex flex-wrap gap-2 shrink-0">
                {isOwner && (
                  <TripButton variant="warning" onClick={() => setIsEditing(true)}>
                    Modifier mon dossier
                  </TripButton>
                )}
                {(canSign || isCompta) && (
                  <TripButton variant="secondary" onClick={handleCancelModificationRequest}>
                    Annuler la demande
                  </TripButton>
                )}
              </div>
            ) : undefined
          }
        >
          <span className="italic">
            &quot;{modificationRequestNote || "Merci d'ajuster le dossier selon les remarques de la direction."}&quot;
          </span>
        </TripAlert>
      )}

      <TripHeroHeader
        title={trip.data.title ?? ""}
        typeLabel={trip.type === "COMPLEX" ? "Voyage scolaire" : "Sortie de proximité"}
        ownerName={trip.ownerName ?? ""}
        etablissement={trip.data.etablissement}
        seriesLabel={
          isRecurrenceTrip && seriesIndex != null && seriesTotal != null
            ? `Série ${seriesIndex}/${seriesTotal}`
            : null
        }
        status={trip.status}
        statusPulse={trip.status === "BESOIN_MODIFICATION"}
      />

      {canReassignTripOwner && (
        <TravelsOwnerRepairSection
          trip={trip}
          onRepaired={(updated) => {
            setTrip(updated);
            setEditedData(updated.data);
          }}
        />
      )}

      <TripQuickStats
        items={[
          { label: "Destination", value: trip.data.destination || "—", icon: "📍" },
          { label: "Dates", value: dateLabel, icon: "📅" },
          {
            label: "Effectif",
            value: `${trip.data.nbEleves || 0} él. · ${trip.data.nbAccompagnateurs || 0} acc.`,
            icon: "👥",
            action: canEditEffectif ? (
              <button
                type="button"
                onClick={openEffectifModal}
                className="text-[10px] font-bold text-indigo-600 hover:underline mt-0.5"
              >
                Modifier
              </button>
            ) : undefined,
          },
          {
            label: "Budget",
            value: trip.data.finalTotalCost
              ? `${trip.data.finalTotalCost} € validé`
              : `${Math.round(Number(trip.data.coutTotal) || 0)} € prévu`,
            icon: "💶",
          },
        ]}
      />

      <TripWorkflowStepper
        steps={currentSteps}
        currentStatus={trip.status}
        busSignatureOnLogistics={withBusLogistics}
      />

      {nextGuidance ? (
        <TripNextStepBanner
          guidance={nextGuidance}
          onOpenTab={(tab) => {
            setHubTab(tab);
            const params = new URLSearchParams(searchParams.toString());
            if (tab === "overview") params.delete("tab");
            else params.set("tab", tab);
            const q = params.toString();
            router.replace(q ? `/travels/${id}?${q}` : `/travels/${id}`, { scroll: false });
            if (tab === "overview") {
              window.setTimeout(() => {
                document.getElementById("trip-decision-panel")?.scrollIntoView({
                  behavior: "smooth",
                  block: "start",
                });
              }, 80);
            }
          }}
        />
      ) : null}

      <div className="mt-4 mb-2">
        <TripHubNav
          active={hubTab}
          onChange={(tab) => {
            setHubTab(tab);
            const params = new URLSearchParams(searchParams.toString());
            if (tab === "overview") params.delete("tab");
            else params.set("tab", tab);
            const q = params.toString();
            router.replace(q ? `/travels/${id}?${q}` : `/travels/${id}`, { scroll: false });
          }}
          badges={hubBadges}
          tabs={visibleHubTabs}
        />
      </div>
      <TripRemindersBanner
        tripId={trip.id}
        highlightReminderId={highlightReminderId}
      />

      {hubTab === "eleves" && (
        <TripElevesListPanel
          trip={trip}
          canEdit={canEditEffectif}
          onTripUpdated={(t) => {
            setTrip(t);
            setEditedData(t.data);
          }}
        />
      )}

      {hubTab === "communication" && participantCount > 0 && (
        <TripParentComPanel
          trip={trip}
          canEdit={canEditEffectif}
          onTripUpdated={(t) => {
            setTrip(t);
            setEditedData(t.data);
          }}
        />
      )}

      {trip.type === "COMPLEX" && !withBusLogistics && hubTab === "overview" && (
        <TripAlert tone="info" icon="ℹ️" title="Sans transport bus">
          L&apos;étape « Choix du devis transport » est ignorée pour ce voyage (sans bus).
        </TripAlert>
      )}

      {hubTab === "transport" && withBusLogistics && (
        <TripTransportHubPanel
          trip={trip}
          setTrip={setTrip}
          loadingAction={loadingAction}
          canRequestAmendedQuote={canRequestAmendedQuote}
          requestAmendedBusQuote={requestAmendedBusQuote}
          canSign={canSign}
          skipTransportToCompta={skipTransportToCompta}
          openSecureFile={openSecureFile}
          effectifChanged={effectifChanged}
          datesChanged={datesChanged}
          snapshotEffectifTotal={snapshotEffectifTotal}
          currentEffectifTotal={currentEffectifTotal}
          isOwner={isOwner}
          selectBusQuote={selectBusQuote}
          selectAndSignBusQuote={selectAndSignBusQuote}
          deleteBusQuote={deleteBusQuote}
          canAddDocuments={canAddDocuments}
          addManualBusQuote={addManualBusQuote}
          manualDevisName={manualDevisName}
          setManualDevisName={setManualDevisName}
          manualDevisEmail={manualDevisEmail}
          setManualDevisEmail={setManualDevisEmail}
          manualDevisBusy={manualDevisBusy}
          signBusQuote={signBusQuote}
          handleAction={handleAction}
          transportReplyTo={transportReplyTo}
          setTransportReplyTo={setTransportReplyTo}
          transportReplyBody={transportReplyBody}
          setTransportReplyBody={setTransportReplyBody}
          transportReplyBusy={transportReplyBusy}
          setTransportReplyBusy={setTransportReplyBusy}
        />
      )}
      {hubTab === "overview" && (
        <TripOverviewFieldsPanel
          trip={trip}
          isEditing={isEditing}
          editedData={editedData}
          setEditedData={setEditedData}
          classOptions={classOptions}
          canEditEffectif={canEditEffectif}
          openEffectifModal={openEffectifModal}
          withBusLogistics={withBusLogistics}
          effectifChanged={effectifChanged}
          cuisineOrderSent={Boolean(cuisineOrderSent)}
          cuisineChanged={cuisineChanged}
          loadingAction={loadingAction}
          requestAmendedBusQuote={requestAmendedBusQuote}
          sendCuisineAmendment={sendCuisineAmendment}
          dateLabel={dateLabel}
          canEditDates={canEditDates}
          datesChanged={datesChanged}
          openDateModal={openDateModal}
          canAccessComptaTab={canAccessComptaTab}
          setHubTab={setHubTab}
          openBudgetModal={openBudgetModal}
          openCuisineModalFromEdit={openCuisineModalFromEdit}
          openCuisineModalForOwner={openCuisineModalForOwner}
          documentCount={documentCount}
        />
      )}

      {hubTab === "cuisine" && hasCuisineOrder && (
        <TripCuisineHubPanel
          trip={trip}
          cuisineOrderSent={Boolean(cuisineOrderSent)}
          cuisineOrderSentAt={cuisineOrderSentAt}
          isOwner={isOwner}
          canSign={canSign}
          loadingAction={loadingAction}
          sendCuisineAmendment={sendCuisineAmendment}
          cuisineChanged={cuisineChanged}
          dateLabel={dateLabel}
        />
      )}

      {hubTab === "documents" && (
        <TripDocumentsHubPanel
          trip={trip}
          isEditing={isEditing}
          editedData={editedData}
          documentCount={documentCount}
          canSign={canSign}
          isOwner={isOwner}
          loadingAction={loadingAction}
          handleRegenerateCircular={handleRegenerateCircular}
          canAddDocuments={canAddDocuments}
          fileInputRef={fileInputRef}
          handleFileUpload={handleFileUpload}
          uploading={uploading}
          openSecureFile={openSecureFile}
          canSeeTravelDocHoverActions={canSeeTravelDocHoverActions}
          prepareSendToZeendoc={prepareSendToZeendoc}
          zeendocSendingUrl={zeendocSendingUrl}
          canManageFiles={canManageFiles}
          removeFile={removeFile}
          withBusLogistics={withBusLogistics}
          deleteBusQuote={deleteBusQuote}
        />
      )}

      {hubTab === "journal" && <TripAmendmentJournal trip={trip} />}

      {hubTab === "actions" && (
        <TripActionsPanel
          trip={trip}
          canManage={isOwner || canSign}
          isGlobalAdmin={isGlobalAdmin}
          onTripUpdated={(t) => {
            setTrip(t);
            setEditedData(t.data);
          }}
        />
      )}

      {(hubTab === "overview" || hubTab === "messages") && canUseInternalThread && (
        <TripInternalThreadPanel
          trip={trip}
          draftMessage={draftMessage}
          setDraftMessage={setDraftMessage}
          postInternalMessage={postInternalMessage}
        />
      )}

      {hubTab === "compta" && canAccessComptaTab && !isEditing && (
        <TravelsComptaSheetForm
          tripId={trip.id}
          documentsRevision={comptaDocumentsFingerprint(trip)}
          readOnly={!isCompta}
          canValidateBudget={isCompta && trip.status === "EN_ATTENTE_COMPTA"}
          budgetValidated={Boolean(trip.data.comptaSheet?.budgetValidatedAt || trip.data.finalTotalCost)}
          onSaved={onComptaSheetSaved}
          onValidateBudget={onComptaValidateBudget}
        />
      )}

      {hubTab === "overview" && (isDirection || isCompta) && !isEditing && trip.status !== "BESOIN_MODIFICATION" && trip.status !== "SEANCE_ANNULEE" && trip.status !== "ANNULE" && (
        <TripDecisionHubPanel
          trip={trip}
          isDirection={isDirection}
          canSign={canSign}
          etabForSign={etabForSign || ""}
          isCompta={isCompta}
          handleAction={handleAction}
          withBusLogistics={withBusLogistics}
          isOwner={isOwner}
          skipTransportToCompta={skipTransportToCompta}
          loadingAction={loadingAction}
          seriesId={seriesId}
          validateSeriesPedagogy={validateSeriesPedagogy}
          handleFinalValidation={handleFinalValidation}
          handleRegenerateCircular={handleRegenerateCircular}
          reopenStepOptions={reopenStepOptions}
          selectedReopenStep={selectedReopenStep}
          setReopenStep={setReopenStep}
          handleReopenDossier={handleReopenDossier}
          canCancelRecurrenceSession={canCancelRecurrenceSession}
          cancelRecurrenceSession={cancelRecurrenceSession}
        />
      )}

      {canCancelRecurrenceSession && !isEditing && !canSign && isOwner && trip.status !== "SEANCE_ANNULEE" && (
        <TripAlert
          tone="warning"
          title="Série récurrente"
          action={
            <TripButton variant="warning" size="sm" disabled={!!loadingAction} onClick={cancelRecurrenceSession}>
              Annuler cette séance seule
            </TripButton>
          }
        >
          Retirer uniquement ce créneau sans modifier les autres dossiers de la série.
        </TripAlert>
      )}

      <TripDetailsModals
        trip={trip}
        showEffectifModal={showEffectifModal}
        setShowEffectifModal={setShowEffectifModal}
        draftNbEleves={draftNbEleves}
        setDraftNbEleves={setDraftNbEleves}
        draftNbAccompagnateurs={draftNbAccompagnateurs}
        setDraftNbAccompagnateurs={setDraftNbAccompagnateurs}
        draftNomsAccompagnateurs={draftNomsAccompagnateurs}
        setDraftNomsAccompagnateurs={setDraftNomsAccompagnateurs}
        draftAccompagnateurs={draftAccompagnateurs}
        setDraftAccompagnateurs={setDraftAccompagnateurs}
        saveEffectifChange={saveEffectifChange}
        effectifFollowUp={effectifFollowUp}
        setEffectifFollowUp={setEffectifFollowUp}
        runEffectifFollowUp={runEffectifFollowUp}
        showBudgetModal={showBudgetModal}
        setShowBudgetModal={setShowBudgetModal}
        draftCoutTotal={draftCoutTotal}
        setDraftCoutTotal={setDraftCoutTotal}
        saveBudgetChange={saveBudgetChange}
        cuisineFollowUp={cuisineFollowUp}
        setCuisineFollowUp={setCuisineFollowUp}
        runCuisineFollowUp={runCuisineFollowUp}
        showDateModal={showDateModal}
        setShowDateModal={setShowDateModal}
        draftStartDate={draftStartDate}
        setDraftStartDate={setDraftStartDate}
        draftEndDate={draftEndDate}
        setDraftEndDate={setDraftEndDate}
        draftStartTime={draftStartTime}
        setDraftStartTime={setDraftStartTime}
        draftEndTime={draftEndTime}
        setDraftEndTime={setDraftEndTime}
        saveDateChange={saveDateChange}
        dateFollowUp={dateFollowUp}
        setDateFollowUp={setDateFollowUp}
        runDateFollowUp={runDateFollowUp}
        showCuisineModal={showCuisineModal}
        isEditing={isEditing}
        cuisineModalStandalone={cuisineModalStandalone}
        setShowCuisineModal={setShowCuisineModal}
        setCuisineModalStandalone={setCuisineModalStandalone}
        activeCuisineDetails={activeCuisineDetails}
        patchCuisineDetails={patchCuisineDetails}
        saveCuisineFromOwnerModal={saveCuisineFromOwnerModal}
      />
    </TripPageShell>
  );
}