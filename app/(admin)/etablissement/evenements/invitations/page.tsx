import type { Metadata } from "next";
import InvitationsAdminListClient from "@/app/components/invitation/InvitationsAdminListClient";

export const metadata: Metadata = {
  title: "Invitations cérémonies",
  description: "Pages d’invitation RSVP (remise de diplôme, etc.)",
};

export default function InvitationsAdminPage() {
  return <InvitationsAdminListClient />;
}
