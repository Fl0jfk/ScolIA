import { PGlite } from "@electric-sql/pglite";
import { drizzle } from "drizzle-orm/pglite";
import { randomUUID } from "node:crypto";
import * as schema from "@/db/schema";
import { etablissement } from "@/db/schema";
import type { Db } from "@/db/index";

const STAGE_TEST_SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS "etablissement" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "slug" text NOT NULL,
  "name" text NOT NULL,
  "data_bucket" text,
  "created_at" timestamptz DEFAULT now() NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS "etablissement_slug_uidx" ON "etablissement" ("slug");

CREATE TABLE IF NOT EXISTS "stage_convention" (
  "id" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "status" text,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE TABLE IF NOT EXISTS "stage_convention_attr" (
  "etablissement_id" uuid NOT NULL,
  "convention_id" text NOT NULL,
  "path" text NOT NULL,
  "value" text NOT NULL,
  PRIMARY KEY("etablissement_id","convention_id","path")
);

CREATE TABLE IF NOT EXISTS "stage_token" (
  "token" text PRIMARY KEY NOT NULL,
  "etablissement_id" uuid NOT NULL,
  "kind" text DEFAULT '' NOT NULL,
  "updated_at" timestamptz DEFAULT now() NOT NULL
);

CREATE TABLE IF NOT EXISTS "stage_token_attr" (
  "etablissement_id" uuid NOT NULL,
  "token" text NOT NULL,
  "path" text NOT NULL,
  "value" text NOT NULL,
  PRIMARY KEY("etablissement_id","token","path")
);

CREATE TABLE IF NOT EXISTS "ent_collection_record" (
  "etablissement_id" uuid NOT NULL,
  "collection" text NOT NULL,
  "record_id" text NOT NULL,
  "status" text,
  "updated_at" timestamptz DEFAULT now() NOT NULL,
  PRIMARY KEY("etablissement_id","collection","record_id")
);

CREATE TABLE IF NOT EXISTS "ent_collection_attr" (
  "etablissement_id" uuid NOT NULL,
  "collection" text NOT NULL,
  "record_id" text NOT NULL,
  "path" text NOT NULL,
  "value" text NOT NULL,
  PRIMARY KEY("etablissement_id","collection","record_id","path")
);
`;

export type StagePgliteFixture = {
  client: PGlite;
  db: Db;
  etablissementId: string;
};

export async function createStagePgliteFixture(): Promise<StagePgliteFixture> {
  const client = new PGlite();
  await client.exec(STAGE_TEST_SCHEMA_SQL);
  const db = drizzle(client, { schema }) as unknown as Db;
  const slug = `stage-test-${randomUUID().slice(0, 8)}`;
  const [row] = await db
    .insert(etablissement)
    .values({ slug, name: "Stage test tenant", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });
  if (!row) throw new Error("etablissement test non créé");
  return { client, db, etablissementId: row.id };
}

export async function destroyStagePgliteFixture(fixture: StagePgliteFixture): Promise<void> {
  await fixture.client.close();
}
