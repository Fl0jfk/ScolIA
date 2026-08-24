"use client";

import { useMemo, useState, useEffect } from "react";
import dynamic from "next/dynamic";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useRouter, useSearchParams } from "next/navigation";
import AbsencesCalendar from "@/app/components/absences/AbsencesCalendar";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import EstablishmentSelect from "@/app/components/establishments/EstablishmentSelect";
import { useAppContext } from "@/app/hooks/useAppContext";
import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import {
  canChooseDeclarationScope,
  canDeclareAbsenceOnBehalf,
  canManageAbsence,
  canManageAbsenceAttachment,
  canViewAbsenceAttachment,
  canViewCalendar,
  isAbsencePendingForManager,
  resolveAbsenceScope,
  resolveSelfDeclarationScope,
  type AbsenceRecord,
} from "@/app/lib/absences-types";
import { formatAbsencePeriod, type AbsencePeriodType } from "@/app/lib/absence-period";
import {
  formatAbsenceHoursTreatment,
  getHoursTreatmentOptions,
  hoursTreatmentFieldLabel,
  validateHoursTreatmentForAbsence,
} from "@/app/lib/absence-hours-treatment";
import { compareAbsenceRecordsAlphabetically } from "@/app/lib/absences-shared-utils";
import {
  isPendingAbsence,
  itemDecision,
  resolvedHoursTreatment,
  transmissionLabel,
  validationConfirmMessage,
  type AbsenceDecision,
  type AbsenceItem,
  type AbsenceScope,
  type AbsenceWorkflowStatus,
  type Etablissement,
} from "@/app/lib/absences-page-model";

const AbsencesDeclareOther = dynamic(
  () => import("@/app/components/absences/AbsencesDeclareOther"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

const AbsencesDeclareOnBehalf = dynamic(
  () => import("@/app/components/absences/AbsencesDeclareOnBehalf"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

const norm = (s: string) => String(s || "").toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/[_\s-]+/g, "");

export default function AbsencesPageClient({
  embeddedInRh = false,
}: {
  /** Affiché dans le hub RH (`/rh?tab=absences&view=…`). */
  embeddedInRh?: boolean;
} = {}) {
  const { user, isLoaded } = useSessionUser();
  const { data: appCtx } = useAppContext();
  const establishments = appCtx?.establishments ?? [];
  const [items, setItems] = useState<AbsenceItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [etablissement, setEtablissement] = useState<Etablissement>("");
  const [periodType, setPeriodType] = useState<AbsencePeriodType>("multi_day");
  const [singleDayDate, setSingleDayDate] = useState("");
  const [startTime, setStartTime] = useState("");
  const [endTime, setEndTime] = useState("");
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");
  const [reason, setReason] = useState("");
  const [details, setDetails] = useState("");
  const [justificationFile, setJustificationFile] = useState<File | null>(null);
  const [managerNotes, setManagerNotes] = useState<Record<string, string>>({});
  const [managerHoursTreatment, setManagerHoursTreatment] = useState<Record<string, string>>({});
  const [uploadingJustificationId, setUploadingJustificationId] = useState<string | null>(null);
  const roles = rolesFromUserLike(user);
  const canChooseScope = canChooseDeclarationScope(roles);
  const [declareScope, setDeclareScope] = useState<AbsenceScope>("ogec");
  const effectiveScope = canChooseScope ? declareScope : resolveSelfDeclarationScope(roles);
  const router = useRouter();
  const searchParams = useSearchParams();
  const showCalendar = canViewCalendar(roles);
  const canTreat = isAnyDirectionRole(roles);
  const canOnBehalf = canDeclareAbsenceOnBehalf(roles);

  const asRecord = (item: AbsenceItem) => item as unknown as AbsenceRecord;

  const canViewJustificatif = (item: AbsenceItem) =>
    canViewAbsenceAttachment(asRecord(item), user?.id || "", roles);

  const canDeleteJustificatif = (item: AbsenceItem) => canManageAbsenceAttachment(asRecord(item), roles);
  const defaultTab = showCalendar ? "calendrier" : "se-declarer";
  const rawTab = embeddedInRh ? searchParams.get("view") : searchParams.get("tab");
  const activeTab =
    rawTab === "declarer" || rawTab === "mes-demandes" ? "se-declarer" : rawTab || defaultTab;
  const absencesHref = (view: string) =>
    embeddedInRh ? `/rh?tab=absences&view=${view}` : `/absences?tab=${view}`;
  const setTab = (tab: string) => router.push(absencesHref(tab));
  const [calendarRefresh, setCalendarRefresh] = useState(0);

  useEffect(() => {
    if (!isLoaded) return;
    if ((activeTab === "calendrier" || activeTab === "autre-personne") && !showCalendar && !canOnBehalf) {
      router.replace(absencesHref("se-declarer"));
    }
    if (rawTab === "declarer" || rawTab === "mes-demandes") {
      router.replace(absencesHref("se-declarer"));
    }
    // absencesHref is stable for a given embeddedInRh
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTab, rawTab, showCalendar, canOnBehalf, isLoaded, router, embeddedInRh]);
  const fetchItems = async () => {
    try {
      setLoading(true);
      const res = await fetch("/api/absences");
      if (!res.ok) throw new Error("Chargement impossible");
      const data = await res.json();
      setItems(data || []);
    } catch (e: any) { setError(e?.message || "Erreur de chargement.");
    } finally { setLoading(false);
    }
  };
  useEffect(() => {
    if (isLoaded && user) fetchItems();
  }, [isLoaded, user]);

  useEffect(() => {
    if (!isLoaded || activeTab !== "se-declarer" || typeof window === "undefined") return;
    if (window.location.hash !== "#nouvelle-absence") return;
    requestAnimationFrame(() => {
      document.getElementById("nouvelle-absence")?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }, [isLoaded, activeTab]);
  const submitAbsence = async () => {
    setError(null);
    if (!reason.trim()) {
      setError("Merci de remplir le motif.");
      return;
    }
    if (periodType === "single_day") {
      if (!singleDayDate || !startTime || !endTime) {
        setError("Pour une journée, indiquez la date et les heures.");
        return;
      }
      if (endTime <= startTime) {
        setError("L'heure de fin doit être après l'heure de début.");
        return;
      }
    } else if (!startDate || !endDate) {
      setError("Indiquez la date de début et la date de fin.");
      return;
    } else if (endDate < startDate) {
      setError("La date de fin doit être après la date de début.");
      return;
    }
    if (effectiveScope === "professeur" && !etablissement) {
      setError("Merci de choisir un établissement.");
      return;
    }
    try {
      setSaving(true);
      let justification: { fileName: string; fileUrl: string } | null = null;
      if (justificationFile) {
        const presignRes = await fetch("/api/travels/upload", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            fileName: justificationFile.name,
            fileType: justificationFile.type || "application/octet-stream",
          }),
        });
        const presignPayload = await presignRes.json();
        if (!presignRes.ok || !presignPayload?.uploadUrl || !presignPayload?.fileUrl) { throw new Error("Impossible de préparer l'upload du justificatif.")}
        await fetch(presignPayload.uploadUrl, {
          method: "PUT",
          body: justificationFile,
          headers: { "Content-Type": justificationFile.type || "application/octet-stream" },
        });
        justification = {
          fileName: justificationFile.name,
          fileUrl: presignPayload.fileUrl,
        };
      }
      const res = await fetch("/api/absences", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          data: {
            scope: effectiveScope,
            etablissement: effectiveScope === "professeur" ? etablissement : null,
            periodType,
            startDate: periodType === "single_day" ? singleDayDate : startDate,
            endDate: periodType === "single_day" ? singleDayDate : endDate,
            startTime: periodType === "single_day" ? startTime : null,
            endTime: periodType === "single_day" ? endTime : null,
            reason: reason.trim(),
            details: details.trim(),
            justification,
          },
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Échec création");
      setPeriodType("multi_day");
      setSingleDayDate("");
      setStartTime("");
      setEndTime("");
      setStartDate("");
      setEndDate("");
      setReason("");
      setDetails("");
      setJustificationFile(null);
      await fetchItems();
    } catch (e: any) { setError(e?.message || "Erreur de création.");
    } finally { setSaving(false);
    }
  };
  const canManageItem = (item: AbsenceItem) =>
    canManageAbsence(asRecord(item), roles, {
      establishments,
      userId: user?.id,
    });
  const updateWorkflow = async (id: string, action: "VALIDER" | "REFUSER" | "RELANCER_JUSTIFICATIF", item?: AbsenceItem) => {
    if (action === "VALIDER" && item) {
      const treatmentCheck = validateHoursTreatmentForAbsence(
        item.data.scope,
        item.data.etablissement,
        resolvedHoursTreatment(item, managerHoursTreatment),
      );
      if (!treatmentCheck.ok) {
        alert(treatmentCheck.error);
        return;
      }
      if (!confirm(validationConfirmMessage(item))) return;
    }
    if (action === "REFUSER" && !confirm("Êtes-vous sûr de refuser cette absence ? Cette action est définitive.")) return;
    if (
      action === "RELANCER_JUSTIFICATIF" &&
      item?.justification?.fileUrl &&
      !confirm(
        "Un justificatif a déjà été déposé. Relancer quand même pour demander un complément ou un autre document ?",
      )
    ) {
      return;
    }
    try {
      const res = await fetch("/api/absences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          id,
          action,
          managerNote: managerNotes[id] || "",
          ...(action === "VALIDER"
            ? { hoursTreatment: resolvedHoursTreatment(item!, managerHoursTreatment) }
            : {}),
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Échec mise à jour");
      if (action === "RELANCER_JUSTIFICATIF") {
        alert("Relance envoyée au demandeur par e-mail.");
      }
      await fetchItems();
    } catch (e: any) { alert(e?.message || "Erreur mise à jour.")}
  };
  const uploadJustification = async (id: string, file: File) => {
    try {
      setUploadingJustificationId(id);
      const presignRes = await fetch("/api/travels/upload", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fileName: file.name, fileType: file.type || "application/octet-stream" }),
      });
      const presignPayload = await presignRes.json();
      if (!presignRes.ok || !presignPayload?.uploadUrl || !presignPayload?.fileUrl) { throw new Error("Impossible de préparer l'upload du justificatif.")}
      await fetch(presignPayload.uploadUrl, {
        method: "PUT",
        body: file,
        headers: { "Content-Type": file.type || "application/octet-stream" },
      });
      const patchRes = await fetch("/api/absences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          id,
          action: "DEPOSER_JUSTIFICATIF",
          justification: {
            fileName: file.name,
            fileUrl: presignPayload.fileUrl,
          },
        }),
      });
      const payload = await patchRes.json().catch(() => ({}));
      if (!patchRes.ok) throw new Error(payload?.error || "Échec dépôt justificatif.");
      await fetchItems();
    } catch (e: any) { alert(e?.message || "Erreur dépôt justificatif.");
    } finally { setUploadingJustificationId(null)}
  };
  const correctScope = async (item: AbsenceItem, newScope: AbsenceScope) => {
    const currentScope = resolveAbsenceScope(asRecord(item));
    if (currentScope === newScope) return;
    let etablissement: Etablissement | null = item.data.etablissement;
    if (newScope === "ogec") {
      etablissement = null;
    } else if (!etablissement) {
      const labels = establishments.map((e) => e.label);
      const picked = prompt(
        `Établissement pour cette absence professeur (${labels.join(", ") || "aucun site configuré"}) :`,
        labels[0] || "",
      );
      if (!picked || !labels.includes(picked)) return;
      etablissement = picked as Etablissement;
    }
    if (!confirm(`Reclasser cette absence en « ${newScope === "ogec" ? "Personnel OGEC" : "Professeur"} » ?`)) return;
    try {
      const res = await fetch("/api/absences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: item.id, action: "CORRIGER_SCOPE", scope: newScope, etablissement }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Correction impossible.");
      await fetchItems();
      setCalendarRefresh((n) => n + 1);
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur correction type.");
    }
  };

  const deleteJustificatif = async (itemId: string) => {
    if (!confirm("Supprimer ce justificatif ?")) return;
    try {
      const res = await fetch("/api/absences/delete-document", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: itemId, target: "justification" }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Suppression impossible.");
      await fetchItems();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur suppression justificatif.");
    }
  };

  const openAbsenceDocument = async (absenceId: string, docIndex = 0) => {
    const newWindow = window.open("", "_blank");
    try {
      const res = await fetch(
        `/api/absences/document-url?id=${encodeURIComponent(absenceId)}&index=${encodeURIComponent(String(docIndex))}`,
        { cache: "no-store" },
      );
      const data = await res.json();
      if (!res.ok || !data?.url) throw new Error(data?.error || "Impossible d'ouvrir le justificatif.");
      if (newWindow) {
        newWindow.location.href = data.url;
      } else {
        window.location.href = data.url;
      }
    } catch (e: unknown) {
      if (newWindow) newWindow.close();
      alert(e instanceof Error ? e.message : "Erreur d'accès au justificatif.");
    }
  };
  const statusStyle = (s: AbsenceWorkflowStatus) =>
    s === "CLOTUREE"
      ? "bg-emerald-50 text-emerald-700 border-emerald-200"
      : s === "JUSTIFICATIF_DEPOSE"
      ? "bg-blue-50 text-blue-700 border-blue-200"
      : "bg-amber-50 text-amber-700 border-amber-200";
  const decisionStyle = (d: AbsenceDecision) =>
    d === "VALIDEE"
      ? "bg-emerald-50 text-emerald-700 border-emerald-200"
      : d === "REFUSEE"
      ? "bg-rose-50 text-rose-700 border-rose-200"
      : "bg-slate-50 text-slate-700 border-slate-200";
  const sorted = useMemo(() => [...items].sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt)), [items]);
  const selfItems = useMemo(() => {
    const mine = sorted.filter((i) => {
      if (i.createdBy.userId !== user?.id) return false;
      const src = (i as { source?: string }).source;
      return !src || src === "self";
    });
    const pending = mine.filter(isPendingAbsence);
    const rest = mine.filter((i) => !isPendingAbsence(i));
    pending.sort((a, b) => compareAbsenceRecordsAlphabetically(asRecord(a), asRecord(b)));
    return [...pending, ...rest];
  }, [sorted, user?.id]);
  const pendingItems = useMemo(
    () =>
      sorted
        .filter((i) => isAbsencePendingForManager(i as unknown as AbsenceRecord, user?.id || "", roles))
        .sort((a, b) => compareAbsenceRecordsAlphabetically(asRecord(a), asRecord(b))),
    [sorted, user?.id, roles],
  );
  const treatedItems = selfItems.filter(
    (i) => !isPendingAbsence(i) && (i.workflowStatus === "CLOTUREE" || itemDecision(i) !== "EN_ATTENTE"),
  );
  const pendingSelfCount = selfItems.filter(isPendingAbsence).length;
  const tabs = [
    { id: "calendrier", label: "Calendrier", show: showCalendar },
    {
      id: "autre-personne",
      label: "Pour un collègue",
      show: showCalendar || canOnBehalf,
    },
    { id: "se-declarer", label: "Se déclarer", show: true },
    { id: "a-traiter", label: "À traiter", show: canTreat },
  ].filter((t) => t.show);

  if (!isLoaded) return null;

  const tabNav = (
    <ModuleTabNav
      className="mb-6"
      navDataTour="absences-tabs"
      tabs={tabs.map((t) => ({
        id: t.id,
        label: t.label,
        dataAttrs: { "data-absences-tab": t.id },
      }))}
      active={activeTab}
      onChange={setTab}
      badges={{
        "a-traiter": pendingItems.length,
        "se-declarer": pendingSelfCount,
      }}
    />
  );

  const body = (
    <>
      {tabNav}

      {activeTab === "calendrier" && showCalendar ? (
        <div data-tour="absences-calendar">
          <AbsencesCalendar refreshKey={calendarRefresh} />
        </div>
      ) : null}

      {activeTab === "autre-personne" && (showCalendar || canOnBehalf) ? (
        <div className="space-y-8">
          {canOnBehalf ? (
            <AbsencesDeclareOnBehalf
              onSuccess={() => {
                setCalendarRefresh((n) => n + 1);
                void fetchItems();
              }}
            />
          ) : null}
          {showCalendar ? (
            <details className="group rounded-3xl border border-slate-200 bg-white open:shadow-sm">
              <summary className="cursor-pointer list-none px-6 py-4 font-bold text-slate-800 marker:content-none [&::-webkit-details-marker]:hidden">
                Saisie calendrier / PDF (sans validation direction)
                <span className="ml-2 text-xs font-semibold text-slate-500">
                  — convocations, import OCR
                </span>
              </summary>
              <div className="border-t border-slate-100 px-6 pb-6 pt-2">
                <AbsencesDeclareOther onSuccess={() => setCalendarRefresh((n) => n + 1)} />
              </div>
            </details>
          ) : null}
        </div>
      ) : null}

      {activeTab === "se-declarer" ? (
      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
        <div id="nouvelle-absence" data-tour="absences-declare" className="xl:col-span-1 bg-white border border-slate-200 rounded-3xl p-6 h-fit scroll-mt-24">
          <h2 className="text-xl font-black text-slate-900 mb-4">Nouvelle absence</h2>
          <div className="space-y-4">
            {canChooseScope ? (
              <div>
                <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                  Type d&apos;absence
                </label>
                <select
                  value={declareScope}
                  onChange={(e) => setDeclareScope(e.target.value as AbsenceScope)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"
                >
                  <option value="ogec">Personnel OGEC</option>
                  <option value="professeur">Professeur</option>
                </select>
                <p className="text-xs text-amber-700 mt-1 font-medium">
                  Votre compte cumule enseignement et personnel OGEC — choisissez la bonne catégorie.
                </p>
              </div>
            ) : (
              <div className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-sm">
                <span className="font-black text-slate-700">Type :</span>{" "}
                <span className="font-semibold text-slate-800">
                  {effectiveScope === "professeur" ? "Professeur" : "Personnel OGEC"}
                </span>
              </div>
            )}
            {effectiveScope === "professeur" && (
              <div>
                <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">Établissement</label>
                <EstablishmentSelect
                  value={etablissement}
                  onChange={setEtablissement}
                  establishments={establishments}
                  includeGroupe={false}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"
                />
              </div>
            )}
            <div>
              <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">Durée</label>
              <select
                value={periodType}
                onChange={(e) => setPeriodType(e.target.value as AbsencePeriodType)}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"
              >
                <option value="single_day">Une journée (créneau horaire)</option>
                <option value="multi_day">Plusieurs jours</option>
              </select>
            </div>
            {periodType === "single_day" ? (
              <>
                <div>
                  <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">Date</label>
                  <input
                    value={singleDayDate}
                    onChange={(e) => setSingleDayDate(e.target.value)}
                    type="date"
                    className="w-full rounded-xl border border-slate-200 px-3 py-2"
                  />
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">De</label>
                    <input
                      value={startTime}
                      onChange={(e) => setStartTime(e.target.value)}
                      type="time"
                      className="w-full rounded-xl border border-slate-200 px-3 py-2"
                    />
                  </div>
                  <div>
                    <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">À</label>
                    <input
                      value={endTime}
                      onChange={(e) => setEndTime(e.target.value)}
                      type="time"
                      className="w-full rounded-xl border border-slate-200 px-3 py-2"
                    />
                  </div>
                </div>
                <p className="text-xs text-slate-500">Ex. rendez-vous médical de 15h à 16h — vous restez disponible le reste de la journée.</p>
              </>
            ) : (
              <>
                <div>
                  <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">Date début</label>
                  <input value={startDate} onChange={(e) => setStartDate(e.target.value)} type="date" className="w-full rounded-xl border border-slate-200 px-3 py-2" />
                </div>
                <div>
                  <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">Date fin</label>
                  <input value={endDate} onChange={(e) => setEndDate(e.target.value)} type="date" className="w-full rounded-xl border border-slate-200 px-3 py-2" />
                </div>
              </>
            )}
            <div>
              <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">Motif</label>
              <input
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                type="text"
                placeholder="Ex: Rendez-vous médical"
                className="w-full rounded-xl border border-slate-200 px-3 py-2"
              />
            </div>
            <div>
              <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">Détails (optionnel)</label>
              <textarea
                value={details}
                onChange={(e) => setDetails(e.target.value)}
                rows={4}
                className="w-full rounded-xl border border-slate-200 px-3 py-2"
              />
            </div>
            <div>
              <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                Pièce justificative (optionnel)
              </label>
              <input type="file" onChange={(e) => setJustificationFile(e.target.files?.[0] || null)} className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"/>
              {justificationFile && (
                <p className="text-xs text-slate-500 mt-1">
                  Fichier sélectionné: <span className="font-semibold">{justificationFile.name}</span>
                </p>
              )}
            </div>
            {error && <div className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">{error}</div>}
            <button
              type="button"
              onClick={submitAbsence}
              disabled={saving}
              className="w-full bg-indigo-600 hover:bg-indigo-700 text-white font-bold py-3 rounded-xl disabled:opacity-60"
            >
              {saving ? "Enregistrement..." : "Déclarer l'absence"}
            </button>
          </div>
        </div>
        <div className="xl:col-span-2 space-y-4">
          <div className="bg-white border border-slate-200 rounded-3xl p-4">
            <h3 className="font-black text-slate-900">Mes demandes</h3>
            <p className="text-xs text-slate-500">Vos déclarations et leur statut.</p>
          </div>
          {loading ? (
            <div className="bg-white border border-slate-200 rounded-3xl p-8 text-slate-500">Chargement…</div>
          ) : selfItems.length === 0 ? (
            <div className="bg-white border border-dashed border-slate-300 rounded-3xl p-8 text-slate-500">Aucune demande enregistrée.</div>
          ) : (
            selfItems.map((item) => (
              <div key={item.id} className="bg-white border border-slate-200 rounded-2xl p-4">
                <div className="flex flex-wrap gap-2 items-center justify-between mb-2">
                  <p className="font-bold text-slate-800">
                    {item.data.scope === "ogec" ? "Personnel OGEC" : `Professeur (${item.data.etablissement})`}
                  </p>
                  <span className={`text-xs font-black px-3 py-1 rounded-xl border ${decisionStyle(itemDecision(item))}`}>
                    {itemDecision(item) === "VALIDEE" ? "VALIDÉE" : itemDecision(item) === "REFUSEE" ? "REFUSÉE" : "EN ATTENTE"}
                  </span>
                </div>
                <p className="text-xs text-slate-500">{formatAbsencePeriod(item.data)}</p>
                <p className="text-sm text-slate-700 mt-1">
                  <span className="font-bold">Motif :</span> {item.data.reason}
                </p>
                {item.justificatifRelanceAt && isPendingAbsence(item) && (
                  <div className="mt-3">
                    <label className="inline-flex items-center gap-2 px-4 py-2 rounded-xl border border-slate-200 text-sm font-bold cursor-pointer hover:bg-slate-50">
                      {uploadingJustificationId === item.id ? "Upload…" : "Déposer justificatif"}
                      <input
                        type="file"
                        className="hidden"
                        onChange={(e) => {
                          const f = e.target.files?.[0];
                          if (!f) return;
                          uploadJustification(item.id, f);
                          e.currentTarget.value = "";
                        }}
                      />
                    </label>
                  </div>
                )}
              </div>
            ))
          )}
          {treatedItems.length > 0 && (
            <div className="mt-6 space-y-3">
              <h3 className="font-black text-slate-900">Historique traité</h3>
              {treatedItems.map((item) => (
                <div key={`t-${item.id}`} className="bg-white border border-slate-200 rounded-2xl p-4 opacity-90">
                  <p className="text-xs text-slate-500">{formatAbsencePeriod(item.data)}</p>
                  {transmissionLabel(item) && <p className="text-sm text-emerald-700 font-semibold mt-1">{transmissionLabel(item)}</p>}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
      ) : null}

      {activeTab === "a-traiter" && canTreat ? (
        <div data-tour="absences-treat" className="space-y-4">
          <div className="bg-white border border-slate-200 rounded-3xl p-4">
            <h3 className="font-black text-slate-900">En attente de décision</h3>
            <p className="text-xs text-slate-500">Absences à valider, refuser ou relancer pour justificatif.</p>
          </div>
          {loading ? (
            <div className="bg-white border border-slate-200 rounded-3xl p-8 text-slate-500">Chargement des absences...</div>
          ) : pendingItems.length === 0 ? (
            <div className="bg-white border border-dashed border-slate-300 rounded-3xl p-8 text-slate-500">Aucune absence en attente.</div>
          ) : (
            pendingItems.map((item) => (
              <div key={item.id} className="bg-white border border-slate-200 rounded-3xl p-5">
                <div className="flex flex-wrap gap-3 items-center justify-between mb-3">
                  <div>
                    <p className="font-black text-slate-900">
                      {item.createdBy.name} —{" "}
                      {resolveAbsenceScope(asRecord(item)) === "ogec"
                        ? "Personnel OGEC"
                        : `Professeur (${item.data.etablissement || "—"})`}
                    </p>
                    <p className="text-xs text-slate-500">
                      {formatAbsencePeriod(item.data)} • {item.createdBy.email || "email non renseigné"}
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <span className={`text-xs font-black px-3 py-1.5 rounded-xl border ${statusStyle(item.workflowStatus)}`}>
                      {item.workflowStatus === "OUVERTE" ? "OUVERTE" : item.workflowStatus === "JUSTIFICATIF_DEPOSE" ? "JUSTIFICATIF DÉPOSÉ" : "CLOTURÉE"}
                    </span>
                    <span className={`text-xs font-black px-3 py-1.5 rounded-xl border ${decisionStyle(itemDecision(item))}`}>
                      {itemDecision(item) === "VALIDEE" ? "VALIDÉE" : itemDecision(item) === "REFUSEE" ? "REFUSÉE" : "DÉCISION EN ATTENTE"}
                    </span>
                  </div>
                </div>
                <p className="text-sm text-slate-700 mb-1">
                  <span className="font-bold">Motif:</span> {item.data.reason}
                </p>
                {item.data.details && (
                  <p className="text-sm text-slate-600 mb-3">
                    <span className="font-bold">Détails:</span> {item.data.details}
                  </p>
                )}
                {item.managerNote && (
                  <p className="text-sm text-indigo-700 mb-3">
                    <span className="font-bold">Note direction/compta:</span> {item.managerNote}
                  </p>
                )}
                {item.justification?.fileUrl && canViewJustificatif(item) && (
                  <p className="text-sm text-slate-700 mb-3 flex flex-wrap items-center gap-2">
                    <span>
                      <span className="font-bold">Justificatif:</span>{" "}
                      <button type="button" onClick={() => openAbsenceDocument(item.id)} className="text-indigo-700 underline font-semibold">
                        {item.justification.fileName || "Voir le fichier"}
                      </button>
                    </span>
                    {canDeleteJustificatif(item) && (
                      <button
                        type="button"
                        onClick={() => deleteJustificatif(item.id)}
                        className="text-xs font-bold text-rose-700 underline"
                      >
                        Supprimer
                      </button>
                    )}
                  </p>
                )}
                {item.justificatifRelanceAt && isPendingAbsence(item) && (
                  <p className="text-sm text-amber-700 mb-3 font-semibold">
                    {item.justification?.fileUrl ? "Complément demandé" : "Justificatif en attente"} (relance envoyée le{" "}
                    {new Date(item.justificatifRelanceAt).toLocaleDateString("fr-FR", {
                      day: "2-digit",
                      month: "2-digit",
                      year: "numeric",
                      hour: "2-digit",
                      minute: "2-digit",
                    })}
                    )
                  </p>
                )}
                {item.createdBy.userId === user?.id && item.justificatifRelanceAt && isPendingAbsence(item) && (
                  <div className="mb-3">
                    <label className="inline-flex items-center gap-2 px-4 py-2 rounded-xl border border-slate-200 text-sm font-bold cursor-pointer hover:bg-slate-50">
                      {uploadingJustificationId === item.id
                        ? "Upload..."
                        : item.justification?.fileUrl
                          ? "Déposer un nouveau justificatif"
                          : "Déposer justificatif"}
                      <input
                        type="file"
                        className="hidden"
                        onChange={(e) => {
                          const f = e.target.files?.[0];
                          if (!f) return;
                          uploadJustification(item.id, f);
                          e.currentTarget.value = "";
                        }}
                      />
                    </label>
                  </div>
                )}
                {canManageItem(item) && isPendingAbsence(item) && (
                  <div className="mt-3 pt-3 border-t border-slate-100">
                    <p className="text-xs text-slate-500 mb-2">
                      Mauvais type déclaré ?{" "}
                      <button
                        type="button"
                        className="font-bold text-indigo-700 underline"
                        onClick={() =>
                          correctScope(item, resolveAbsenceScope(asRecord(item)) === "ogec" ? "professeur" : "ogec")
                        }
                      >
                        Reclasser en {resolveAbsenceScope(asRecord(item)) === "ogec" ? "Professeur" : "Personnel OGEC"}
                      </button>
                    </p>
                    <p className="text-xs text-slate-500 mb-2">
                      Valider ou refuser même sans pièce jointe. « Relancer » invite le demandeur à déposer un justificatif — ou un complément si le premier ne suffit pas.
                    </p>
                    <div className="mb-2">
                      <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-1">
                        {hoursTreatmentFieldLabel(item.data.scope)} <span className="text-rose-600">*</span>
                      </label>
                      <select
                        required
                        value={resolvedHoursTreatment(item, managerHoursTreatment)}
                        onChange={(e) =>
                          setManagerHoursTreatment((p) => ({ ...p, [item.id]: e.target.value }))
                        }
                        className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm bg-white"
                      >
                        <option value="">— Choisir —</option>
                        {getHoursTreatmentOptions(item.data.scope, item.data.etablissement).map((option) => (
                          <option key={option.value} value={option.value}>
                            {option.label}
                          </option>
                        ))}
                      </select>
                      <p className="text-xs text-slate-400 mt-1">
                        {item.data.scope === "ogec"
                          ? "Obligatoire — heures rattrapées ou déduites du salaire."
                          : "Obligatoire — rattrapage en interne ou déclaration ONISE / rectorat."}
                      </p>
                    </div>
                    <textarea
                      rows={2}
                      placeholder="Note interne (optionnel)"
                      value={managerNotes[item.id] ?? item.managerNote ?? ""}
                      onChange={(e) => setManagerNotes((p) => ({ ...p, [item.id]: e.target.value }))}
                      className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                    />
                    <div className="flex flex-wrap gap-2 mt-2">
                      <button
                        type="button"
                        onClick={() => updateWorkflow(item.id, "VALIDER", item)}
                        className="px-4 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-sm"
                      >
                        Valider
                      </button>
                      <button
                        type="button"
                        onClick={() => updateWorkflow(item.id, "REFUSER")}
                        className="px-4 py-2 rounded-xl bg-rose-600 hover:bg-rose-700 text-white font-bold text-sm"
                      >
                        Refuser
                      </button>
                      <button
                        type="button"
                        onClick={() => updateWorkflow(item.id, "RELANCER_JUSTIFICATIF", item)}
                        className="px-4 py-2 rounded-xl bg-amber-500 hover:bg-amber-600 text-white font-bold text-sm"
                      >
                        {item.justification?.fileUrl ? "Demander un complément" : "Relancer pour justificatif"}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      ) : null}
    </>
  );

  if (embeddedInRh) {
    return (
      <div className="space-y-4">
        <div className="mb-2">
          <h2 className="text-xl font-black text-slate-900">Absences</h2>
          <p className="text-sm text-slate-500 mt-1">
            Déclaration, suivi et calendrier — intégré au module RH.
          </p>
        </div>
        {body}
      </div>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1500px]" tourModuleId="absences">
      <ModulePageHeader
        title="Absences"
        description="Déclaration, suivi et calendrier des absences."
      />
      {body}
    </ModulePageShell>
  );
}