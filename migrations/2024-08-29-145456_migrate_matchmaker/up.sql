-- Your SQL goes here
CREATE TYPE pr_status AS ENUM (
    'Created',
    'Accepted',
    'Cancelled',
    'Rejected',
    'Assigned',
    'AcknowledgedAssignment',
    'ProofBeingTested',
    'Proven'
);

CREATE TYPE pr_payment AS ENUM (
    'Nothing',
    'ToReserve',
    'Reserved',
    'ReadyToPay',
    'Paid',
    'Refund'
);

CREATE TABLE mm_proof_requests (
    id BYTEA PRIMARY KEY,
    assigned BYTEA NULL DEFAULT NULL,
    last_status_update TIMESTAMP NOT NULL,
    -- payment
    payment pr_payment NOT NULL,
    amount NUMERIC NULL DEFAULT NULL,
    -- payload
    hash BYTEA NOT NULL,
    public_key BYTEA NOT NULL,
    payload BYTEA NOT NULL,
    signature BYTEA NOT NULL,
    requester BYTEA NULL DEFAULT NULL,
    -- request status
    status pr_status NOT NULL,
    rejection_message VARCHAR NULL DEFAULT NULL,
    operator_id BYTEA NULL DEFAULT NULL,
    proof BYTEA NULL DEFAULT NULL
);

CREATE TABLE mm_operators (
    id BYTEA PRIMARY KEY,
    last_interaction TIMESTAMP NOT NULL,
    resource BYTEA NOT NULL,
    reputation BIGINT NOT NULL DEFAULT 0,
    online BOOLEAN NOT NULL
)