-- CREATE SCHEMA IF NOT EXISTS api;

CREATE SCHEMA IF NOT EXISTS api;

-- Type: group_status

-- DROP TYPE IF EXISTS api.group_status;

CREATE TYPE api.group_status AS ENUM
    ('normal', 'banned');

ALTER TYPE api.group_status
    OWNER TO prim;

-- Table: api.group

-- DROP TABLE IF EXISTS api."group";

CREATE TABLE IF NOT EXISTS api."group"
(
    id          bigint                   NOT NULL DEFAULT nextval('api.group_id_seq'::regclass),
    group_id    bigint,
    name        character varying(255) COLLATE pg_catalog."default",
    avatar      text COLLATE pg_catalog."default",
    admin_list  json[],
    member_list json[],
    status      api.group_status,
    info        json,
    create_at   timestamp with time zone NOT NULL,
    update_at   timestamp with time zone NOT NULL,
    delete_at   timestamp with time zone,
    CONSTRAINT group_pkey PRIMARY KEY (id)
)
    TABLESPACE pg_default;

ALTER TABLE IF EXISTS api."group"
    OWNER to prim;

-- Type: user_status

-- DROP TYPE IF EXISTS api.user_status;

CREATE TYPE api.user_status AS ENUM
    ('online', 'busy', 'away');

ALTER TYPE api.user_status
    OWNER TO prim;

-- Table: api.user

-- DROP TABLE IF EXISTS api."user";

CREATE TABLE IF NOT EXISTS api."user"
(
    id         bigserial,
    account_id bigint                                             NOT NULL,
    credential character varying(64) COLLATE pg_catalog."default" NOT NULL,
    salt       character varying(32) COLLATE pg_catalog."default" NOT NULL,
    nickname   character varying(32) COLLATE pg_catalog."default" DEFAULT ''::character varying,
    avatar     text                                               DEFAULT ''::character varying,
    signature  character varying(64) COLLATE pg_catalog."default" DEFAULT ''::character varying,
    status     api.user_status,
    info       json,
    create_at  timestamp with time zone                           NOT NULL,
    update_at  timestamp with time zone                           NOT NULL,
    delete_at  timestamp with time zone                           DEFAULT '1970-01-01 00:00:00 +00:00:00',
    CONSTRAINT user_pkey PRIMARY KEY (id)
)
    TABLESPACE pg_default;

ALTER TABLE IF EXISTS api."user"
    OWNER to prim;

-- CREATE SCHEMA IF NOT EXISTS msg;

CREATE SCHEMA IF NOT EXISTS msg;

-- Type: message_status

-- DROP TYPE IF EXISTS msg.message_status;

CREATE TYPE msg.message_status AS ENUM
    ('normal', 'withdraw', 'edit');

ALTER TYPE msg.message_status
    OWNER TO prim;

-- Table: msg.message

-- DROP TABLE IF EXISTS msg.message;

CREATE TABLE IF NOT EXISTS msg.message
(
    id          bigserial,
    sender      bigint                   NOT NULL,
    receiver    bigint                   NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    seq_num     bigint                   NOT NULL,
    type        smallint                 NOT NULL,
    version     smallint                 NOT NULL,
    extension   character varying(86)    COLLATE pg_catalog."default",
    payload     character varying(5462)  COLLATE pg_catalog."default",
    status      msg.message_status       NOT NULL,
    CONSTRAINT message_pkey PRIMARY KEY (id)
)
    TABLESPACE pg_default;

ALTER TABLE IF EXISTS msg.message
    OWNER to prim;

-- Index: receiver_index

-- DROP INDEX IF EXISTS msg.receiver_index;

CREATE INDEX IF NOT EXISTS receiver_index
    ON msg.message USING btree
    (receiver ASC NULLS LAST)
    TABLESPACE pg_default;

-- Index: sender_index

-- DROP INDEX IF EXISTS msg.sender_index;

CREATE INDEX IF NOT EXISTS sender_index
    ON msg.message USING btree
    (sender ASC NULLS LAST)
    TABLESPACE pg_default;

-- Index: msg_history_index

-- DROP INDEX IF EXISTS msg.msg_history_index;

CREATE INDEX IF NOT EXISTS msg_history_index
    ON msg.message (sender, receiver, seq_num);

-- Type: user_relationship_status

-- DROP TYPE IF EXISTS api.user_relationship_status;

CREATE TYPE api.user_relationship_status AS ENUM
    ('normal', 'lover', 'best_friend', 'deleting', 'deleted', 'blocked');

ALTER TYPE api.user_relationship_status
    OWNER TO prim;


-- Table: api.user_relationship

-- DROP TABLE IF EXISTS api.user_relationship;

CREATE TABLE IF NOT EXISTS api.user_relationship
(
    id             bigserial,
    user_id        bigint                                              NOT NULL,
    peer_id        bigint                                              NOT NULL,
    remark         character varying(128) COLLATE pg_catalog."default",
    status         api.user_relationship_status                        NOT NULL,
    classification character varying(128) COLLATE pg_catalog."default" NOT NULL,
    tag_list       character varying(128)[] COLLATE pg_catalog."default",
    create_at      timestamp with time zone                            NOT NULL,
    update_at      timestamp with time zone                            NOT NULL,
    delete_at      timestamp with time zone DEFAULT '1970-01-01 00:00:00 +00:00:00',
    CONSTRAINT user_id_peer_id_delete_at UNIQUE (user_id, peer_id, delete_at)
)
    TABLESPACE pg_default;

ALTER TABLE IF EXISTS api.user_relationship
    OWNER to prim;

-- Type: user_group_role

-- DROP TYPE IF EXISTS api.user_group_role;

CREATE TYPE api.user_group_role AS ENUM
    ('member', 'admin');

ALTER TYPE api.user_group_role
    OWNER TO prim;

-- Table: api.user_group_list

-- DROP TABLE IF EXISTS api.user_group_list;

CREATE TABLE IF NOT EXISTS api.user_group_list
(
    id        bigserial,
    user_id   bigint                   NOT NULL,
    group_id  bigint                   NOT NULL,
    role      api.user_group_role      NOT NULL,
    create_at timestamp with time zone NOT NULL,
    update_at timestamp with time zone NOT NULL,
    delete_at timestamp with time zone DEFAULT '1970-01-01 00:00:00 +00:00:00',
    CONSTRAINT user_group_list_pkey PRIMARY KEY (id),
    CONSTRAINT user_id_group_id_delete_at UNIQUE (user_id, group_id, delete_at)
)
    TABLESPACE pg_default;

ALTER TABLE IF EXISTS api.user_group_list
    OWNER to prim;