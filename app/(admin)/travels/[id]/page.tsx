"use client";

import { useUser } from "@clerk/nextjs";
import { useState, useEffect, useRef, useCallback } from "react";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import { useTravelsPermissions } from "@/app/hooks/useTravelsPermissions";
import {
  CUISINE_DAYS_UI,
  CUISINE_ROWS_UI,
  emptyCuisineDetails,
  getTotalMeals,
} from "@/app/lib/travels-cuisine-form";
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
import { TRAVELS_HUB_TABS, TRAVELS_STATUS_LABELS } from "@/app/lib/travels-types";
import { orderEmailForQuote } from "@/app/lib/travels-transport-shared";
import { TripActionsPanel } from "@/app/components/travels/hub/TripActionsPanel";
import { TripAmendmentJournal } from "@/app/components/travels/hub/TripAmendmentJournal";
import { TripElevesListPanel } from "@/app/components/travels/hub/TripElevesListPanel";
import { TripHubNav } from "@/app/components/travels/hub/TripHubNav";
import { TripParentComPanel } from "@/app/components/travels/hub/TripParentComPanel";
import { TripRemindersBanner } from "@/app/components/travels/hub/TripRemindersBanner";
import TravelsOwnerRepairSection from "@/app/components/travels/TravelsOwnerRepairSection";
import TravelsComptaSheetForm from "@/app/components/travels/TravelsComptaSheetForm";
import type { TravelsComptaSheet } from "@/app/lib/travels-compta-sheet";
import { comptaDocumentsFingerprint, comptaDefinitiveCostPerStudent, computeComptaSheetDerived } from "@/app/lib/travels-compta-sheet";
import {
  TripAlert,
  TripBusQuoteCard,
  TripButton,
  TripDecisionPanel,
  TripDocumentChip,
  TripField,
  TripFieldActions,
  TripFieldValue,
  TripHeroHeader,
  TripInput,
  TripLoadingOverlay,
  TripPageShell,
  TripQuickStats,
  TripSection,
  TripTextarea,
  TripWorkflowStepper,
} from "@/app/components/travels/TripDetailUI";

export default function TripDetails() {
  const { id } = useParams();
  const router = useRouter();
  const searchParams = useSearchParams();
  const remindersFocus = searchParams.get("focus") === "reminders";
  const highlightReminderId = searchParams.get("reminder");
  const tabFromUrl = searchParams.get("tab");
  const fileInputRef = useRef<HTMLInputElement>(null);
  const { user, isLoaded: isUserLoaded } = useUser();
  const [trip, setTrip] = useState<TravelsTrip | null>(null);
  const [hubTab, setHubTab] = useState<TravelsHubTab>(() => {
    const t = tabFromUrl as TravelsHubTab | null;
    return t && TRAVELS_HUB_TABS.some((x) => x.id === t) ? t : "overview";
  });
  const [loadingAction, setLoadingAction] = useState<string | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [editedData, setEditedData] = useState<any>(null);
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
  } = perms;
  const CUISINE_DAYS = CUISINE_DAYS_UI;
  const CUISINE_ROWS = CUISINE_ROWS_UI;
  useEffect(() => {
    if (!trip) return;
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
    if (!trip || !isCompta) return;
    if (trip.status === "EN_ATTENTE_COMPTA" && comptaTabAutoOpened.current !== trip.id) {
      comptaTabAutoOpened.current = trip.id;
      setHubTab("compta");
    }
  }, [trip, isCompta]);

  useEffect(() => {
    if (!trip || !remindersFocus) return;
    const t = window.setTimeout(() => {
      document.getElementById("trip-reminders")?.scrollIntoView({ behavior: "smooth", block: "start" });
    }, 400);
    return () => window.clearTimeout(t);
  }, [trip, remindersFocus]);

  useEffect(() => {
    const fetchTrip = async () => {
      try {
        const res = await fetch(`/api/travels/get?id=${id}`);
        if (res.ok) {
          const data = await res.json();
          setTrip(data);
          setEditedData({
            ...data.data,
            piqueNiqueDetails: data.data?.piqueNiqueDetails || emptyCuisineDetails(),
          });
        }
      } catch (err) {
        console.error("Erreur lors de la récupération du dossier:", err);
      }
    };
    if (id) void fetchTrip();
  }, [id]);
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
  const generateCircularAttachment = async (): Promise<{ name: string; url: string }> => {
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
    const uploadRes = await fetch("/api/travels/upload", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ fileName, fileType: "application/pdf" }),
    });
    if (!uploadRes.ok) throw new Error("Impossible de préparer l'enregistrement de la circulaire.");
    const { uploadUrl, fileUrl } = await uploadRes.json();
    if (!uploadUrl || !fileUrl) throw new Error("Réponse upload circulaire invalide.");
    const base64Content = pdf.split(",")[1];
    const byteArray = new Uint8Array(atob(base64Content).split("").map((c) => c.charCodeAt(0)));
    const putRes = await fetch(uploadUrl, {
      method: "PUT",
      body: new Blob([byteArray], { type: "application/pdf" }),
    });
    if (!putRes.ok) throw new Error("Enregistrement de la circulaire sur le serveur impossible.");
    return { name: CIRCULAR_ATTACHMENT_NAME, url: fileUrl };
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
            user: user?.fullName,
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
      const res = await fetch('/api/travels/upload', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ fileName: file.name, fileType: file.type })
      });
      const { uploadUrl, fileUrl } = await res.json();
      await fetch(uploadUrl, { method: 'PUT', body: file, headers: { 'Content-Type': file.type } });
      const newAttachment = { name: file.name, url: fileUrl };
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
        { date: new Date().toISOString(), user: user?.fullName, action: newStatus, note: note },
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
    setDraftNbAccompagnateurs(String(trip.data?.nbAccompagnateurs ?? ""));
    setDraftNomsAccompagnateurs(String(trip.data?.nomsAccompagnateurs ?? ""));
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
          user: user?.fullName,
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
          user: user?.fullName,
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
    const nbAcc = Number(draftNbAccompagnateurs);
    const nomsAccompagnateurs = draftNomsAccompagnateurs.trim();
    if (!Number.isFinite(nbEleves) || nbEleves < 0 || !Number.isFinite(nbAcc) || nbAcc < 0) {
      return alert("Indiquez des effectifs valides (nombres positifs).");
    }
    const prevEleves = Number(trip.data?.nbEleves) || 0;
    const prevAcc = Number(trip.data?.nbAccompagnateurs) || 0;
    const prevNoms = String(trip.data?.nomsAccompagnateurs || "").trim();
    if (nbEleves === prevEleves && nbAcc === prevAcc && nomsAccompagnateurs === prevNoms) {
      setShowEffectifModal(false);
      return alert("Aucun changement.");
    }

    const updatedTrip = {
      ...trip,
      data: { ...trip.data, nbEleves, nbAccompagnateurs: nbAcc, nomsAccompagnateurs },
      history: [
        ...(trip.history || []),
        {
          date: new Date().toISOString(),
          user: user?.fullName,
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
          user: user?.fullName,
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
      const res = await fetch("/api/travels/upload", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fileName: file.name, fileType: "application/pdf" }),
      });
      const { uploadUrl, fileUrl } = await res.json();
      if (!uploadUrl || !fileUrl) throw new Error("Réponse upload invalide");
      await fetch(uploadUrl, { method: "PUT", body: file, headers: { "Content-Type": "application/pdf" } });
      const newQuote = {
        id: `${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
        providerName: name,
        providerEmail: email,
        fileUrl,
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
      alert("Devis ajouté. Vous pouvez le sélectionner à l’étape logistique comme un devis reçu par mail.");
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
    if (!confirm("Voulez-vous signer le devis et envoyer la commande au transporteur ?")) return;
    const transporteurEmail = orderEmailForQuote(quote);
    if (!transporteurEmail) {
      return alert(
        "Erreur : aucun e-mail pour envoyer la commande (ni adresse lue sur le devis, ni expéditeur du mail, ni e-mail transporteur enregistré)."
      );
    }
    setLoadingAction("signing");
    const etab = trip.data?.etablissement || "";
    let sigType = etab === "École" ? "ecole" : etab === "Collège" ? "college" : "lycee";
    if (!sigType) return alert("Erreur de rôle.");
    try {
      const signRes = await fetch('/api/travels/sign-pdf', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ quoteUrl: quote.fileUrl, signatureType: sigType })
      });
      const { signedPdfData } = await signRes.json();
      const fileName = `Devis_Signe_${quote.providerName.replace(/\s+/g, '_')}.pdf`;
      const uploadRes = await fetch('/api/travels/upload', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ fileName, fileType: "application/pdf" })
      });
      const { uploadUrl, fileUrl } = await uploadRes.json();
      const byteArray = new Uint8Array(atob(signedPdfData.split(',')[1]).split("").map(c => c.charCodeAt(0)));
      await fetch(uploadUrl, { method: 'PUT', body: new Blob([byteArray], { type: 'application/pdf' }) });
      await fetch('/api/travels/send-order', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          tripId: trip.id,
          tripTitle: trip.data.title,
          tripData: trip.data,
          providerEmail: transporteurEmail,
          signedQuoteUrl: fileUrl,
          providerName: quote.providerName,
        })
      });
      const newAttachment = { name: `✅ ${fileName}`, url: fileUrl };
      handleAction("EN_ATTENTE_COMPTA", `Devis signé et commande envoyée`, {
        selectedBusQuote: quote,
        attachments: [...(trip.data.attachments || []), newAttachment],
        signedQuoteUrl: fileUrl,
      });
    } catch (err) {
      alert("Erreur lors de la signature.");
    } finally {
      setLoadingAction(null);
    }
  };
  const formatSafeDate = (dateStr: any) => {
    if (!dateStr) return "N/C";
    const d = new Date(dateStr);
    return isNaN(d.getTime()) ? "Date à préciser" : d.toLocaleDateString('fr-FR');
  };
  if (!isUserLoaded || !trip) {
    return (
      <TripPageShell>
        <div className="flex flex-col items-center justify-center py-24 gap-4">
          <div className="w-10 h-10 border-4 border-indigo-100 border-t-indigo-600 rounded-full animate-spin" />
          <p className="text-sm font-medium text-slate-500">Chargement du dossier…</p>
        </div>
      </TripPageShell>
    );
  }
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
            { n: "2", label: "Logistique", key: "PROF_LOGISTICS" },
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
        title={trip.data.title}
        typeLabel={trip.type === "COMPLEX" ? "Voyage scolaire" : "Sortie de proximité"}
        ownerName={trip.ownerName}
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
          L&apos;étape logistique (devis transporteurs) est ignorée pour ce voyage.
        </TripAlert>
      )}
      {hubTab === "transport" && withBusLogistics && (
        <TripSection
          title="Devis transport"
          subtitle="Offres reçues par e-mail ou ajoutées manuellement"
          icon="🚌"
          accent="amber"
          action={
            canRequestAmendedQuote ? (
              <TripButton
                variant="warning"
                size="sm"
                onClick={() => requestAmendedBusQuote()}
                disabled={loadingAction === "amendment-quote"}
              >
                {loadingAction === "amendment-quote" ? "Envoi…" : "Devis rectifié (effectif)"}
              </TripButton>
            ) : undefined
          }
        >
          {trip.data.transportPhaseBypassedAt && (
            <TripAlert tone="info" title="Étape transport contournée" icon="ℹ️">
              <p className="text-xs leading-relaxed">
                La direction a validé le passage aux finances sans devis bus signé
                {trip.data.transportPhaseBypassedBy ? ` (${trip.data.transportPhaseBypassedBy})` : ""}.
                {trip.data.transportPhaseBypassNote ? (
                  <>
                    <br />
                    <span className="italic">{trip.data.transportPhaseBypassNote}</span>
                  </>
                ) : null}
              </p>
            </TripAlert>
          )}
          {trip.data.pendingAmendedQuote && trip.status === "PROF_LOGISTICS" && (
            <TripAlert tone="warning" title="Devis rectifié en attente" icon="⏳">
              <p className="text-xs leading-relaxed">
                Une demande de devis (avenant) a été envoyée aux transporteurs. Si le devis n&apos;est plus
                nécessaire, la direction peut passer aux finances sans attendre.
              </p>
              {canSign && (
                <TripButton
                  variant="warning"
                  size="sm"
                  className="mt-3"
                  onClick={() => void skipTransportToCompta()}
                  disabled={!!loadingAction}
                >
                  Passer aux finances sans devis signé
                </TripButton>
              )}
            </TripAlert>
          )}
          {trip.data.transportProviderConfirmation && (
            <TripAlert tone="success" title="Confirmation transporteur reçue" icon="✅">
              <p className="text-xs leading-relaxed">{trip.data.transportProviderConfirmation.summary}</p>
              {trip.data.transportProviderConfirmation.receivedAt && (
                <p className="text-[10px] text-emerald-800/80 mt-1">
                  {new Date(trip.data.transportProviderConfirmation.receivedAt).toLocaleString("fr-FR")}
                  {trip.data.transportProviderConfirmation.providerName
                    ? ` · ${trip.data.transportProviderConfirmation.providerName}`
                    : ""}
                </p>
              )}
              {trip.data.transportProviderConfirmation.pdfUrl && (
                <TripButton
                  variant="ghost"
                  size="sm"
                  type="button"
                  className="mt-2"
                  onClick={() =>
                    openSecureFile(
                      trip.data.transportProviderConfirmation!.pdfUrl!,
                      trip.data.transportProviderConfirmation!.s3KeyIncoming,
                    )
                  }
                >
                  Voir le PDF de confirmation
                </TripButton>
              )}
            </TripAlert>
          )}
          {(effectifChanged || datesChanged) && (
            <TripAlert tone="warning" title="Dossier modifié depuis la dernière demande transport">
              <span className="text-xs">
                Dernier envoi : {snapshotEffectifTotal} pers. · Actuel : {currentEffectifTotal} pers.
                {trip.data?.selectedBusQuote
                  ? ` → avenant vers ${trip.data.selectedBusQuote.providerName} uniquement.`
                  : " → avenant vers tous les transporteurs."}
              </span>
            </TripAlert>
          )}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-4">
            <div className="space-y-3">
              <p className="text-[10px] font-bold uppercase tracking-wider text-amber-800">Offres reçues</p>
              {trip.receivedDevis && trip.receivedDevis.length > 0 ? (
                trip.receivedDevis.map((quote: any, idx: number) => {
                  const reviewBus =
                    quote.matchReviewRequired === true ||
                    (quote.source === "email" &&
                      quote.matchConfidence &&
                      quote.matchConfidence !== "high");
                  const borderSelected = trip.data.selectedBusQuote?.fileUrl === quote.fileUrl;
                  return (
                  <TripBusQuoteCard
                    key={quote.id || idx}
                    selected={borderSelected}
                    review={reviewBus}
                    actions={
                      <>
                        <div className="flex gap-2">
                          <TripButton
                            variant="ghost"
                            size="sm"
                            type="button"
                            onClick={() => openSecureFile(quote.fileUrl, quote.s3KeyIncoming)}
                          >
                            Voir PDF
                          </TripButton>
                          {isOwner && !canSign && trip.status === "PROF_LOGISTICS" && (
                            <TripButton variant="primary" size="sm" onClick={() => selectBusQuote(quote)}>
                              Choisir
                            </TripButton>
                          )}
                          {canSign && trip.status === "PROF_LOGISTICS" && (
                            <TripButton
                              variant="success"
                              size="sm"
                              onClick={() => selectAndSignBusQuote(quote)}
                              disabled={!!loadingAction}
                            >
                              Choisir et signer
                            </TripButton>
                          )}
                        </div>
                        {canSign && (
                          <button
                            type="button"
                            onClick={() => deleteBusQuote(quote)}
                            disabled={!!loadingAction}
                            className="text-[10px] font-bold text-rose-600 hover:text-rose-800 disabled:opacity-50 text-center"
                          >
                            {loadingAction === `delete-quote-${quote.id}` ? "Suppression…" : "Supprimer"}
                          </button>
                        )}
                      </>
                    }
                  >
                      <p className="font-bold text-slate-900">{quote.providerName}</p>
                      <p className="text-indigo-600 font-semibold text-xs mt-0.5">
                        Devis reçu
                        {quote.source === "email" ? " · e-mail" : quote.source === "manual" ? " · manuel" : ""}
                      </p>
                      {reviewBus && (
                        <p className="mt-1 text-[10px] font-bold text-orange-800 uppercase tracking-wide">
                          À vérifier — rattachement automatique ({quote.matchConfidence || "?"})
                        </p>
                      )}
                      {quote.matchMotif && reviewBus && (
                        <p className="mt-0.5 text-[10px] text-orange-900/90 leading-snug">{quote.matchMotif}</p>
                      )}
                      {(quote.extractedPrice || quote.extractedCompany) && (
                        <p className="mt-1 text-[11px] text-slate-600">
                          {quote.extractedCompany ? <span className="font-medium">{quote.extractedCompany}</span> : null}
                          {quote.extractedCompany && quote.extractedPrice ? " · " : null}
                          {quote.extractedPrice ? <span className="font-semibold text-slate-800">{quote.extractedPrice}</span> : null}
                        </p>
                      )}
                      {(() => {
                        const to = orderEmailForQuote(quote);
                        return to ? (
                          <p className="mt-1.5 text-[10px] text-slate-700">
                            <span className="font-bold text-slate-500">Commande →</span>{" "}
                            <span className="font-mono">{to}</span>
                            {quote.extractedContactEmail?.trim() ? (
                              <span className="text-emerald-700 font-semibold"> (sur le devis)</span>
                            ) : quote.providerEmail?.trim() ? (
                              <span className="text-slate-500">
                                {" "}
                                ({quote.source === "manual" ? "saisi à la main" : "expéditeur mail"})
                              </span>
                            ) : null}
                          </p>
                        ) : (
                          <p className="mt-1.5 text-[10px] font-bold text-rose-700">
                            Aucun e-mail pour la commande — renseigner ou vérifier le devis.
                          </p>
                        );
                      })()}
                  </TripBusQuoteCard>
                  );
                })
              ) : (
                <p className="text-sm text-slate-400 italic py-4 text-center rounded-xl border border-dashed border-amber-200">
                  En attente de devis par e-mail…
                </p>
              )}
              {canAddDocuments && (
                <form onSubmit={addManualBusQuote} className="mt-4 p-4 rounded-xl border border-dashed border-amber-300 bg-amber-50/50 space-y-3 text-left">
                  <p className="text-xs font-bold text-amber-900 uppercase tracking-wide">Ajout manuel</p>
                  <p className="text-[11px] text-amber-900/70 leading-snug">
                    Déposez le PDF et l&apos;e-mail du transporteur si le devis n&apos;arrive pas par la boîte dédiée.
                  </p>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <label className="block text-[11px] font-semibold text-slate-600">
                      Transporteur
                      <TripInput
                        className="mt-1"
                        value={manualDevisName}
                        onChange={(ev) => setManualDevisName(ev.target.value)}
                        placeholder="ex. Cars Dupont"
                        disabled={manualDevisBusy}
                      />
                    </label>
                    <label className="block text-[11px] font-semibold text-slate-600">
                      E-mail (commande)
                      <TripInput
                        type="email"
                        required
                        className="mt-1"
                        value={manualDevisEmail}
                        onChange={(ev) => setManualDevisEmail(ev.target.value)}
                        placeholder="contact@transporteur.fr"
                        disabled={manualDevisBusy}
                      />
                    </label>
                  </div>
                  <label className="block text-[11px] font-semibold text-slate-600">
                    PDF du devis
                    <input
                      name="manualDevisPdf"
                      type="file"
                      accept="application/pdf"
                      className="mt-1 block w-full text-sm text-slate-600 file:mr-3 file:py-1.5 file:px-3 file:rounded-lg file:border-0 file:bg-amber-100 file:font-semibold file:text-amber-900"
                      disabled={manualDevisBusy}
                    />
                  </label>
                  <TripButton type="submit" variant="warning" size="sm" disabled={manualDevisBusy}>
                    {manualDevisBusy ? "Envoi…" : "Ajouter ce devis"}
                  </TripButton>
                </form>
              )}
            </div>
            <div className="rounded-xl border border-amber-200 bg-white p-6 flex flex-col justify-center min-h-[12rem]">
              {trip.data.selectedBusQuote ? (
                <div className="text-center space-y-4">
                  <div>
                    <p className="text-[10px] font-bold uppercase tracking-wider text-amber-700">Devis retenu</p>
                    <p className="text-lg font-bold text-slate-900 mt-1">{trip.data.selectedBusQuote.providerName}</p>
                    {orderEmailForQuote(trip.data.selectedBusQuote) && (
                      <p className="text-xs text-slate-500 mt-1 font-mono">{orderEmailForQuote(trip.data.selectedBusQuote)}</p>
                    )}
                  </div>
                  {canSign && trip.status === "EN_ATTENTE_BUS_SIGNATURE" && (
                    <div className="flex flex-col gap-3 w-full">
                      <TripButton
                        variant="success"
                        size="lg"
                        onClick={() => signBusQuote()}
                        disabled={!!loadingAction}
                        className="w-full"
                      >
                        ✍️ Signer et commander
                      </TripButton>
                      <TripButton
                        variant="warning"
                        size="sm"
                        onClick={() => void skipTransportToCompta()}
                        disabled={!!loadingAction}
                        className="w-full"
                      >
                        Passer aux finances sans signer
                      </TripButton>
                      <button
                        type="button"
                        onClick={() => { const n = prompt("Pourquoi refusez-vous ce devis ?"); if (n) handleAction("PROF_LOGISTICS", n); }}
                        className="text-xs font-bold text-rose-600 hover:underline"
                      >
                        Refuser ce choix
                      </button>
                    </div>
                  )}
                  {isOwner && !canSign && trip.status === "EN_ATTENTE_BUS_SIGNATURE" && (
                    <p className="text-sm text-amber-800 bg-amber-50 border border-amber-200 rounded-xl px-4 py-3">
                      Devis choisi — en attente de signature par la direction.
                    </p>
                  )}
                  {(trip.status === "EN_ATTENTE_COMPTA" || trip.status === "EN_ATTENTE_DIR_FINAL" || trip.status === "VALIDE") && (
                    <p className="inline-flex items-center gap-2 text-emerald-700 font-bold text-sm bg-emerald-50 px-4 py-2 rounded-full">
                      ✓ Commandé et signé
                    </p>
                  )}
                </div>
              ) : (
                <p className="text-sm text-slate-400 italic text-center">
                  Le créateur choisira un devis à l&apos;étape logistique.
                </p>
              )}
            </div>
          </div>
          {((Array.isArray(trip.data.transportEmailMessages) && trip.data.transportEmailMessages.length > 0) ||
            busLogisticsActive(trip)) && (
            <div className="mt-6 pt-6 border-t border-amber-200/80">
              <p className="text-[10px] font-bold uppercase tracking-wider text-amber-800 mb-3">
                Messagerie transporteur (e-mail)
              </p>
              {Array.isArray(trip.data.transportEmailMessages) && trip.data.transportEmailMessages.length > 0 && (
              <div className="space-y-3">
                {trip.data.transportEmailMessages.map((msg: {
                  id: string;
                  messageType?: string;
                  summary?: string;
                  subject?: string;
                  fromEmail?: string;
                  toEmail?: string;
                  direction?: string;
                  driverName?: string | null;
                  driverPhone?: string | null;
                  details?: string | null;
                  pdfUrl?: string | null;
                  s3KeyIncoming?: string | null;
                  receivedAt?: string;
                  matchConfidence?: string | null;
                }) => {
                  const isOut = msg.direction === "outbound";
                  const typeLabel = isOut
                    ? "Envoyé"
                    : msg.messageType === "confirmation_commande"
                      ? "Confirmation"
                      : msg.messageType === "info_transport"
                        ? "Info transport"
                        : msg.messageType === "devis_pdf"
                          ? "Devis"
                          : "Message reçu";
                  return (
                    <div
                      key={msg.id}
                      className={`rounded-xl border px-4 py-3 text-sm ${
                        isOut
                          ? "border-indigo-100 bg-indigo-50/50 text-slate-800"
                          : "border-amber-100 bg-amber-50/40 text-slate-800"
                      }`}
                    >
                      <div className="flex flex-wrap items-center gap-2 mb-1">
                        <span
                          className={`text-[10px] font-bold uppercase tracking-wide px-2 py-0.5 rounded-full ${
                            isOut ? "text-indigo-800 bg-indigo-100" : "text-amber-800 bg-amber-100"
                          }`}
                        >
                          {typeLabel}
                        </span>
                        {msg.receivedAt && (
                          <span className="text-[10px] text-slate-500">
                            {new Date(msg.receivedAt).toLocaleString("fr-FR")}
                          </span>
                        )}
                        {!isOut && msg.matchConfidence && msg.matchConfidence !== "high" && (
                          <span className="text-[10px] text-amber-700 font-semibold">À vérifier</span>
                        )}
                      </div>
                      {msg.summary && <p className="leading-snug whitespace-pre-wrap">{msg.summary}</p>}
                      {(msg.driverName || msg.driverPhone) && (
                        <p className="mt-2 text-xs text-slate-700">
                          {msg.driverName && <span className="font-semibold">Chauffeur : {msg.driverName}</span>}
                          {msg.driverName && msg.driverPhone ? " · " : null}
                          {msg.driverPhone && <span className="font-mono">{msg.driverPhone}</span>}
                        </p>
                      )}
                      {msg.details && msg.details !== msg.summary && (
                        <p className="mt-2 text-xs text-slate-600 whitespace-pre-wrap">{msg.details}</p>
                      )}
                      {msg.pdfUrl && (
                        <TripButton
                          variant="ghost"
                          size="sm"
                          type="button"
                          className="mt-2"
                          onClick={() => openSecureFile(msg.pdfUrl!, msg.s3KeyIncoming)}
                        >
                          Voir la pièce jointe PDF
                        </TripButton>
                      )}
                      <p className="mt-2 text-[10px] text-slate-500 truncate" title={msg.subject}>
                        {isOut
                          ? `À ${msg.toEmail || "—"} — ${msg.subject || ""}`
                          : `${msg.fromEmail} — ${msg.subject || ""}`}
                      </p>
                      {!isOut && msg.fromEmail && (
                        <TripButton
                          variant="ghost"
                          size="sm"
                          type="button"
                          className="mt-2"
                          onClick={() => {
                            setTransportReplyTo(msg.fromEmail || "");
                            setTransportReplyBody("");
                          }}
                        >
                          Répondre à {msg.fromEmail}
                        </TripButton>
                      )}
                    </div>
                  );
                })}
              </div>
              )}
              <div className="mt-4 rounded-xl border border-slate-200 bg-white/80 p-4 space-y-3">
                <p className="text-xs font-bold text-slate-800">Répondre au transporteur</p>
                <TripInput
                  type="email"
                  placeholder="Adresse e-mail du transporteur"
                  value={transportReplyTo}
                  onChange={(e) => setTransportReplyTo(e.target.value)}
                />
                <TripTextarea
                  rows={4}
                  placeholder="Votre message…"
                  value={transportReplyBody}
                  onChange={(e) => setTransportReplyBody(e.target.value)}
                />
                <TripButton
                  type="button"
                  disabled={transportReplyBusy || !transportReplyTo.trim() || transportReplyBody.trim().length < 2}
                  onClick={async () => {
                    if (!trip?.id) return;
                    setTransportReplyBusy(true);
                    try {
                      const res = await fetch("/api/travels/reply-transport-email", {
                        method: "POST",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({
                          tripId: trip.id,
                          toEmail: transportReplyTo.trim(),
                          bodyText: transportReplyBody.trim(),
                        }),
                      });
                      const payload = await res.json().catch(() => ({}));
                      if (!res.ok) {
                        alert(payload.error || "Échec de l'envoi");
                        return;
                      }
                      if (payload.trip) setTrip(payload.trip);
                      setTransportReplyBody("");
                    } catch {
                      alert("Erreur réseau");
                    } finally {
                      setTransportReplyBusy(false);
                    }
                  }}
                >
                  {transportReplyBusy ? "Envoi…" : "Envoyer l'e-mail"}
                </TripButton>
              </div>
            </div>
          )}
        </TripSection>
      )}
      {hubTab === "overview" && (
      <TripSection title="Détails du dossier" subtitle="Informations logistiques et pédagogiques" icon="📋">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-6">
          <TripField label="Destination" span={2}>
            <TripFieldValue value={trip.data.destination} multiline />
          </TripField>
          <TripField label="Classes concernées">
            {isEditing ? (
              <TripInput value={editedData.classes} onChange={(e) => setEditedData({ ...editedData, classes: e.target.value })} />
            ) : (
              <TripFieldValue value={trip.data.classes} />
            )}
          </TripField>
          <TripField label="Accompagnateurs">
            {isEditing ? (
              <TripInput value={editedData.nomsAccompagnateurs} onChange={(e) => setEditedData({ ...editedData, nomsAccompagnateurs: e.target.value })} />
            ) : (
              <>
                <TripFieldValue value={trip.data.nomsAccompagnateurs || "—"} />
                {canEditEffectif && (
                  <TripFieldActions>
                    <button
                      type="button"
                      onClick={openEffectifModal}
                      className="text-xs font-bold text-indigo-600 hover:underline"
                    >
                      Modifier effectifs &amp; accompagnateurs
                    </button>
                  </TripFieldActions>
                )}
              </>
            )}
          </TripField>
          <TripField label="Effectifs">
            {isEditing ? (
              <div className="flex gap-3">
                <div className="flex-1">
                  <span className="text-[9px] text-slate-400">Élèves</span>
                  <TripInput type="number" value={editedData.nbEleves} onChange={(e) => setEditedData({ ...editedData, nbEleves: e.target.value })} />
                </div>
                <div className="flex-1">
                  <span className="text-[9px] text-slate-400">Accomp.</span>
                  <TripInput type="number" value={editedData.nbAccompagnateurs} onChange={(e) => setEditedData({ ...editedData, nbAccompagnateurs: Number(e.target.value) })} />
                </div>
              </div>
            ) : (
              <>
                <TripFieldValue value={`${trip.data.nbEleves} élèves · ${trip.data.nbAccompagnateurs || "0"} accompagnateurs`} />
                {(canEditEffectif ||
                  (withBusLogistics && effectifChanged) ||
                  (cuisineOrderSent && trip.data?.piqueNiqueDetails?.active && cuisineChanged)) && (
                  <TripFieldActions>
                    {canEditEffectif && (
                      <button
                        type="button"
                        onClick={openEffectifModal}
                        className="text-xs font-bold text-indigo-600 hover:underline"
                      >
                        Modifier l&apos;effectif
                      </button>
                    )}
                    {withBusLogistics && effectifChanged && (
                      <button
                        type="button"
                        onClick={() => requestAmendedBusQuote()}
                        disabled={loadingAction === "amendment-quote"}
                        className="text-xs font-bold text-amber-700 hover:underline disabled:opacity-50"
                      >
                        Demander un devis rectifié (transport)
                      </button>
                    )}
                    {cuisineOrderSent && trip.data?.piqueNiqueDetails?.active && cuisineChanged && (
                      <button
                        type="button"
                        onClick={() => sendCuisineAmendment()}
                        disabled={loadingAction === "cuisine-amendment"}
                        className="text-xs font-bold text-emerald-700 hover:underline disabled:opacity-50"
                      >
                        Renvoyer commande cuisine (annule et remplace)
                      </button>
                    )}
                  </TripFieldActions>
                )}
              </>
            )}
          </TripField>
          <TripField label="Dates">
            {isEditing ? (
              <div className="flex gap-2 flex-wrap">
                <TripInput type="date" value={editedData.startDate || editedData.date || ""} onChange={(e) => setEditedData({ ...editedData, startDate: e.target.value, date: e.target.value })} />
                {trip.type === "COMPLEX" && (
                  <TripInput type="date" value={editedData.endDate || ""} onChange={(e) => setEditedData({ ...editedData, endDate: e.target.value })} />
                )}
              </div>
            ) : (
              <>
                <TripFieldValue value={dateLabel} />
                {(canEditDates || datesChanged) && (
                  <TripFieldActions>
                    {canEditDates && (
                      <button type="button" onClick={openDateModal} className="text-xs font-bold text-indigo-600 hover:underline">
                        Modifier dates & horaires
                      </button>
                    )}
                    {datesChanged && (
                      <p className="text-[10px] text-amber-700 font-semibold">Dates modifiées depuis le dernier envoi transport</p>
                    )}
                  </TripFieldActions>
                )}
              </>
            )}
          </TripField>
          <TripField label="Horaires">
            {isEditing ? (
              <div className="flex gap-2">
                <TripInput placeholder="Départ" value={editedData.startTime} onChange={(e) => setEditedData({ ...editedData, startTime: e.target.value })} />
                <TripInput placeholder="Retour" value={editedData.endTime} onChange={(e) => setEditedData({ ...editedData, endTime: e.target.value })} />
              </div>
            ) : (
              <TripFieldValue value={`Départ ${trip.data.startTime || "—"} · Retour ${trip.data.endTime || "—"}`} />
            )}
          </TripField>
          <TripField label="Budget">
            {isEditing ? (
              <div className="flex items-center gap-2">
                <TripInput type="number" className="max-w-[8rem]" value={editedData.coutTotal} onChange={(e) => setEditedData({ ...editedData, coutTotal: Number(e.target.value) })} />
                <span className="text-xs font-bold text-slate-500">€ total</span>
              </div>
            ) : (
              <div>
                <TripFieldValue value={`${Math.round(Number(trip.data.coutTotal))} € prévisionnel`} />
                {trip.data.finalTotalCost && (
                  <p className="text-emerald-700 font-bold text-sm mt-1">
                    Validé compta : {trip.data.finalTotalCost} € ({trip.data.costPerStudent} €/élève)
                  </p>
                )}
                {canAccessComptaTab && (
                  <button
                    type="button"
                    onClick={() => setHubTab("compta")}
                    className="text-xs font-bold text-indigo-600 hover:underline mt-1 block"
                  >
                    Ouvrir l&apos;onglet Compta
                  </button>
                )}
                {canEditEffectif && (
                  <TripFieldActions>
                    <button
                      type="button"
                      onClick={openBudgetModal}
                      className="text-xs font-bold text-indigo-600 hover:underline"
                    >
                      Modifier le budget prévisionnel
                    </button>
                  </TripFieldActions>
                )}
              </div>
            )}
          </TripField>
          <TripField label="Restauration">
            {isEditing ? (
              <button
                type="button"
                onClick={openCuisineModalFromEdit}
                className={`w-full p-4 rounded-xl border-2 flex items-center justify-between transition-all text-left ${
                  editedData?.piqueNiqueDetails?.active ? "border-emerald-400 bg-emerald-50" : "border-slate-200 bg-slate-50"
                }`}
              >
                <div>
                  <p className="font-bold text-slate-900 text-sm">Commande restauration</p>
                  <p className="text-xs text-slate-500 mt-0.5">
                    {editedData?.piqueNiqueDetails?.active
                      ? `${getTotalMeals(editedData.piqueNiqueDetails)} repas configurés`
                      : "Configurer"}
                  </p>
                </div>
                <span className="text-xl">🥪</span>
              </button>
            ) : (
              <div>
                <TripFieldValue value={trip.data.piqueNiqueDetails?.active ? "Commande cuisine configurée" : "Pas de commande cuisine"} />
                {trip.data.piqueNiqueDetails?.active && (
                  <>
                    <span className="inline-block mt-1 text-[10px] font-bold text-indigo-700 bg-indigo-50 px-2 py-0.5 rounded-full">
                      {getTotalMeals(trip.data.piqueNiqueDetails)} repas ·{" "}
                      {Object.values(trip.data.piqueNiqueDetails.daysSelection || {}).filter(Boolean).length} jour(s)
                    </span>
                    <button
                      type="button"
                      onClick={() => setHubTab("cuisine")}
                      className="mt-2 block text-xs font-bold text-emerald-700 hover:underline"
                    >
                      Voir le détail restauration →
                    </button>
                  </>
                )}
                {canEditEffectif && (
                  <TripFieldActions>
                    <button
                      type="button"
                      onClick={openCuisineModalForOwner}
                      className="text-xs font-bold text-indigo-600 hover:underline"
                    >
                      {trip.data.piqueNiqueDetails?.active
                        ? "Modifier la commande cuisine"
                        : "Configurer une commande cuisine"}
                    </button>
                  </TripFieldActions>
                )}
              </div>
            )}
          </TripField>
          <TripField label="Objectifs pédagogiques" span={2}>
            {isEditing ? (
              <TripTextarea value={editedData.objectifs} onChange={(e) => setEditedData({ ...editedData, objectifs: e.target.value })} />
            ) : (
              <TripFieldValue value={trip.data.objectifs || "Aucun objectif renseigné."} multiline />
            )}
          </TripField>
        </div>
        {documentCount > 0 && (
          <p className="mt-6 text-xs text-slate-500">
            {documentCount} document{documentCount > 1 ? "s" : ""} dans le dossier —{" "}
            <button type="button" onClick={() => setHubTab("documents")} className="font-bold text-indigo-600 hover:underline">
              voir l&apos;onglet Documents
            </button>
          </p>
        )}
      </TripSection>
      )}

      {hubTab === "cuisine" && hasCuisineOrder && (
        <TripSection
          title="Commande restauration"
          subtitle="Bon de commande envoyé au chef ou en préparation"
          icon="🍽️"
          accent="emerald"
          action={
            cuisineOrderSent && (isOwner || canSign) ? (
              <TripButton
                variant="warning"
                size="sm"
                onClick={() => sendCuisineAmendment()}
                disabled={loadingAction === "cuisine-amendment"}
              >
                {loadingAction === "cuisine-amendment" ? "Envoi…" : "Annule et remplace"}
              </TripButton>
            ) : undefined
          }
        >
          {(() => {
            const details = trip.data.piqueNiqueDetails as {
              deliveryTime?: string;
              deliveryPlace?: string;
              daysSelection?: Record<string, boolean>;
              orders?: Record<string, Record<string, string>>;
            };
            const selectedDayKeys = CUISINE_DAYS.filter((d) => details?.daysSelection?.[d.key]);
            return (
              <div className="space-y-5">
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
                  <div className="rounded-xl bg-emerald-50 border border-emerald-100 p-4">
                    <p className="text-[10px] font-bold uppercase text-emerald-700">Livraison</p>
                    <p className="font-bold text-slate-900 mt-1">
                      {details?.deliveryPlace || "—"} à {details?.deliveryTime || "—"}
                    </p>
                  </div>
                  <div className="rounded-xl bg-slate-50 border border-slate-100 p-4">
                    <p className="text-[10px] font-bold uppercase text-slate-500">Repas commandés</p>
                    <p className="font-bold text-slate-900 mt-1">{getTotalMeals(trip.data.piqueNiqueDetails)} au total</p>
                  </div>
                  <div className="rounded-xl bg-slate-50 border border-slate-100 p-4">
                    <p className="text-[10px] font-bold uppercase text-slate-500">Envoi au chef</p>
                    <p className="font-bold text-slate-900 mt-1">
                      {cuisineOrderSent
                        ? `Envoyé le ${new Date(cuisineOrderSentAt!).toLocaleDateString("fr-FR")}`
                        : trip.status === "VALIDE"
                          ? "Statut validé — envoi cuisine non tracé dans le dossier"
                          : "Pas encore envoyé (validation finale)"}
                    </p>
                    {(trip.data.cuisineAmendments?.length || 0) > 0 && (
                      <p className="text-[10px] text-amber-700 mt-1">{trip.data.cuisineAmendments!.length} rectification(s)</p>
                    )}
                  </div>
                </div>
                {cuisineChanged && (
                  <TripAlert tone="warning" title="Effectif modifié depuis la dernière commande">
                    <button
                      type="button"
                      onClick={() => sendCuisineAmendment()}
                      className="text-xs font-bold text-emerald-800 underline mt-1"
                    >
                      Renvoyer la commande au chef
                    </button>
                  </TripAlert>
                )}
                <div className="overflow-x-auto rounded-xl border border-slate-200">
                  <table className="w-full text-xs border-collapse min-w-[520px]">
                    <thead>
                      <tr className="bg-emerald-600 text-white">
                        <th className="text-left p-2.5 font-semibold">Désignation</th>
                        {selectedDayKeys.map((d) => (
                          <th key={d.key} className="p-2.5 text-center font-semibold">{d.label}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {CUISINE_ROWS.map((row, rowIdx) => (
                        <tr key={row.key} className={rowIdx % 2 === 0 ? "bg-white" : "bg-slate-50"}>
                          <td className="p-2 font-medium text-slate-700">{row.label}</td>
                          {selectedDayKeys.map((d) => (
                            <td key={d.key} className="p-2 text-center text-slate-600">
                              {details?.orders?.[d.key]?.[row.key] || "—"}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                <p className="text-[10px] text-slate-500 italic">
                  Effectif dossier : {trip.data.nbEleves} élèves, {trip.data.nbAccompagnateurs || 0} accompagnateurs — {dateLabel}
                </p>
              </div>
            );
          })()}
        </TripSection>
      )}

      {hubTab === "documents" && (
        <TripSection title="Documents du dossier" subtitle="Pièces jointes, devis transport et circulaire" icon="📁">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-6">
            <p className="text-sm text-slate-600">{documentCount} document{documentCount > 1 ? "s" : ""} au total</p>
            <div className="flex flex-wrap gap-2">
              {trip.status === "VALIDE" && (canSign || isOwner) && (
                <TripButton variant="secondary" size="sm" onClick={handleRegenerateCircular} disabled={!!loadingAction}>
                  {loadingAction === "regenerate-circular" ? "Génération…" : "Régénérer circulaire"}
                </TripButton>
              )}
              {canAddDocuments && (
                <>
                  <input type="file" ref={fileInputRef} className="hidden" onChange={handleFileUpload} />
                  <TripButton variant="primary" size="sm" onClick={() => fileInputRef.current?.click()} disabled={uploading}>
                    {uploading ? "Upload…" : "+ Document"}
                  </TripButton>
                </>
              )}
            </div>
          </div>

          {documentCount === 0 ? (
            <p className="text-sm text-slate-400 italic py-8 text-center">Aucun document dans ce dossier.</p>
          ) : (
            <div className="space-y-8">
              {((isEditing ? editedData.attachments : trip.data.attachments) || []).length > 0 && (
                <div>
                  <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400 mb-3">Pièces jointes</p>
                  <div className="flex flex-wrap gap-2">
                    {((isEditing ? editedData.attachments : trip.data.attachments) || []).map((file: {
                      name: string;
                      url: string;
                      s3Key?: string;
                    }, idx: number) => (
                      <TripDocumentChip
                        key={`att_${idx}`}
                        name={file.name}
                        onOpen={() => openSecureFile(file.url, file.s3Key)}
                        onZeendoc={canSeeTravelDocHoverActions ? () => prepareSendToZeendoc(file) : undefined}
                        zeendocBusy={zeendocSendingUrl === file.url}
                        showZeendoc={canSeeTravelDocHoverActions}
                        onRemove={canManageFiles ? () => removeFile(idx) : undefined}
                        canRemove={canManageFiles}
                      />
                    ))}
                  </div>
                </div>
              )}

              {withBusLogistics && ((trip.receivedDevis?.length || 0) > 0 || trip.data.signedQuoteUrl) && (
                <div>
                  <p className="text-[10px] font-bold uppercase tracking-wider text-amber-700 mb-3">Devis transport bus</p>
                  <div className="flex flex-wrap gap-2">
                    {(trip.receivedDevis || []).map((quote: {
                      id?: string;
                      providerName?: string;
                      fileUrl?: string;
                      s3KeyIncoming?: string;
                    }, idx: number) => {
                      const selected = trip.data.selectedBusQuote?.fileUrl === quote.fileUrl;
                      const label = `🚌 ${quote.providerName || "Transporteur"}${selected ? " (retenu)" : ""}`;
                      return (
                        <TripDocumentChip
                          key={quote.id || `devis_${idx}`}
                          name={label}
                          onOpen={() => quote.fileUrl && openSecureFile(quote.fileUrl, quote.s3KeyIncoming)}
                          onZeendoc={
                            quote.fileUrl && canSeeTravelDocHoverActions
                              ? () => prepareSendToZeendoc({ name: label, url: quote.fileUrl! })
                              : undefined
                          }
                          zeendocBusy={zeendocSendingUrl === quote.fileUrl}
                          showZeendoc={canSeeTravelDocHoverActions}
                          onRemove={canSign && quote.id ? () => deleteBusQuote(quote) : undefined}
                          canRemove={canSign}
                        />
                      );
                    })}
                    {trip.data.signedQuoteUrl && (
                      <TripDocumentChip
                        key="signed_bus"
                        name="🚌 Devis bus signé"
                        onOpen={() => openSecureFile(trip.data.signedQuoteUrl!)}
                        onZeendoc={
                          canSeeTravelDocHoverActions
                            ? () =>
                                prepareSendToZeendoc({
                                  name: "Devis bus signé",
                                  url: trip.data.signedQuoteUrl!,
                                })
                            : undefined
                        }
                        zeendocBusy={zeendocSendingUrl === trip.data.signedQuoteUrl}
                        showZeendoc={canSeeTravelDocHoverActions}
                      />
                    )}
                  </div>
                  {(trip.receivedDevis?.length || 0) > 0 && (
                    <p className="text-[10px] text-slate-500 mt-2">
                      Choix et validation des devis : onglet Transport.
                    </p>
                  )}
                </div>
              )}
            </div>
          )}
        </TripSection>
      )}

      {hubTab === "journal" && <TripAmendmentJournal trip={trip} />}

      {hubTab === "actions" && (
        <TripActionsPanel
          trip={trip}
          canManage={isOwner || canSign}
          onTripUpdated={(t) => {
            setTrip(t);
            setEditedData(t.data);
          }}
        />
      )}

      {(hubTab === "overview" || hubTab === "messages") && canUseInternalThread && (
        <TripSection
          title="Fil interne"
          subtitle="Échanges entre créateur, direction et comptabilité"
          icon="💬"
          action={
            <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400">
              {(trip.messages || []).length} message{(trip.messages || []).length > 1 ? "s" : ""}
            </span>
          }
        >
          <div className="max-h-72 overflow-y-auto rounded-xl border border-slate-100 bg-slate-50/80 p-4 space-y-3 mb-4">
            {(trip.messages || []).length === 0 ? (
              <p className="text-sm text-slate-400 italic text-center py-6">Aucun message pour le moment.</p>
            ) : (
              [...(trip.messages || [])]
                .sort((a: any, b: any) => new Date(a.date).getTime() - new Date(b.date).getTime())
                .map((msg: any) => (
                  <div key={msg.id || `${msg.user}_${msg.date}`} className="bg-white border border-slate-100 rounded-xl p-4 shadow-sm">
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-xs font-bold text-slate-800">
                        {msg.user}{" "}
                        <span className="text-slate-400 font-medium">· {msg.role || "—"}</span>
                      </p>
                      <p className="text-[10px] text-slate-400">{new Date(msg.date).toLocaleString("fr-FR")}</p>
                    </div>
                    <p className="text-sm text-slate-700 mt-2 whitespace-pre-wrap leading-relaxed">{msg.text}</p>
                  </div>
                ))
            )}
          </div>
          <TripTextarea
            value={draftMessage}
            onChange={(e) => setDraftMessage(e.target.value)}
            placeholder="Message interne… (ex. : proposer une alternative d'hébergement)"
          />
          <div className="flex justify-end mt-3">
            <TripButton onClick={postInternalMessage} disabled={!draftMessage.trim()}>
              Envoyer
            </TripButton>
          </div>
        </TripSection>
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
        <TripDecisionPanel title="Espace décisionnaire">
          <div className="flex flex-wrap gap-2 items-center lg:flex-1">
            {isDirection && !canSign && (
              <div className="w-full sm:max-w-md rounded-xl bg-white/5 border border-white/10 px-4 py-3 text-sm text-slate-300">
                <p className="font-bold text-white text-xs uppercase tracking-wide mb-1">Lecture seule</p>
                Dossier <span className="text-amber-300 font-semibold">{etabForSign || "groupe scolaire"}</span> — validation réservée à la direction concernée.
              </div>
            )}
          </div>
          <div className="flex flex-wrap gap-2 justify-end">
            {((canSign && (trip.status === "EN_ATTENTE_DIR_INITIAL" || trip.status === "EN_ATTENTE_BUS_SIGNATURE" || trip.status === "EN_ATTENTE_DIR_FINAL")) || (isCompta && trip.status === "EN_ATTENTE_COMPTA")) && (
              <>
                {canSign && (
                  <TripButton variant="danger" size="sm" onClick={() => { const n = prompt("Motif du refus définitif :"); if (n) handleAction("REJETE", n); }}>
                    Refus définitif
                  </TripButton>
                )}
                <TripButton
                  variant="warning"
                  size="sm"
                  onClick={() => {
                    const n = prompt("Précisez les changements attendus :");
                    if (n) {
                      const returnTo = trip.status === "EN_ATTENTE_DIR_FINAL" ? "EN_ATTENTE_COMPTA" : trip.status;
                      handleAction("BESOIN_MODIFICATION", n, { previousStatus: returnTo });
                    }
                  }}
                >
                  Demander des modifs
                </TripButton>
              </>
            )}
            {canSign && trip.status === "EN_ATTENTE_DIR_INITIAL" && trip.type === "COMPLEX" && (
              <TripButton
                variant="primary"
                size="sm"
                onClick={() =>
                  handleAction(
                    withBusLogistics ? "PROF_LOGISTICS" : "EN_ATTENTE_COMPTA",
                    withBusLogistics ? "Pédagogie validée" : "Pédagogie validée (sans transport bus)",
                  )
                }
              >
                Valider pédagogie
              </TripButton>
            )}
            {(isOwner || canSign) && trip.type === "COMPLEX" && !withBusLogistics && trip.status === "PROF_LOGISTICS" && (
              <TripButton variant="primary" size="sm" onClick={() => handleAction("EN_ATTENTE_COMPTA", "Sans bus — étape logistique non requise")}>
                Passer aux finances
              </TripButton>
            )}
            {canSign &&
              withBusLogistics &&
              (trip.status === "PROF_LOGISTICS" || trip.status === "EN_ATTENTE_BUS_SIGNATURE") && (
                <TripButton variant="warning" size="sm" onClick={() => void skipTransportToCompta()} disabled={!!loadingAction}>
                  Passer aux finances sans devis signé
                </TripButton>
              )}
            {canSign && trip.status === "EN_ATTENTE_DIR_INITIAL" && trip.type !== "COMPLEX" && !seriesId && (
              <TripButton variant="primary" size="sm" onClick={() => handleAction("EN_ATTENTE_COMPTA", "Pédagogie validée")}>
                Valider pédagogie
              </TripButton>
            )}
            {canSign && trip.status === "EN_ATTENTE_DIR_INITIAL" && trip.type !== "COMPLEX" && seriesId && (
              <TripButton variant="primary" size="sm" onClick={validateSeriesPedagogy}>
                {loadingAction ? "Validation série…" : "Valider toute la série"}
              </TripButton>
            )}
            {canSign && trip.status === "EN_ATTENTE_DIR_FINAL" && (
              <TripButton variant="success" size="sm" onClick={handleFinalValidation}>
                {loadingAction ? "Finalisation…" : "Validation finale"}
              </TripButton>
            )}
            {trip.status === "VALIDE" && (canSign || isOwner) && (
              <TripButton variant="success" size="sm" onClick={handleRegenerateCircular} disabled={!!loadingAction}>
                {loadingAction === "regenerate-circular" ? "Génération…" : "Régénérer circulaire"}
              </TripButton>
            )}
            {canSign && trip.status === "VALIDE" && reopenStepOptions.length > 0 && (
              <div className="flex flex-wrap items-center gap-2 rounded-xl bg-white/5 border border-white/10 px-3 py-2">
                <label htmlFor="reopen-step-select" className="text-xs text-slate-300 shrink-0">
                  Réouvrir :
                </label>
                <select
                  id="reopen-step-select"
                  value={selectedReopenStep}
                  onChange={(e) => setReopenStep(e.target.value)}
                  className="bg-slate-800 text-white text-sm font-medium rounded-lg px-2 py-1.5 border border-slate-600 outline-none focus:ring-2 focus:ring-indigo-400 min-w-[8rem]"
                >
                  {reopenStepOptions.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
                <TripButton
                  variant="dark"
                  size="sm"
                  onClick={() => {
                    const opt = reopenStepOptions.find((o) => o.value === selectedReopenStep);
                    if (opt) handleReopenDossier(opt.value, opt.label);
                  }}
                >
                  {loadingAction ? "…" : "Réouvrir"}
                </TripButton>
              </div>
            )}
            {canCancelRecurrenceSession && canSign && (
              <TripButton variant="secondary" size="sm" onClick={cancelRecurrenceSession}>
                Annuler cette séance
              </TripButton>
            )}
          </div>
        </TripDecisionPanel>
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
      {showEffectifModal && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[75] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-md w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Modifier effectifs &amp; accompagnateurs</h2>
            <p className="text-sm text-slate-500 mb-6">
              Créateur ou direction — mise à jour sans rouvrir tout le dossier.
            </p>
            <div className="grid grid-cols-2 gap-4 mb-4">
              <div>
                <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Élèves</label>
                <TripInput
                  type="number"
                  min={0}
                  value={draftNbEleves}
                  onChange={(e) => setDraftNbEleves(e.target.value)}
                />
              </div>
              <div>
                <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Nb accompagnateurs</label>
                <TripInput
                  type="number"
                  min={0}
                  value={draftNbAccompagnateurs}
                  onChange={(e) => setDraftNbAccompagnateurs(e.target.value)}
                />
              </div>
            </div>
            <div className="mb-6">
              <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Noms des accompagnateurs</label>
              <TripTextarea
                value={draftNomsAccompagnateurs}
                onChange={(e) => setDraftNomsAccompagnateurs(e.target.value)}
                placeholder="Ex. Mme Dupont, M. Martin…"
              />
            </div>
            <p className="text-xs text-amber-800 bg-amber-50 border border-amber-100 rounded-lg p-3 mb-6">
              Si un devis transport ou une commande cuisine a déjà été envoyé(e), vous pourrez déclencher les relances juste après l&apos;enregistrement.
            </p>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setShowEffectifModal(false)}>
                Annuler
              </TripButton>
              <TripButton variant="primary" className="flex-1" onClick={saveEffectifChange}>
                Enregistrer
              </TripButton>
            </div>
          </div>
        </div>
      )}

      {effectifFollowUp && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[80] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-lg w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Effectif enregistré</h2>
            <p className="text-sm text-slate-500 mb-5">Souhaitez-vous notifier les prestataires du changement ?</p>
            <div className="space-y-3 mb-6">
              {effectifFollowUp.sendTransport && (
                <label className="flex items-start gap-3 p-3 rounded-xl border border-amber-100 bg-amber-50 cursor-pointer">
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={effectifFollowUp.sendTransport}
                    onChange={(e) =>
                      setEffectifFollowUp({ ...effectifFollowUp, sendTransport: e.target.checked })
                    }
                  />
                  <span className="text-sm text-slate-700">
                    <strong>Demander un nouveau devis transport</strong>
                    <br />
                    <span className="text-xs text-slate-500">
                      {effectifFollowUp.savedTrip.data?.selectedBusQuote
                        ? `Uniquement ${effectifFollowUp.savedTrip.data.selectedBusQuote.providerName}.`
                        : "À tous les transporteurs référencés."}
                    </span>
                  </span>
                </label>
              )}
              {effectifFollowUp.sendCuisine && (
                <label className="flex items-start gap-3 p-3 rounded-xl border border-emerald-100 bg-emerald-50 cursor-pointer">
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={effectifFollowUp.sendCuisine}
                    onChange={(e) =>
                      setEffectifFollowUp({ ...effectifFollowUp, sendCuisine: e.target.checked })
                    }
                  />
                  <span className="text-sm text-slate-700">
                    <strong>Renvoyer la commande cuisine (annule et remplace)</strong>
                    <br />
                    <span className="text-xs text-slate-500">
                      Mail au chef avec formules de politesse et nouvelle commande en pièce jointe.
                    </span>
                  </span>
                </label>
              )}
            </div>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setEffectifFollowUp(null)}>
                Plus tard
              </TripButton>
              <TripButton
                variant="primary"
                className="flex-1"
                onClick={runEffectifFollowUp}
                disabled={
                  !effectifFollowUp.sendTransport && !effectifFollowUp.sendCuisine
                }
              >
                Envoyer les relances
              </TripButton>
            </div>
          </div>
        </div>
      )}

      {showBudgetModal && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[75] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-md w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Modifier le budget prévisionnel</h2>
            <p className="text-sm text-slate-500 mb-6">
              Montant total estimé au départ du projet (hors validation compta).
            </p>
            <div className="flex items-center gap-2 mb-6">
              <TripInput
                type="number"
                min={0}
                className="flex-1"
                value={draftCoutTotal}
                onChange={(e) => setDraftCoutTotal(e.target.value)}
              />
              <span className="text-sm font-bold text-slate-500">€</span>
            </div>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setShowBudgetModal(false)}>
                Annuler
              </TripButton>
              <TripButton variant="primary" className="flex-1" onClick={saveBudgetChange}>
                Enregistrer
              </TripButton>
            </div>
          </div>
        </div>
      )}

      {cuisineFollowUp && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[80] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-lg w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Commande cuisine enregistrée</h2>
            <p className="text-sm text-slate-500 mb-5">
              {cuisineFollowUp.mode === "initial"
                ? "Souhaitez-vous envoyer la commande au chef maintenant ?"
                : "Une commande avait déjà été envoyée — renvoyer au chef (annule et remplace) ?"}
            </p>
            <p className="text-xs text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-lg p-3 mb-6">
              {cuisineFollowUp.mode === "initial"
                ? "Un PDF sera joint au mail (chef + copies direction et organisateur)."
                : "Le mail précisera qu'il s'agit de la dernière commande en date."}
            </p>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setCuisineFollowUp(null)}>
                Plus tard
              </TripButton>
              <TripButton variant="primary" className="flex-1" onClick={runCuisineFollowUp}>
                {cuisineFollowUp.mode === "initial" ? "Envoyer au chef" : "Renvoyer (avenant)"}
              </TripButton>
            </div>
          </div>
        </div>
      )}

      {showDateModal && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[75] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-md w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Modifier dates & horaires</h2>
            <p className="text-sm text-slate-500 mb-6">Créateur ou direction — relances transport/cuisine proposées après enregistrement.</p>
            <div className="space-y-4 mb-6">
              <div className="flex gap-2 flex-wrap">
                <div className="flex-1 min-w-[8rem]">
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Date début</label>
                  <TripInput type="date" value={draftStartDate} onChange={(e) => setDraftStartDate(e.target.value)} />
                </div>
                {trip.type === "COMPLEX" && (
                  <div className="flex-1 min-w-[8rem]">
                    <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Date fin</label>
                    <TripInput type="date" value={draftEndDate} onChange={(e) => setDraftEndDate(e.target.value)} />
                  </div>
                )}
              </div>
              <div className="flex gap-2">
                <div className="flex-1">
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Départ</label>
                  <TripInput type="time" value={draftStartTime} onChange={(e) => setDraftStartTime(e.target.value)} />
                </div>
                <div className="flex-1">
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Retour</label>
                  <TripInput type="time" value={draftEndTime} onChange={(e) => setDraftEndTime(e.target.value)} />
                </div>
              </div>
            </div>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setShowDateModal(false)}>Annuler</TripButton>
              <TripButton variant="primary" className="flex-1" onClick={saveDateChange}>Enregistrer</TripButton>
            </div>
          </div>
        </div>
      )}

      {dateFollowUp && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[80] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-lg w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Dates enregistrées</h2>
            <p className="text-sm text-slate-500 mb-5">Notifier les prestataires du changement de planning ?</p>
            <div className="space-y-3 mb-6">
              {dateFollowUp.sendTransport && (
                <label className="flex items-center gap-3 p-3 rounded-xl border border-amber-100 bg-amber-50">
                  <input type="checkbox" checked={dateFollowUp.sendTransport} onChange={(e) => setDateFollowUp({ ...dateFollowUp, sendTransport: e.target.checked })} />
                  <span className="text-sm"><strong>Relancer le transporteur</strong> (avenant dates)</span>
                </label>
              )}
              {dateFollowUp.sendCuisine && (
                <label className="flex items-center gap-3 p-3 rounded-xl border border-emerald-100 bg-emerald-50">
                  <input type="checkbox" checked={dateFollowUp.sendCuisine} onChange={(e) => setDateFollowUp({ ...dateFollowUp, sendCuisine: e.target.checked })} />
                  <span className="text-sm"><strong>Renvoyer commande cuisine</strong></span>
                </label>
              )}
            </div>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setDateFollowUp(null)}>Plus tard</TripButton>
              <TripButton variant="primary" className="flex-1" onClick={runDateFollowUp} disabled={!dateFollowUp.sendTransport && !dateFollowUp.sendCuisine}>Envoyer</TripButton>
            </div>
          </div>
        </div>
      )}

      {showCuisineModal && (isEditing || cuisineModalStandalone) && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[70] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-5xl w-full shadow-2xl overflow-y-auto max-h-[90vh]">
            <div className="flex justify-between items-start mb-6 border-b border-slate-100 pb-4">
              <div>
                <h2 className="text-xl font-bold text-slate-900">Bon de commande cuisine</h2>
                <p className="text-slate-500 text-sm mt-0.5">
                  {cuisineModalStandalone
                    ? "Ajouter ou modifier la commande — envoi au chef proposé après enregistrement."
                    : "Configuration de la commande restauration"}
                </p>
              </div>
              <TripButton
                variant="ghost"
                size="sm"
                onClick={() => {
                  setShowCuisineModal(false);
                  setCuisineModalStandalone(false);
                }}
              >
                ✕
              </TripButton>
            </div>
            <div className="space-y-5">
              {cuisineModalStandalone && (
                <label className="flex items-center gap-3 p-4 rounded-xl border-2 border-slate-200 bg-slate-50 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={!!activeCuisineDetails?.active}
                    onChange={(e) =>
                      patchCuisineDetails((prev) => ({
                        ...prev,
                        active: e.target.checked,
                      }))
                    }
                  />
                  <span className="text-sm font-bold text-slate-800">Commander une restauration (pique-nique / self)</span>
                </label>
              )}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4 bg-slate-50 p-4 rounded-2xl">
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Heure récupération / livraison</label>
                  <input
                    type="time"
                    className="w-full p-2 border rounded-lg"
                    value={activeCuisineDetails?.deliveryTime || ""}
                    disabled={cuisineModalStandalone && !activeCuisineDetails?.active}
                    onChange={(e) =>
                      patchCuisineDetails((prev) => ({
                        ...prev,
                        active: true,
                        deliveryTime: e.target.value,
                      }))
                    }
                  />
                </div>
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Lieu de récupération</label>
                  <select
                    className="w-full p-2 border rounded-lg"
                    value={activeCuisineDetails?.deliveryPlace || "Self"}
                    disabled={cuisineModalStandalone && !activeCuisineDetails?.active}
                    onChange={(e) =>
                      patchCuisineDetails((prev) => ({
                        ...prev,
                        active: true,
                        deliveryPlace: e.target.value,
                      }))
                    }
                  >
                    <option value="Self">Au self</option>
                    <option value="Bosco">Église Bosco</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-2 text-center">Jours concernés</label>
                  <div className="flex gap-1.5 justify-center">
                    {CUISINE_DAYS.map(({ key: dayKey, label }) => {
                      const isSelected = !!activeCuisineDetails?.daysSelection?.[dayKey];
                      return (
                        <button
                          key={dayKey}
                          type="button"
                          disabled={cuisineModalStandalone && !activeCuisineDetails?.active}
                          onClick={() =>
                            patchCuisineDetails((prev) => ({
                              ...prev,
                              active: true,
                              daysSelection: {
                                ...(prev.daysSelection || emptyCuisineDetails().daysSelection),
                                [dayKey]: !isSelected,
                              },
                            }))
                          }
                          className={`w-9 h-9 rounded-lg text-[11px] font-black transition-all ${isSelected ? 'bg-indigo-600 text-white shadow-md' : 'bg-white border-2 text-slate-400 hover:border-indigo-300'}`}
                        >
                          {label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
              <div className="overflow-x-auto rounded-xl border border-slate-200">
                <table className="w-full text-xs border-collapse min-w-[620px]">
                  <thead>
                    <tr className="bg-indigo-600 text-white">
                      <th className="text-left p-2.5 font-semibold w-52">Désignation</th>
                      {CUISINE_DAYS.map(({ key: dayKey, label }) => (
                        <th key={dayKey} className={`p-2.5 text-center font-semibold transition-opacity ${activeCuisineDetails?.daysSelection?.[dayKey] ? "opacity-100" : "opacity-30"}`}>{label}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {CUISINE_ROWS.map(({ key: rowKey, label, type }, rowIdx) => (
                      <tr key={rowKey} className={rowIdx % 2 === 0 ? 'bg-white' : 'bg-slate-50'}>
                        <td className={`p-2 font-medium text-slate-700 whitespace-nowrap ${rowKey === 'picnicNoPork' || rowKey === 'picnicVeg' ? 'pl-5 text-slate-500 italic' : ''}`}>{label}</td>
                        {CUISINE_DAYS.map(({ key: dayKey }) => {
                          const isActive = !!activeCuisineDetails?.daysSelection?.[dayKey];
                          const val = activeCuisineDetails?.orders?.[dayKey]?.[rowKey] ?? "";
                          return (
                            <td key={dayKey} className="p-1">
                              <input
                                type={type}
                                disabled={!isActive || (cuisineModalStandalone && !activeCuisineDetails?.active)}
                                value={val}
                                onChange={(e) =>
                                  patchCuisineDetails((prev) => ({
                                    ...prev,
                                    active: true,
                                    orders: {
                                      ...(prev.orders || emptyCuisineDetails().orders),
                                      [dayKey]: {
                                        ...((prev.orders || emptyCuisineDetails().orders)[dayKey]),
                                        [rowKey]: e.target.value,
                                      },
                                    },
                                  }))
                                }
                                className={`w-full p-1.5 border rounded text-center transition-all ${isActive ? 'bg-white hover:border-indigo-300 focus:border-indigo-500 outline-none' : 'bg-slate-100 text-slate-300 cursor-not-allowed border-transparent'}`}
                              />
                            </td>
                          );
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="text-[10px] text-slate-500 italic bg-amber-50 border border-amber-100 p-2.5 rounded-lg">⚠️ Rappel : fournir la liste des élèves/adultes 15 jours avant, et l’affiner 24h avant.</p>
            </div>
            <div className="flex gap-3 mt-8 pt-4 border-t border-slate-100">
              {isEditing && !cuisineModalStandalone && (
                <TripButton
                  variant="danger"
                  className="flex-1"
                  onClick={() => {
                    patchCuisineDetails((prev) => ({ ...prev, active: false }));
                    setShowCuisineModal(false);
                  }}
                >
                  Annuler la commande
                </TripButton>
              )}
              <TripButton
                variant="secondary"
                className="flex-1"
                onClick={() => {
                  setShowCuisineModal(false);
                  setCuisineModalStandalone(false);
                }}
              >
                Fermer
              </TripButton>
              <TripButton
                variant="primary"
                className="flex-[2]"
                onClick={() => {
                  if (cuisineModalStandalone) {
                    void saveCuisineFromOwnerModal();
                  } else {
                    setShowCuisineModal(false);
                  }
                }}
              >
                {cuisineModalStandalone ? "Enregistrer la commande" : "Enregistrer le bon"}
              </TripButton>
            </div>
          </div>
        </div>
      )}
    </TripPageShell>
  );
}