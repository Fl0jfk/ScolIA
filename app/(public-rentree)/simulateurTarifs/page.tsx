import type { Metadata } from "next";
import { loadAppConfig } from "@/app/lib/app-config";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import { requireSimulateurTarifsPublicPage } from "@/app/lib/toolbox-public-gate";
import SimulateurTarifsClient from "./SimulateurTarifsClient";

export async function generateMetadata(): Promise<Metadata> {
  const toolbox = await getToolboxConfig();
  const year = toolbox.tools["simulateur-tarifs"].schoolYear?.trim();
  return {
    title: year ? `Simulateur de tarifs ${year}` : "Simulateur de tarifs",
    description: "Estimation des tarifs scolaires (enseignement, demi-pension, garderie).",
  };
}

export default async function SimulateurTarifsPage() {
  const [tarifs, app] = await Promise.all([requireSimulateurTarifsPublicPage(), loadAppConfig()]);
  return (
    <SimulateurTarifsClient
      siteName={app.identity.shortName || app.identity.name}
      tarifs={tarifs}
    />
  );
}
