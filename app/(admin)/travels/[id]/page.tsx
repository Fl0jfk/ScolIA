"use client";

import { useUser } from "@clerk/nextjs";
import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { TripPageShell } from "@/app/components/travels/TripDetailUI";
import { TripDetailsLoaded } from "./TripDetailsLoaded";

export default function TripDetails() {
  const { id } = useParams();
  const { isLoaded: isUserLoaded } = useUser();
  const [trip, setTrip] = useState<TravelsTrip | null>(null);

  useEffect(() => {
    const fetchTrip = async () => {
      try {
        const res = await fetch(`/api/travels/get?id=${id}`);
        if (res.ok) {
          setTrip(await res.json());
        }
      } catch (err) {
        console.error("Erreur lors de la récupération du dossier:", err);
      }
    };
    if (id) void fetchTrip();
  }, [id]);

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

  return <TripDetailsLoaded key={trip.id} trip={trip} setTrip={setTrip} />;
}
