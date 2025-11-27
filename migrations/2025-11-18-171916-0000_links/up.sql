CREATE TABLE "public"."links" (
    "id" VARCHAR PRIMARY KEY,
    "url" TEXT NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
