-- Database: prim

-- DROP DATABASE IF EXISTS prim;

CREATE DATABASE prim
    WITH
    OWNER = prim
    ENCODING = 'UTF8'
    CONNECTION LIMIT = -1
    IS_TEMPLATE = False;

-- SCHEMA: api

-- DROP SCHEMA IF EXISTS api ;

CREATE SCHEMA IF NOT EXISTS api
    AUTHORIZATION prim;

-- Table: api.user

-- DROP TABLE IF EXISTS api."user";

CREATE TABLE IF NOT EXISTS api."user"
(
    id bigint NOT NULL DEFAULT nextval('api.user_id_seq'::regclass),
    account_id bigint NOT NULL DEFAULT nextval('api.user_account_id_seq'::regclass),
    credential character varying(64) COLLATE pg_catalog."default" NOT NULL,
    salt character varying(32) COLLATE pg_catalog."default" NOT NULL,
    nickname character varying(32) COLLATE pg_catalog."default" DEFAULT ''::character varying,
    signature character varying(64) COLLATE pg_catalog."default" DEFAULT ''::character varying,
    create_at timestamp with time zone NOT NULL,
    update_at timestamp with time zone NOT NULL,
    delete_at timestamp with time zone,
    CONSTRAINT user_pkey PRIMARY KEY (id)
)

    TABLESPACE pg_default;

ALTER TABLE IF EXISTS api."user"
    OWNER to prim;