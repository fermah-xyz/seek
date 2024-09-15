use ethers::types::Address;
use opentelemetry::{global::meter, metrics::Counter, KeyValue};

#[derive(Clone)]
pub struct Metrics {
    proof_requests: Counter<u64>,
}

impl Metrics {
    pub fn init() -> Self {
        let m = meter("rpc metrics");
        let proof_requests = m.u64_counter("proof_requests").init();

        Self { proof_requests }
    }

    pub fn inc_proof_requests(&self, seeker: Address, valid: bool) {
        self.proof_requests.add(
            1,
            &[
                KeyValue::new("valid", valid),
                KeyValue::new("seeker", format!("{:#?}", seeker)),
            ],
        )
    }
}
