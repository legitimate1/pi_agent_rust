//! Shared retry state for RPC and print modes.
//!
//! Extracts the duplicated `consecutive`/`total`/`has_progress` retry machinery
//! previously inlined in `rpc::run_prompt_with_retry` and
//! `main::run_print_prompt_with_retry`. The module is intentionally
//! single-dependency: callers own the policy differences (`can_retry`,
//! `context_window`, `is_json` gating, interruptible sleep) and only delegate
//! counter bookkeeping and progress probing here.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::agent::AgentEvent;
use crate::config::Config;
use crate::model::AssistantMessageEvent;

/// Dual-counter retry bookkeeping.
///
/// * `consecutive` — resets to 0 when progress was observed before the retry.
/// * `total` — monotonic total attempts, caps the `*3` ceiling that prevents
///   infinite loops when progress keeps resetting `consecutive`.
///
/// `max_total` is derived as `max_retries.saturating_mul(3).max(max_retries)`.
#[derive(Debug, Clone, Copy)]
pub struct RetryCounters {
    pub consecutive: u32,
    pub total: u32,
    pub max_retries: u32,
    pub max_total: u32,
}

impl RetryCounters {
    #[must_use]
    pub fn new(max_retries: u32) -> Self {
        let max_total = max_retries.saturating_mul(3).max(max_retries);
        Self {
            consecutive: 0,
            total: 0,
            max_retries,
            max_total,
        }
    }

    #[must_use]
    pub fn max_total(&self) -> u32 {
        self.max_total
    }

    #[must_use]
    pub fn is_first(&self) -> bool {
        self.consecutive == 0 && self.total == 0
    }

    #[must_use]
    pub fn has_retried(&self) -> bool {
        self.consecutive > 0 || self.total > 0
    }

    /// Advance counters for the next retry attempt.
    ///
    /// When `has_progress` is true the consecutive counter is reset before
    /// incrementing, mirroring the existing `if has_progress { consecutive = 0; }`
    /// pattern in both call sites.
    pub fn advance(&mut self, has_progress: bool) -> u32 {
        if has_progress {
            self.consecutive = 0;
        }
        self.consecutive = self.consecutive.saturating_add(1);
        self.total = self.total.saturating_add(1);
        self.consecutive
    }

    #[must_use]
    pub fn should_retry(&self) -> bool {
        self.consecutive < self.max_retries && self.total < self.max_total
    }

    #[must_use]
    pub fn exhausted(&self) -> bool {
        self.consecutive >= self.max_retries || self.total >= self.max_total
    }

    #[must_use]
    pub fn delay_ms(&self, config: &Config) -> u32 {
        config.retry_delay_ms(self.consecutive)
    }
}

/// Progress probe shared between the agent event handler and the retry loop.
///
/// Wraps an `Arc<AtomicBool>` so clones can be moved into event-handler
/// closures while the loop retains a handle.
#[derive(Debug, Clone)]
pub struct RetryProgress(pub Arc<AtomicBool>);

impl Default for RetryProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryProgress {
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn set(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn clear(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    #[must_use]
    pub fn has_progress(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Atomically take the flag (read + clear).
    pub fn take(&self) -> bool {
        self.0.swap(false, Ordering::SeqCst)
    }

    #[must_use]
    pub fn inner(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.0)
    }

    /// Mark progress if `event` carries a non-empty delta.
    ///
    /// Centralises the `TextDelta/ThinkingDelta/ToolCallDelta` non-empty check
    /// previously duplicated in both retry paths.
    pub fn mark_if_progressed(&self, event: &AgentEvent) {
        if is_progress_event(event) {
            self.set();
        }
    }

    /// Wrap a base event handler so progress is tracked automatically.
    ///
    /// The returned closure first probes `event` for progress, then delegates
    /// to `base`. Used directly by `rpc.rs`; `main.rs` uses the same probe
    /// inside its `make_event_handler` factory.
    pub fn wrap<F>(&self, base: F) -> impl Fn(AgentEvent) + Send + Sync + 'static
    where
        F: Fn(AgentEvent) + Send + Sync + 'static,
    {
        let progress = Arc::clone(&self.0);
        move |event: AgentEvent| {
            if is_progress_event(&event) {
                progress.store(true, Ordering::SeqCst);
            }
            base(event);
        }
    }
}

/// Whether `event` represents observable forward progress (non-empty delta).
#[must_use]
pub fn is_progress_event(event: &AgentEvent) -> bool {
    if let AgentEvent::MessageUpdate {
        assistant_message_event:
            AssistantMessageEvent::TextDelta { delta, .. }
            | AssistantMessageEvent::ThinkingDelta { delta, .. }
            | AssistantMessageEvent::ToolCallDelta { delta, .. },
        ..
    } = event
    {
        !delta.is_empty()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_advance_resets_consecutive_on_progress() {
        let mut c = RetryCounters::new(3);
        c.advance(false);
        c.advance(false);
        assert_eq!(c.consecutive, 2);
        assert_eq!(c.total, 2);
        c.advance(true);
        assert_eq!(c.consecutive, 1);
        assert_eq!(c.total, 3);
    }

    #[test]
    fn counters_exhaustion_and_delay() {
        let config = Config::default();
        let mut c = RetryCounters::new(2);
        assert!(c.is_first());
        assert!(!c.has_retried());
        assert!(c.should_retry());
        assert!(!c.exhausted());
        c.advance(false);
        assert_eq!(c.delay_ms(&config), config.retry_delay_ms(1));
        c.advance(false);
        assert!(c.exhausted());
        assert!(!c.should_retry());
        assert!(c.has_retried());
    }

    #[test]
    fn progress_flag_behaviour() {
        let p = RetryProgress::new();
        assert!(!p.has_progress());
        p.set();
        assert!(p.has_progress());
        assert!(p.take());
        assert!(!p.has_progress());
        p.set();
        p.clear();
        assert!(!p.has_progress());
    }
}
