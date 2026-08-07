"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import FaireUneDemandeForm from "@/app/components/requests/FaireUneDemandeForm";
import MesDemandesSuivi from "@/app/(admin)/requests/MesDemandesSuivi";
import {
  RH_REQUEST_ROUTE_ID,
  RH_REQUEST_SUBJECT_PREFIX,
} from "@/app/lib/requests-routing-defaults";

type SubmittedRequest = {
  id: string;
  status: "NOUVELLE" | "EN_COURS" | "EN_ATTENTE" | "TERMINEE";
  category: string;
  subject: string;
  description: string;
  assignedTo: {
    routeId?: string;
    unit: string;
    roleLabel: string;
  };
  routing?: { taskId?: string };
};

function isRhRequest(r: SubmittedRequest) {
  if (r.routing?.taskId === RH_REQUEST_ROUTE_ID) return true;
  if (r.assignedTo.routeId === RH_REQUEST_ROUTE_ID || r.assignedTo.unit === RH_REQUEST_ROUTE_ID) {
    return true;
  }
  if (r.category === "RH") return true;
  return r.subject.trim().startsWith(RH_REQUEST_SUBJECT_PREFIX);
}

export default function RhDemandePanel() {
  const [items, setItems] = useState<SubmittedRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshKey, setRefreshKey] = useState(0);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/requests/list?scope=submitted", { cache: "no-store" });
      if (!res.ok) {
        setItems([]);
        return;
      }
      const list = (await res.json()) as SubmittedRequest[];
      setItems((Array.isArray(list) ? list : []).filter(isRhRequest).slice(0, 20));
    } catch {
      setItems([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load, refreshKey]);

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border border-indigo-100 bg-gradient-to-br from-indigo-50/70 via-white to-white p-5 sm:p-6">
        <p className="text-[11px] font-black uppercase tracking-widest text-indigo-600">
          Ticketing RH
        </p>
        <h2 className="mt-1 text-2xl font-black text-slate-900">Demande RH</h2>
        <p className="mt-1 max-w-2xl text-sm text-slate-600">
          Attestation, dossier, question sur un contrat… Votre demande utilise le même outil de
          tickets que l’établissement, orientée directement vers la file RH.
        </p>
      </section>

      <FaireUneDemandeForm
        variant="inline"
        forceRouteId={RH_REQUEST_ROUTE_ID}
        subjectPrefix={RH_REQUEST_SUBJECT_PREFIX}
        mesDemandesHref="/rh?tab=demande#mes-demandes-rh"
        heading="Nouvelle demande"
        intro="Décrivez votre besoin. Un destinataire RH (ou la corbeille si la file n’est pas encore affectée) traitera votre ticket."
        placeholder="Ex. : j’ai besoin d’une attestation de travail, d’une mise à jour de mon dossier, d’une info sur mon contrat…"
        hideIdentityCard
        onSuccess={() => setRefreshKey((k) => k + 1)}
      />

      <MesDemandesSuivi
        id="mes-demandes-rh"
        title="Mes demandes RH"
        intro="Suivi des tickets ouverts depuis le module RH (statut et service destinataire)."
        items={items}
        loading={loading}
      />

      <p className="text-xs text-slate-400">
        Pour une demande non RH (maintenance, scolarité…), utilisez{" "}
        <Link href="/requests?nouvelle=1" className="font-bold text-indigo-600 underline">
          Faire une demande
        </Link>
        .
      </p>
    </div>
  );
}
