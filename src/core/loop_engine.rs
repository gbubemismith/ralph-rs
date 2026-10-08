use std::{sync::Arc, time::Instant};

use tokio::time::sleep;

use crate::{
    config::Config,
    core::run_agentic_loop,
    events::{Event, EventSender, RunOutcome},
    provider::Provider,
    tools::Registry,
};

pub struct LoopEngine {
    provider: Arc<dyn Provider>,
    registry: Arc<Registry>,
    config: Config,
}

impl LoopEngine {
    pub fn new(provider: Arc<dyn Provider>, registry: Arc<Registry>, config: Config) -> Self {
        Self {
            provider,
            registry,
            config,
        }
    }

    pub async fn run(self, tx: EventSender) -> RunOutcome {
        let started = Instant::now();
        let timeout = self.config.timeout;

        let mut iteration = 0;
        let mut last_outcome = RunOutcome::Complete;

        for i in 1..=self.config.max_iterations {
            iteration = i;
            let _ = tx.send(Event::IterationStart {
                iteration: i,
                max: self.config.max_iterations,
            });

            let elapsed = started.elapsed();

            if elapsed >= timeout {
                last_outcome = RunOutcome::TimedOut;
                break;
            }

            let remaining = timeout - elapsed;

            let inner = run_agentic_loop(
                self.provider.clone(),
                self.registry.clone(),
                self.config.system_prompt.clone(),
                self.config.model.clone(),
                self.config.prompt.clone(),
                tx.clone(),
            );

            tokio::select! {
                biased; // check signal/timeout first, then iteration

                _ = tokio::signal::ctrl_c() => {
                    let _ = tx.send(Event::Notice {
                        message: "received Ctrl+C, shutting down…".to_string(),
                    });
                    last_outcome = RunOutcome::Cancelled;
                    break;
                }
                _ = sleep(remaining) => {
                    last_outcome = RunOutcome::TimedOut;
                    break;
                }
                result = inner => {
                    match result {
                        Ok(()) => {
                            // Iteration finished normally.
                            // The model decided it was done within this iteration.
                            // We continue to the next outer iteration unless that's the last one.
                            if i == self.config.max_iterations {
                                last_outcome = RunOutcome::MaxIterationsReached;
                            }
                            // else: keep looping
                        }
                        Err(e) => {
                            let _ = tx.send(Event::Error {
                                message: format!("iteration {i} failed: {e}"),
                            });
                            last_outcome = RunOutcome::Failed;
                            break;
                        }
                    }
                }
            }
        }

        let _ = tx.send(Event::RunComplete {
            outcome: last_outcome,
            iterations: iteration,
        });

        last_outcome
    }
}
