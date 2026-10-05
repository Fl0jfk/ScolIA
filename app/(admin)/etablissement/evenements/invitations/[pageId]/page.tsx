import type { Metadata } from "next";
import InvitationPageAdminClient from "@/app/components/invitation/InvitationPageAdminClient";

export const metadata: Metadata = {
  title: "Invitation — tableau de bord",
};

type Props = { params: Promise<{ pageId: string }> };

export default async function InvitationPageAdminPage({ params }: Props) {
  const { pageId } = await params;
  return <InvitationPageAdminClient pageId={pageId} />;
}
