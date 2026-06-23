pub mod config;
pub mod python;
pub mod throttling;

use crate::cli::args::ExplainTopic;
use pi_doctor_core::ProbeContext;

pub fn render(topic: ExplainTopic, ctx: &ProbeContext) -> String {
    match topic {
        ExplainTopic::Throttling => throttling::render(ctx),
        ExplainTopic::Config => config::render(ctx),
        ExplainTopic::Python => python::render(ctx),
    }
}
