//! Spam scoring module
//!
//! Provides advanced spam detection with rule-based scoring and Bayesian learning.

pub mod manager;
pub mod scorer;
pub mod types;

pub use manager::{SpamManager, SpamStats};
pub use scorer::{BayesianClassifier, SpamScorer};
pub use types::*;
