# ATS Scoring Microservice — Agent TODO

> **Stack:** Rust · PostgreSQL (read source data + write scores) · Qdrant (read vectors) · RabbitMQ (events)
> **Models:** BGEM3 (Bi-Encoder, via `fastembed`) · `BAAI/bge-reranker-v2-m3` (Cross-Encoder / Reranker, via `fastembed`)
> **Algorithm:** TF-IDF keyword score → Bi-Encoder semantic score → Cross-Encoder rerank score → PDF parseability score
> **Trigger:** Manual only (user-initiated rescore)

---

## Phase 0 — Dependencies & Directory Layout

- [ ] Add dependencies to `Cargo.toml`:
  ```toml
  tokio = { version = "1", features = ["full"] }
  fastembed = "3"               # BGEM3 embeddings + bge-reranker-v2-m3 reranker, no sidecar
  sqlx = { version = "0.7", features = ["postgres", "uuid", "runtime-tokio-rustls", "json"] }
  qdrant-client = "1"
  lapin = "2"
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  uuid = { version = "1", features = ["v4", "serde"] }
  anyhow = "1"
  tracing = "1"
  tracing-subscriber = { version = "0.3", features = ["env-filter"] }
  dotenvy = "0.15"
  clap = { version = "4", features = ["derive", "env"] }
  tokio-retry = "0.3"
  axum = "0.7"
  rust-stemmers = "1"           # Porter stemmer for TF-IDF keyword normalisation
  stop-words = "0.8"            # English stop-word filter
  ```
- [ ] Create directory layout:
  ```
  src/
    main.rs
    config.rs
    cli.rs
    db/
      mod.rs
      queries.rs          # fetch resume/job raw text + parseability markers
      writer.rs           # UPSERT ats_scores
    scoring/
      mod.rs
      tfidf.rs            # Stage 1 — keyword_match_rate
      biencoder.rs        # Stage 2a — Bi-Encoder per-section similarities
      reranker.rs         # Stage 2b — bge-reranker-v2-m3 for skill_alignment + experience_relevance
      parseability.rs     # Stage 3 — format_and_parseability
      pipeline.rs         # orchestrates all stages → ScoreResult
    queue/
      mod.rs
      consumer.rs
      publisher.rs
    vector_store/
      mod.rs
      qdrant.rs           # fetch vectors by source_id + section_type
    handlers/
      mod.rs
      manual.rs           # manual rescore trigger only
    models.rs
  ```

---

## Phase 1 — Configuration

- [ ] Create `.env.example`:
  ```env
  POSTGRES_URL=postgres://user:pass@localhost:5432/app

  QDRANT_URL=http://localhost:6334
  QDRANT_COLLECTION=document_vectors

  RABBITMQ_URL=amqp://guest:guest@localhost:5672
  RABBITMQ_CONSUME_QUEUE=ats.score.requests
  RABBITMQ_PUBLISH_EXCHANGE=ats.events
  RABBITMQ_PUBLISH_ROUTING_KEY=ats.score.ready

  # How many TF-IDF top keyword matches to pass into the Bi-Encoder stage
  TFIDF_TOP_K=20
  # How many Bi-Encoder section pairs to pass into the reranker
  RERANKER_TOP_K=5

  EMBEDDING_BATCH_SIZE=32
  ```
- [ ] Implement `config.rs` — `Config` struct mapping 1:1 to env vars above; load via `dotenvy` + `std::env::var`
- [ ] Validate all required fields at startup; fail fast naming the missing variable

---

## Phase 2 — CLI

- [ ] Implement `cli.rs`:
  ```rust
  #[derive(Parser)]
  #[command(name = "ats-scoring-service")]
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
          #[arg(long)] resume_id: Uuid,
          #[arg(long)] job_id: Uuid,
      },
  }
  ```
- [ ] Wire dispatch in `main.rs`:
  - `Command::Serve` → full startup
  - `Command::DownloadModels` → initialise both `TextEmbedding` and `TextRerank` (triggers ONNX downloads), print cache paths, exit 0
  - `Command::Score { .. }` → load config, connect to deps, run full pipeline once, pretty-print `ScoreResult` as JSON, exit 0

---

## Phase 3 — Domain Models

- [ ] Define in `models.rs`:
  ```rust
  // ── Inbound event ────────────────────────────────────────────────

  pub struct ManualScoreRequest {
      pub resume_id: Uuid,
      pub job_id:    Uuid,
      pub user_id:   Uuid,
  }

  // ── Internal pipeline types ──────────────────────────────────────

  pub struct ScoringInput {
      pub resume_id:       Uuid,
      pub job_id:          Uuid,
      pub user_id:         Uuid,
      pub resume_sections: DocumentSections,
      pub job_sections:    DocumentSections,
      pub resume_vectors:  SectionVectors,
      pub job_vectors:     SectionVectors,
      pub parse_markers:   ParseMarkers,
  }

  pub struct DocumentSections {
      pub full_text:                   String,
      pub skills:                      Option<String>,
      pub experience_or_requirements:  Option<String>,
      pub education:                   Option<String>,   // resume only
  }

  pub struct SectionVectors {
      pub full_vec:          Vec<f32>,
      pub skills_vec:        Option<Vec<f32>>,
      pub experience_vec:    Option<Vec<f32>>,
      pub education_vec:     Option<Vec<f32>>,    // resume only
      pub requirements_vec:  Option<Vec<f32>>,    // job only
  }

  pub struct ParseMarkers {
      pub has_complex_layout:      bool,
      pub has_graphics:            bool,
      pub has_headers_footers:     bool,
      pub has_non_standard_fonts:  bool,
  }

  // ── Pipeline output (maps 1:1 to the breakdown JSON schema) ─────

  pub struct ScoreResult {
      pub resume_id:  Uuid,
      pub job_id:     Uuid,
      pub user_id:    Uuid,
      pub total_score: u8,        // sum of all earned points, 0–100
      pub breakdown:  Breakdown,
  }

  pub struct Breakdown {
      pub keyword_match_rate:   KeywordMatchRate,   // max 40
      pub skill_alignment:      SkillAlignment,     // max 25
      pub experience_relevance: ExperienceRelevance,// max 15
      pub format_and_parseability: FormatParseability, // max 20
  }

  pub struct KeywordMatchRate {
      pub earned:  u8,
      pub max:     u8,   // always 40
      pub details: KeywordDetails,
  }
  pub struct KeywordDetails {
      pub matched:  Vec<String>,
      pub partial:  Vec<String>,
      pub missing:  Vec<String>,
  }

  pub struct SkillAlignment {
      pub earned:  u8,
      pub max:     u8,   // always 25
      pub details: Vec<SkillAlignmentItem>,
  }
  pub struct SkillAlignmentItem {
      pub required_skill:        String,
      pub closest_match:         String,
      pub vector_similarity_score: f32,
      pub flag:                  AlignmentFlag,
  }

  pub struct ExperienceRelevance {
      pub earned:  u8,
      pub max:     u8,   // always 15
      pub details: Vec<ExperienceRelevanceItem>,
  }
  pub struct ExperienceRelevanceItem {
      pub job_responsibility:    String,
      pub closest_match:         String,
      pub vector_similarity_score: f32,
      pub flag:                  AlignmentFlag,
  }

  pub enum AlignmentFlag {
      Good,
      NeedsReframe,
      MissingMetrics,
      Weak,
  }

  pub struct FormatParseability {
      pub earned:        u8,
      pub max:           u8,   // always 20
      pub parsing_flags: ParseMarkers,  // echo the markers back into the breakdown
  }

  // ── Outbound event ───────────────────────────────────────────────

  pub struct AtsScoreReadyEvent {
      pub ats_score_id:        Uuid,
      pub resume_id:           Uuid,
      pub job_id:              Uuid,
      pub user_id:             Uuid,
      pub total_score:         u8,
  }
  ```
- [ ] Implement `Breakdown::to_json(&self) -> serde_json::Value` — serialises to the exact schema shown in the spec:
  ```json
  {
    "score": 68,
    "breakdown": {
      "keyword_match_rate":    { "earned": ..., "max": 40, "details": { ... } },
      "skill_alignment":       { "earned": ..., "max": 25, "details": [ ... ] },
      "experience_relevance":  { "earned": ..., "max": 15, "details": [ ... ] },
      "format_and_parseability": { "earned": ..., "max": 20, "parsing_flags": { ... } }
    }
  }
  ```

---

## Phase 4 — PostgreSQL

- [ ] Implement `db/queries.rs`:
  - [ ] `fetch_resume_sections(pool, resume_id) -> anyhow::Result<DocumentSections>`
    - Pull raw text sections from the resume document (same fields the embedding service embeds)
  - [ ] `fetch_job_sections(pool, job_id) -> anyhow::Result<DocumentSections>`
  - [ ] `fetch_parse_markers(pool, resume_id) -> anyhow::Result<ParseMarkers>`
    - Read `has_complex_layout`, `has_graphics`, `has_headers_footers`, `has_non_standard_fonts` from the parsed resume table

- [ ] Implement `db/writer.rs`:
  - [ ] `upsert_ats_score(pool, result: &ScoreResult) -> anyhow::Result<Uuid>`
    - `INSERT INTO ats_scores (id, resume_id, job_analysis_id, score, breakdown) VALUES (...)`
    - `ON CONFLICT (resume_id, job_analysis_id) DO UPDATE SET score = EXCLUDED.score, breakdown = EXCLUDED.breakdown`
    - `breakdown` column stores the full JSON from `Breakdown::to_json()`
    - Return the `ats_score_id`

- [ ] Connection pool init in `db/mod.rs` with retry (exponential backoff, 5 attempts)

---

## Phase 5 — Qdrant Vector Fetching

- [ ] Implement `vector_store/qdrant.rs`:
  - [ ] `fetch_section_vectors(client, source_id: Uuid) -> anyhow::Result<Option<SectionVectors>>`
    - Filter Qdrant points by payload `source_id`
    - Map each point's `section_type` payload field to the correct `SectionVectors` field
    - Return `None` if zero points found (signals vectors not yet ready)
  - [ ] Return type is `Option` so the handler can fail fast with a clear error when vectors are missing for a manual trigger (unexpected state)

---

## Phase 6 — Scoring Pipeline

### Stage 1 — TF-IDF (`scoring/tfidf.rs`)

- [ ] Implement `fn tfidf_score(resume: &DocumentSections, job: &DocumentSections) -> (u8, KeywordMatchRate)`:
  - Tokenise both texts: lowercase → strip punctuation → remove stop words (`stop-words` crate) → stem (`rust-stemmers` Porter stemmer)
  - Build TF-IDF vectors for both corpora
  - For each job keyword: classify as `matched` (exact stem match in resume), `partial` (stem overlap > 0.5), or `missing`
  - Score formula:
    - `matched` keywords: full weight
    - `partial` keywords: half weight
    - Scale weighted sum to 0–40 (this category's max)
  - Populate `KeywordMatchRate { earned, max: 40, details: KeywordDetails { matched, partial, missing } }`
  - Pass the top `TFIDF_TOP_K` matched + partial keywords forward to the Bi-Encoder stage

### Stage 2a — Bi-Encoder (`scoring/biencoder.rs`)

- [ ] Implement `fn biencoder_pairs(resume_vectors: &SectionVectors, job_vectors: &SectionVectors) -> Vec<SectionPair>`:
  - Compute cosine similarity for each present section pair:
    - `skills_vec` vs `skills_vec` — produces candidates for `skill_alignment`
    - `experience_vec` vs `requirements_vec` — produces candidates for `experience_relevance`
  - A `SectionPair` carries: `resume_text_chunk`, `job_text_chunk`, `section_kind` (Skill | Experience), `cosine_similarity: f32`
  - Sort by `cosine_similarity` descending; keep top `RERANKER_TOP_K` per section kind
  - These pairs are the reranker's input — do **not** compute a score here; scoring happens after reranking

### Stage 2b — Reranker (`scoring/reranker.rs`)

- [ ] Initialise `fastembed::TextRerank` with `RerankerModel::BGERerankerV2M3` (load once, store in `AppContext`)
- [ ] Implement `fn rerank(reranker: &TextRerank, pairs: Vec<SectionPair>) -> anyhow::Result<Vec<RankedPair>>`:
  - Call `reranker.rerank(query, documents, ...)` where:
    - For skill pairs: `query` = job skill text, `document` = resume skill text
    - For experience pairs: `query` = job responsibility text, `document` = resume experience text
  - `fastembed` returns scores as logits; apply sigmoid to get [0.0, 1.0]
  - Map sigmoid score → `AlignmentFlag`:
    - ≥ 0.75 → `Good`
    - 0.55–0.74 → `NeedsReframe`
    - 0.35–0.54 → `MissingMetrics`
    - < 0.35 → `Weak`
  - Return `Vec<RankedPair>` carrying: `required_text`, `closest_match_text`, `similarity_score: f32`, `flag`

- [ ] Implement `fn skill_alignment_score(ranked: &[RankedPair]) -> SkillAlignment`:
  - Average sigmoid score across all skill pairs → scale to 0–25
  - Populate `SkillAlignment { earned, max: 25, details: [SkillAlignmentItem { ... }] }`

- [ ] Implement `fn experience_relevance_score(ranked: &[RankedPair]) -> ExperienceRelevance`:
  - Same pattern, scale to 0–15
  - Populate `ExperienceRelevance { earned, max: 15, details: [ExperienceRelevanceItem { ... }] }`

### Stage 3 — Parseability (`scoring/parseability.rs`)

- [ ] Implement `fn parseability_score(markers: &ParseMarkers) -> FormatParseability`:
  - Start from 20, apply deductions:
    - `has_complex_layout` → −5
    - `has_graphics` → −5
    - `has_headers_footers` → −3
    - `has_non_standard_fonts` → −7
  - Clamp to 0–20
  - Populate `FormatParseability { earned, max: 20, parsing_flags: markers.clone() }`

### Pipeline Orchestrator (`scoring/pipeline.rs`)

- [ ] Implement `async fn run_pipeline(ctx: &AppContext, input: ScoringInput) -> anyhow::Result<ScoreResult>`:
  1. `tfidf_score(&input.resume_sections, &input.job_sections)` → `(_, keyword_match_rate)`
  2. `biencoder_pairs(&input.resume_vectors, &input.job_vectors)` → `pairs`
  3. `rerank(&ctx.reranker, pairs)` → `ranked`
  4. `skill_alignment_score(&ranked.skill_pairs)` → `skill_alignment`
  5. `experience_relevance_score(&ranked.experience_pairs)` → `experience_relevance`
  6. `parseability_score(&input.parse_markers)` → `format_and_parseability`
  7. `total_score = keyword_match_rate.earned + skill_alignment.earned + experience_relevance.earned + format_and_parseability.earned`
  8. Assemble and return `ScoreResult { total_score, breakdown: Breakdown { keyword_match_rate, skill_alignment, experience_relevance, format_and_parseability } }`

---

## Phase 7 — Event Handler (Manual Only)

- [ ] Implement `handlers/manual.rs` — `async fn handle_manual(ctx: &AppContext, req: ManualScoreRequest) -> anyhow::Result<()>`:
  1. `db::queries::fetch_resume_sections(pool, req.resume_id)`
  2. `db::queries::fetch_job_sections(pool, req.job_id)`
  3. `vector_store::qdrant::fetch_section_vectors(client, req.resume_id)` — if `None`, return `Err(ScoringError::VectorsNotReady("resume"))` — nack without requeue at consumer level
  4. `vector_store::qdrant::fetch_section_vectors(client, req.job_id)` — same check
  5. `db::queries::fetch_parse_markers(pool, req.resume_id)`
  6. Assemble `ScoringInput`, call `scoring::pipeline::run_pipeline`
  7. `db::writer::upsert_ats_score(pool, &result)` → `ats_score_id`
  8. `queue::publisher::publish_score_ready(channel, AtsScoreReadyEvent { ats_score_id, ..., total_score })`
  9. Return `Ok(())`

---

## Phase 8 — RabbitMQ Consumer & Publisher

- [ ] Implement `queue/consumer.rs`:
  - [ ] Declare durable queue bound to routing key `ats.score.manual`
  - [ ] Deserialise delivery as `ManualScoreRequest`
  - [ ] On deserialisation failure: `basic_nack` `requeue=false`, log error
  - [ ] On `VectorsNotReady`: `basic_nack` `requeue=false`, log warning (vectors should exist for a manual trigger)
  - [ ] On transient error (DB / Qdrant / reranker): `basic_nack` `requeue=true`
  - [ ] On success: `basic_ack`
  - [ ] Dead-letter queue binding for repeated failures

- [ ] Implement `queue/publisher.rs`:
  - [ ] `fn publish_score_ready(channel, event: AtsScoreReadyEvent) -> anyhow::Result<()>`
  - [ ] Routing key: `ats.score.ready`, exchange: `RABBITMQ_PUBLISH_EXCHANGE`
  - [ ] `delivery_mode: 2` (persistent), `content_type: application/json`

---

## Phase 9 — Main Entry Point

- [ ] Implement `main.rs`:
  - [ ] `dotenvy::dotenv().ok()` first
  - [ ] Initialise `tracing_subscriber`
  - [ ] `Cli::parse()` → dispatch
  - [ ] For `Command::Serve`:
    - Load config
    - Connect PostgreSQL (retry), Qdrant (retry), RabbitMQ (retry)
    - Initialise `fastembed::TextEmbedding` with `EmbeddingModel::BGEM3`
    - Initialise `fastembed::TextRerank` with `RerankerModel::BGERerankerV2M3`
    - Build `AppContext`: `PgPool`, `QdrantClient`, `lapin::Channel`, `Arc<TextEmbedding>`, `Arc<TextRerank>`, `Config`
    - Spawn consumer loop as `tokio::task`
    - Spawn `/healthz` axum server as `tokio::task`
    - Await `SIGTERM` / `SIGINT`

---

## Phase 10 — Error Handling & Observability

- [ ] Define `ScoringError` enum: `VectorsNotReady(&'static str)`, `DbError(anyhow::Error)`, `QdrantError(anyhow::Error)`, `RerankerError(anyhow::Error)`, `PipelineError(anyhow::Error)`
- [ ] Structured log on every scored pair: `resume_id`, `job_id`, `total_score`, `tfidf_earned`, `skill_earned`, `experience_earned`, `parseability_earned`, `duration_ms`
- [ ] `tracing::instrument` on `run_pipeline` and each stage function
- [ ] `/healthz` checks PostgreSQL ping + Qdrant collections list; returns `200` if all pass, `503` with JSON body listing failing deps otherwise

---

## Phase 11 — Containerisation

- [ ] Write `Dockerfile`:
  ```dockerfile
  FROM rust:1.77 as builder
  WORKDIR /app
  COPY . .
  RUN cargo build --release

  FROM debian:bookworm-slim
  RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
  COPY --from=builder /app/target/release/ats-scoring-service /usr/local/bin/
  WORKDIR /app
  CMD ["ats-scoring-service", "serve"]
  ```
- [ ] Pre-download both ONNX models at image build time:
  ```dockerfile
  RUN ats-scoring-service download-models
  ```
- [ ] Add `ats-scoring-service` to `docker-compose.yml`
- [ ] Wire `.env.example` via `env_file:` so local startup needs zero manual config
