-- Your SQL goes here
ALTER TABLE
    mm_operators
ADD
    last_assignment TIMESTAMP NOT NULL DEFAULT to_timestamp(0);