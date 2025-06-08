use crate::solver::engine::SearchStats;

/// A trait for strategies that determine when to restart the search.
pub trait RestartPolicy {
    /// Given the current search statistics, decides whether to restart.
    ///
    /// # Arguments
    ///
    /// * `stats`: The statistics from the most recent search attempt.
    ///
    /// # Returns
    ///
    /// * `true` if the search should be restarted.
    /// * `false` if the search should terminate.
    fn should_restart(&self, stats: &SearchStats) -> bool;
}

/// A policy that never triggers a restart.
pub struct NoRestartPolicy;

impl RestartPolicy for NoRestartPolicy {
    fn should_restart(&self, _stats: &SearchStats) -> bool {
        false
    }
}

/// A policy that restarts the search after a fixed number of backtracks.
pub struct RestartAfterNBacktracks {
    pub max_backtracks: u64,
}

impl RestartPolicy for RestartAfterNBacktracks {
    fn should_restart(&self, stats: &SearchStats) -> bool {
        stats.backtracks >= self.max_backtracks
    }
}
