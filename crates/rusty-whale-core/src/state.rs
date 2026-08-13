//! rusty-whale-core — state machine for the agent loop
//!
//! States: Idle → Thinking → Acting → Verifying → Done|Waiting

use crate::lib::{ToolCall, AgentResult};
use std::fmt;

/// Agent state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// No task active, waiting for user input.
    Idle,
    /// LLM call in flight.
    Thinking,
    /// Tool dispatch in flight.
    Acting,
    /// Auto-verify (cargo/npm/pip/go) in flight.
    Verifying,
    /// Verify passed, task complete.
    Done,
    /// Verify failed, waiting for user decision or retry.
    Waiting,
}

impl State {
    /// Whether this state is terminal (no further transitions).
    pub fn is_terminal(self) -> bool {
        matches!(self, State::Done)
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Idle => write!(f, "idle"),
            State::Thinking => write!(f, "thinking"),
            State::Acting => write!(f, "acting"),
            State::Verifying => write!(f, "verifying"),
            State::Done => write!(f, "done"),
            State::Waiting => write!(f, "waiting"),
        }
    }
}

/// A single transition in the state machine.
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: State,
    pub to: State,
    pub trigger: Trigger,
}

/// What caused a state transition.
#[derive(Debug, Clone)]
pub enum Trigger {
    /// User submitted a task.
    UserInput(String),
    /// LLM returned a response (with or without tool calls).
    LlmResponse { has_tool_calls: bool },
    /// Tool finished executing.
    ToolComplete(ToolCall),
    /// Verify command finished.
    VerifyComplete { passed: bool, output: String },
    /// Max turns exceeded, abort.
    MaxTurnsExceeded,
    /// User cancelled the task.
    UserCancel,
}

/// The agent state machine.
pub struct AgentStateMachine {
    state: State,
    turns: usize,
    max_turns: usize,
    history: Vec<Transition>,
}

impl AgentStateMachine {
    /// Create a new state machine with a max turn budget.
    pub fn new(max_turns: usize) -> Self {
        Self {
            state: State::Idle,
            turns: 0,
            max_turns,
            history: Vec::new(),
        }
    }

    /// Current state.
    pub fn state(&self) -> State {
        self.state
    }

    /// Number of turns completed so far.
    pub fn turns(&self) -> usize {
        self.turns
    }

    /// Whether the agent should stop (terminal or budget exhausted).
    pub fn should_stop(&self) -> bool {
        self.state.is_terminal() || self.turns >= self.max_turns
    }

    /// Apply a trigger, advancing the state machine.
    pub fn transition(&mut self, trigger: Trigger) -> Result<State, StateError> {
        let from = self.state;
        let to = match (&self.state, &trigger) {
            (State::Idle, Trigger::UserInput(_)) => State::Thinking,
            (State::Thinking, Trigger::LlmResponse { has_tool_calls: true }) => State::Acting,
            (State::Thinking, Trigger::LlmResponse { has_tool_calls: false }) => State::Done,
            (State::Acting, Trigger::ToolComplete(_)) => State::Verifying,
            (State::Verifying, Trigger::VerifyComplete { passed: true, .. }) => State::Done,
            (State::Verifying, Trigger::VerifyComplete { passed: false, .. }) => State::Waiting,
            (State::Waiting, Trigger::UserInput(_)) => State::Thinking,
            (_, Trigger::UserCancel) => State::Done,
            (_, Trigger::MaxTurnsExceeded) => State::Done,
            (from, trigger) => {
                return Err(StateError::InvalidTransition {
                    from,
                    trigger: format!("{:?}", trigger),
                });
            }
        };

        if matches!(trigger, Trigger::LlmResponse { .. }) {
            self.turns += 1;
        }

        self.state = to;
        self.history.push(Transition { from, to, trigger });
        Ok(to)
    }

    /// Transition history (for debugging / trajectory replay).
    pub fn history(&self) -> &[Transition] {
        &self.history
    }

    /// Summarize the run as an AgentResult.
    pub fn finalize(self, summary: String, tool_calls: Vec<ToolCall>, total_tokens: usize) -> AgentResult {
        AgentResult {
            summary,
            turns: self.turns,
            total_tokens,
            tool_calls,
            verified: matches!(self.state, State::Done),
        }
    }
}

/// Invalid state transition error.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("invalid transition from {from:?} with trigger {trigger}")]
    InvalidTransition { from: State, trigger: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_to_thinking_on_user_input() {
        let mut sm = AgentStateMachine::new(10);
        assert_eq!(sm.state(), State::Idle);
        sm.transition(Trigger::UserInput("refactor auth".into())).unwrap();
        assert_eq!(sm.state(), State::Thinking);
    }

    #[test]
    fn thinking_with_tools_goes_to_acting() {
        let mut sm = AgentStateMachine::new(10);
        sm.transition(Trigger::UserInput("task".into())).unwrap();
        sm.transition(Trigger::LlmResponse { has_tool_calls: true }).unwrap();
        assert_eq!(sm.state(), State::Acting);
    }

    #[test]
    fn thinking_without_tools_goes_to_done() {
        let mut sm = AgentStateMachine::new(10);
        sm.transition(Trigger::UserInput("task".into())).unwrap();
        sm.transition(Trigger::LlmResponse { has_tool_calls: false }).unwrap();
        assert_eq!(sm.state(), State::Done);
    }

    #[test]
    fn full_loop_think_act_verify_done() {
        let mut sm = AgentStateMachine::new(10);
        sm.transition(Trigger::UserInput("task".into())).unwrap();
        sm.transition(Trigger::LlmResponse { has_tool_calls: true }).unwrap();
        sm.transition(Trigger::ToolComplete(ToolCall {
            id: "tc1".into(),
            name: "edit_file".into(),
            arguments: Default::default(),
        })).unwrap();
        sm.transition(Trigger::VerifyComplete {
            passed: true,
            output: "cargo build OK".into(),
        }).unwrap();
        assert_eq!(sm.state(), State::Done);
        assert_eq!(sm.turns(), 1);
    }

    #[test]
    fn verify_fail_goes_to_waiting() {
        let mut sm = AgentStateMachine::new(10);
        sm.transition(Trigger::UserInput("task".into())).unwrap();
        sm.transition(Trigger::LlmResponse { has_tool_calls: true }).unwrap();
        sm.transition(Trigger::ToolComplete(ToolCall {
            id: "tc1".into(),
            name: "edit_file".into(),
            arguments: Default::default(),
        })).unwrap();
        sm.transition(Trigger::VerifyComplete {
            passed: false,
            output: "error[E0599]".into(),
        }).unwrap();
        assert_eq!(sm.state(), State::Waiting);
    }
}
