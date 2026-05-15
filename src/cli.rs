use clap::{Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "ats-scoring-service", about = "ATS Scoring microservice")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the service (consumer loop + healthz endpoint)
    Serve,
    /// Download and cache BGEM3 + bge-reranker-v2-m3 ONNX models without starting the service
    DownloadModels,
    /// Score a specific resume/job pair on demand, print JSON result to stdout, exit 0
    Score {
        #[arg(long)]
        resume_id: Uuid,
        #[arg(long)]
        job_id: Uuid,
    },
}
