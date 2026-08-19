//! rusty-whale-core: agent loop, state machine, tool dispatch.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Agent state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Thinking,
    Acting,
    Verifying,
    Done,
    Waiting,
}

/// A single tool call dispatched during the Acting phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// The result of running one iteration of the agent loop.
#[derive(Debug, Clone)]
pub struct LoopResult {
    pub final_state: AgentState,
    pub iterations: u32,
    pub tool_calls: Vec<ToolCall>,
}

/// Run the agent loop until it reaches Done or Waiting.
///
/// `step` is invoked once per iteration and returns the next state plus any
/// tool calls produced. This keeps the core loop agnostic to the concrete LLM
/// provider and verify strategy.
pub async fn run_loop<F, Fut>(mut step: F) -> Result<LoopResult>
where
    F: FnMut(AgentState) -> Fut,
    Fut: std::future::Future<Output = Result<(AgentState, Vec<ToolCall>)>>,
{
    let mut state = AgentState::Idle;
    let mut iterations = 0u32;
    let mut tool_calls = Vec::new();

    loop {
        iterations += 1;
        let (next, mut calls) = step(state).await?;
        tool_calls.append(&mut calls);
        state = next;
        match state {
            AgentState::Done | AgentState::Waiting => break,
            _ if iterations > 1000 => break,
            _ => continue,
        }
    }

    Ok(LoopResult {
        final_state: state,
        iterations,
        tool_calls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn loop_terminates_on_done() {
        let step = |s: AgentState| async move {
            assert_eq!(s, AgentState::Idle);
            Ok::<_, anyhow::Error>((AgentState::Done, vec![]))
        };
        let r = run_loop(step).await.unwrap();
        assert_eq!(r.final_state, AgentState::Done);
        assert_eq!(r.iterations, 1);
    }
}
