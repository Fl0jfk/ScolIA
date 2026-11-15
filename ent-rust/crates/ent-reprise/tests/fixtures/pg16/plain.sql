--
-- PostgreSQL database dump
--

\restrict DxJbE8LOqx0QwJrGhBMDPdNbSqNfPXi653IZLlYuH1c1i28DvjOB3ZKZzzlp8xR

-- Dumped from database version 16.15 (Ubuntu 16.15-0ubuntu0.24.04.1)
-- Dumped by pg_dump version 16.15 (Ubuntu 16.15-0ubuntu0.24.04.1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: etablissement; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.etablissement (
    id uuid NOT NULL,
    slug text NOT NULL,
    name text NOT NULL,
    data_bucket text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    tags text[]
);


--
-- Name: user; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public."user" (
    id text NOT NULL,
    name text,
    by text,
    montant numeric(12,2),
    options jsonb
);


--
-- Data for Name: etablissement; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.etablissement (id, slug, name, data_bucket, created_at, tags) FROM stdin;
11111111-1111-1111-1111-111111111111	fictif	Collège des Brumes (fictif)	\N	2026-09-01 08:00:00+00	{"a b",c,NULL}
22222222-2222-2222-2222-222222222222	tab	Ligne avec\ttabulation et \\ antislash	ligne1\nligne2	2026-09-02 08:00:00+00	{}
\.


--
-- Data for Name: user; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public."user" (id, name, by, montant, options) FROM stdin;
u1	Aldric Zorglub	x	12.50	{"n": 3, "nom": "Zorglub"}
u2	\N		0.00	\N
\.


--
-- PostgreSQL database dump complete
--

\unrestrict DxJbE8LOqx0QwJrGhBMDPdNbSqNfPXi653IZLlYuH1c1i28DvjOB3ZKZzzlp8xR

