"use client";

import { useParams } from "next/navigation";
import { TripFeedPublisher } from "@/app/components/travels/feed/TripFeedPublisher";

/** Page mobile de publication du fil du voyage (organisateur, accompagnateurs, direction : les droits sont contrôlés côté serveur). */
export default function TripFeedPage() {
  const { id } = useParams();
  const tripId = Array.isArray(id) ? id[0] : id;
  if (!tripId) return null;
  return <TripFeedPublisher tripId={String(tripId)} />;
}
