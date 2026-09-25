pub mod concurrency;
pub mod correction;
pub mod durable;
pub mod external;
pub mod kernel;
pub mod vectors;

use kernel::{state_sha256, JournalEntry, Kernel, State};
use vectors::Vector;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorRun {
    pub journal: Vec<JournalEntry>,
    pub final_state: State,
    pub state_sha256: String,
}

pub fn run_vector(vector: &Vector) -> VectorRun {
    let mut kernel = Kernel::new(vector.initial_state_v1_2b.clone());
    let journal = vector.events.iter().map(|e| kernel.apply(e)).collect();
    let final_state = kernel.into_state();
    let state_sha256 = state_sha256(&final_state);
    VectorRun {
        journal,
        final_state,
        state_sha256,
    }
}
