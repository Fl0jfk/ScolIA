import "server-only";

import fs from "node:fs";
import https from "node:https";
import { URL } from "node:url";

export type OmogenFetchResult = {
  ok: boolean;
  status: number;
  contentType: string;
  buffer: Buffer;
};

function readOptionalFile(path: string | undefined): Buffer | undefined {
  const p = path?.trim();
  if (!p) return undefined;
  return fs.readFileSync(p);
}

/** Télécharge le flux Omogen avec certificat client (mTLS). */
export function fetchOmogenPayload(url: string): Promise<OmogenFetchResult> {
  const certPath = process.env.OMOGEN_CLIENT_CERT?.trim();
  const keyPath = process.env.OMOGEN_CLIENT_KEY?.trim();
  if (!certPath || !keyPath) {
    return Promise.reject(
      new Error("Certificat Omogen manquant (OMOGEN_CLIENT_CERT / OMOGEN_CLIENT_KEY)."),
    );
  }

  const cert = fs.readFileSync(certPath);
  const key = fs.readFileSync(keyPath);
  const ca = readOptionalFile(process.env.OMOGEN_CA_CERT);
  const insecure = process.env.OMOGEN_TLS_INSECURE?.trim() === "1";
  const parsed = new URL(url);

  const options: https.RequestOptions = {
    protocol: parsed.protocol,
    hostname: parsed.hostname,
    port: parsed.port || undefined,
    path: `${parsed.pathname}${parsed.search}`,
    method: "GET",
    cert,
    key,
    ...(ca ? { ca } : {}),
    rejectUnauthorized: !insecure,
    headers: { Accept: "application/zip, application/xml, text/xml" },
  };

  return new Promise((resolve, reject) => {
    const req = https.request(options, (res) => {
      const chunks: Buffer[] = [];
      res.on("data", (chunk: Buffer) => chunks.push(chunk));
      res.on("end", () => {
        const buffer = Buffer.concat(chunks);
        const status = res.statusCode ?? 500;
        resolve({
          ok: status >= 200 && status < 300,
          status,
          contentType: String(res.headers["content-type"] || ""),
          buffer,
        });
      });
    });
    req.on("error", reject);
    req.end();
  });
}
