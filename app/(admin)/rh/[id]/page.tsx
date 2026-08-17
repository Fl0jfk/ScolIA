"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import StaffDossier from "@/app/components/personnel/StaffDossier";
import type { PersonnelRecord } from "@/app/lib/personnel-types";

export default function RhDossierPage() {
  const { id } = useParams<{ id: string }>();
  const router = useRouter();
  const [record, setRecord] = useState<PersonnelRecord | null>(null);
  const [canManage, setCanManage] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const staffId = Array.isArray(id) ? id[0] : id;

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/personnel/${staffId}`, { cache: "no-store" });
      const j = await res.json();
      if (res.status === 403) {
        router.replace("/rh/moi");
        return;
      }
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setRecord(j.record);
      setCanManage(!!j.canManage);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (staffId) void load();
  }, [staffId]);

  if (loading) return <p className="p-10 text-center text-slate-500">Chargement du dossier…</p>;
  if (error) return <p className="p-10 text-center text-rose-600">{error}</p>;
  if (!record) return <p className="p-10 text-center text-slate-500">Dossier introuvable.</p>;

  return (
    <div className="max-w-[1280px] mx-auto p-4 sm:p-6">
      <StaffDossier record={record} canManage={canManage} onRefresh={load} backHref="/rh" />
    </div>
  );
}
