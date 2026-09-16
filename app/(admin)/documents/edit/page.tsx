import OfficeEditClient from "@/app/components/documents/OfficeEditClient";
import type { OfficeKind } from "@/app/lib/office-types";
import { officeKindFromExt } from "@/app/lib/office-types";
import { redirect } from "next/navigation";

type Search = {
  kind?: string;
  scope?: string;
  path?: string;
  shareId?: string;
  fileShareId?: string;
  draft?: string;
};

export default async function OfficeEditPage({
  searchParams,
}: {
  searchParams: Promise<Search>;
}) {
  const sp = await searchParams;
  const path = sp.path || "";
  if (!path) redirect("/documents");

  const ext = path.includes(".") ? path.split(".").pop() : "";
  const kindFromExt = officeKindFromExt(ext);
  const kindRaw = sp.kind || kindFromExt || "writer";
  const kind: OfficeKind =
    kindRaw === "calc" || kindRaw === "impress" || kindRaw === "writer" ? kindRaw : "writer";

  const scope = sp.scope === "shared" || sp.scope === "fileshare" ? sp.scope : "personal";

  return (
    <OfficeEditClient
      kind={kind}
      scope={scope}
      path={path}
      shareId={sp.shareId || null}
      fileShareId={sp.fileShareId || null}
      draft={sp.draft === "1"}
    />
  );
}
