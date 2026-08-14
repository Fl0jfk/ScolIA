import { computeOutingStatus } from "@/app/lib/internat-outing";
import { notifyInternatOutingParents } from "@/app/lib/internat-notify";
import type {
  InternatEtablissement,
  InternatOuting,
  InternatOutingDirectionStatus,
  InternatOutingParentStatus,
} from "@/app/lib/internat-types";

type OutingTokenRole =
  | { role: "direction"; index: number; etablissement: InternatEtablissement }
  | { role: "parent"; index: number; studentName: string };

function resolveOutingToken(outing: InternatOuting, token: string): OutingTokenRole | null {
  if (outing.status === "cancelled") return null;
  const dirIdx = outing.directionDecisions.findIndex((d) => d.token === token);
  if (dirIdx >= 0) {
    return {
      role: "direction",
      index: dirIdx,
      etablissement: outing.directionDecisions[dirIdx]!.etablissement,
    };
  }
  const parentIdx = outing.participants.findIndex((p) => p.parentToken === token);
  if (parentIdx >= 0) {
    return {
      role: "parent",
      index: parentIdx,
      studentName: outing.participants[parentIdx]!.studentName,
    };
  }
  return null;
}

export async function notifyParentsIfReady(outing: InternatOuting): Promise<InternatOuting> {
  const status = computeOutingStatus(outing);
  if (status !== "pending_parents") return { ...outing, status };

  let updated = { ...outing, status };
  for (let i = 0; i < updated.participants.length; i++) {
    const p = updated.participants[i]!;
    if (p.parentEmailsSentAt) continue;
    const result = await notifyInternatOutingParents({ outing: updated, participantIndex: i });
    if (result.sent) {
      updated = {
        ...updated,
        participants: updated.participants.map((x, idx) =>
          idx === i ? { ...x, parentEmailsSentAt: new Date().toISOString() } : x,
        ),
      };
    }
  }
  return updated;
}

type OutingDecisionInput = {
  outing: InternatOuting;
  decision: "approve" | "refuse";
  decidedBy?: string;
  note?: string;
  clientIp?: string;
} & (
  | { kind: "direction"; etablissement: InternatEtablissement }
  | { kind: "parent"; studentId: string }
  | { kind: "token"; token: string }
);

function mapDirectionStatus(decision: "approve" | "refuse"): InternatOutingDirectionStatus {
  return decision === "approve" ? "approved" : "refused";
}

function mapParentStatus(decision: "approve" | "refuse"): InternatOutingParentStatus {
  return decision === "approve" ? "authorized" : "refused";
}

export async function applyOutingDecision(input: OutingDecisionInput): Promise<{
  outing: InternatOuting;
  alreadyDecided: boolean;
}> {
  const now = new Date().toISOString();
  let outing = { ...input.outing };
  let alreadyDecided = false;

  if (input.kind === "token") {
    const resolved = resolveOutingToken(outing, input.token);
    if (!resolved) throw new Error("Jeton invalide ou sortie annulée.");

    if (resolved.role === "direction") {
      const d = outing.directionDecisions[resolved.index]!;
      if (d.status !== "pending") {
        alreadyDecided = true;
      } else {
        outing.directionDecisions = outing.directionDecisions.map((x, i) =>
          i === resolved.index
            ? {
                ...x,
                status: mapDirectionStatus(input.decision),
                decidedAt: now,
                decidedBy: input.decidedBy || "Lien e-mail",
                note: input.note?.trim() || undefined,
              }
            : x,
        );
      }
    } else {
      const p = outing.participants[resolved.index]!;
      if (p.parentStatus !== "pending") {
        alreadyDecided = true;
      } else {
        const allDirectionsApproved = outing.directionDecisions.every((d) => d.status === "approved");
        if (!allDirectionsApproved) {
          throw new Error("La direction doit valider la sortie avant la réponse des parents.");
        }
        outing.participants = outing.participants.map((x, i) =>
          i === resolved.index
            ? {
                ...x,
                parentStatus: mapParentStatus(input.decision),
                parentRespondedAt: now,
                parentRespondedBy: input.decidedBy || "Parent",
                parentResponseIp: input.clientIp,
              }
            : x,
        );
      }
    }
  } else if (input.kind === "direction") {
    const idx = outing.directionDecisions.findIndex((d) => d.etablissement === input.etablissement);
    if (idx < 0) throw new Error("Établissement introuvable pour cette sortie.");
    const d = outing.directionDecisions[idx]!;
    if (d.status !== "pending") alreadyDecided = true;
    else {
      outing.directionDecisions = outing.directionDecisions.map((x, i) =>
        i === idx
          ? {
              ...x,
              status: mapDirectionStatus(input.decision),
              decidedAt: now,
              decidedBy: input.decidedBy || "Équipe internat",
              note: input.note?.trim() || undefined,
            }
          : x,
      );
    }
  } else {
    const idx = outing.participants.findIndex((p) => p.studentId === input.studentId);
    if (idx < 0) throw new Error("Interne introuvable pour cette sortie.");
    const p = outing.participants[idx]!;
    if (p.parentStatus !== "pending") alreadyDecided = true;
    else {
      outing.participants = outing.participants.map((x, i) =>
        i === idx
          ? {
              ...x,
              parentStatus: mapParentStatus(input.decision),
              parentRespondedAt: now,
              parentRespondedBy: input.decidedBy || "Équipe internat",
            }
          : x,
      );
    }
  }

  outing.status = computeOutingStatus(outing);
  outing.updatedAt = now;
  outing = await notifyParentsIfReady(outing);
  outing.status = computeOutingStatus(outing);
  return { outing, alreadyDecided };
}

export function sanitizeOutingForTokenView(outing: InternatOuting, token: string) {
  const resolved = resolveOutingToken(outing, token);
  if (!resolved) return null;

  const base = {
    id: outing.id,
    title: outing.title,
    activity: outing.activity,
    destination: outing.destination,
    accompanists: outing.accompanists,
    outingDate: outing.outingDate,
    departureTime: outing.departureTime,
    returnTime: outing.returnTime,
    status: outing.status,
    role: resolved.role,
  };

  if (resolved.role === "direction") {
    const d = outing.directionDecisions[resolved.index]!;
    const students = outing.participants.filter((p) => p.etablissement === d.etablissement);
    return {
      ...base,
      etablissement: d.etablissement,
      decisionStatus: d.status,
      decidedAt: d.decidedAt,
      students: students.map((s) => ({ name: s.studentName, classe: s.classe })),
    };
  }

  const p = outing.participants[resolved.index]!;
  return {
    ...base,
    studentName: p.studentName,
    classe: p.classe,
    decisionStatus: p.parentStatus,
    decidedAt: p.parentRespondedAt,
    directionApproved: outing.directionDecisions.every((d) => d.status === "approved"),
  };
}
