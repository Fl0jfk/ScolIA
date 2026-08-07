import { describeTripWorkflowForAi } from "@/app/lib/travels-next-guidance";
import {
  buildTripWorkflowAudit,
  formatTripAuditForAiPrompt,
} from "@/app/lib/travels-workflow-audit";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  TRAVELS_STATUS_LABELS,
  type TravelsTrip,
} from "@/app/lib/travels-types";
import { syncTripActualite } from "@/app/lib/brain-ai/sync/knowledge-writer";
import { choicesResult, loadTripChoiceCatalog, matchCatalogValue } from "@/app/lib/brain-ai/choice-options";
import {
  parseClassesSelection,
  serializeClassesSelection,
  splitClassesValue,
  TRAVELS_CLASSES_AUTRES_LABEL,
  TRAVELS_CLASSES_AUTRES_VALUE,
} from "@/app/lib/travels-classes";
import {
  buildDateQuickOptions,
  weekdayLabelFr,
  wizardStep,
  WIZARD_DATE_OTHER,
} from "@/app/lib/brain-ai/wizard";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

function tripDates(t: TravelsTrip): string {
  const d = t.data || {};
  if (d.startDate && d.endDate && d.startDate !== d.endDate) {
    return `${d.startDate} → ${d.endDate}`;
  }
  return String(d.date || d.startDate || d.endDate || "");
}

function tripBrief(t: TravelsTrip) {
  const status = String(t.status || "");
  return {
    id: t.id,
    title: t.data?.title || "(sans titre)",
    destination: t.data?.destination || null,
    dates: tripDates(t) || null,
    classes: t.data?.classes || null,
    type: t.type,
    status,
    statusLabel: TRAVELS_STATUS_LABELS[status] || status,
    ownerName: t.ownerName || null,
  };
}

async function loadTripsIndex(): Promise<TravelsTrip[]> {
  const hit = await getJson<TravelsTrip[]>("travels/index.json");
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function handleListTripsBrief(
  _ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const limit = Math.min(Math.max(Number(args.limit) || 12, 1), 40);
  const trips = await loadTripsIndex();
  const sorted = [...trips].sort((a, b) => {
    const da = tripDates(a) || a.updatedAt || a.createdAt || "";
    const db = tripDates(b) || b.updatedAt || b.createdAt || "";
    return db.localeCompare(da);
  });
  const items = sorted.slice(0, limit).map(tripBrief);
  return {
    ok: true,
    data: { trips: items, total: trips.length },
    summaryFr:
      items.length === 0
        ? "Aucun séjour trouvé."
        : `${items.length} séjour(s) : ${items
            .slice(0, 5)
            .map((t) => `${t.title} (${t.statusLabel})`)
            .join(" · ")}.`,
  };
}

export async function handleGetTripStatus(
  _ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const tripId = typeof args.tripId === "string" ? args.tripId.trim() : "";
  const query = typeof args.query === "string" ? args.query.trim().toLowerCase() : "";
  const trips = await loadTripsIndex();

  let trip: TravelsTrip | undefined;
  if (tripId) {
    trip = trips.find((t) => t.id === tripId);
    if (!trip) {
      const full = await getJson<TravelsTrip>(`travels/${tripId}.json`);
      trip = full?.data;
    } else {
      // Index = résumé : recharger le JSON complet pour l’audit (devis, compta…).
      const full = await getJson<TravelsTrip>(`travels/${tripId}.json`);
      if (full?.data) trip = full.data;
    }
  } else if (query) {
    const hit = trips.find((t) => {
      const hay = `${t.data?.title || ""} ${t.data?.destination || ""} ${t.data?.classes || ""}`.toLowerCase();
      return hay.includes(query);
    });
    if (hit) {
      const full = await getJson<TravelsTrip>(`travels/${hit.id}.json`);
      trip = full?.data || hit;
    }
  } else {
    return { ok: false, error: "Indiquez tripId ou query (titre / destination)." };
  }

  if (!trip) {
    return { ok: false, error: "Séjour introuvable." };
  }

  const brief = tripBrief(trip);
  const audit = buildTripWorkflowAudit(trip);
  const workflowHelp = describeTripWorkflowForAi(trip);
  return {
    ok: true,
    data: {
      ...brief,
      workflowHelp,
      audit,
      auditText: formatTripAuditForAiPrompt(audit),
    },
    summaryFr:
      `« ${brief.title} » — ${brief.dates || "dates non renseignées"} — statut : ${brief.statusLabel}. ` +
      workflowHelp,
  };
}

/**
 * Wizard sortie / séjour :
 * type → titre → destination → date départ → (date retour si COMPLEX)
 * → établissement → classes → nb élèves → confirmation
 */
export async function handleCreateTrip(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  let title = String(args.title || "").trim();
  let destination = String(args.destination || "").trim();
  let startDate = String(args.startDate || args.date || "").trim();
  let endDate = String(args.endDate || "").trim();
  let classes = String(args.classes || "").trim();
  let etablissement = String(args.etablissement || "").trim();
  const nbElevesRaw = args.nbEleves;
  const nbEleves =
    nbElevesRaw != null && String(nbElevesRaw).trim() !== ""
      ? Number(nbElevesRaw)
      : undefined;
  const nbElevesResolved = Boolean(args.nbElevesResolved);
  const typeRaw = String(args.type || "").trim();
  let type: "SIMPLE" | "COMPLEX" | "" =
    typeRaw.toUpperCase() === "COMPLEX" ? "COMPLEX" : typeRaw ? "SIMPLE" : "";

  const catalog = await loadTripChoiceCatalog();
  const hasEtab = catalog.establishments.length > 0;
  const hasClasses = catalog.allClasses.length > 0;

  const totalSteps =
    4 + // type, title, destination, startDate
    (type === "COMPLEX" || !type ? 1 : 0) + // endDate (count as potential; adjust dynamically)
    (hasEtab ? 1 : 0) +
    (hasClasses ? 1 : 0) +
    1; // nbEleves

  let step = 1;
  const label = (body: string) => wizardStep(step, Math.max(totalSteps, step), body);

  const draft = (): Record<string, unknown> => ({
    title,
    destination,
    date: startDate === WIZARD_DATE_OTHER ? "" : startDate,
    startDate: startDate === WIZARD_DATE_OTHER ? "" : startDate,
    endDate: endDate === WIZARD_DATE_OTHER ? "" : endDate,
    classes,
    etablissement,
    ...(Number.isFinite(nbEleves) ? { nbEleves } : {}),
    ...(nbElevesResolved ? { nbElevesResolved: true } : {}),
    ...(type ? { type } : {}),
    ...(args.classesResolved ? { classesResolved: true } : {}),
  });

  // 1 — Type
  if (!type) {
    return choicesResult(
      "create_trip",
      "type",
      label("Créons une sortie scolaire. Quel type de dossier ?"),
      [
        { value: "SIMPLE", label: "Sortie simple (journée / proximité)" },
        { value: "COMPLEX", label: "Séjour / sortie complexe (plusieurs jours)" },
      ],
      draft(),
    );
  }
  step += 1;

  // 2 — Titre
  if (!title) {
    return choicesResult(
      "create_trip",
      "title",
      label("Quel est l'intitulé de la sortie ? (ex. Musée des Beaux-Arts, Laser Game…)"),
      [],
      draft(),
      "text",
    );
  }
  step += 1;

  // 3 — Destination
  if (!destination) {
    return choicesResult(
      "create_trip",
      "destination",
      label(`« ${title} » — où se déroule la sortie ? (lieu / adresse)`),
      [],
      draft(),
      "text",
    );
  }
  step += 1;

  // 4 — Date de départ
  if (startDate === WIZARD_DATE_OTHER) {
    return choicesResult(
      "create_trip",
      "startDate",
      label("Choisissez la date de départ dans le calendrier :"),
      [],
      { ...draft(), startDate: "", date: "" },
      "date",
    );
  }
  if (!startDate || !/^\d{4}-\d{2}-\d{2}$/.test(startDate)) {
    return choicesResult(
      "create_trip",
      "startDate",
      label("Quelle est la date de départ ?"),
      buildDateQuickOptions(calendarDateKeyParis()),
      draft(),
    );
  }
  if (startDate < calendarDateKeyParis()) {
    return choicesResult(
      "create_trip",
      "startDate",
      label(`La date ${startDate} est déjà passée. Choisissez une date à venir :`),
      buildDateQuickOptions(calendarDateKeyParis()),
      { ...draft(), startDate: "", date: "" },
    );
  }
  step += 1;

  // 5 — Date de retour (COMPLEX) ou = départ (SIMPLE)
  if (type === "COMPLEX") {
    if (endDate === WIZARD_DATE_OTHER) {
      return choicesResult(
        "create_trip",
        "endDate",
        label("Choisissez la date de retour :"),
        [],
        { ...draft(), endDate: "" },
        "date",
      );
    }
    if (!endDate || !/^\d{4}-\d{2}-\d{2}$/.test(endDate)) {
      return choicesResult(
        "create_trip",
        "endDate",
        label(`Départ le ${weekdayLabelFr(startDate)} — quelle date de retour ?`),
        buildDateQuickOptions(startDate),
        draft(),
      );
    }
    if (endDate < startDate) {
      return choicesResult(
        "create_trip",
        "endDate",
        label("La date de retour doit être après le départ. Choisissez à nouveau :"),
        buildDateQuickOptions(startDate),
        { ...draft(), endDate: "" },
      );
    }
    step += 1;
  } else {
    endDate = startDate;
  }

  // 6 — Établissement
  if (hasEtab) {
    if (!etablissement) {
      return choicesResult(
        "create_trip",
        "etablissement",
        label("Quel établissement est concerné ?"),
        catalog.establishments.map((e) => ({ value: e, label: e })),
        draft(),
      );
    }
    const matchedEtab = matchCatalogValue(etablissement, catalog.establishments);
    if (!matchedEtab) {
      return choicesResult(
        "create_trip",
        "etablissement",
        label(`Établissement « ${etablissement} » non reconnu. Choisissez dans la liste :`),
        catalog.establishments.map((e) => ({ value: e, label: e })),
        draft(),
      );
    }
    etablissement = matchedEtab;
    step += 1;
  }

  // 7 — Classes
  if (hasClasses) {
    const tokens = splitClassesValue(classes);
    const wantsAutres = tokens.some(
      (t) => t === TRAVELS_CLASSES_AUTRES_VALUE || t === TRAVELS_CLASSES_AUTRES_LABEL,
    );
    const classesResolved = Boolean(args.classesResolved);

    if (!classes && !classesResolved) {
      return choicesResult(
        "create_trip",
        "classes",
        label("Quelles classes participent ? (plusieurs possibles, ou Autres)"),
        [
          ...catalog.allClasses.map((c) => ({ value: c, label: c })),
          { value: TRAVELS_CLASSES_AUTRES_VALUE, label: TRAVELS_CLASSES_AUTRES_LABEL },
        ],
        draft(),
        "multi",
      );
    }

    if (wantsAutres) {
      const known = tokens.filter(
        (t) => t !== TRAVELS_CLASSES_AUTRES_VALUE && t !== TRAVELS_CLASSES_AUTRES_LABEL,
      );
      const otherDraft = String(args.classesOther || "").trim();
      if (!otherDraft) {
        return choicesResult(
          "create_trip",
          "classesOther",
          label("Précisez les autres classes :"),
          [],
          { ...draft(), classes: known.join(", "), classesOtherPending: true },
          "text",
        );
      }
      classes = serializeClassesSelection(
        known.map((c) => matchCatalogValue(c, catalog.allClasses) || c).filter(Boolean),
        otherDraft,
      );
    } else if (!classesResolved) {
      const parsed = parseClassesSelection(classes, catalog.allClasses);
      const unresolved = splitClassesValue(classes).filter(
        (c) =>
          c !== TRAVELS_CLASSES_AUTRES_VALUE &&
          c !== TRAVELS_CLASSES_AUTRES_LABEL &&
          !matchCatalogValue(c, catalog.allClasses),
      );
      if (unresolved.length > 0 && !parsed.otherText) {
        return choicesResult(
          "create_trip",
          "classes",
          label("Certaines classes ne sont pas reconnues. Recochez ou choisissez Autres :"),
          [
            ...catalog.allClasses.map((c) => ({ value: c, label: c })),
            { value: TRAVELS_CLASSES_AUTRES_VALUE, label: TRAVELS_CLASSES_AUTRES_LABEL },
          ],
          draft(),
          "multi",
        );
      }
      classes = serializeClassesSelection(parsed.selected, parsed.otherText);
    }
    step += 1;
  }

  // 8 — Nombre d'élèves (optionnel mais guidé)
  if (!nbElevesResolved && !(Number.isFinite(nbEleves) && (nbEleves as number) > 0)) {
    return choicesResult(
      "create_trip",
      "nbEleves",
      label("Combien d'élèves environ ? (nombre entier, ex. 28)"),
      [],
      draft(),
      "text",
    );
  }
  const elevesCount =
    Number.isFinite(nbEleves) && (nbEleves as number) > 0 ? Math.round(nbEleves as number) : undefined;

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_trip",
      args: {
        title,
        destination,
        date: startDate,
        startDate,
        endDate: endDate || startDate,
        classes,
        etablissement,
        nbEleves: elevesCount,
        nbElevesResolved: true,
        classesResolved: true,
        type,
      },
      summaryFr:
        `Récap — Créer le ${type === "COMPLEX" ? "séjour" : "sortie"} « ${title} »` +
        ` à ${destination}` +
        (startDate === endDate
          ? ` le ${weekdayLabelFr(startDate)}`
          : ` du ${weekdayLabelFr(startDate)} au ${weekdayLabelFr(endDate)}`) +
        (etablissement ? ` — ${etablissement}` : "") +
        (classes ? ` (${classes})` : "") +
        (elevesCount ? ` — ${elevesCount} élèves` : "") +
        " ?",
    };
  }

  const now = new Date().toISOString();
  const id = crypto.randomUUID();
  const trip: TravelsTrip = {
    id,
    type,
    status: "EN_ATTENTE_DIR_INITIAL",
    ownerId: ctx.userId || undefined,
    ownerName: [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") || undefined,
    ownerEmail: ctx.email || undefined,
    createdAt: now,
    updatedAt: now,
    data: {
      title,
      destination: destination || undefined,
      date: startDate,
      startDate,
      endDate: endDate || startDate,
      classes: classes || undefined,
      etablissement: etablissement || undefined,
      nbEleves: elevesCount,
    },
    history: [
      {
        date: now,
        user: [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") || "Assistant IA",
        action: "CREE",
        note: "Créé via ScolIA (wizard)",
      },
    ],
  };

  await putJson(`travels/${id}.json`, trip);
  const indexHit = await getJson<unknown[]>("travels/index.json");
  const currentIndex = Array.isArray(indexHit?.data) ? [...indexHit.data] : [];
  currentIndex.unshift({
    id: trip.id,
    type: trip.type,
    status: trip.status,
    ownerName: trip.ownerName,
    ownerEmail: trip.ownerEmail,
    ownerId: trip.ownerId,
    createdAt: trip.createdAt,
    updatedAt: trip.updatedAt,
    data: {
      title: trip.data.title,
      destination: trip.data.destination,
      date: trip.data.date,
      startDate: trip.data.startDate,
      endDate: trip.data.endDate,
      classes: trip.data.classes,
      etablissement: trip.data.etablissement,
      nbEleves: trip.data.nbEleves,
    },
  });
  await putJson("travels/index.json", currentIndex);

  void syncTripActualite({
    id,
    title,
    dates: startDate === endDate ? startDate : `${startDate} → ${endDate}`,
    classes,
    statusLabel: TRAVELS_STATUS_LABELS.EN_ATTENTE_DIR_INITIAL,
  });

  return {
    ok: true,
    data: {
      id,
      followUrl: `/travels/${id}`,
      status: trip.status,
      statusLabel: TRAVELS_STATUS_LABELS.EN_ATTENTE_DIR_INITIAL,
    },
    summaryFr: `Séjour « ${title} » créé. Complétez le dossier sur /travels/${id}.`,
  };
}
