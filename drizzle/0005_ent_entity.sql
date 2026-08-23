CREATE TABLE "ent_entity" (
	"etablissement_id" uuid NOT NULL,
	"kind" text NOT NULL,
	"record_id" text NOT NULL,
	"status" text,
	"payload" jsonb NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "ent_entity_pk" PRIMARY KEY("etablissement_id","kind","record_id")
);--> statement-breakpoint
ALTER TABLE "ent_entity" ADD CONSTRAINT "ent_entity_etablissement_id_etablissement_id_fk" FOREIGN KEY ("etablissement_id") REFERENCES "public"."etablissement"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "ent_entity_kind_idx" ON "ent_entity" USING btree ("etablissement_id","kind");--> statement-breakpoint
CREATE INDEX "ent_entity_kind_status_idx" ON "ent_entity" USING btree ("etablissement_id","kind","status");
