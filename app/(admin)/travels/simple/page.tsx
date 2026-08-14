"use client";

import { useUser } from "@clerk/nextjs";
import { useState, useEffect, useMemo, Suspense } from "react";
import {
  enumerateWeeklyDatesInRange,
  WEEKDAY_JS_OPTIONS,
} from "@/app/lib/simple-trip-recurrence";
import { useRouter, useSearchParams } from "next/navigation";
import { GROUPE_SCOLAIRE_LABEL } from "@/app/lib/travels-establishments";
import { useAppContext } from "@/app/hooks/useAppContext";
import TravelsOwnerAssignSection, {
  resolveTripOwnerFields,
  type TravelsOwnerFields,
} from "@/app/components/travels/TravelsOwnerAssignSection";
import TripClassesMultiSelect from "@/app/components/travels/TripClassesMultiSelect";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { mergeTripClassCatalogs } from "@/app/lib/travels-classes";

const CUISINE_DAYS = [
  { key: "lundi",    label: "Lun." },
  { key: "mardi",    label: "Mar." },
  { key: "mercredi", label: "Mer." },
  { key: "jeudi",    label: "Jeu." },
  { key: "vendredi", label: "Ven." },
];

const CUISINE_ROWS = [
  { key: "picnicTotal",  label: "Pique-nique (total)",     type: "number" },
  { key: "picnicNoPork", label: "dont Sans porc",           type: "number" },
  { key: "picnicVeg",    label: "dont Végétarien",          type: "number" },
  { key: "selfAdults",   label: "Repas au self (adultes)",  type: "number" },
  { key: "selfStudents", label: "Repas au self (élèves)",   type: "number" },
  { key: "coffee",       label: "Café / thé / chocolat",   type: "number" },
  { key: "juice",        label: "Jus de fruits",            type: "number" },
  { key: "cakes",        label: "Petits gâteaux",           type: "number" },
  { key: "pastries",     label: "Viennoiserie",             type: "number" },
  { key: "other",        label: "Autre",                    type: "text"   },
];

function SimpleTripFormContent() {
  const { user, isLoaded } = useUser();
  const { data: appCtx } = useAppContext();
  const classOptions = useMemo(
    () =>
      mergeTripClassCatalogs(
        appCtx?.profRoom?.classesByPole,
        appCtx?.domainPlanning?.classesByPole,
      ),
    [appCtx?.profRoom?.classesByPole, appCtx?.domainPlanning?.classesByPole],
  );
  const router = useRouter();
  const searchParams = useSearchParams();
  const editId = searchParams.get("edit");
  const [recurrenceEnabled, setRecurrenceEnabled] = useState(false);
  const [recurrenceWeekday, setRecurrenceWeekday] = useState(4);
  const [recurrenceFrom, setRecurrenceFrom] = useState("");
  const [recurrenceTo, setRecurrenceTo] = useState("");
  const [skipPublicHolidays, setSkipPublicHolidays] = useState(true);
  const [loading, setLoading] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [ownerOverride, setOwnerOverride] = useState<TravelsOwnerFields | null>(null);
  const [loadedOwner, setLoadedOwner] = useState<TravelsOwnerFields | null>(null);
  const [ownerAssignPending, setOwnerAssignPending] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [formData, setFormData] = useState({
    title: "",
    etablissement: "",
    destination: "",
    date: "",
    startTime: "",
    endTime: "",
    nbEleves: "",
    classes: "",
    nbAccompagnateurs: 1,
    nomsAccompagnateurs: "",
    coutTotal: 0,
    piqueNiqueDetails: {
      active: false,
      deliveryTime: "",
      deliveryPlace: "Self",
      daysSelection: { lundi: false, mardi: false, mercredi: false, jeudi: false, vendredi: false },
      orders: {
        lundi:    { picnicTotal: "", picnicNoPork: "", picnicVeg: "", selfAdults: "", selfStudents: "", coffee: "", juice: "", cakes: "", pastries: "", other: "" },
        mardi:    { picnicTotal: "", picnicNoPork: "", picnicVeg: "", selfAdults: "", selfStudents: "", coffee: "", juice: "", cakes: "", pastries: "", other: "" },
        mercredi: { picnicTotal: "", picnicNoPork: "", picnicVeg: "", selfAdults: "", selfStudents: "", coffee: "", juice: "", cakes: "", pastries: "", other: "" },
        jeudi:    { picnicTotal: "", picnicNoPork: "", picnicVeg: "", selfAdults: "", selfStudents: "", coffee: "", juice: "", cakes: "", pastries: "", other: "" },
        vendredi: { picnicTotal: "", picnicNoPork: "", picnicVeg: "", selfAdults: "", selfStudents: "", coffee: "", juice: "", cakes: "", pastries: "", other: "" },
      }
    },
    description: "",
    attachments: [] as { name: string; url: string }[]
  });
  useEffect(() => {
    if (editId && isLoaded) {
      setFetching(true);
      fetch(`/api/travels/get?id=${editId}`)
        .then((res) => res.json())
        .then((trip) => {
          if (trip && trip.data) {
            const rawNoms = trip.data.nomsAccompagnateurs;
            const nomsStr =
              typeof rawNoms === "string"
                ? rawNoms
                : Array.isArray(rawNoms)
                  ? rawNoms.filter(Boolean).join(", ")
                  : "";
            setFormData({
              ...trip.data,
              nomsAccompagnateurs: nomsStr,
              nbAccompagnateurs: Number(trip.data.nbAccompagnateurs) >= 0 ? Number(trip.data.nbAccompagnateurs) : 1,
              attachments: trip.data.attachments || [],
              piqueNiqueDetails: trip.data.piqueNiqueDetails || formData.piqueNiqueDetails,
            });
            if (trip.ownerId) {
              setLoadedOwner({
                ownerId: trip.ownerId,
                ownerName: trip.ownerName || "Enseignant",
                ownerEmail: trip.ownerEmail || "",
              });
            }
          }
          setFetching(false);
        })
        .catch((err) => {
          console.error("Erreur de récupération:", err);
          setFetching(false);
        });
    }
  }, [editId, isLoaded]);
  const recurrencePreview = useMemo(() => {
    if (!recurrenceEnabled || editId) return [];
    return enumerateWeeklyDatesInRange(
      recurrenceWeekday,
      recurrenceFrom,
      recurrenceTo,
      skipPublicHolidays
    );
  }, [recurrenceEnabled, editId, recurrenceWeekday, recurrenceFrom, recurrenceTo, skipPublicHolidays]);
  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
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
      await fetch(uploadUrl, {
        method: 'PUT',
        body: file,
        headers: { 'Content-Type': file.type }
      });
      setFormData(prev => ({
        ...prev,
        attachments: [...(prev.attachments || []), { name: file.name, url: fileUrl }]
      }));
    } catch (error) {
      console.error(error);
      alert("Erreur lors de l'envoi du fichier.");
    } finally {
      setUploading(false);
    }
  };
  const removeFile = (index: number) => {
    const updatedFiles = (formData.attachments || []).filter((_, i) => i !== index);
    setFormData({ ...formData, attachments: updatedFiles });
  };
  if (!isLoaded || fetching) return <div className="p-10 text-center font-medium">Chargement du dossier...</div>;
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (ownerAssignPending) {
      alert("Sélectionnez l'enseignant responsable du dossier.");
      return;
    }
    const ownerFields =
      editId && loadedOwner?.ownerId
        ? loadedOwner
        : resolveTripOwnerFields(user, ownerOverride);
    if (formData.coutTotal > 0 && (!formData.nbEleves || Number(formData.nbEleves) <= 0)) {
      alert("Veuillez préciser le nombre d'élèves pour un voyage avec budget.");
      return;
    }
    if (!editId && recurrenceEnabled) {
      if (!recurrenceFrom || !recurrenceTo) {
        alert("Indiquez la date de début et la date de fin de la période pour la série.");
        return;
      }
      const dates = enumerateWeeklyDatesInRange(
        recurrenceWeekday,
        recurrenceFrom,
        recurrenceTo,
        skipPublicHolidays
      );
      if (dates.length < 1) {
        alert(
          "Aucune date dans cette période pour le jour choisi (ou uniquement des jours fériés exclus). Ajustez la plage ou le jour."
        );
        return;
      }
    }
    setLoading(true);
    try {
      if (!editId && recurrenceEnabled) {
        const dates = enumerateWeeklyDatesInRange(
          recurrenceWeekday,
          recurrenceFrom,
          recurrenceTo,
          skipPublicHolidays
        );
        const seriesId = crypto.randomUUID();
        const now = new Date().toISOString();
        let imageReuse: { imageUrl?: string; imageConfigId?: string } = {};
        for (let i = 0; i < dates.length; i++) {
          const id = crypto.randomUUID();
          const dataPayload = {
            ...formData,
            date: dates[i],
            recurrenceSeriesId: seriesId,
            recurrenceIndex: i + 1,
            recurrenceTotal: dates.length,
          };
          const tripData: Record<string, unknown> = {
            id,
            ownerId: ownerFields.ownerId,
            ownerName: ownerFields.ownerName,
            ownerEmail: ownerFields.ownerEmail,
            type: "SIMPLE",
            status: "EN_ATTENTE_DIR_INITIAL",
            data: dataPayload,
            updatedAt: now,
            createdAt: now,
            history: [
              {
                date: now,
                user: user?.fullName,
                action: "CREE",
                note:
                  ownerOverride?.ownerId && ownerOverride.ownerId !== user?.id
                    ? `Créé par l'administratif pour ${ownerFields.ownerName} (${i + 1}/${dates.length})`
                    : `Série récurrente (${i + 1}/${dates.length})`,
              },
            ],
          };
          if (imageReuse.imageUrl) {
            tripData.imageUrl = imageReuse.imageUrl;
            tripData.imageConfigId = imageReuse.imageConfigId;
          }
          const response = await fetch("/api/travels/update", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              id,
              data: tripData,
              suppressNewTripEmail: i > 0,
            }),
          });
          if (!response.ok) {
            alert(`Erreur lors de la sauvegarde du dossier ${i + 1} / ${dates.length}. Les dossiers précédents peuvent déjà être créés.`);
            return;
          }
          const payload = await response.json();
          if (i === 0 && payload.imageUrl) {
            imageReuse = { imageUrl: payload.imageUrl, imageConfigId: payload.imageConfigId };
          }
        }
        router.push("/travels");
        router.refresh();
        return;
      }
      const tripId = editId || crypto.randomUUID();
      const tripData = {
        id: tripId,
        ownerId: ownerFields.ownerId,
        ownerName: ownerFields.ownerName,
        ownerEmail: ownerFields.ownerEmail,
        type: "SIMPLE",
        status: "EN_ATTENTE_DIR_INITIAL",
        data: formData,
        updatedAt: new Date().toISOString(),
        createdAt: editId ? undefined : new Date().toISOString(),
        history: [],
      };
      const response = await fetch("/api/travels/update", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: tripData.id, data: tripData }),
      });
      if (response.ok) {
        router.push("/travels");
        router.refresh();
      } else {
        alert("Erreur lors de la sauvegarde");
      }
    } catch (error) {
      console.error(error);
      alert("Erreur réseau");
    } finally {
      setLoading(false);
    }
  };
  return (
    <ModulePageShell maxWidthClass="max-w-4xl">
      <ModulePageHeader
        eyebrow="Voyages"
        title={editId ? "Modifier la sortie" : "Nouvelle Sortie Simple"}
        description="Proximité, sans transport complexe ni nuitée."
        actions={
          <button type="button" onClick={() => router.back()} className="text-sm font-bold text-indigo-600 hover:underline">
            Annuler
          </button>
        }
      />
      <form onSubmit={handleSubmit} className="grid grid-cols-1 md:grid-cols-2 gap-6 bg-white p-8 border rounded-3xl shadow-sm">
        <div className="md:col-span-2 space-y-4 border-b pb-4 text-slate-400 uppercase text-xs font-bold tracking-widest">
          Informations Générales
        </div>
        <TravelsOwnerAssignSection
          disabled={Boolean(editId)}
          onOwnerFieldsChange={setOwnerOverride}
          onPendingChange={setOwnerAssignPending}
        />
        <div className="md:col-span-2">
          <label className="block text-sm font-semibold mb-2">Intitulé de la sortie</label>
          <input required value={formData.title} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" placeholder="Ex: Sortie Laser Game" onChange={e => setFormData({...formData, title: e.target.value})} />
        </div>
        <div>
          <label className="block text-sm font-semibold mb-2">Établissement concerné</label>
          <select required value={formData.etablissement} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" onChange={e => setFormData({...formData, etablissement: e.target.value})}>
            <option value="">— Sélectionner —</option>
            {(appCtx?.establishments || []).map((e) => (
              <option key={e.id} value={e.label}>
                {e.label}
              </option>
            ))}
            {(appCtx?.establishments?.length ?? 0) > 1 && (
              <option value={GROUPE_SCOLAIRE_LABEL}>{GROUPE_SCOLAIRE_LABEL}</option>
            )}
          </select>
        </div>
        <div className="md:col-span-2">
          <label className="block text-sm font-semibold mb-2">Lieu et programme (Champ libre)</label>
          <textarea required rows={2} value={formData.destination} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" placeholder="Détails du lieu..." onChange={e => setFormData({...formData, destination: e.target.value})} />
        </div>
        {!editId && (
          <div className="md:col-span-2 flex flex-col gap-3 p-4 bg-indigo-50/60 border border-indigo-100 rounded-2xl">
            <label className="flex items-start gap-3 cursor-pointer">
              <input
                type="checkbox"
                className="mt-1 w-5 h-5 accent-indigo-600"
                checked={recurrenceEnabled}
                onChange={(e) => setRecurrenceEnabled(e.target.checked)}
              />
              <span>
                <span className="block text-sm font-bold text-slate-900">Série récurrente (même sortie, plusieurs dates)</span>
                <span className="block text-xs text-slate-600 mt-0.5">
                  Une période du <strong>au</strong> (vous évitez les vacances en choisissant les dates). Les{" "}
                  <strong>jours fériés France métropolitaine</strong> peuvent être exclus automatiquement.
                </span>
              </span>
            </label>
            {recurrenceEnabled && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-2 border-t border-indigo-100">
                <div>
                  <label className="block text-sm font-semibold mb-2">Jour de la semaine</label>
                  <select
                    value={recurrenceWeekday}
                    className="w-full p-3 bg-white border rounded-xl outline-indigo-500"
                    onChange={(e) => setRecurrenceWeekday(Number(e.target.value))}
                  >
                    {WEEKDAY_JS_OPTIONS.map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="flex items-end">
                  <label className="flex items-center gap-2 text-sm font-medium text-slate-700 cursor-pointer">
                    <input
                      type="checkbox"
                      className="w-4 h-4 accent-indigo-600"
                      checked={skipPublicHolidays}
                      onChange={(e) => setSkipPublicHolidays(e.target.checked)}
                    />
                    Exclure les jours fériés
                  </label>
                </div>
                <div>
                  <label className="block text-sm font-semibold mb-2">Début de période</label>
                  <input
                    type="date"
                    value={recurrenceFrom}
                    className="w-full p-3 bg-white border rounded-xl outline-indigo-500"
                    onChange={(e) => setRecurrenceFrom(e.target.value)}
                  />
                </div>
                <div>
                  <label className="block text-sm font-semibold mb-2">Fin de période</label>
                  <input
                    type="date"
                    value={recurrenceTo}
                    className="w-full p-3 bg-white border rounded-xl outline-indigo-500"
                    onChange={(e) => setRecurrenceTo(e.target.value)}
                  />
                </div>
                <div className="md:col-span-2 text-sm text-slate-700 bg-white/80 rounded-xl p-3 border border-indigo-100">
                  <span className="font-bold text-indigo-800">{recurrencePreview.length}</span> séance
                  {recurrencePreview.length > 1 ? "s" : ""} prévue
                  {recurrencePreview.length > 1 ? "s" : ""}
                  {recurrencePreview.length > 0 && (
                    <span className="text-slate-500">
                      {" "}
                      (ex. {recurrencePreview.slice(0, 4).join(", ")}
                      {recurrencePreview.length > 4 ? "…" : ""})
                    </span>
                  )}
                </div>
              </div>
            )}
          </div>
        )}
        <div>
          <label className="block text-sm font-semibold mb-2">
            {recurrenceEnabled && !editId ? "Date de référence (optionnel)" : "Date de la sortie"}
          </label>
          <input
            required={!recurrenceEnabled || !!editId}
            type="date"
            value={formData.date}
            className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500"
            onChange={(e) => setFormData({ ...formData, date: e.target.value })}
          />
          {recurrenceEnabled && !editId && (
            <p className="text-xs text-slate-500 mt-1">
              En série, chaque dossier a sa propre date ; ce champ reste optionnel (ex. pour un brouillon).
            </p>
          )}
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div>
            <label className="block text-sm font-semibold mb-2">Départ</label>
            <input required type="time" value={formData.startTime} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" onChange={e => setFormData({...formData, startTime: e.target.value})} />
          </div>
          <div>
            <label className="block text-sm font-semibold mb-2">Retour</label>
            <input required type="time" value={formData.endTime} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" onChange={e => setFormData({...formData, endTime: e.target.value})} />
          </div>
        </div>
        <div>
          <label className="block text-sm font-semibold mb-2">Coût total estimé (€)</label>
          <input type="number" value={formData.coutTotal} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" onChange={e => setFormData({...formData, coutTotal: Number(e.target.value)})} />
        </div>
        <div className="md:col-span-2 space-y-4 border-b pb-4 mt-4 text-slate-400 uppercase text-xs font-bold tracking-widest">
          Participants & Encadrement
        </div>
        <div>
          <label className="block text-sm font-semibold mb-2">Classes concernées</label>
          <TripClassesMultiSelect
            value={formData.classes}
            options={classOptions}
            onChange={(classes) => setFormData({ ...formData, classes })}
          />
        </div>
        <div>
          <label className="block text-sm font-semibold mb-2">Nombre d&apos;élèves total</label>
          <input type="number" value={formData.nbEleves} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" onChange={e => setFormData({...formData, nbEleves: e.target.value})} />
        </div>
        <div>
          <label className="block text-sm font-semibold mb-2">Nombre d&apos;accompagnateurs</label>
          <input
            type="number"
            min={0}
            value={formData.nbAccompagnateurs}
            className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500"
            onChange={(e) => setFormData({ ...formData, nbAccompagnateurs: Number(e.target.value) })}
          />
        </div>
        <div className="md:col-span-2">
          <label className="block text-sm font-semibold mb-2">Noms des accompagnateurs</label>
          <textarea
            rows={2}
            value={formData.nomsAccompagnateurs}
            className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500"
            placeholder="Ex. M. Dupont, Mme Martin, …"
            onChange={(e) => setFormData({ ...formData, nomsAccompagnateurs: e.target.value })}
          />
        </div>
        <div className="md:col-span-2 mt-4">
          <div className="flex items-center gap-4 p-4 bg-orange-50 border border-orange-200 rounded-2xl">
            <input type="checkbox" id="pique" checked={formData.piqueNiqueDetails.active} className="w-6 h-6 accent-orange-600" onChange={e => setFormData({...formData, piqueNiqueDetails: {...formData.piqueNiqueDetails, active: e.target.checked}})} />
            <label htmlFor="pique" className="text-sm font-bold text-orange-900 cursor-pointer flex flex-col">
              Besoin de prestations fournies par la cantine ?
              <span className="text-xs font-normal text-orange-700">Pique-niques, repas self, goûters...</span>
            </label>
          </div>
        </div>
        {formData.piqueNiqueDetails.active && (
          <div className="md:col-span-2 space-y-5 bg-slate-50 p-6 rounded-3xl border border-slate-200 animate-in fade-in duration-300">
            <h3 className="font-bold text-slate-800 border-b pb-2">Bon de commande Cuisine</h3>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Heure récupération / livraison</label>
                <input type="time" value={formData.piqueNiqueDetails.deliveryTime} className="w-full p-2 border rounded-lg" onChange={e => setFormData({...formData, piqueNiqueDetails: {...formData.piqueNiqueDetails, deliveryTime: e.target.value}})} />
              </div>
              <div>
                <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Lieu de récupération</label>
                <select value={formData.piqueNiqueDetails.deliveryPlace} className="w-full p-2 border rounded-lg" onChange={e => setFormData({...formData, piqueNiqueDetails: {...formData.piqueNiqueDetails, deliveryPlace: e.target.value}})}>
                  <option value="Self">Au self</option>
                  <option value="Bosco">Église Bosco</option>
                </select>
              </div>
              <div>
                <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Jours concernés</label>
                <div className="flex gap-1.5">
                  {CUISINE_DAYS.map(({ key: dayKey, label }) => {
                    const isSelected = (formData.piqueNiqueDetails.daysSelection as Record<string, boolean>)[dayKey];
                    return (
                      <button key={dayKey} type="button"
                        onClick={() => setFormData(prev => ({ ...prev, piqueNiqueDetails: { ...prev.piqueNiqueDetails, daysSelection: { ...prev.piqueNiqueDetails.daysSelection, [dayKey]: !isSelected } } }))}
                        className={`w-9 h-9 rounded-lg text-[11px] font-black transition-all ${isSelected ? 'bg-indigo-600 text-white shadow-md' : 'bg-white border-2 text-slate-400 hover:border-indigo-300'}`}>
                        {label}
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>
            <div className="overflow-x-auto rounded-xl border border-slate-200">
              <table className="w-full text-xs border-collapse min-w-[480px]">
                <thead>
                  <tr className="bg-indigo-600 text-white">
                    <th className="text-left p-2.5 font-semibold w-40">Désignation</th>
                    {CUISINE_DAYS.map(({ key: dayKey, label }) => (
                      <th key={dayKey} className={`p-2.5 text-center font-semibold transition-opacity ${(formData.piqueNiqueDetails.daysSelection as Record<string, boolean>)[dayKey] ? 'opacity-100' : 'opacity-30'}`}>{label}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {CUISINE_ROWS.map(({ key: rowKey, label, type }, rowIdx) => (
                    <tr key={rowKey} className={rowIdx % 2 === 0 ? 'bg-white' : 'bg-slate-50'}>
                      <td className={`p-2 font-medium text-slate-700 whitespace-nowrap ${rowKey === 'picnicNoPork' || rowKey === 'picnicVeg' ? 'pl-5 text-slate-500 italic' : ''}`}>{label}</td>
                      {CUISINE_DAYS.map(({ key: dayKey }) => {
                        const isActive = (formData.piqueNiqueDetails.daysSelection as Record<string, boolean>)[dayKey];
                        const val = (formData.piqueNiqueDetails.orders as Record<string, Record<string, string>>)[dayKey][rowKey];
                        return (
                          <td key={dayKey} className="p-1">
                            <input
                              type={type}
                              disabled={!isActive}
                              value={val}
                              onChange={e => setFormData(prev => ({
                                ...prev,
                                piqueNiqueDetails: {
                                  ...prev.piqueNiqueDetails,
                                  orders: {
                                    ...prev.piqueNiqueDetails.orders,
                                    [dayKey]: {
                                      ...(prev.piqueNiqueDetails.orders as Record<string, Record<string, string>>)[dayKey],
                                      [rowKey]: e.target.value
                                    }
                                  }
                                }
                              }))}
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
            <p className="text-[10px] text-slate-500 italic bg-amber-50 border border-amber-100 p-2.5 rounded-lg">
              ⚠️ Fournir la liste des élèves et adultes <strong>15 jours avant</strong> la sortie. Affiner 24h avant (toute absence non signalée 24h avant sera facturée). Commande à envoyer à : <strong>chef.0056isi@newrest.eu</strong>
            </p>
          </div>
        )}
        <div className="md:col-span-2 space-y-4 border-t pt-8 mt-4">
          <label className="block text-sm font-bold text-slate-700">Pièces jointes (Devis, Programme, liste des élèves...)</label>
          <div className="flex flex-wrap gap-4">
            {formData.attachments?.map((file, idx) => (
              <div key={idx} className="flex items-center gap-2 p-3 bg-indigo-50 text-indigo-700 rounded-xl border border-indigo-100 text-sm font-medium">
                <span className="truncate max-w-[200px]">📄 {file.name}</span>
                <button type="button" onClick={() => removeFile(idx)} className="text-indigo-400 hover:text-indigo-600 ml-2">✕</button>
              </div>
            ))}
            <label className="cursor-pointer flex items-center justify-center border-2 border-dashed border-slate-200 rounded-xl p-6 hover:bg-slate-50 transition-all w-full md:w-auto min-w-[200px]">
              <input type="file" className="hidden" onChange={handleFileUpload} disabled={uploading} />
              <span className="text-slate-500 text-sm font-medium text-center">
                {uploading ? "Envoi en cours..." : "+ Ajouter un document (PDF, Image)"}
              </span>
            </label>
          </div>
        </div>
        <div className="md:col-span-2 mt-8">
          <button type="submit" disabled={loading || uploading} className="w-full py-4 bg-indigo-600 text-white rounded-2xl font-bold shadow-lg shadow-indigo-200 hover:bg-indigo-700 disabled:bg-slate-300 transition-all">
            {loading
              ? "Enregistrement en cours..."
              : editId
                ? "Mettre à jour la demande"
                : recurrenceEnabled
                  ? `Créer la série (${recurrencePreview.length} dossier${recurrencePreview.length > 1 ? "s" : ""})`
                  : "Soumettre la demande"}
          </button>
        </div>
      </form>
    </ModulePageShell>
  );
}

export default function SimpleTripForm() {
  return (
    <Suspense fallback={<div>Chargement...</div>}>
      <SimpleTripFormContent />
    </Suspense>
  );
}