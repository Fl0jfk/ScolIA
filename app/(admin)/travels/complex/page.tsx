"use client";

import { useUser } from "@clerk/nextjs";
import { useState, Suspense, useRef, useMemo } from "react";
import { useRouter } from "next/navigation";
import { useAppContext } from "@/app/hooks/useAppContext";
import { GROUPE_SCOLAIRE_LABEL } from "@/app/lib/travels-establishments";
import { mergeTripClassCatalogs } from "@/app/lib/travels-classes";

import { CUISINE_DAYS_UI as CUISINE_DAYS, CUISINE_ROWS_UI as CUISINE_ROWS } from "@/app/lib/travels-cuisine-form";
import TravelsOwnerAssignSection, {
  resolveTripOwnerFields,
  type TravelsOwnerFields,
} from "@/app/components/travels/TravelsOwnerAssignSection";
import TripClassesMultiSelect from "@/app/components/travels/TripClassesMultiSelect";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

function ComplexTripFormContent() {
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
  const fileInputRef = useRef<HTMLInputElement>(null);
  const busProgramRef = useRef<HTMLInputElement>(null);
  const [loading, setLoading] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [ownerOverride, setOwnerOverride] = useState<TravelsOwnerFields | null>(null);
  const [ownerAssignPending, setOwnerAssignPending] = useState(false);
  const [showPiqueNiqueModal, setShowPiqueNiqueModal] = useState(false);
  const [showBusRecapModal, setShowBusRecapModal] = useState(false);
  const [formData, setFormData] = useState({
    title: "",
    etablissement: "",
    destination: "",
    objectifs: "",
    startDate: "",
    startTime: "",
    endDate: "",
    endTime: "",
    nbEleves: "",
    classes: "",
    nbAccompagnateurs: 1,
    nomsAccompagnateurs: "",
    needsBus: false,
    transportRequest: {
      pickupPoint: "",
      stayOnSite: false,
      freeText: "", 
      busProgramFile: null as { name: string, url: string } | null,
    },
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
    coutTotal: 0,
    partFamille: 0,
    partEtablissement: 0,
    attachments: [] as { name: string, url: string }[]
  });

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>, isBusProgram = false) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    setUploading(true);
    const newAttachments = [...formData.attachments];
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
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
        if (isBusProgram) {
          setFormData(prev => ({
            ...prev,
            transportRequest: { ...prev.transportRequest, busProgramFile: { name: file.name, url: fileUrl } }
          }));
        } else {
          newAttachments.push({ name: file.name, url: fileUrl });
        }
      } catch (error) {
        console.error("Erreur upload:", error);
        alert(`Erreur pour le fichier ${file.name}`);
      }
    }
    if (!isBusProgram) setFormData(prev => ({ ...prev, attachments: newAttachments }));
    setUploading(false);
  };

  const removeFile = (index: number) => {
    const updated = formData.attachments.filter((_, i) => i !== index);
    setFormData({ ...formData, attachments: updated });
  };

  const executeSubmit = async () => {
    setLoading(true);
    const tripId = `trip-${Date.now()}`;
    const ownerFields = resolveTripOwnerFields(user, ownerOverride);
    const payload = {
      id: tripId,
      ownerId: ownerFields.ownerId,
      ownerName: ownerFields.ownerName,
      ownerEmail: ownerFields.ownerEmail,
      status: "EN_ATTENTE_DIR_INITIAL",
      type: "COMPLEX",
      createdAt: new Date().toISOString(),
      data: formData
    };

    try {
      const res = await fetch("/api/travels/update", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: tripId, data: payload }),
      });
      if (res.ok) {
        if (formData.needsBus) {
          await fetch("/api/travels/send-transport", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ 
              tripData: payload,
              userEmail: ownerFields.ownerEmail,
              userName: ownerFields.ownerName,
            }),
          });
        }
        router.push("/travels");
        router.refresh();
      } else {
        alert("Erreur lors de la sauvegarde.");
      }
    } catch (err) {
      alert("Impossible de contacter le serveur.");
    } finally {
      setLoading(false);
    }
  };
  const handleSubmitAttempt = (e: React.FormEvent) => {
    e.preventDefault();
    if (ownerAssignPending) {
      alert("Sélectionnez l'enseignant responsable du dossier.");
      return;
    }
    if (formData.needsBus) {
      setShowBusRecapModal(true);
    } else {
      executeSubmit();
    }
  };
  if (!isLoaded) return <div className="p-10 text-center font-medium">Chargement...</div>;
  return (
    <ModulePageShell maxWidthClass="max-w-[1280px]" className="pb-24">
      <ModulePageHeader
        eyebrow="Voyages"
        title="Nouveau Voyage Scolaire"
        description="Séjours, nuitées et projets complexes."
        actions={
          <button type="button" onClick={() => router.back()} className="text-sm font-bold text-indigo-600 hover:underline">
            Annuler
          </button>
        }
      />
      <form onSubmit={handleSubmitAttempt} className="space-y-6">
        <div className="bg-white p-8 border rounded-3xl shadow-sm space-y-6">
          <div className="text-slate-400 uppercase text-xs font-bold tracking-widest border-b pb-4">1. Projet Pédagogique</div>
          <TravelsOwnerAssignSection
            onOwnerFieldsChange={setOwnerOverride}
            onPendingChange={setOwnerAssignPending}
          />
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="md:col-span-2">
              <label className="block text-sm font-semibold mb-2">Intitulé du séjour</label>
              <input required value={formData.title} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" placeholder="Ex: Séjour d'intégration en Auvergne" onChange={e => setFormData({...formData, title: e.target.value})} />
            </div>
            <div>
              <label className="block text-sm font-semibold mb-2">Établissement concerné</label>
              <select required value={formData.etablissement} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" onChange={e => setFormData({...formData, etablissement: e.target.value})}>
                <option value="">— Sélectionner —</option>
                {(appCtx?.establishments || []).map((e) => (
                  <option key={e.id} value={e.label}>{e.label}</option>
                ))}
                {(appCtx?.establishments?.length ?? 0) > 1 && (
                  <option value={GROUPE_SCOLAIRE_LABEL}>{GROUPE_SCOLAIRE_LABEL}</option>
                )}
              </select>
            </div>
            <div className="md:col-span-2">
              <label className="block text-sm font-semibold mb-2">Objectifs pédagogiques</label>
              <textarea required rows={3} value={formData.objectifs} className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500" placeholder="Quels sont les buts de ce voyage ?" onChange={e => setFormData({...formData, objectifs: e.target.value})} />
            </div>
            <div className="md:col-span-2">
              <label className="block text-sm font-semibold mb-2">Lieu et destination exacte</label>
              <input required value={formData.destination} className="w-full p-3 bg-slate-50 border rounded-xl" placeholder="Adresse du centre, ville, pays..." onChange={e => setFormData({...formData, destination: e.target.value})} />
            </div>
          </div>
        </div>
        <div className="bg-white p-8 border rounded-3xl shadow-sm space-y-6">
          <div className="text-slate-400 uppercase text-xs font-bold tracking-widest border-b pb-4">2. Participants & Budget estimé</div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div>
              <label className="block text-sm font-semibold mb-2">Nombre d'élèves</label>
              <input type="number" required value={formData.nbEleves} className="w-full p-3 bg-slate-50 border rounded-xl" onChange={e => setFormData({...formData, nbEleves: e.target.value})} />
            </div>
            <div>
              <label className="block text-sm font-semibold mb-2">Classes concernées</label>
              <TripClassesMultiSelect
                required
                value={formData.classes}
                options={classOptions}
                onChange={(classes) => setFormData({ ...formData, classes })}
              />
            </div>
            <div>
              <label className="block text-sm font-semibold mb-2">Nb accompagnateurs</label>
              <input type="number" required value={formData.nbAccompagnateurs} className="w-full p-3 bg-slate-50 border rounded-xl" onChange={e => setFormData({...formData, nbAccompagnateurs: Number(e.target.value)})} />
            </div>
            <div className="md:col-span-2">
              <label className="block text-sm font-semibold mb-2">Noms des accompagnateurs</label>
              <input required value={formData.nomsAccompagnateurs} className="w-full p-3 bg-slate-50 border rounded-xl" placeholder="Liste des accompagnateurs..." onChange={e => setFormData({...formData, nomsAccompagnateurs: e.target.value})} />
            </div>
            <div>
              <label className="block text-sm font-semibold mb-2 text-indigo-600">Coût global estimé (€)</label>
              <input type="number" required className="w-full p-3 bg-indigo-50 border border-indigo-100 rounded-xl font-bold" placeholder="Total du projet" value={formData.coutTotal} onChange={e => setFormData({...formData, coutTotal: Number(e.target.value)})} />
            </div>
          </div>
        </div>
        <div className="bg-white p-8 border rounded-3xl shadow-sm space-y-6">
          <div className="text-slate-400 uppercase text-xs font-bold tracking-widest border-b pb-4">3. Dates et Horaires</div>
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div className="md:col-span-2"><label className="block text-xs font-bold mb-1">Date Départ</label><input type="date" required className="w-full p-3 bg-slate-50 border rounded-xl" value={formData.startDate} onChange={e => setFormData({...formData, startDate: e.target.value})} /></div>
            <div className="md:col-span-2"><label className="block text-xs font-bold mb-1">Heure de RDV</label><input type="time" required className="w-full p-3 bg-slate-50 border rounded-xl" value={formData.startTime} onChange={e => setFormData({...formData, startTime: e.target.value})} /></div>
            <div className="md:col-span-2"><label className="block text-xs font-bold mb-1">Date Retour</label><input type="date" required className="w-full p-3 bg-slate-50 border rounded-xl" value={formData.endDate} onChange={e => setFormData({...formData, endDate: e.target.value})} /></div>
            <div className="md:col-span-2"><label className="block text-xs font-bold mb-1">Heure retour prévue</label><input type="time" required className="w-full p-3 bg-slate-50 border rounded-xl" value={formData.endTime} onChange={e => setFormData({...formData, endTime: e.target.value})} /></div>
          </div>
        </div>
        <div className="bg-white p-8 border rounded-3xl shadow-sm space-y-6">
          <div className="text-slate-400 uppercase text-xs font-bold tracking-widest border-b pb-4">4. Logistique & Restauration</div>
          <div className={`p-6 rounded-2xl border-2 transition-all ${formData.needsBus ? 'border-amber-400 bg-amber-50' : 'border-slate-100 bg-slate-50'}`}>
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-bold text-slate-900">Besoin d'un Autocar ?</h3>
                <p className="text-xs text-slate-500">Activer pour soumettre une demande de devis transporteur.</p>
              </div>
              <input type="checkbox" className="w-6 h-6 accent-amber-600 cursor-pointer" checked={formData.needsBus} onChange={e => setFormData({...formData, needsBus: e.target.checked})} />
            </div>
            {formData.needsBus && (
              <div className="mt-6 p-4 bg-white border border-amber-200 rounded-2xl space-y-4 animate-in fade-in slide-in-from-top-2">
                <h4 className="text-sm font-bold text-amber-800">Détails pour le transporteur</h4>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label className="block text-xs font-bold mb-1">Lieu de prise en charge (RDV)</label>
                    <input className="w-full p-3 bg-slate-50 border rounded-xl" placeholder="Ex: Devant le gymnase" value={formData.transportRequest.pickupPoint} onChange={e => setFormData({...formData, transportRequest: {...formData.transportRequest, pickupPoint: e.target.value}})} />
                  </div>
                  <div className="flex items-center gap-3">
                    <input type="checkbox" id="stayOnSite" className="w-5 h-5" checked={formData.transportRequest.stayOnSite} onChange={e => setFormData({...formData, transportRequest: {...formData.transportRequest, stayOnSite: e.target.checked}})} />
                    <label htmlFor="stayOnSite" className="text-xs font-bold text-slate-600 cursor-pointer">Le bus reste sur place pour les visites</label>
                  </div>
                </div>
                <div>
                  <label className="block text-xs font-bold mb-1">Informations complémentaires</label>
                  <textarea rows={2} className="w-full p-3 bg-slate-50 border rounded-xl text-sm" placeholder="Ex: Numéro de vol AF123..." value={formData.transportRequest.freeText} onChange={e => setFormData({...formData, transportRequest: {...formData.transportRequest, freeText: e.target.value}})} />
                </div>
                <div className="border-t pt-4">
                  <label className="block text-xs font-bold mb-2">Programme complet chauffeur</label>
                  <input type="file" ref={busProgramRef} className="hidden" onChange={(e) => handleFileUpload(e, true)} />
                  <button type="button" onClick={() => busProgramRef.current?.click()} className="text-xs py-2 px-4 bg-amber-600 text-white rounded-lg font-bold hover:bg-amber-700">
                    {formData.transportRequest.busProgramFile ? "✅ Programme transport joint" : "📎 Joindre un programme PDF / Excel"}
                  </button>
                </div>
              </div>
            )}
          </div>
          <button type="button" onClick={() => setShowPiqueNiqueModal(true)} className={`w-full p-6 rounded-2xl border-2 flex items-center justify-between transition-all ${formData.piqueNiqueDetails.active ? 'border-emerald-400 bg-emerald-50' : 'border-slate-100 bg-slate-50'}`}>
            <div className="text-left">
              <h3 className="font-bold text-slate-900">Commande Restauration (Cantine)</h3>
              <p className="text-xs text-slate-500">{formData.piqueNiqueDetails.active ? `✅ Commande configurée (${Object.values(formData.piqueNiqueDetails.daysSelection).filter(Boolean).length} jour(s) sélectionné(s))` : "Cliquer pour configurer le bon de commande cuisine"}</p>
            </div>
            <span className="text-xl">🥪</span>
          </button>
        </div>
        <div className="bg-white p-8 border rounded-3xl shadow-sm space-y-6">
          <div className="text-slate-400 uppercase text-xs font-bold tracking-widest border-b pb-4">5. Autres documents</div>
          <div className="space-y-4">
            <input type="file" multiple ref={fileInputRef} className="hidden" onChange={(e) => handleFileUpload(e)} />
            <button type="button" onClick={() => fileInputRef.current?.click()} disabled={uploading} className="w-full py-4 border-2 border-dashed border-slate-200 rounded-2xl font-bold text-slate-500 hover:bg-slate-50 flex flex-col items-center">
              <span>{uploading ? "Envoi en cours..." : "📎 Ajouter des documents généraux"}</span>
            </button>
            <div className="flex flex-wrap gap-2">
              {formData.attachments.map((file, idx) => (
                <div key={idx} className="flex items-center gap-2 p-2 bg-indigo-50 border border-indigo-100 rounded-xl text-xs font-bold text-indigo-700">
                  <span className="max-w-[150px] truncate">{file.name}</span>
                  <button type="button" onClick={() => removeFile(idx)} className="text-red-500">✕</button>
                </div>
              ))}
            </div>
          </div>
        </div>
        <button disabled={loading || uploading} className="w-full py-5 bg-indigo-600 text-white rounded-2xl font-bold shadow-xl hover:bg-indigo-700 transition-all disabled:opacity-50">
          {loading ? "Création du dossier..." : "Soumettre le dossier complet"}
        </button>
      </form>
      {showBusRecapModal && (
        <div className="fixed inset-0 bg-slate-900/60 backdrop-blur-md flex items-center justify-center z-[60] p-4">
          <div className="bg-white rounded-[2.5rem] p-10 max-w-2xl w-full shadow-2xl border border-amber-100">
            <div className="flex items-center gap-4 mb-6">
              <div className="w-12 h-12 bg-amber-100 rounded-2xl flex items-center justify-center text-2xl">🚌</div>
              <div>
                <h2 className="text-2xl font-black text-slate-900 leading-tight">Vérification transport</h2>
                <p className="text-slate-500 text-sm font-medium">Récapitulatif de la demande chauffeur.</p>
              </div>
            </div>
            <div className="space-y-4 bg-slate-50 p-6 rounded-3xl border border-slate-100 mb-8 overflow-y-auto max-h-[50vh]">
              <div className="grid grid-cols-2 gap-6 text-sm">
                <div><p className="text-slate-400 font-bold uppercase text-[10px] mb-1">Destination</p><p className="font-bold text-slate-900">{formData.destination}</p></div>
                <div><p className="text-slate-400 font-bold uppercase text-[10px] mb-1">Effectif</p><p className="font-bold text-slate-900">{Number(formData.nbEleves || 0) + Number(formData.nbAccompagnateurs || 0)} pers.</p></div>
                <div><p className="text-slate-400 font-bold uppercase text-[10px] mb-1">Départ</p><p className="font-bold text-slate-900">{formData.startDate} à {formData.startTime}</p></div>
                <div><p className="text-slate-400 font-bold uppercase text-[10px] mb-1">Retour</p><p className="font-bold text-slate-900">{formData.endDate} à {formData.endTime}</p></div>
              </div>
            </div>
            <div className="flex gap-4">
              <button type="button" onClick={() => setShowBusRecapModal(false)} className="flex-1 py-4 bg-slate-100 text-slate-600 rounded-2xl font-black">MODIFIER</button>
              <button type="button" onClick={() => { setShowBusRecapModal(false); executeSubmit(); }} className="flex-[2] py-4 bg-amber-500 text-white rounded-2xl font-black shadow-lg">VALIDER ET ENVOYER</button>
            </div>
          </div>
        </div>
      )}
      {showPiqueNiqueModal && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-[70] p-4">
          <div className="bg-white rounded-[2rem] p-8 max-w-2xl w-full shadow-2xl overflow-y-auto max-h-[90vh]">
            <div className="flex justify-between items-start mb-6">
              <div>
                <h2 className="text-2xl font-bold text-slate-900">Bon de commande Cuisine</h2>
                <p className="text-slate-500 text-sm">Structure conforme au PDF du chef</p>
              </div>
              <button onClick={() => setShowPiqueNiqueModal(false)} className="text-slate-400 hover:text-slate-600 text-2xl">✕</button>
            </div>
            <div className="space-y-5">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4 bg-slate-50 p-4 rounded-2xl">
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Heure récupération / livraison</label>
                  <input type="time" className="w-full p-2 border rounded-lg" value={formData.piqueNiqueDetails.deliveryTime} onChange={e => setFormData({...formData, piqueNiqueDetails: {...formData.piqueNiqueDetails, deliveryTime: e.target.value, active: true}})} />
                </div>
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Lieu de récupération</label>
                  <select className="w-full p-2 border rounded-lg" value={formData.piqueNiqueDetails.deliveryPlace} onChange={e => setFormData({...formData, piqueNiqueDetails: {...formData.piqueNiqueDetails, deliveryPlace: e.target.value}})}>
                    <option value="Self">Au self</option>
                    <option value="Bosco">Église Bosco</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-2 text-center">Jours concernés</label>
                  <div className="flex gap-1.5 justify-center">
                    {CUISINE_DAYS.map(({ key: dayKey, label }) => {
                      const isSelected = (formData.piqueNiqueDetails.daysSelection as Record<string, boolean>)[dayKey];
                      return (
                        <button key={dayKey} type="button"
                          onClick={() => setFormData(prev => ({ ...prev, piqueNiqueDetails: { ...prev.piqueNiqueDetails, active: true, daysSelection: { ...prev.piqueNiqueDetails.daysSelection, [dayKey]: !isSelected } } }))}
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
                                    active: true,
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
            <div className="flex gap-3 mt-10">
              <button type="button" onClick={() => { setFormData({...formData, piqueNiqueDetails: {...formData.piqueNiqueDetails, active: false}}); setShowPiqueNiqueModal(false); }} className="flex-1 py-3 bg-red-50 text-red-600 rounded-xl font-bold">ANNULER LA COMMANDE</button>
              <button type="button" onClick={() => setShowPiqueNiqueModal(false)} className="flex-[2] py-3 bg-indigo-600 text-white rounded-xl font-bold shadow-lg shadow-indigo-100">ENREGISTRER LE BON</button>
            </div>
          </div>
        </div>
      )}
    </ModulePageShell>
  );
}

export default function ComplexTripForm() {
  return (
    <Suspense fallback={<div>Chargement...</div>}>
      <ComplexTripFormContent />
    </Suspense>
  );
}