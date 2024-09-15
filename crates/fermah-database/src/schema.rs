// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "pr_payment"))]
    pub struct PrPayment;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "pr_status"))]
    pub struct PrStatus;
}

diesel::table! {
    avs_operators (id) {
        id -> Bytea,
        socket -> Nullable<Text>,
        is_el_registered -> Bool,
        registered_till_block -> Numeric,
    }
}

diesel::table! {
    avs_proof_requesters (id) {
        id -> Bytea,
        deposit -> Numeric,
    }
}

diesel::table! {
    mm_deadlines (pr_id) {
        pr_id -> Bytea,
        deadline -> Timestamp,
    }
}

diesel::table! {
    mm_operators (id) {
        id -> Bytea,
        last_interaction -> Timestamp,
        resource -> Bytea,
        reputation -> Int8,
        online -> Bool,
        last_assignment -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PrPayment;
    use super::sql_types::PrStatus;

    mm_proof_requests (id) {
        id -> Bytea,
        assigned -> Nullable<Bytea>,
        last_status_update -> Timestamp,
        payment -> PrPayment,
        amount -> Nullable<Numeric>,
        hash -> Bytea,
        public_key -> Bytea,
        payload -> Bytea,
        signature -> Bytea,
        requester -> Nullable<Bytea>,
        status -> PrStatus,
        rejection_message -> Nullable<Varchar>,
        operator_id -> Nullable<Bytea>,
        proof -> Nullable<Bytea>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    avs_operators,
    avs_proof_requesters,
    mm_deadlines,
    mm_operators,
    mm_proof_requests,
);
