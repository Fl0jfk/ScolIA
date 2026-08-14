import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { getObjectBytes, getSignedReadUrl } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { getTenant } from "@/app/lib/tenant-context";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";

export type PdfLogo = {
  dataUri: string;
  format: "PNG" | "JPEG";
  width?: number;
  height?: number;
};

function imageDimensions(buf: Buffer): { width: number; height: number } | null {
  if (buf.length < 24) return null;
  if (buf[0] === 0x89 && buf[1] === 0x50) {
    return { width: buf.readUInt32BE(16), height: buf.readUInt32BE(20) };
  }
  if (buf[0] === 0xff && buf[1] === 0xd8) {
    let i = 2;
    while (i < buf.length - 9) {
      if (buf[i] !== 0xff) {
        i++;
        continue;
      }
      const marker = buf[i + 1];
      if (marker === 0xc0 || marker === 0xc1 || marker === 0xc2 || marker === 0xc3) {
        return { width: buf.readUInt16BE(i + 7), height: buf.readUInt16BE(i + 5) };
      }
      const len = buf.readUInt16BE(i + 2);
      i += 2 + len;
    }
  }
  return null;
}

function enrichPdfLogo(logo: PdfLogo, buf?: Buffer): PdfLogo {
  if (logo.width && logo.height) return logo;
  const source =
    buf ??
    (() => {
      const b64 = logo.dataUri.split(",")[1];
      return b64 ? Buffer.from(b64, "base64") : null;
    })();
  if (!source) return logo;
  const dims = imageDimensions(source);
  return dims ? { ...logo, ...dims } : logo;
}

/** Taille dans une boîte max en conservant le ratio largeur / hauteur. */
export function fitImageInBox(
  imgW: number,
  imgH: number,
  maxW: number,
  maxH: number,
): { width: number; height: number } {
  if (!imgW || !imgH) return { width: maxW, height: maxH };
  const scale = Math.min(maxW / imgW, maxH / imgH);
  return { width: imgW * scale, height: imgH * scale };
}

function bytesToPdfLogo(buf: Buffer): PdfLogo | null {
  if (!buf.length) return null;
  const dims = imageDimensions(buf);
  if (buf[0] === 0xff && buf[1] === 0xd8) {
    return enrichPdfLogo(
      {
        dataUri: `data:image/jpeg;base64,${buf.toString("base64")}`,
        format: "JPEG",
      },
      buf,
    );
  }
  if (buf[0] === 0x89 && buf[1] === 0x50) {
    return enrichPdfLogo(
      {
        dataUri: `data:image/png;base64,${buf.toString("base64")}`,
        format: "PNG",
      },
      buf,
    );
  }
  return null;
}

/** Même logique que GET /api/site/public — logo du tenant courant, pas la plateforme. */
async function resolveTenantLogoRawRef(): Promise<string> {
  const [bundle, tenant] = await Promise.all([loadAppConfig(), getTenant()]);
  return bundle.identity.headerLogoUrl?.trim() || tenant.logoUrl?.trim() || "";
}

export async function loadImageForPdfFromRef(rawRef: string): Promise<PdfLogo | null> {
  const rawLogo = rawRef?.trim();
  if (!rawLogo) return null;

  const parsedKey = await parseTravelsS3KeyFromUrl(rawLogo);
  const keys = new Set<string>();
  if (parsedKey) keys.add(parsedKey);
  if (!rawLogo.startsWith("http://") && !rawLogo.startsWith("https://")) {
    keys.add(s3Key(rawLogo.split("?")[0].split("#")[0]));
  }

  for (const key of keys) {
    const bytes = await getObjectBytes(key);
    if (bytes?.length) {
      const img = bytesToPdfLogo(bytes);
      if (img) return img;
    }
  }

  let fetchUrl = rawLogo;
  if (parsedKey) {
    const signed = await getSignedReadUrl(parsedKey, 3600);
    if (signed) fetchUrl = signed;
  } else if (!rawLogo.startsWith("http://") && !rawLogo.startsWith("https://")) {
    const signed = await getSignedReadUrl(rawLogo, 3600);
    if (signed) fetchUrl = signed;
  }

  if (fetchUrl.startsWith("http://") || fetchUrl.startsWith("https://")) {
    return loadImageDataUriFromUrl(fetchUrl);
  }

  return null;
}

/**
 * Logo du sous-domaine / tenant (Paramètres → identité, ou logoUrl du tenant).
 * Lecture S3 directe quand possible ; pas de repli sur les fichiers public/ de la plateforme.
 */
export async function loadSchoolLogoForPdf(): Promise<PdfLogo | null> {
  const rawLogo = await resolveTenantLogoRawRef();
  if (!rawLogo) return null;
  return loadImageForPdfFromRef(rawLogo);
}

export async function getSchoolLetterhead() {
  const bundle = await loadAppConfig();
  const identity = bundle.identity;
  const addr = identity.address;
  const addressLine =
    addr?.fullCompact?.trim() ||
    addr?.full?.trim() ||
    [addr?.street, [addr?.zip, addr?.city].filter(Boolean).join(" ")].filter(Boolean).join(" — ") ||
    "";
  const cityLine = [addr?.zip, addr?.city].filter(Boolean).join(" ").trim();
  const name = identity.name?.trim() || "Mon établissement";

  return {
    name,
    subtitle: identity.organizationKind === "groupe"
      ? "Groupe scolaire catholique sous contrat"
      : "Établissement catholique sous contrat",
    addressLine,
    phone: identity.phone?.display?.trim() || "",
    cityLine,
    footerLeft: `${name}  ·  Établissement catholique sous contrat avec l'État`,
  };
}

async function loadImageDataUriFromUrl(url: string): Promise<PdfLogo | null> {
  try {
    const res = await fetch(url);
    if (!res.ok) return null;
    const buf = Buffer.from(await res.arrayBuffer());
    const fromBytes = bytesToPdfLogo(buf);
    if (fromBytes) return fromBytes;
    const ct = (res.headers.get("content-type") || "").toLowerCase();
    if (ct.includes("jpeg") || ct.includes("jpg")) {
      return enrichPdfLogo(
        {
          dataUri: `data:image/jpeg;base64,${buf.toString("base64")}`,
          format: "JPEG",
        },
        buf,
      );
    }
    if (ct.includes("png")) {
      return enrichPdfLogo(
        {
          dataUri: `data:image/png;base64,${buf.toString("base64")}`,
          format: "PNG",
        },
        buf,
      );
    }
    return null;
  } catch {
    return null;
  }
}

/** En-tête visuel commun (logo + coordonnées à droite + bandeau). */
export function drawPdfLetterhead(
  doc: import("jspdf").jsPDF,
  letterhead: Awaited<ReturnType<typeof getSchoolLetterhead>>,
  logo: PdfLogo | null,
  accentRgb: [number, number, number] = [37, 99, 235],
) {
  const W = doc.internal.pageSize.getWidth();
  const ML = 15;
  const MR = W - 15;
  const LOGO_MAX_W = 28;
  const LOGO_MAX_H = 26;

  if (logo) {
    const fitted = fitImageInBox(
      logo.width || LOGO_MAX_W,
      logo.height || LOGO_MAX_H,
      LOGO_MAX_W,
      LOGO_MAX_H,
    );
    const logoY = 6 + (LOGO_MAX_H - fitted.height) / 2;
    doc.addImage(logo.dataUri, logo.format, ML, logoY, fitted.width, fitted.height);
  }

  doc.setFont("helvetica", "bold");
  doc.setFontSize(13);
  doc.setTextColor(30, 41, 59);
  doc.text(letterhead.name, MR, 13, { align: "right" });
  doc.setFont("helvetica", "normal");
  doc.setFontSize(7.5);
  doc.setTextColor(100, 116, 139);
  doc.text(letterhead.subtitle, MR, 19, { align: "right" });
  if (letterhead.addressLine) {
    doc.text(letterhead.addressLine, MR, 24.5, { align: "right" });
  }
  if (letterhead.phone) {
    doc.text(letterhead.phone, MR, letterhead.addressLine ? 30 : 24.5, { align: "right" });
  }

  doc.setFillColor(30, 41, 59);
  doc.rect(0, 35, W, 1.8, "F");
  doc.setFillColor(...accentRgb);
  doc.rect(0, 36.8, W, 0.6, "F");
}

export function drawPdfFooter(
  doc: import("jspdf").jsPDF,
  letterhead: Awaited<ReturnType<typeof getSchoolLetterhead>>,
  rightText?: string,
) {
  const W = doc.internal.pageSize.getWidth();
  const H = doc.internal.pageSize.getHeight();
  const ML = 15;
  const MR = W - 15;

  doc.setFillColor(241, 245, 249);
  doc.rect(0, H - 14, W, 14, "F");
  doc.setDrawColor(226, 232, 240);
  doc.setLineWidth(0.3);
  doc.line(0, H - 14, W, H - 14);
  doc.setFont("helvetica", "normal");
  doc.setFontSize(6.5);
  doc.setTextColor(148, 163, 184);
  doc.text(letterhead.footerLeft, ML, H - 7);
  if (rightText || letterhead.cityLine) {
    doc.text(rightText || letterhead.cityLine, MR, H - 7, { align: "right" });
  }
}
