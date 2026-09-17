import { redirect } from "next/navigation";
import { Suspense } from "react";
import { getAppSession } from "@/app/lib/intranet-session";
import { canManageElevePreinscriptions } from "@/app/lib/eleve-dossier-scope";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import EleveInscriptionDocsClient from "./EleveInscriptionDocsClient";

export default async function EleveInscriptionDocsPage() {
  const session = await getAppSession();
  if (!session?.user) redirect("/auth/sign-in");
  const etabId = await resolveCurrentEtablissementId();
  const roles =
    session.user.roles.length > 0
      ? session.user.roles
      : etabId
        ? await listUserRolesFromDb(session.user.id, etabId)
        : [];
  if (
    !canManageElevePreinscriptions({
      roles,
      orgAdmin: Boolean(session.user.orgAdmin),
      platformAdmin: Boolean(session.user.platformAdmin),
    })
  ) {
    redirect("/eleves/dossiers");
  }

  return (
    <Suspense
      fallback={
        <div className="p-8 text-sm text-slate-500">Chargement des documents d’inscription…</div>
      }
    >
      <EleveInscriptionDocsClient />
    </Suspense>
  );
}
