//! Pure turn-recovery classification, mode gating, and per-run nudge budget.
//!
//! This module deliberately has no provider, session, or Agent-loop side effects.
//! Callers classify the completed assistant text and use [`TurnRecoveryState`]
//! to decide whether a normal user nudge should be injected.

use serde::{Deserialize, Serialize};

use crate::model::StopReason;

/// Maximum number of automatic continuation nudges in one logical run.
pub const MAX_AUTO_CONTINUATIONS: u8 = 2;

/// How aggressively to recover assistant turns that ended unexpectedly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TurnRecoveryMode {
    /// Never inject an automatic continuation.
    Off,
    /// Recover only token-budget truncation and clearly unfinished structure.
    #[default]
    Conservative,
    /// Also recover an assistant promise to begin work that was not followed
    /// by another sentence or paragraph.
    Aggressive,
}

/// The heuristic classification of a completed assistant turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryClass {
    /// The turn ended without a recovery signal.
    CleanStop,
    /// The provider stopped because the token budget was exhausted.
    BudgetTruncated,
    /// The visible output ends in an unclosed code fence or list item.
    UnclosedStructure,
    /// The visible output promises imminent work and then stops.
    SemanticPrematureStop,
}

impl RecoveryClass {
    /// Return the stable, human-readable reason embedded in a recovery nudge.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::CleanStop => "clean stop",
            Self::BudgetTruncated => "response truncated by token budget",
            Self::UnclosedStructure => "response ended inside unfinished output",
            Self::SemanticPrematureStop => "announced work was not started",
        }
    }

    const fn is_actionable(self, mode: TurnRecoveryMode) -> bool {
        match self {
            Self::CleanStop => false,
            Self::BudgetTruncated | Self::UnclosedStructure => {
                !matches!(mode, TurnRecoveryMode::Off)
            }
            Self::SemanticPrematureStop => matches!(mode, TurnRecoveryMode::Aggressive),
        }
    }
}

/// Classify a completed assistant turn from its provider stop reason and text.
///
/// The caller is responsible for passing only visible assistant text. Thinking
/// blocks, tool calls, and tool arguments are intentionally outside this pure
/// text classifier.
#[must_use]
pub fn classify(stop_reason: StopReason, text: &str) -> RecoveryClass {
    match stop_reason {
        StopReason::Length => return RecoveryClass::BudgetTruncated,
        StopReason::Stop => {}
        // This wildcard also keeps the intended CleanStop behavior if the
        // model later gains another provider-specific stop reason.
        _ => return RecoveryClass::CleanStop,
    }

    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return RecoveryClass::CleanStop;
    }
    if has_unclosed_fence(trimmed) || ends_on_dangling_list_item(trimmed) {
        return RecoveryClass::UnclosedStructure;
    }
    if ends_on_unfulfilled_promise(trimmed) {
        return RecoveryClass::SemanticPrematureStop;
    }
    RecoveryClass::CleanStop
}

/// A recovery decision and the exact ordinary-user-message nudge to inject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAction {
    /// The class that caused this action.
    pub class: RecoveryClass,
    /// One-based continuation number within the logical run.
    pub attempt: u8,
    /// Stable human-readable explanation used in [`Self::nudge_text`].
    pub reason: &'static str,
    /// Exact user-message text for the continuation.
    pub nudge_text: String,
}

impl RecoveryAction {
    /// Borrow the exact nudge text without exposing its allocation details.
    #[must_use]
    pub fn nudge(&self) -> &str {
        &self.nudge_text
    }
}

/// Per-logical-run recovery budget.
#[derive(Debug, Default)]
pub struct TurnRecoveryState {
    continuations: u8,
}

impl TurnRecoveryState {
    /// Create an unused recovery state for a new logical run.
    #[must_use]
    pub const fn new() -> Self {
        Self { continuations: 0 }
    }

    /// Return the number of nudges already issued in this logical run.
    #[must_use]
    pub const fn continuations(&self) -> u8 {
        self.continuations
    }

    /// Evaluate a classified stop and consume one nudge slot when actionable.
    ///
    /// Once a logical run has issued any recovery nudge, only the provider's
    /// explicit budget-truncation signal can consume the remaining budget.
    /// Structure and semantic heuristics are intentionally one-shot per run.
    pub fn evaluate(
        &mut self,
        mode: TurnRecoveryMode,
        class: RecoveryClass,
    ) -> Option<RecoveryAction> {
        if !class.is_actionable(mode) {
            return None;
        }
        if self.continuations > 0 && !matches!(class, RecoveryClass::BudgetTruncated) {
            return None;
        }
        if self.continuations >= MAX_AUTO_CONTINUATIONS {
            return None;
        }

        self.continuations += 1;
        let reason = class.reason();
        let attempt = self.continuations;
        Some(RecoveryAction {
            class,
            attempt,
            reason,
            nudge_text: format!(
                "[auto-continue {attempt}/{MAX_AUTO_CONTINUATIONS}: {reason}] Continue from exactly where you stopped. Do not repeat content you already produced; finish the remaining work."
            ),
        })
    }

    /// Classify text and evaluate it in one call for Agent-loop callers.
    pub fn evaluate_turn(
        &mut self,
        mode: TurnRecoveryMode,
        stop_reason: StopReason,
        text: &str,
    ) -> Option<RecoveryAction> {
        self.evaluate(mode, classify(stop_reason, text))
    }
}

fn has_unclosed_fence(text: &str) -> bool {
    let delimiter_count = text
        .lines()
        .filter(|line| line.trim_start().starts_with("```"))
        .count();
    delimiter_count % 2 == 1
}

fn ends_on_dangling_list_item(text: &str) -> bool {
    let Some(last_line) = text.lines().next_back() else {
        return false;
    };
    let last_line = last_line.trim();
    if last_line.is_empty() {
        return false;
    }
    if matches!(last_line, "-" | "*" | "+") {
        return true;
    }
    last_line.strip_suffix('.').is_some_and(|prefix| {
        !prefix.is_empty() && prefix.chars().all(|character| character.is_ascii_digit())
    })
}

fn ends_on_unfulfilled_promise(text: &str) -> bool {
    const PROMISE_WINDOW_CHARS: usize = 240;
    const PROMISE_PHRASES: &[&str] = &[
        "i will now",
        "i'll now",
        "let me now",
        "i am going to",
        "i'm going to",
        "next, i will",
        "next, i'll",
        "now i will",
        "now i'll",
        "proceeding to",
    ];

    let tail: String = text
        .chars()
        .rev()
        .take(PROMISE_WINDOW_CHARS)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let tail_lower = tail.to_lowercase();
    let Some(position) = PROMISE_PHRASES
        .iter()
        .filter_map(|phrase| tail_lower.rfind(phrase))
        .max()
    else {
        return false;
    };

    let announcement =
        tail_lower[position..].trim_end_matches(['.', ':', '!', '?', '…', ' ', '\n', '\r']);
    !has_obvious_following_sentence(announcement)
}

fn has_obvious_following_sentence(announcement: &str) -> bool {
    if announcement.contains(":\n\n") {
        return true;
    }

    let bytes = announcement.as_bytes();
    for index in 0..bytes.len().saturating_sub(1) {
        if !matches!(bytes[index], b'.' | b'!' | b'?') {
            continue;
        }
        if !bytes[index + 1].is_ascii_whitespace() {
            continue;
        }
        if !announcement[index + 1..].trim().is_empty() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_serde_and_default_are_stable() {
        assert_eq!(TurnRecoveryMode::default(), TurnRecoveryMode::Conservative);
        assert_eq!(
            serde_json::to_string(&TurnRecoveryMode::Off).unwrap(),
            "\"off\""
        );
        assert_eq!(
            serde_json::to_string(&TurnRecoveryMode::Conservative).unwrap(),
            "\"conservative\""
        );
        assert_eq!(
            serde_json::from_str::<TurnRecoveryMode>("\"aggressive\"").unwrap(),
            TurnRecoveryMode::Aggressive
        );
        assert!(serde_json::from_str::<TurnRecoveryMode>("\"unknown\"").is_err());
    }

    #[test]
    fn length_is_budget_truncated_and_other_stop_reasons_are_clean() {
        assert_eq!(
            classify(StopReason::Length, "anything"),
            RecoveryClass::BudgetTruncated
        );
        for reason in [StopReason::ToolUse, StopReason::Error, StopReason::Aborted] {
            assert_eq!(
                classify(reason, "```\nunfinished\n-"),
                RecoveryClass::CleanStop
            );
        }
        assert_eq!(classify(StopReason::Stop, ""), RecoveryClass::CleanStop);
        assert_eq!(
            classify(StopReason::Stop, "The work is complete."),
            RecoveryClass::CleanStop
        );
    }

    #[test]
    fn code_fence_delimiters_use_odd_even_parity() {
        assert_eq!(
            classify(StopReason::Stop, "before\n```rust\nunfinished"),
            RecoveryClass::UnclosedStructure
        );
        assert_eq!(
            classify(StopReason::Stop, "before\n```rust\ncomplete\n```"),
            RecoveryClass::CleanStop
        );
        assert_eq!(
            classify(StopReason::Stop, "```\none\n```\ntwo\n```"),
            RecoveryClass::UnclosedStructure
        );
    }

    #[test]
    fn dangling_and_complete_list_items_are_distinguished() {
        for item in ["-", "*", "+", "2.", "42."] {
            assert_eq!(
                classify(StopReason::Stop, &format!("Plan:\n{item}")),
                RecoveryClass::UnclosedStructure,
                "dangling item: {item}"
            );
        }
        for item in [
            "- finish",
            "* finish",
            "+ finish",
            "2. finish",
            "42. finish",
        ] {
            assert_eq!(
                classify(StopReason::Stop, &format!("Plan:\n{item}")),
                RecoveryClass::CleanStop,
                "complete item: {item}"
            );
        }
    }

    #[test]
    fn promise_phrases_are_detected_only_at_the_tail() {
        for phrase in [
            "i will now",
            "i'll now",
            "let me now",
            "i am going to",
            "i'm going to",
            "next, i will",
            "next, i'll",
            "now i will",
            "now i'll",
            "proceeding to",
        ] {
            assert_eq!(
                classify(StopReason::Stop, &format!("{phrase} update the file.")),
                RecoveryClass::SemanticPrematureStop,
                "promise phrase: {phrase}"
            );
        }

        let outside_window = format!("i will now update the file.{}", "x".repeat(240));
        assert_eq!(
            classify(StopReason::Stop, &outside_window),
            RecoveryClass::CleanStop
        );
        assert_eq!(
            classify(
                StopReason::Stop,
                "I will now update the file.\n\nThe update is complete."
            ),
            RecoveryClass::CleanStop
        );
        assert_eq!(
            classify(
                StopReason::Stop,
                "I will now update the file. Done and verified."
            ),
            RecoveryClass::CleanStop
        );
    }

    #[test]
    fn mode_gating_matches_the_contract() {
        for mode in [
            TurnRecoveryMode::Off,
            TurnRecoveryMode::Conservative,
            TurnRecoveryMode::Aggressive,
        ] {
            let mut state = TurnRecoveryState::new();
            assert_eq!(
                state
                    .evaluate(mode, RecoveryClass::CleanStop)
                    .map(|action| action.class),
                None
            );
        }

        let mut off = TurnRecoveryState::new();
        assert!(
            off.evaluate(TurnRecoveryMode::Off, RecoveryClass::BudgetTruncated)
                .is_none()
        );
        assert!(
            off.evaluate(TurnRecoveryMode::Off, RecoveryClass::UnclosedStructure)
                .is_none()
        );
        assert!(
            off.evaluate(TurnRecoveryMode::Off, RecoveryClass::SemanticPrematureStop)
                .is_none()
        );

        let mut conservative = TurnRecoveryState::new();
        assert!(
            conservative
                .evaluate(
                    TurnRecoveryMode::Conservative,
                    RecoveryClass::BudgetTruncated
                )
                .is_some()
        );
        assert!(
            conservative
                .evaluate(
                    TurnRecoveryMode::Conservative,
                    RecoveryClass::SemanticPrematureStop
                )
                .is_none()
        );

        let mut aggressive = TurnRecoveryState::new();
        assert!(
            aggressive
                .evaluate(
                    TurnRecoveryMode::Aggressive,
                    RecoveryClass::SemanticPrematureStop
                )
                .is_some()
        );
    }

    #[test]
    fn state_caps_at_two_and_nudge_has_exact_reason_and_number() {
        let mut state = TurnRecoveryState::new();
        let first = state
            .evaluate(
                TurnRecoveryMode::Conservative,
                RecoveryClass::BudgetTruncated,
            )
            .unwrap();
        assert_eq!(first.attempt, 1);
        assert_eq!(first.reason, "response truncated by token budget");
        assert_eq!(
            first.nudge(),
            "[auto-continue 1/2: response truncated by token budget] Continue from exactly where you stopped. Do not repeat content you already produced; finish the remaining work."
        );

        let second = state
            .evaluate(
                TurnRecoveryMode::Conservative,
                RecoveryClass::BudgetTruncated,
            )
            .unwrap();
        assert_eq!(second.attempt, 2);
        assert!(
            state
                .evaluate(
                    TurnRecoveryMode::Conservative,
                    RecoveryClass::BudgetTruncated
                )
                .is_none()
        );
        assert_eq!(state.continuations(), MAX_AUTO_CONTINUATIONS);
    }

    #[test]
    fn clean_stops_do_not_consume_budget() {
        let mut state = TurnRecoveryState::new();
        for _ in 0..5 {
            assert!(
                state
                    .evaluate(TurnRecoveryMode::Conservative, RecoveryClass::CleanStop)
                    .is_none()
            );
        }
        assert_eq!(state.continuations(), 0);
    }

    #[test]
    fn structure_recovery_does_not_repeat_after_any_recovery_but_length_remains_available() {
        let mut state = TurnRecoveryState::new();
        assert!(
            state
                .evaluate(
                    TurnRecoveryMode::Conservative,
                    RecoveryClass::BudgetTruncated
                )
                .is_some()
        );
        assert!(
            state
                .evaluate(
                    TurnRecoveryMode::Conservative,
                    RecoveryClass::UnclosedStructure
                )
                .is_none()
        );
        assert_eq!(state.continuations(), 1);
        assert!(
            state
                .evaluate(
                    TurnRecoveryMode::Conservative,
                    RecoveryClass::BudgetTruncated
                )
                .is_some()
        );
        assert_eq!(state.continuations(), 2);
    }

    #[test]
    fn a_new_state_resets_the_logical_run_budget() {
        let mut old_run = TurnRecoveryState::new();
        assert!(
            old_run
                .evaluate(
                    TurnRecoveryMode::Conservative,
                    RecoveryClass::BudgetTruncated
                )
                .is_some()
        );
        assert_eq!(old_run.continuations(), 1);

        let new_run = TurnRecoveryState::new();
        assert_eq!(new_run.continuations(), 0);
    }

    #[test]
    fn evaluate_turn_composes_classification_and_state() {
        let mut state = TurnRecoveryState::new();
        let action = state
            .evaluate_turn(
                TurnRecoveryMode::Conservative,
                StopReason::Length,
                "partial output",
            )
            .unwrap();
        assert_eq!(action.class, RecoveryClass::BudgetTruncated);
        assert_eq!(state.continuations(), 1);
    }
}
