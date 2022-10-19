-- Table: msg.message

-- DROP TABLE IF EXISTS msg.message;

CREATE ROLE prim WITH
    LOGIN
    NOSUPERUSER
    CREATEDB
    NOCREATEROLE
    INHERIT
    REPLICATION
    CONNECTION LIMIT -1
    PASSWORD 'prim.123456';

CREATE DATABASE prim_msg
    WITH
    OWNER = prim
    ENCODING = 'UTF8'
    CONNECTION LIMIT = -1
    IS_TEMPLATE = False;

CREATE SCHEMA msg
    AUTHORIZATION prim;

CREATE TABLE IF NOT EXISTS msg.message
(
    id bigint NOT NULL DEFAULT nextval('msg.message_id_seq'::regclass),
    sender bigint NOT NULL,
    receiver bigint NOT NULL,
    "timestamp" time with time zone NOT NULL,
                         seq_num bigint NOT NULL,
                         type smallint NOT NULL,
                         version smallint NOT NULL,
                         extension character varying(86) COLLATE pg_catalog."default",
    payload character varying(5462) COLLATE pg_catalog."default",
    status smallint NOT NULL DEFAULT 1,
    CONSTRAINT message_pkey PRIMARY KEY (id)
    )

    TABLESPACE pg_default;

ALTER TABLE IF EXISTS msg.message
    OWNER to prim;