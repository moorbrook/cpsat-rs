//! Solver parameters builder.

use crate::proto::SatParameters;
use prost::Message;

impl SatParameters {
    /// Set the maximum solve time in seconds.
    #[must_use]
    pub fn with_max_time(mut self, seconds: f64) -> Self {
        self.max_time_in_seconds = Some(seconds);
        self
    }

    /// Set the number of parallel workers.
    #[must_use]
    pub fn with_num_workers(mut self, n: i32) -> Self {
        self.num_workers = Some(n);
        self
    }

    /// Enable or disable search progress logging to stderr.
    #[must_use]
    pub fn with_log_search_progress(mut self, enable: bool) -> Self {
        self.log_search_progress = Some(enable);
        self
    }

    /// Set the random seed for reproducibility.
    #[must_use]
    pub fn with_random_seed(mut self, seed: i32) -> Self {
        self.random_seed = Some(seed);
        self
    }

    /// Enable enumeration of all solutions.
    #[must_use]
    pub fn with_enumerate_all_solutions(mut self, enable: bool) -> Self {
        self.enumerate_all_solutions = Some(enable);
        self
    }

    /// Serialize to protobuf bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.encode_to_vec()
    }
}
