/**
 * Répare les ACL public-read sur travels/auto/* du bucket CDN scolia-images.
 * Usage: node --env-file=.env.local scripts/fix-travel-auto-image-acl.mjs
 */
import {
  ListObjectsV2Command,
  PutObjectAclCommand,
  S3Client,
} from "@aws-sdk/client-s3";

const bucket = process.env.IMAGE_BUCKET?.trim() || "scolia-images";
const accessKeyId = process.env.ACCESS_KEY_ID?.trim();
const secretAccessKey = process.env.SECRET_ACCESS_KEY?.trim();
if (!accessKeyId || !secretAccessKey) {
  console.error("ACCESS_KEY_ID / SECRET_ACCESS_KEY manquants");
  process.exit(1);
}

const client = new S3Client({
  region: "fr-par",
  endpoint: process.env.S3_ENDPOINT?.trim() || "https://s3.fr-par.scw.cloud",
  forcePathStyle: process.env.S3_FORCE_PATH_STYLE !== "false",
  credentials: { accessKeyId, secretAccessKey },
  requestChecksumCalculation: "WHEN_REQUIRED",
  responseChecksumValidation: "WHEN_REQUIRED",
});

let token;
let fixed = 0;
let failed = 0;
const keys = [];

do {
  const listed = await client.send(
    new ListObjectsV2Command({
      Bucket: bucket,
      Prefix: "travels/auto/",
      ContinuationToken: token,
    }),
  );
  for (const obj of listed.Contents || []) {
    if (obj.Key) keys.push(obj.Key);
  }
  token = listed.IsTruncated ? listed.NextContinuationToken : undefined;
} while (token);

console.log(`Found ${keys.length} objects in ${bucket}/travels/auto/`);

for (const key of keys) {
  try {
    await client.send(
      new PutObjectAclCommand({
        Bucket: bucket,
        Key: key,
        ACL: "public-read",
      }),
    );
    fixed += 1;
    console.log("OK", key);
  } catch (err) {
    failed += 1;
    console.error("FAIL", key, err instanceof Error ? err.message : err);
  }
}

console.log({ fixed, failed, bucket });
if (keys[0]) {
  const url = `https://${bucket}.s3.fr-par.scw.cloud/${keys[0]
    .split("/")
    .map(encodeURIComponent)
    .join("/")}`;
  const probe = await fetch(url);
  console.log("probe", probe.status, url);
}
