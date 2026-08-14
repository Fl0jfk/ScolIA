import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";

import { ensureFolderPath } from "@/app/lib/graph-onedrive-folders";
import { GRAPH_API_BASE, graphDriveRootItemUrl } from "@/app/lib/graph-onedrive-path";

export async function POST(req: Request) {
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const session = await resolveSession();
    const userId = session?.userId;
    if (!userId) {
      return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
    }
    const body = await req.json();
    const {
      accessToken,
      sourcePath,
      targetFolderPath,
      newFileName,
    } = body as {
      accessToken: string;
      sourcePath: string;  
      targetFolderPath: string; 
      newFileName?: string; 
    };
    if (!accessToken) {
      return NextResponse.json({ error: "accessToken manquant" },{ status: 400 });
    }
    if (!sourcePath || !targetFolderPath) {
      return NextResponse.json({ error: "sourcePath et targetFolderPath sont requis" },{ status: 400 });
    }
    const sourceRes = await fetch(graphDriveRootItemUrl(sourcePath), {
        headers: { Authorization: `Bearer ${accessToken}` },
      }
    );
    if (!sourceRes.ok) {
      const errText = await sourceRes.text();
      return NextResponse.json({ error: "Erreur récupération fichier source", details: errText },{ status: sourceRes.status });
    }
    const sourceItem = await sourceRes.json();
    const sourceItemId = sourceItem.id as string | undefined;
    const driveId = sourceItem.parentReference?.driveId as | string | undefined;
    if (!sourceItemId || !driveId) {
      return NextResponse.json({ error: "Impossible de récupérer l'ID du fichier source ou du drive" },{ status: 500 });
    }
    try {
      await ensureFolderPath(accessToken, targetFolderPath);
    } catch (ensureErr) {
      return NextResponse.json(
        {
          error: "Impossible de créer le dossier cible OneDrive",
          details: ensureErr instanceof Error ? ensureErr.message : String(ensureErr),
        },
        { status: 500 },
      );
    }

    const targetFolderRes = await fetch(graphDriveRootItemUrl(targetFolderPath), {
        headers: { Authorization: `Bearer ${accessToken}` },
      }
    );
    if (!targetFolderRes.ok) {
      const errText = await targetFolderRes.text();
      return NextResponse.json({ error: "Erreur récupération dossier cible", details: errText },{ status: targetFolderRes.status });
    }
    const targetFolder = await targetFolderRes.json();
    const targetFolderId = targetFolder.id as string | undefined;
    if (!targetFolderId) {
      return NextResponse.json({ error: "Impossible de récupérer l'ID du dossier cible" },{ status: 500 });
    }
    const finalFileName =
      newFileName && newFileName.trim().length > 0  ? newFileName.trim() : (sourceItem.name as string);
    const childrenRes = await fetch(`${GRAPH_API_BASE}/drives/${driveId}/items/${targetFolderId}/children`,
      {
        headers: { Authorization: `Bearer ${accessToken}` },
      }
    );
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let children: any[] = [];
    if (childrenRes.ok) {
      const childrenJson = await childrenRes.json();
      children = childrenJson.value || [];
    }
    const dotIndex = finalFileName.lastIndexOf(".");
    const base = dotIndex > 0 ? finalFileName.slice(0, dotIndex) : finalFileName;
    const ext = dotIndex > 0 ? finalFileName.slice(dotIndex) : "";
    let safeName = finalFileName;
    let suffix = 2;
    while (children.some((c) => c.name === safeName)) {
      safeName = `${base} (${suffix})${ext}`;
      suffix++;
    }
    const moveRes = await fetch(`${GRAPH_API_BASE}/drives/${driveId}/items/${sourceItemId}`,
      {
        method: "PATCH",
        headers: {
          Authorization: `Bearer ${accessToken}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          parentReference: { id: targetFolderId },
          name: safeName,
        }),
      }
    );
    if (!moveRes.ok) {
      const errText = await moveRes.text();
      return NextResponse.json({ error: "Erreur lors du déplacement du fichier", details: errText },{ status: moveRes.status });
    }
    const movedItem = await moveRes.json();
    return NextResponse.json({
      success: true,
      message: "Fichier déplacé vers le dossier cible",
      itemId: movedItem.id,
      finalFileName: safeName,
      targetFolderId,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
