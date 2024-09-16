-- Your SQL goes here
CREATE TABLE avs_proof_requesters (
    id BYTEA PRIMARY KEY,
    deposit NUMERIC NOT NULL
);

CREATE TABLE avs_operators (
    id BYTEA PRIMARY KEY,
    socket TEXT DEFAULT NULL,
    is_el_registered BOOLEAN  NOT NULL DEFAULT FALSE,
    registered_till_block NUMERIC NOT NULL DEFAULT 0
);