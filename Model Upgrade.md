
I have two research resposes.

The 2026 Architectural Blueprint for Autonomous Software Engineering: Optimizing the Codex-RS and SpecKit Pipeline on NVIDIA RTX 5090 Infrastructure1. Introduction: The Convergence of Safety, Specification, and SiliconThe trajectory of automated software engineering has undergone a radical transformation in the first half of 2026. The industry has moved decisively past the experimental "script-kiddie" phase of 2024—characterized by fragile Python wrappers and non-deterministic "vibe coding"—into an era of compiled, type-safe, and architecturally rigorous agentic workflows. This shift is driven by the simultaneous maturation of three critical pillars: the Codex-RS Rust-native runtime, the SpecKit protocol for structured requirements engineering, and the consumer availability of datacenter-class inference capabilities via the NVIDIA RTX 5090.This report provides a comprehensive architectural analysis and implementation guide for deploying a sovereign, high-performance coding agent pipeline. It is specifically calibrated for the constraint of a single workstation equipped with an RTX 5090 (32GB VRAM). Unlike cloud-scale deployments where compute is elastic, the "Single-Card Architect" paradigm requires a ruthlessly efficient allocation of resources. Every megabyte of VRAM must be adjudicated between model parameters (intelligence), Key-Value (KV) cache (context), and activation overhead (throughput).The analysis that follows draws upon extensive performance benchmarks, codebase audits of the openai/codex repository, and comparative evaluations of 2026-era foundation models including Llama 4, Mistral Small 3, and Qwen 3. It establishes that the optimal configuration for this hardware constraint is not merely a selection of the largest possible model, but a precision-engineered stack: the Qwen 3 32B logic engine, served via the SGLang inference server for structured generation, grounded by LanceDB for zero-copy vector retrieval, and orchestrated by the codex-rs secure runtime.2. The Computational Substrate: NVIDIA RTX 5090 and Blackwell ArchitectureTo architect software for a specific hardware constraint, one must first understand the physics of the constraint itself. The NVIDIA GeForce RTX 5090, powered by the Blackwell GB202 architecture, represents a massive leap in local inference potential, yet it imposes a strict 32GB memory boundary that dictates every subsequent software decision.2.1 The Blackwell GB202 ArchitectureThe GB202 GPU is built on the TSMC 4N process and introduces fifth-generation Tensor Cores explicitly designed for low-precision AI workloads.1 The architecture moves beyond the Ada Lovelace generation by implementing native support for FP8 (8-bit floating point) and FP4 precision formats.2This capability is critical for the "Single-Card Architect." In previous generations (e.g., RTX 4090), running large models often required 4-bit integer quantization (INT4) to fit into VRAM. While efficient, INT4 quantization can introduce perplexity degradation, subtly impairing the model's ability to handle complex logic chains found in software architecture tasks.3 The Blackwell architecture's native FP8 support allows for a "Goldilocks" precision—retaining near-FP16 reasoning quality while halving the memory footprint and doubling the throughput via the new Tensor Engines.The RTX 5090 features 21,760 CUDA cores and a boost clock of 2407 MHz, delivering theoretical compute performance that dwarfs the previous flagship.1 However, for Large Language Model (LLM) inference, compute is rarely the bottleneck; memory bandwidth is. The card utilizes GDDR7 memory running at 28 Gbps, providing a memory bandwidth of 1,792 GB/s.4 This bandwidth is the primary determinant of token generation speed (tokens/second) during the decoding phase.2.2 The 32GB VRAM Envelope: A Hard ConstraintDespite the bandwidth improvements, the 32GB VRAM capacity remains the defining constraint for this architecture.1 For a developer workstation, this memory must be partitioned carefully. Unlike a dedicated server, a local workstation incurs system overhead. The operating system (Windows or Linux with a GUI) and display drivers typically reserve 1.5GB to 2.5GB of VRAM. This leaves approximately 29.5GB to 30.5GB of effective usable memory for the AI stack.This "effective capacity" must accommodate three distinct components:Model Weights: The static footprint of the neural network. A 32-billion parameter model in FP8 precision requires approximately 32GB, which immediately exceeds the available headroom. This necessitates either smaller models or aggressive quantization.KV Cache (Context Window): The dynamic memory required to store the Key and Value matrices for the attention mechanism. As the conversation length (context) grows, so does the KV cache. The SpecKit protocol, which relies on long-context "Constitutions" and iterative refinement, places heavy demands here. A 128k context window can easily consume 10GB+ of VRAM depending on the attention implementation (e.g., PagedAttention).Activation Overhead: Temporary memory used for tensor operations during the forward pass. While SGLang and vLLM optimize this, it is non-zero and scales with batch size.The implications of this constraint are profound. It categorically excludes the use of "frontier" class models like the full Llama 4 Maverick (402B parameters) or even its mid-sized distillations without severe degradation.6 It forces the architect to optimize for density—finding the model that delivers the highest reasoning IQ per gigabyte of VRAM.2.3 Power and Thermal DynamicsThe RTX 5090 is rated for a 575W TDP 1, significantly higher than the 450W of the RTX 4090. This necessitates a robust power supply unit (1200W recommended) and adequate case cooling.4 For continuous inference workloads—common in agentic "thinking" loops where the model may reason for minutes—thermal throttling can become a factor. The analysis suggests that while the card is efficient per token (consuming ~28% more power for 72% more performance than the 4090) 4, sustained heavy loads during the SpecKit /plan phase can saturate the thermal envelope if airflow is restricted.3. The Runtime Environment: Codex-RS and the Rust AdvantageThe infrastructure hardware hosts the software runtime. In 2026, the industry standard for autonomous coding agents has shifted from interpreted languages (Python/JavaScript) to Rust. This transition is embodied by codex-rs, the core component of the openai/codex repository.83.1 Why Rust? Concurrency and SafetyThe codex-rs codebase is approximately 97.4% Rust.9 This choice is not aesthetic but functional. Coding agents are inherently asynchronous and concurrent systems. They must manage multiple streams of information: the LLM's token stream, the shell's standard output/error streams, file system I/O, and the user's input via the TUI (Text User Interface).10Python-based agents often struggle with the "Global Interpreter Lock" (GIL) and runtime exceptions that leave the agent in an undefined state—potentially dangerous when the agent has file deletion privileges. Rust’s ownership model ensures memory safety without a garbage collector, preventing the class of bugs where an agent might hang or crash while holding a lock on a critical file.8 The codex-rs implementation uses the tokio runtime for asynchronous execution, allowing it to handle the "thinking" process (network I/O to the model) and the "doing" process (file system operations) in parallel without race conditions.3.2 The Codex-RS Module StructureThe codex-rs directory is structured into several key crates that define its capabilities:codex-rs/core: This contains the fundamental logic for the agent's lifecycle, including the turn loop (Read-Eval-Print Loop equivalent for agents) and the prompt management system.8 It is responsible for constructing the context window sent to the LLM.codex-rs/tui: A high-performance terminal user interface built with ratatui (a Rust library).11 This replaces the "janky" React Ink interfaces of previous generations (like Claude Code's early versions), offering 60fps rendering and low-latency input handling even when the system is under heavy load.codex-rs/execpolicy: This is the security kernel of the agent. It enforces the rules of engagement, deciding which commands the agent is permitted to run. This module parses commands before execution, checking them against a whitelist or a heuristic analysis engine to prevent accidental destruction (e.g., rm -rf /).8codex-rs/apply-patch: A specialized tool for robustly applying code modifications. Unlike simple string replacement (which often fails due to whitespace mismatches) or standard git apply (which requires rigid context), this tool uses a "fuzzy" context-aware algorithm designed specifically to handle the slightly hallucinated context often produced by LLMs.83.3 The "Architect" and "Builder" Rolescodex-rs introduces the concept of distinct agent roles, configured via the AGENTS.md file.14 This file allows the user to define different personas with specific system prompts and tool access.The Architect: Configured with a high-reasoning temperature and access to the /speckit tools. Its role is to ask questions, clarify requirements, and produce a plan. It consumes more context (reading existing files) but generates less code.The Builder: Configured for precision and lower temperature. Its primary tool is apply_patch. It executes the plan generated by the Architect.This separation of concerns allows for efficient resource usage. The Architect might use the full FP8 precision capabilities of the model to reason through complex dependencies, while the Builder focuses on syntactic correctness.4. The Protocol: SpecKit and the Maieutic LoopInfrastructure without a protocol is merely potential. SpecKit provides the rigorous methodology required to harness the raw intelligence of the model.16 It moves the interaction model from "Chat" to "Specification-Driven Development."4.1 The "Vibe Coding" ProblemPrior to protocols like SpecKit, developers engaged in "vibe coding"—iteratively prompting an LLM with vague requests like "fix the login bug" and hoping for a correct solution. This approach is computationally inefficient and prone to regression loops. It relies on the model "guessing" the user's intent, leading to wasted tokens and developer frustration.164.2 The Maieutic Clarification PhaseSpecKit solves this via the Maieutic (Socratic) phase, implemented as the /speckit.clarify command.18 Before a single line of code is written, the agent is forced to interview the developer.Ingest Constitution: The model reads SPECS/CONSTITUTION.md, which defines the project's non-negotiable architectural rules (e.g., "Use Rust thiserror for all errors", "No unwrap() allowed").Analyze Request: It compares the user's prompt against the constitution and the current codebase state.Interrogate: It generates a series of clarifying questions to resolve ambiguity. Does the login feature need OAuth? What is the session timeout?This phase places a unique demand on the infrastructure. The model must hold the entire constitution, the relevant file summaries, and the growing conversation history in its context window simultaneously. This validates the requirement for a high-capacity KV cache, prioritizing memory quantity and bandwidth over raw compute speed.4.3 The Planning and Implementation PhasesOnce clarified, the /speckit.plan command generates a deterministic JSON plan. This plan acts as a contract. The /speckit.implement command then executes this plan. The codex-rs runtime ensures that the implementation adheres to the plan, preventing "scope creep" where the model hallucinates new features mid-implementation.19 This structured approach reduces the number of inference turns required to reach a working solution, effectively trading higher up-front context usage for reduced code-generation tokens.5. Model Selection Strategy: The Search for the Optimal 32GB BrainWith the hardware (RTX 5090) and runtime (codex-rs) defined, the critical variable remains the model itself. The "Architect" role demands high reasoning capabilities, while the "Builder" role demands coding proficiency. In the 32GB constraints of 2026, three primary candidates emerge.5.1 The Unsuitable Giant: Llama 4 MaverickMeta's Llama 4 Maverick is a massive Mixture-of-Experts (MoE) model with 402 billion total parameters and 17 billion active parameters.6Analysis: While the active parameter count (17B) suggests high inference speed, the total parameter count (402B) is the disqualifying factor. Even with 4-bit quantization, the model weights alone would require over 200GB of storage. The RTX 5090's 32GB VRAM is wholly insufficient to load even a fraction of the experts required for functional routing.7Conclusion: Llama 4 Maverick is strictly a datacenter or multi-GPU (8x A100) model. It cannot be run on a single workstation, rendering it unsuitable for this specific architecture.5.2 The High-Speed Alternative: Mistral Small 3Mistral's Mistral Small 3 is a 24-billion parameter dense model released explicitly to target the high-end consumer hardware segment.22Pros: At 24B, it fits comfortably into 32GB VRAM even at higher precisions (e.g., FP8 or 6-bit). It is extremely fast, capable of 50+ tokens/second on the RTX 5090.5 It has a 128k context window.Cons: Benchmarks indicate that while it excels at general tasks and simple coding, it trails larger models in complex logic and multi-step reasoning—the exact traits required for the "Architect" role in SpecKit.24 It lacks a dedicated "thinking" or reasoning mode, often looping during complex planning tasks.25Verdict: An excellent fallback for the "Builder" role if speed is paramount, but insufficient for the primary "Architect" role.5.3 The Primary Recommendation: Qwen 3 32BAlibaba's Qwen 3 32B represents the optimal point on the efficiency-reasoning curve for 2026.26Reasoning Capabilities: Benchmarks show Qwen 3 32B outperforming Llama 4 in pure coding tasks and approaching the reasoning capabilities of much larger models.26 It is a "dense" model, meaning its knowledge is accessible without complex MoE routing overheads.The Fit: At 32 billion parameters, a standard FP16 load (64GB) is impossible. However, utilizing the Blackwell-native FP8 format or 4-bit (AWQ) quantization changes the equation:4-bit Quantization (AWQ/GPTQ): Reduces the model footprint to approximately 18-19 GB.VRAM Calculus: 32GB (Total) - 2GB (System) - 19GB (Model) = ~11 GB Free.Context Capacity: 11 GB of free VRAM is sufficient to host a KV cache for approximately 32k to 64k tokens of context, depending on the specific quantization of the KV cache itself (e.g., FP8 KV cache). This fits the SpecKit workflow perfectly, allowing for the ingestion of the Constitution and detailed clarification loops without hitting an OOM (Out of Memory) error.Recommendation: Deploy Qwen 3 32B-Instruct quantized to 4-bit AWQ. This configuration maximizes reasoning capability while preserving the critical VRAM headroom required for the context window.6. Inference Infrastructure: SGLang vs. vLLMThe choice of the inference engine server—the software that actually runs the model—is as critical as the model itself. In 2026, the two dominant frameworks are vLLM and SGLang (Structured Generation Language).6.1 The Shortcomings of vLLM for AgentsvLLM has long been the standard for high-throughput serving, utilizing PagedAttention to manage memory efficiently.28 Its primary design goal is maximizing throughput (requests per second) for batch processing in multi-user environments.Latency: While throughput is high, vLLM's latency for single-user, sequential workloads (like an agent thinking loop) is often higher than optimized alternatives.28Prefix Caching: While vLLM supports prefix caching, its implementation is optimized for exact matches in batch requests rather than the dynamic, tree-structured conversation history of an agentic workflow.296.2 The SGLang Advantage: RadixAttention and Structured OutputSGLang has emerged as the superior engine for agentic workloads due to two key innovations: RadixAttention and native Structured Decoding.28RadixAttention is a memory management technique designed specifically for multi-turn conversations. Instead of a simple LRU cache, it maintains a radix tree of the KV cache.Mechanism: When codex-rs sends a prompt (e.g., Constitution + File A + Question), SGLang caches the KV states for "Constitution + File A". If the agent then asks a follow-up question regarding "Constitution + File A", SGLang detects the prefix match in the radix tree and reuses the computed attention states instantly.29Impact: This dramatically reduces the Time-To-First-Token (TTFT) for the "Maieutic" clarification loops of SpecKit, where the large system prompt (Constitution) is repeated in every turn.Structured Decoding: codex-rs relies on the agent outputting valid JSON for tool calls and plans. SGLang incorporates a compressed finite state machine (FSM) to enforce regex constraints (like JSON schema) during the decoding process. This ensures 100% syntactical correctness of the output with zero latency penalty, unlike vLLM which often requires post-hoc validation and retries.30Benchmark Data:Comparative analysis reveals significant performance deltas for this specific workload.FeatureSGLang (Recommended)vLLMImplications for SpecKit PipelineStructured Gen Latency1.08s (Low)1.55s (Higher)SGLang is ~30% faster at generating the JSON plans required by SpecKit.31Prefix Cache ReuseRadix Tree (High)Block Cache (Med)SGLang efficiently reuses the heavy "Constitution" prompt across turns, reducing wait times.29Throughput (1 req)230 tok/s187 tok/sFor a single user (developer), SGLang offers better raw speed.28Constraint EnforcementNative FSMPost-processSGLang guarantees valid JSON for codex-rs tool calls, preventing crash loops.Recommendation: Deploy SGLang with the FlashInfer backend kernel to leverage the RTX 5090's Blackwell FP8 acceleration.7. Memory Systems: The RAG Layer and Vector StorageAn "Architect" agent cannot rely solely on its context window; it requires long-term memory to index the entire codebase and retrieve relevant snippets. This is the domain of Retrieval-Augmented Generation (RAG).7.1 The Vector Database: LanceDBFor a Rust-based pipeline like codex-rs, the choice of vector database is clear: LanceDB.32Architecture: LanceDB is an embedded, serverless vector database written in Rust. It runs in the same process space as the application (or as a lightweight sidecar), eliminating the network overhead and operational complexity of running a heavy containerized service like Qdrant, Milvus, or Weaviate.32Zero-Copy Retrieval: It utilizes the Lance columnar format, which is designed for high-performance ML data. It supports zero-copy access, meaning data can be read from disk directly into memory without serialization overhead. This is crucial when the agent needs to scan gigabytes of code embeddings to find relevant architectural patterns.34Rust Synergy: Being native to Rust, it integrates seamlessly with codex-rs, allowing for extremely low-latency queries (<1ms) compared to the HTTP/gRPC roundtrips required by other databases.357.2 The Embedding Model: GTE-QwenTo convert code and specifications into vectors, we require an embedding model. The GTE-Qwen series (specifically the 1.5B or 4B variant) is the recommended choice.36Provenance: Built on the same Qwen architecture as our logic model, ensuring a semantic alignment between how the retriever "sees" the code and how the LLM "understands" it.Performance: It achieves state-of-the-art results on the MTEB benchmark, particularly for code retrieval and cross-lingual tasks.36 This is vital for modern codebases that may contain mixed-language documentation or comments.Efficiency: The smaller variants fit easily into the system RAM (64GB recommended for the workstation), leaving the VRAM dedicated entirely to the Qwen 3 logic model.8. Security Governance: ExecPolicy and The Human-in-the-LoopThe defining feature of a "Sovereign" agent is its ability to execute code. This capability introduces significant risk. codex-rs mitigates this via the ExecPolicy module, a robust governance layer.8.1 Configuration and EnforcementThe ExecPolicy is configured via a policy.json or config.toml file. It operates on a "default deny" principle.Syscall Interception: The codex-rs runtime wraps the command execution. Before a command is passed to the OS shell, it is parsed and checked against the policy rules.13Whitelist Strategy:Safe Commands (Allow): ls, cat, grep, git status, cargo build, npm test. These are read-only or build-process safe.Dangerous Commands (Block/Ask): rm, mv, sudo, curl, wget, ssh. These modify the system state or exfiltrate data.Approvals: For commands in the "Ask" category, codex-rs triggers an interrupt in the TUI. The user must manually approve the action. This Human-in-the-Loop (HITL) mechanism is the ultimate safety net.378.2 SandboxingBeyond policy, execution isolation is required. codex-rs supports integration with Bubblewrap (on Linux) or platform-specific containerization to create ephemeral execution environments.8 This ensures that even if a malicious command slips past the policy, it executes in a chroot-style jail with no access to the host's sensitive directories (like ~/.ssh or /etc).9. Implementation Guide: The "Single-Card Architect" BuildThis section synthesizes the architectural decisions into a concrete implementation roadmap for the RTX 5090 workstation.9.1 Step 1: Hardware PreparationGPU Driver: Ensure NVIDIA Driver 570.xx+ is installed to support Blackwell CUDA 13.x features.System RAM: 64GB DDR5 (minimum) is recommended to support the OS, LanceDB in-memory indices, and the GTE-Qwen embedding model, leaving VRAM for the LLM.9.2 Step 2: Inference Engine DeploymentInstall and launch SGLang with the specific flags for the RTX 5090.Bash# Install SGLang with FlashInfer for Blackwell acceleration

pip install sglang[all] flashinfer

# Launch Server with Qwen 3 32B (4-bit AWQ)

# --mem-fraction-static 0.85 reserves ~27GB for Model + KV Cache

# leaving ~5GB buffer for system and display.

python -m sglang.launch_server \

--model-path Qwen/Qwen3-32B-Instruct-AWQ \

--port 30000 \

--quantization awq \

--context-length 32768 \

--mem-fraction-static 0.85 \

--disable-cuda-graph-padding # Optimization for variable length inputs

9.3 Step 3: Codex-RS ConfigurationConfigure the ~/.config/codex/config.toml to bind the Rust runtime to the local infrastructure.Ini, TOML[core]

# Define the Architect Persona

system_prompt = "You are a Senior Systems Architect. Adhere strictly to SpecKit protocols."

[llm]

# Point to local SGLang server

provider = "openai_compatible"

base_url = "http://localhost:30000/v1"

model = "Qwen3-32B-Instruct-AWQ"

timeout = 600 # Planning takes time

[vector_store]

type = "lancedb"

path = "~/.codex/memory_index"

embedding_model = "Alibaba-NLP/gte-qwen-slm"

[security]

# Enforce the security policy

policy_file = "~/.config/codex/execpolicy.json"

sandbox = true

[speckit]

# Enable maieutic clarification loop

clarify_depth = "high"

9.4 Step 4: The AGENTS.md RouterDefine the roles in the repository's AGENTS.md to leverage the router.14Agent ConfigurationsArchitectModel: Qwen3-32B-Instruct-AWQTemperature: 0.7Context: Full Constitution + File SummariesRole: Clarify requirements, generate SpecKit plans.BuilderModel: Qwen3-32B-Instruct-AWQ (Low Temp)Temperature: 0.1Role: Execute 'apply_patch', run tests.10. ConclusionThe "Single-Card Architect" using an NVIDIA RTX 5090 is not a compromise; it is a specialized discipline. By rejecting the hype of massive, unrunnable models like Llama 4 Maverick and embracing the efficiency of Qwen 3 32B (4-bit), we fit a high-IQ reasoning engine into the local constraint. By choosing SGLang over vLLM, we optimize for the specific caching patterns of agentic conversations. By utilizing LanceDB and Codex-RS, we build on a foundation of Rust-native safety and performance.This stack delivers a functional, sovereign AI software engineer capable of understanding complex specifications, planning architectures, and writing secure code—all within the thermal and silicon limits of a single workstation. It represents the state of the art for 2026 local infrastructure.

AND

Speckit Pipeline Model & Infrastructure Upgrade Plan (2026)

Executive Summary

Single-owner pipeline architecture: The current speckit.auto workflow is a linear 7-stage pipeline (Specify → Plan → Tasks → Implement → Validate → Audit → Unlock) with one primary AI role per stage (Architect, Implementer, Validator, Judge) and optional sidecar reviewers

GitHub

GitHub

. This avoids multi-agent debates – consensus is enforced deterministically via tests and policy gates, not via model voting

GitHub

GitHub

.

Model routing policy: The guiding philosophy is “cloud where quality wins; local where speed/volume wins”

GitHub

. In practice, cloud LLMs (e.g. Anthropic Claude or OpenAI GPT) are used for complex reasoning (Architect/Planner, major code refactors, final Judge), while a local model handles fast “reflex” tasks (Implementer quick fixes, Tutor/maieutic coaching, routine Librarian tasks)

GitHub

GitHub

. Privacy is not a constraint, so routing optimizes cost and performance instead

GitHub

.

Current implementation: As of early 2026, the pipeline uses hardcoded model mappings in code for development (Claude 4.5 models for most roles by default)

GitHub

, with config placeholders for dynamic routing. Stage 0 (“Shadow Stage”) fetches context from a local memory store and optionally calls Google’s NotebookLM (Tier-2 synthesis) to produce a “divine truth” summary and context brief before planning

GitHub

GitHub

. Deterministic guardrails are in place: if the Architect’s self-reported confidence falls below a threshold (default ~0.65), or if critical issues are flagged, the pipeline escalates to a more powerful model or requires human review

GitHub

GitHub

. An Implementer that fails to produce passing code after 2 retries similarly triggers escalation

GitHub

.

Infrastructure recommendation: Deploy a single high-capability local model (e.g. Qwen3-Coder-30B-MoE) on the RTX 5090 via vLLM for all “fast reflex” roles (Implementer quick fixes, Tutor self-queries, Librarian summarization). This model can stay loaded in memory (no swapping) and deliver <200 ms responses for moderate prompts

GitHub

. Use cloud models only when needed for quality: e.g. Anthropic Claude Sonnet or OpenAI GPT-5.1 for initial planning and complex code generation, and GPT-5.1-High or Claude-Opus for final auditing

GitHub

. This hybrid maximizes quality-per-dollar by reserving paid API calls for high-risk or complex tasks, and leveraging the free local model for iterative loops and bulk work.

2026 model upgrades: Today’s top models outperform those from 2025 in both reasoning and coding. Google Gemini 3-Pro is a frontier general model (ranked #1 on LMArena with strong code and reasoning performance

lmarena.ai

), and OpenAI’s GPT-5.1 series offers reliable formatting and function calling with large context windows (up to 128k). Anthropic Claude 4.5 (“Opus” and “Sonnet” tiers) remains excellent for code, especially with “thinking” mode (chain-of-thought) for tricky logic

lmarena.ai

. For specialized reasoning, DeepSeek models (v3.x) excel in step-by-step problem solving

linkedin.com

, and for huge contexts or multi-document analysis, Moonshot Kimi K2 (a trillion-parameter MoE) can handle >100k tokens with robust tool use

GitHub

huggingface.co

. The upgrade plan proposes integrating these as needed: e.g. use DeepSeek R2 as an intermediate fallback for coding tasks that the local model struggles with, and call Kimi K2 only when a long-context sweep or contradictory information demands it (escalation-only use).

Maieutic enhancements: To incorporate Socratic “self-questioning” and error-checking, we recommend activating the Critic sidecar and/or adding an explicit self-review step. Currently, the pipeline can spawn a non-blocking Critic agent that outputs risks and missing requirements

GitHub

, but this is disabled by default. Enabling SPEC_KIT_SIDECAR_CRITIC=true will have a secondary model critique each plan and implementation

GitHub

GitHub

. High-severity critiques already reduce the stage confidence and can block auto-merge

GitHub

GitHub

. In addition, we propose a lightweight “clarity check” after the Plan stage: the Architect (or Tutor role) should generate a short checklist of assumptions or open questions and immediately resolve them (via the same model or a brief Q&A with a more knowledgeable model). This maieutic step can be implemented with minimal changes using the existing /speckit.clarify and /speckit.checklist command logic

GitHub

, and would improve plan robustness before coding begins (e.g. catching ambiguous requirements or edge cases early).

Cost management: The project currently tracks NotebookLM usage with a daily query limit (500/day) and warns at 80% usage

GitHub

GitHub

. We recommend extending this budget enforcement to all cloud API calls. Using the config cost.daily_limit_usd and alert_threshold fields (already defined)

GitHub

, the pipeline can log each API token cost and refuse to continue if the monthly or daily budget is exhausted. For example, at ~80% of the monthly budget, the system should issue a warning (and perhaps require a manual override to continue), and at 100% it should halt further cloud calls unless SPEC_OPS_ALLOW_BUDGET_OVERFLOW is set by an operator. Implementing this will require integrating with the token counting already done for model usage (OpenAI and Anthropic APIs return usage counts) – a manageable code change. By monitoring cost in real time and preferring local models for high-volume steps (tests, multiple refactor attempts), the team can prevent surprise overruns.

Upgrade roadmap: The plan is structured in three phases: a Minimal-change upgrade that can be applied quickly by adjusting configuration and swapping model endpoints (no code changes, focusing on using a local model via the OpenAI-compatible API and updating model IDs to current bests); a Medium upgrade that modifies the router logic and pipeline flow (introducing automated escalation after two failures, dynamic role→model mapping per policy, and enabling sidecar critics by default); and a Long-term refactor that implements more sweeping improvements (e.g. plugin the Stage 0 engine fully, adopt SGLang serving for multi-modal outputs, or redesigning certain stages for better clarity and integration with memory). Each stage of the roadmap is designed to be testable in isolation – with metrics like turn-around time, tokens/sec throughput, success rate of code generation, and cost per spec all collected to guide iterative tuning. By gradually rolling out these upgrades, we can achieve a state-of-the-art 2026 AI coding assistant that balances speed, quality, and cost, all while maintaining deterministic guardrails for safety.

Implemented Surface Area (Current Pipeline)

Pipeline stages and roles: The speckit.auto command orchestrates a fixed sequence of stages, each producing a specific artifact. An enum Stage defines these steps: Specify (pre-plan context gathering), Plan (high-level design), Tasks (task breakdown), Implement (code generation), Validate (testing), Audit (policy compliance review), and Unlock (final merge decision)

GitHub

GitHub

. In the standard flow, after a SPEC PRD is created (Specify stage), the pipeline runs through Plan → Tasks → Implement → Validate → Audit automatically. Each stage is “owned” by a single AI agent role under the single-owner design. The mapping of stages to roles is roughly: Architect owns Specify/Plan/Tasks (producing plan.md and tasks.md), Implementer owns Implement (producing code diffs in the repo), Validator owns Validate (running tests and evaluating results), and Judge owns Audit/Unlock (final verdict). These roles correspond to abstract responsibilities defined in Role enum

GitHub

. Sidecar roles exist as non-authoritative advisors: e.g. SidecarCritic for flaw-finding, SecurityReviewer for static analysis of risky changes, PerformanceReviewer for performance regressions, and Librarian for memory/context management

GitHub

. By design, sidecars cannot directly produce artifacts or halt the pipeline – they emit signals (warnings, risk flags) consumed by the gate policy

GitHub

. The code explicitly marks these sidecar roles with is_sidecar() and ensures primary stages only use one owner agent

GitHub

GitHub

(unless consensus mode is enabled, which is deprecated).

Deterministic quality gates: After each stage, a “gate evaluation” runs to decide if the output is acceptable or if escalation is needed. This is implemented in the gate_policy module and associated gate evaluation logic. Key inputs to a gate include: the owner’s self-reported confidence, results of deterministic checks (e.g. did code compile? did tests pass?), and any counter-signals from sidecars

GitHub

GitHub

. The pipeline avoids any non-deterministic merging of model outputs – no majority voting or LLM “consensus” step is used under default settings (the env flag SPEC_KIT_CONSENSUS=false enforces this)

GitHub

. Instead, the code sets expected_agents_for_stage() to always a single agent

GitHub

and treats a stage as passing if its required fields are present and no blocking issues are found

GitHub

GitHub

. Confidence is a numeric value (0.0–1.0) that the agent includes in its output JSON; by policy, if confidence < 0.75 for Architect stages, the pipeline should not auto-apply the changes

GitHub

. In code, the default threshold (min_confidence_for_auto_apply) is 0.65

GitHub

, slightly lower than policy – this likely will be updated to 0.75 per MODEL-POLICY v2.0. The DecisionRule config allows this threshold to be overridden

GitHub

. When evaluating a stage, if the effective confidence falls below the min (e.g. due to a Critic flag), the gate verdict is set to Escalate

GitHub

GitHub

. For example, a “block” severity counter-signal (such as a critical security flaw identified by the SecurityReviewer) will drop confidence to Low and trigger escalation logic (the test case in code shows confidence 0.45 with a block present

GitHub

). Escalation can mean different things: for Architect/Plan, it could mean asking a more powerful model to re-plan or seeking human input; for Implementer, the stated rule is after 2 failed attempts (e.g. code still not compiling or tests failing on second try), escalate to a cloud coder or require human review

GitHub

. The code tracks retry_count in the stage context for this purpose

GitHub

GitHub

. (At present, the enforcement of the “2 strikes” rule happens via config/policy rather than an explicit counter – the RoutingContext.retry_count is available to the router to choose a different model on retries

GitHub

GitHub

, but the default router doesn’t yet implement a switch. This is addressed in the upgrade plan.)

Model routing and configuration: The system defines a Router trait that maps a role to a WorkerSpec (which includes provider, model, and allowed tools)

GitHub

GitHub

. In practice, the DefaultRouter is used (unless overridden), which currently contains hardcoded mappings for development/testing

GitHub

GitHub

. In default mode, if prefer_local is false, Architect/Implementer/Validator roles are all mapped to Anthropic’s Claude model (“claude-sonnet-4” in code, likely Claude 4.5 Sonnet) and Judge to “claude-opus-4”

GitHub

. Sidecar Critic and PerfReviewer use a smaller Claude model (“claude-haiku-4”), and Librarian uses google: gemini-2.0-flash

GitHub

. These strings correspond to model presets (Claude 4.5 and a hypothetical Gemini 2.0). If local_only or prefer_local is true, the router maps all roles to "local: claude-code" (a placeholder for a local model)

GitHub

. In short, the production routing policy is not fully implemented in code yet – it’s written in MODEL-POLICY.md and partially reflected in config files, but the active code paths still mostly assume either a single cloud provider or a dev override. For example, the CLI config.toml.example sets a default model gpt-5-codex with model_provider="openai"

GitHub

, meaning the CLI might default to an OpenAI model for all tasks unless overridden. Meanwhile, the TUI logic classifies model names by provider: any model name containing “claude” is routed through the Claude CLI, “gemini” through the Gemini CLI, and others (GPT) use OpenAI’s API

GitHub

GitHub

. This dual system (config vs. hardcoded) exists because the app is in transition – SPEC-KIT-952 aimed to allow multiple providers via config. The effect is that today’s implemented surface uses OpenAI API for GPT models and external CLI calls for Anthropic/Google models, and you must configure which agents are enabled. The config supports listing multiple agents and subagents for consensus or multi-agent scenarios

GitHub

GitHub

, but by default consensus is off. Only one agent is used per stage (the first in list) unless explicitly turned on. The installed agents include “claude”, “gemini”, “code” (for local) and even “gpt_pro” (likely GPT-4/5)

GitHub

GitHub

– but these appear to relate to older consensus mode where multiple models would attempt a stage in parallel. In summary, the pipeline currently runs in single-agent mode, generally using whichever model is set as default (OpenAI or Claude) for everything. The structure to support dynamic routing (based on stage, risk, etc.) is there (via the Router and config flags) but needs to be populated with the actual policy rules (this is addressed in the upgrade plan).

Stage 0 and memory integration: Before the Plan stage begins, the system performs a “Stage 0” pre-processing if enabled (this corresponds to the Specify stage in code terms). This is implemented in a separate stage0 module (with an engine likely written in Rust and Node for NotebookLM). Stage 0’s goal is to gather relevant context from past specs, documentation, and codebase (the “memory”), and produce two things: (1) a “Divine Truth” – a high-level summary or key insight synthesized from all relevant info, and (2) a “Task Brief” in Markdown that enumerates important details (patterns, pitfalls, requirements) for the upcoming work

GitHub

GitHub

. Under the hood, Stage 0 uses an Intent Query Object (IQO) approach: it generates a JSON query describing what to retrieve (domains, tags, keywords) by analyzing the spec and environment

GitHub

GitHub

. A small local LLM (or heuristic) generates the IQO from the spec description

GitHub

GitHub

. Then, a local memory service (backed by a vector database and embeddings) executes the query – filtering by tags, doing semantic similarity search, etc. – to get candidate memory entries (design decisions, bug fixes, prior related specs)

GitHub

GitHub

. These candidates are scored using a combination of static priority, recency, usage frequency (tracked in an “overlay” SQLite DB), and embedding similarity

GitHub

GitHub

. The top-K are selected via a diversity-promoting algorithm (Maximal Marginal Relevance)

GitHub

GitHub

. Each selected memory is then summarized (to “compress” it) by a local summarizer model, and the summaries are concatenated into the final TASK_BRIEF.md

GitHub

. This brief and potentially the spec itself are then provided to NotebookLM (if Tier2 is enabled) for a higher-level synthesis. Specifically, the system spins up a local NotebookLM service (a Node.js process exposing a REST API) and creates a notebook with those sources. It then sends an “ask” query to NotebookLM, such as “Given the above context, outline the best solution approach”

GitHub

GitHub

. NotebookLM returns an answer (the “Divine Truth”) which is a structured summary or key insight. The code ensures this call is budget-limited: it uses a BudgetTracker to count how many NotebookLM queries have been made today and will refuse to call if the daily limit is reached

GitHub

GitHub

. By default the limit is 500 queries/day (with a warning at 400)

GitHub

GitHub

, presumably aligning with NotebookLM’s allowance. If NotebookLM is disabled or fails, Stage 0 falls back to just using the local Tier1 summarizer (so the pipeline can continue offline)

GitHub

GitHub

. The outputs of Stage 0 are written to the SPEC directory (e.g. task_brief.md) and also stored in memory for later reference. This Stage 0 runs quickly (a few seconds for retrieval + a few seconds for NotebookLM). It is integrated such that if Stage 0 fails or is disabled, the rest of the pipeline still proceeds (“graceful degradation”)

GitHub

– in practice, Stage 0 is optional but highly useful for non-trivial tasks, as it provides the relevant context that later stages rely on.

Testing and guardrails: After the Implement stage produces code, the Validate stage runs the project’s test suite (via a sandboxed shell command)

GitHub

GitHub

. The Validator role (if implemented as an LLM) might parse the test results and produce a summary (validation.md), but currently this stage is largely deterministic – the tests either pass or fail. On failure, the pipeline typically triggers the Implementer to attempt fixes (this loop is handled by the spec executor logic). There is an retry module providing backoff and retry strategies for such loops

GitHub

. The Audit stage involves the Judge role reviewing all artifacts (plan, code diff, test results) against compliance checklists (requirements, “constitution” rules, etc.). In practice, the Judge calls a top-tier model (GPT-4.5/5 or Claude Opus) and is instructed to produce an audit_verdict.json with findings and a pass/fail decision. By policy, merge (Unlock) cannot be auto-approved by a local model alone – either the cloud Judge must sign off or a human must intervene for high-risk changes

GitHub

GitHub

. This is enforced by routing the Judge role to a cloud-only model in config (and potentially requiring certain signals). For example, Role::Judge is never mapped to a local model in DefaultRouter unless local_only=true debug mode

GitHub

. Additionally, certain guardrails (like “no merge if tests below X% coverage” or “if any critical security issue is unsolved, do not merge”) are configured in quality_gates (e.g. min_test_coverage) and checked by the Audit stage

GitHub

. The pipeline also has a failsafe to prevent running on a dirty git tree (requires a clean working directory unless overridden)

GitHub

and to limit evidence size (the evidence archive of all artifacts is capped ~25 MB by default)

GitHub

. These ensure the automation doesn’t run in unsafe conditions or generate unbounded output.

Summary of key files (current):

Agents & roles: codex-rs/spec-kit/src/gate_policy.rs defines Stage and Role enums and guardrail parameters (confidence levels, risk levels)

GitHub

GitHub

. docs/MODEL-POLICY.md (v2.0) is the declarative source for intended routing rules

GitHub

GitHub

, though not fully realized in code yet.

Router & config: codex-rs/spec-kit/src/router.rs implements WorkerSpec and DefaultRouter (hardcoded dev mappings)

GitHub

GitHub

. The user config file (~/.code/config.toml) can override model/provider (example sets GPT-5 Codex via OpenAI)

GitHub

and enable/disable multi-agent features

GitHub

. CLI vs TUI have slightly different routing: see codex-rs/tui/src/providers/ for Anthropic/Google CLI call implementations

GitHub

GitHub

.

Stage 0 engine: Primarily in codex-rs/stage0/ (Rust) with a companion local-memory-mcp Node service. Key logic in STAGE0_SCORING_AND_DCC.md and STAGE0_SPECKITAUTO_INTEGRATION.md docs

GitHub

GitHub

. The NotebookLM client and budget logic is in codex-rs/core/src/architect/nlm_service.rs

GitHub

GitHub

and budget.rs

GitHub

GitHub

.

Execution & guards: codex-rs/core/src/executor (SpeckitExecutor) orchestrates stage advancement and uses PolicyToggles (from env) to decide if critic sidecar, consensus, etc., are enabled

GitHub

GitHub

. codex-rs/tui/src/chatwidget/spec_kit/gate_evaluation.rs contains the consensus/gate eval logic (with single-agent assumption)

GitHub

and evidence persistence (saving each agent output to JSON).

Testing & CI: The repository includes tests for the router and gate policy (e.g. ensuring single agent selection

GitHub

, local vs cloud mapping, etc.). A Git pre-commit hook ensures formatting and lint rules (no direct relation to pipeline, but part of the dev workflow)

GitHub

.

Overall, the current implementation establishes a solid single-agent chain-of-thought pipeline with deterministic checkpoints. The main areas for improvement are in realizing the dynamic model routing as per policy (right now the model selection is mostly static), upgrading the models themselves to 2026’s state-of-the-art, and enhancing the maieutic feedback loop (currently optional) to further reduce errors.

2026 Recommendations Table (Models & Tech per Role/Stage)

Area / Role Recommended Model & Provider Local or Cloud Rationale (Why Best) Cost Considerations Performance Notes Integration Notes

A. Architect / Planner Claude Sonnet 4.5 (Anthropic) or GPT‑5.1 Medium/High (OpenAI)

lmarena.ai

GitHub

(Fallback: DeepSeek R2) Cloud-first Top-tier reasoning needed for accurate plans. Claude Sonnet-4.5 (32k context) excels at multi-file and strategic planning with high reliability

lmarena.ai

. GPT-5.1 offers strong instruction-following and function calling, ensuring the plan JSON is well-structured. DeepSeek (latest R2) provides a second opinion if primary model shows uncertainty, leveraging its step-by-step strength

linkedin.com

. Claude Sonnet 4.5: ~$3 per 1k tokens in, $15 per 1k out

GitHub

. GPT-5.1: ~$1.25 per 1k in, $10 per 1k out

GitHub

. Planning prompts are ~2–5k tokens, output ~1–2k, so ~$0.02–0.05 per plan on GPT-5.1. DeepSeek R2 may be open-source (if self-hosted, cost is GPU time; if via API, likely low cost as it’s competitively priced to attract usage). Latency ~5–8s for a complex plan (Claude/GPT-5 at 32k context). Acceptable for this one-time stage. Streaming plan outlines is possible (Claude streams well). DeepSeek R2 fallback might be slower if self-hosted on smaller GPU, but triggered rarely (only on low-confidence). Integrate via existing Anthropic/OpenAI API calls (just update model IDs). Ensure plan template prompts match new model formats (e.g. function calling for GPT-5.1). For DeepSeek: either run local model (requires ~80GB GPU memory for 70B, not feasible on a single 5090) or call via an aggregator API (OpenRouter) – the system can treat it as an OpenAI-style provider with a custom endpoint. Add DeepSeek in config and Router logic as an EscalationTarget when confidence<0.75

GitHub

GitHub

.

B. Implementer (Coder) Local MoE Model: “Qwen-3 Coder 30B MoE” (AWQ 4-bit quantized)

GitHub

GitHub

(Escalation: DeepSeek Coder or OpenAI GPT-5.1 Codex High) Local for first 1–2 attempts, cloud if needed The local Qwen3-Coder-MoE is a state-of-the-art coding model optimized for single-GPU use (30B parameters with mixture-of-experts giving effective ~3.3B active)

GitHub

. It’s fine-tuned for code and can handle ~262k context, covering entire files or multiple diffs. Using it for initial codegen attempts is virtually free (no API cost) and fast. If it fails to produce correct code after 2 tries, escalate to a more powerful cloud model: DeepSeek’s coding model (R2 with “thinking” mode) can apply advanced reasoning to tough bugs, and if that fails, GPT-5.1 Codex (High reasoning mode) will likely succeed on complex refactors. This two-step escalation prioritizes cheaper (DeepSeek is often open or low-cost) before expensive OpenAI calls. Local model: one-time GPU cost (already owned hardware); no incremental cost per use. DeepSeek: if self-hosted, cost is electricity; if API, pricing for v3 is modest (e.g. some open tiers or ~$0.002/token). GPT-5.1 Codex High: ~$1.25/1k input, $10/1k output similar to base GPT-5

GitHub

. Implement outputs can be large (several hundred lines of code); a full file ~2k tokens output might cost ~$0.02. Even multiple attempts are cheap relative to human time, but heavy usage of GPT-5.1 could add up – hence only use it on difficult cases. Qwen-30B on 5090: ~100–150 tokens/sec generation with AWQ 4-bit (approx. 12–16 GB VRAM used)

GitHub

GitHub

. This yields sub-1s latency for small diff generation (<100 tokens) and ~2–3s for larger outputs – ideal for tight code/test loops. DeepSeek R2 (if cloud) might be slower per token but often enables a “thinking” mode that trades speed for quality; still, as a 70B-class model it can be ~10 tokens/sec, so a 500-token diff ~50s. GPT-5.1 via OpenAI is fast (~30–50 tokens/sec) and streams, so user perceives partial output in a couple seconds. Overall, local-first keeps the common case latency very low, and the rare escalations can tolerate higher latency. Deploy Qwen3 locally via vLLM serving with OpenAI API compatibility

GitHub

– this allows the existing code (which expects an OpenAI model for coder) to use model_provider=openai with endpoint=http://localhost:8000

GitHub

GitHub

. The local model should be loaded at startup and kept in memory. Verify that vLLM (or alternative backend) supports MoE routing and large context – it likely will, given Qwen’s popularity. Adjust the Router: implementer role should pick LocalModel first (we will set WorkerSpec.kind=LocalModel). After a failure, increment retry_count and then allow router to select a cloud Worker (DeepSeek or GPT)

GitHub

GitHub

. This can be done by extending DefaultRouter or replacing it with a PolicyRouter that checks ctx.retry_count. Integration of DeepSeek can reuse OpenAI API format if using OpenRouter, or require a new provider type (if so, add to ProviderType enum and implement a client). GPT-5.1 Codex via OpenAI is straightforward (just a different model name on existing OpenAI client). Also ensure tool permissions for Implementer remain (needs file write and shell exec to run tests)

GitHub

.

C. Validator (Test Runner) Local deterministic + GPT helper

(Model: Qwen-3 (same as Implementer) or Claude Instant for analysis if needed) Local by default The Validate stage primarily executes tests – a deterministic action. We recommend keeping this fully local: run the test suite on the machine, and parse results. If test output is straightforward (e.g. all passed or clear assertion logs), no LLM is needed. For complex failures or triaging multiple failing tests, we can employ a lightweight model to summarize errors. The same local Qwen model can do this (since it’s already loaded and adept at code), or use a smaller/fast model like Claude Instant 1 (haiku-4.5) which is cheap and fine for quick text analysis

GitHub

GitHub

. This ensures quick turnaround. Cloud LLMs are generally unnecessary here unless we plan to have the AI devise new tests or perform in-depth coverage analysis (not in scope yet). Virtually no cost for deterministic test execution (just compute). Using the local model for a brief log summary costs no money. If a cloud model were used (not recommended regularly), Claude Instant’s cost is ~$1/1k out (but here output is maybe 100 tokens, so <$0.001). Overall, cost is negligible in this stage. Running tests on local hardware (128GB RAM, presumably ample CPU) might take a few seconds to a couple minutes depending on project size. The AI overhead to parse results is minor (<2s). The local 30B model might be overkill for log parsing but will handle it in under a second. This keeps Validate stage latency bounded by test runtime primarily. Integration: continue to use the existing sandbox execution in Validator (no changes needed). For parsing, the Validator role in Router can point to LocalModel (the same Qwen) with read access to test output. Possibly leverage the spec-kit tool permission that allows the Validator to execute shell (already true in DefaultRouter)

GitHub

. If logs need summarizing, add a step where the log is fed to the model with a prompt “Summarize test failures succinctly.” This can be done inline in the validate stage code. No new external calls required. If extremely detailed reasoning is ever needed (like diagnosing flaky tests), we might allow an escalation to cloud (GPT-5.1) under a flag, but by default it’s unnecessary.

D. Judge / Auditor GPT-5.1 High (OpenAI) or Claude Opus 4.5 (Anthropic)

GitHub

GitHub

(No local fallback) Cloud-only The final audit demands the highest reliability and compliance with rules. GPT-5.1-High (or GPT-5.2 by late 2026) is one of the most capable models for complex reasoning and catching subtle issues. It has shown excellent performance on code and policy adherence tasks, and can output structured JSON verdicts consistently. Claude Opus 4.5 is an alternative with similarly top-tier ability and a 100k+ context – useful if the evidence (all stage artifacts) is very large

GitHub

. We do not use local models here because a misjudgment could merge broken code; the policy explicitly forbids local-only approval for high-risk changes

GitHub

. Using two cloud models in tandem (for cross-check) is possible but not default – single Judge model is authoritative unless it defers. This stage is infrequent (one call per spec). Cost can be higher since we provide all artifacts (plan, code diff, test results, brief, etc.). That could be ~20k tokens of input. GPT-5.1 at that size: ~$0.025 * 20 = $0.50 in, and output maybe 1k tokens ~$0.01, so ~$0.51 per audit. Claude Opus 4.5’s pricing is higher: ~$15/1k out

GitHub

, but if using only ~500 tokens out it’s manageable ($7.5). Given audits are critical but rare, the cost is acceptable. Budget gating should ensure we don’t overuse this on trivial changes. GPT-5.1 and Claude Opus both handle large inputs efficiently with streaming. The audit response (a verdict JSON) is typically ready in ~10–15 seconds for 20k input, which is fine at pipeline end. They both support citing evidence by ID if prompted, which is useful for NotebookLM-grounded answers. No real-time requirement here. Integration: Use OpenAI ChatCompletions API with the highest model available (the config model = gpt-5.1-high and model_provider = openai covers it). Ensure the system sends all necessary artifacts in the prompt (the current system likely does – audit.md prompt template). The output schema is known, so just verify the new model respects it (GPT-5 series are good at JSON when instructed). Since we rely on this for gating merges, implement a double-check: if the Judge’s confidence field is low or it flags uncertainty, the pipeline should require human review. This can be done by parsing the audit verdict (it may include a confidence or require “manual approval” flag). No code changes needed beyond updating model name and maybe adjusting prompt for any new syntax.

E. Sidecars (Critic, Security, Performance) Claude Instant 4.5 (Anthropic Haiku) for Critic

GitHub

GitHub

;

Claude or GPT-4.5 for Security;

None default for Performance Cloud (on-demand) The Critic sidecar is meant to quickly point out obvious issues in an output. A fast, cost-efficient model like Claude Instant (Haiku 4.5) is ideal – it’s tuned for pinpointing errors and is inexpensive, so we can run it in parallel without much overhead. SecurityReviewer could use a Claude model with a prompt focusing on secure coding (Claude has context to handle code and known vuln patterns). Alternatively, OpenAI’s GPT-4.5 could be used for security reviews (it has a good knowledge of CVEs). PerformanceReviewer is less crucial (and not always enabled) – we can leave it off by default or use the same model as Critic with a performance-oriented prompt if needed. Claude Instant 4.5: ~$1 per 1k output

GitHub

and even less for input. Sidecar prompts are relatively small (the agent sees the artifact, maybe 1–2k tokens, and outputs a few bullet points). So each sidecar run costs maybe $0.002–$0.005. Enabling them by default adds only pennies per spec. Security analysis might be slightly longer output if issues found, but still cheap. Given these low costs and potential to catch critical issues, it’s worthwhile. Claude Instant responds in ~1–2 seconds for a short critique of a plan or diff (it’s very quick, hence the “Instant” moniker). Running it concurrently during stage gate means we add minimal total latency. The pipeline can fetch main output and sidecar output in parallel threads. Even with two sidecars (Critic + Security), we’re looking at a couple seconds added. This is negligible compared to manual review time it saves. Integration: The code already supports a sidecar Critic if SPEC_KIT_SIDECAR_CRITIC=true

GitHub

. We just need to enable it via config or env. Critic and Security sidecars are mapped in DefaultRouter to Claude models currently

GitHub

– we will update those model names to the latest Claude Instant (“claude-haiku-4.5” in presets)

GitHub

. Also, ensure is_high_risk flag triggers the SecurityReviewer; according to tests, high-risk implement stages include Security sidecar

GitHub

. We define “high-risk” conditions (e.g. diff touches auth code or uses unsafe in Rust) – those can be detected via simple heuristics or tags in the spec. The PerformanceReviewer can remain disabled unless performance-critical code is being changed (perhaps enable if a diff is > X lines or touches known hotspots). These sidecars don’t require new infra – they use the same provider (Anthropic) with a smaller model, invoked via the CLI wrapper in TUI or direct API in CLI. The Critic’s output is recorded in the evidence and influences confidence (already implemented in gate logic). We should double-check that their severity mapping aligns with our thresholds (e.g., Critic should mark truly blocking issues as SignalSeverity::Block so that auto-apply is prevented

GitHub

).

F. Librarian (Memory & Context) Local Qwen-3 30B (same as main local model)

(Escalation: Kimi K2 for extreme cases) Local routine, cloud if huge The Librarian role is responsible for long-context synthesis and cross-referencing large knowledge bases. In our design, the Librarian duties are largely handled by Stage 0 and the memory system. We recommend continuing to use the local model for summarizing or reconciling context (since Qwen-3 has a 262k context, it can handle quite a lot). Only if an extremely large context (>200k tokens) or complex multi-doc reasoning is required would we call Kimi K2 – a specialized MoE model known for handling massive context and “agentic” tool use

GitHub

huggingface.co

. Kimi K2 (32B active, 1T total params) can ingest hundreds of thousands of tokens and perform deep analysis, so it’s perfect for an “escalation librarian” when needed. But Kimi usage should be rare (e.g. end-of-quarter documentation sweep or a contradictory knowledge reconciliation) given its complexity. The local Librarian tasks (e.g. summarizing 50KB of notes) cost no money, just GPU time. Kimi K2 via API is surprisingly affordable for its scale: roughly $0.60 per million input tokens, $2.50 per million output

reddit.com

. So even a 200k token input (~0.2M) costs only ~$0.12. Output would be maybe a few thousand tokens ($<0.01). This means even huge context calls are a few dimes – significant but not prohibitive if truly needed. We’d set a budget rule to not exceed e.g. $1 on Kimi per spec without approval. The local Qwen can process ~200k tokens but with some latency – reading ~150k might take a few dozen seconds. If that becomes a bottleneck, that’s when offloading to Kimi (which likely runs on a server cluster with optimized architecture) makes sense: Kimi K2 could handle 200k tokens and still respond within tens of seconds due to heavy parallelism. For day-to-day, Librarian operations are small (Stage0 retrieving maybe 10–20 memories, summing up maybe 5–10k tokens of text). Qwen does that in a couple seconds. So performance is fine. Kimi’s use (if any) might incur ~30–60s latency for a giant context, which is acceptable for rare maintenance tasks. Integration: The Librarian role surfaces in a few places – e.g. ACE “Reflector/Curator” which curates memory entries. By policy, Librarian is local by default

GitHub

. The code already maps Librarian to a local model (“local: claude-code” dev placeholder)

GitHub

. We’ll point that to our Qwen model. We should also incorporate the Kimi escalation: e.g. if a memory recall query exceeds certain size or the local summary is incoherent (perhaps measured by low confidence or overlapping content), then call Kimi. Implementation-wise, we add Kimi as a new provider (if using an API) or route via OpenAI-compatible interface if Moonshot provides one. Kimi’s output (e.g. a huge summary) would then feed back into our memory. Because Kimi might use tools, ensure the prompt given includes relevant tools info or that we disable tool use (unless we want it – possibly not needed in this pipeline). This addition can be done without disrupting other stages: it would be a conditional call in the Stage0 or Librarian utility code. Also, Kimi being MoE and multi-node, we obviously can’t run it on our single GPU – it must be API. So we’ll restrict its use via config flags (e.g. use_kimi_for_context_if >100k).

G. Stage0 “Tier2” Synthesis NotebookLM (Google)

(Fallback: Local Qwen or GPT-5.1 with citations) Cloud (NotebookLM) Stage0 Tier2 is essentially an advanced Q&A or summarization over retrieved documents. NotebookLM remains a strong choice in 2026 for this because of its ability to ingest multiple sources and produce citation-grounded answers (crucial to avoid hallucinations in the plan)

GitHub

. We keep NotebookLM as the default Tier2 for now – it’s specialized for this use. However, we note that OpenAI and others have introduced tools for grounded synthesis (e.g. GPT-5 with “browse” plugins or Retro-enhanced models). As a fallback, we can use our local model or GPT-5.1 to synthesize context if NotebookLM is unavailable. The local Qwen has decent summarization ability, but it may hallucinate without grounding. Alternatively, GPT-5.1 can be prompted with something like “Use the provided sources only” and it will often produce a good summary with inline citations (if we format sources as context passages with tags). So our plan is: NotebookLM primary; if tier2_enabled=false or it hits an error/limit, fall back to a cloud GPT-5.1 call that is instructed to output a source-backed summary (this requires us to supply the memory snippets as part of the prompt). NotebookLM pricing in 2026 is subscription-based (Plus tier allows ~1k queries/day). The project’s use (1 query per spec) easily falls under free quota if any. If charged per use, expect something like $0.05 per query (just an estimate; it’s likely negligible given Google’s strategy to integrate it with Google Docs). GPT-5.1 fallback would cost around $0.10 (for maybe a 4k-token prompt with sources and a 1k output). So cost is minor here. NotebookLM responses come in a few seconds typically (it’s optimized for interactive use). Our usage is non-interactive, and we already have an asynchronous call in place with a ~120s timeout

GitHub

. That is plenty. In tests so far, the Stage0 query returns in ~5–15 seconds for moderate content. The fallback GPT-5.1 approach might actually be faster if run on Azure or OpenAI’s infra (due to more scale). Either way, Tier2 will not bottleneck the pipeline significantly (Stage0’s overall latency ~ under 20s). Integration: Currently implemented via nlm_service which starts a local NotebookLM service and hits POST /api/ask

GitHub

GitHub

. We keep this. To upgrade, ensure our NotebookLM session can accept more documents (should handle dozens of sources if needed – check if any new version increase limits). Also, enforce citation grounding: NotebookLM already returns answer with references to source indices. The Judge will expect that any claims from Tier2 are backed (the pipeline could even fail Audit if citations are missing). If we use GPT-5.1 fallback, we must instruct it explicitly to include source identifiers (we can tag each snippet as e.g. [A], [B], and ask it to cite [A] etc. – GPT-5 series can do that). It’s not as guaranteed as NotebookLM’s system, so we use it only if needed. This fallback could be coded in the Stage0 engine: if NotebookLM returns success=false or times out, call an OpenAI completion with the combined text. Also, track cost if using fallback (since that would hit OpenAI – though as noted cost is low). The existing budget tracker can be extended to log usage of Tier2 fallback as well.

H. Local Inference Backend vLLM (OpenAI-compatible server)

GitHub

(Alternate: SGLang framework; TensorRT-LLM for static engine) Local We recommend vLLM for serving the local Qwen model because it natively supports the OpenAI Chat API, allowing us to plug it in without code refactor. vLLM is highly optimized for GPU memory and throughput with PagedAttention (efficient KV caching) and can handle multiple contexts smoothly

kanerika.com

. It also supports streaming token output, which our UI can leverage. Alternatives: SGLang is a cutting-edge serving framework that could yield even better throughput, especially if we later run multiple models or need structured output control

github.com

github.com

. However, SGLang integration would be more complex (it introduces its own front-end language for prompts and might require modifying our prompt handling). TensorRT-LLM is NVIDIA’s optimized engine that could maximize raw tokens/sec for Qwen, but it requires converting the model to a TensorRT plan and is less flexible (harder to handle 262k context or MoE). For our needs (single model, high context, some dynamic prompt sizes), vLLM offers the best trade-off of ease and performance. vLLM is open-source and free. It uses the GPU efficiently such that we likely can handle dozens of prompts concurrently if needed (though our pipeline is mostly sequential). No direct monetary cost. SGLang is also open-source, backed by LMSYS, so free – but engineering time is a cost to consider if adopting it. TensorRT-LLM is free, but optimizing models is time-consuming and any minor model change requires re-building the engine. On a single RTX 5090, vLLM can approach near-hardware limits for generation. Empirically, vLLM often achieves >70% of the theoretical throughput due to its optimized batching and memory management. For Qwen-30B, that could be ~150 tok/sec, as mentioned. SGLang claims even better efficiency for certain scenarios (they reported 7× faster than naive for Llama in 2024)

github.com

, but it shines with batch serving, which we might not utilize fully. TensorRT-LLM could possibly generate even faster for a single stream (maybe 20–30% faster than vLLM)

northflank.com

, but that difference (~0.2s saved per request) is not critical. Also, vLLM’s support for long prompts is proven; TensorRT might struggle with 100k+ token contexts due to memory fragmentation (not a common use, but possible in Librarian tasks). In summary, vLLM meets our latency needs (<0.5s reflex responses, <3s long ones) with minimal fuss. Integration: Launch vLLM API server with the Qwen model at startup (we can integrate it into our scripts/tui-session.sh or as a service). Use the --max-model-len 32768 (or higher, if Qwen supports 262k we set that) and appropriate GPU memory fraction

GitHub

. Then update config.toml: under [models.openai], set endpoint = "http://localhost:8000"

GitHub

so that all OpenAI API calls go to vLLM. Because our config’s model_provider for local model is “openai” (as a hack to use OpenAI client code), this will seamlessly redirect calls intended for GPT-5-codex to our local model. We should also adjust Model_Name if needed – e.g. define a provider “local” and handle that in code to avoid the hacky overload of openai. (A cleaner approach is adding a ProviderType::Local in code and treat it similar to ChatGPT type, pointing at vLLM; but not strictly necessary). Monitor vLLM’s memory usage – Qwen 30B 4-bit will use ~16GB VRAM, leaving headroom on the 5090 for other overhead. If we ever want to try SGLang: it could be swapped in as it also offers an OpenAI-compatible mode, but it may require different initialization. We’d consider SGLang in the long term if we run concurrent conversations or want to utilize advanced features (like the structured output validation it supports). For now, vLLM is the path of least resistance.

I. Embeddings & Vector Store BGE-M3 embedding model (BAAI)

ollama.com

+ Local HNSW index (Qdrant or Faiss) Local For document and code embeddings, BGE-M3 is a top choice in 2026. It’s a multilingual, multi-domain model that produces 1024-dim embeddings and excels at both semantic similarity and lexical matching

bge-model.com

inference.readthedocs.io

(it has a mechanism to encode some keyword info, reducing need for separate BM25). BGE-M3 is well-suited to code+text, and being local avoids any privacy or cost issues. We pair this with a local vector database – our current memory likely uses Qdrant (we saw references to flushing) or an SQLite+Faiss hybrid. We can stick with Qdrant (GPU-accelerated ANN search) or even use an in-memory HNSW via Faiss since our data isn’t huge. BGE’s embeddings combined with a high-performance ANN index ensure Stage0 recall is fast and relevant. OpenAI’s text-embedding-ada could be a fallback, but we avoid it to keep things reproducible and to not depend on external service for every run. BGE-M3 is open-source (from BAAI); running inference on it for embedding is a one-time cost per document. It’s a ~3B param model (based on XLM-R), which might use a few GBs of RAM – we can run it on CPU if needed for small volumes, or on GPU for speed. Either way, no API fees. The vector store (Qdrant) is self-hosted and free. Storage overhead is minimal (embedding 1k documents * 1024 dims * 4 bytes ~ 4MB). BGE-M3 embedding throughput is decent: on a GPU it can embed a sentence in ~20ms. If our memory has, say, 1000 entries, updating or querying is sub-second. Qdrant (with HNSW) can find nearest neighbors in ~10ms for 1000 vectors. Overall, the retrieval step in Stage0 will remain <1s. This is negligible in pipeline terms. Also, BGE’s mixed semantic-lexical approach improves relevance, potentially reducing how many candidates we need to fetch (increasing precision). Integration: We likely already have an embedding pipeline in the local-memory service. We swap its model to BGE-M3 (if not already). The local-memory-mcp might currently use sentence-transformers or older models – upgrading to BGE may involve changing the model name in that service’s config and ensuring the embedding dimension (1024) matches the index expectations. Qdrant was likely used (the code snippet about collection.flush() suggests Qdrant) – we should continue with Qdrant and upgrade it if needed to latest version for best performance. (Qdrant can run in-memory with persistence, and has GPU support for some ops). Alternatively, if using Faiss, just re-index with new embeddings. The key is to maintain stable embeddings – BGE’s open release means we won’t get version drift. Also, consider enabling multi-vector search if BGE-M3 supports it (it can produce multiple vectors per document for better recall

bge-model.com

, though this complicates index logic; probably not necessary for our scale). Finally, update our Stage0 scoring to incorporate embedding similarity appropriately – likely a cosine similarity weighting as currently done

GitHub

. With BGE, we might not need a separate BM25 step, simplifying the pipeline. Ensure to test retrieval quality on a few specs to fine-tune any search parameters (top_k etc.).

Citations: The recommendations above are grounded in model card data and benchmarks: e.g., LMArena rankings

lmarena.ai

show Gemini 3 and Claude 4.5 leading, model preset descriptions from our repo confirm costs

GitHub

GitHub

, DeepSeek and Kimi info from public announcements

linkedin.com

huggingface.co

, and infrastructure capabilities from vLLM and SGLang docs

GitHub

github.com

.

Alternatives Table (Top 1–2 Alternatives per Area)

While the above choices represent our top recommendation, here are the next best alternatives for each area, with reasons one might consider them:

Area Alternative 1 Alternative 2

Architect Gemini 3-Pro (Google): A cutting-edge general model, top-ranked on reasoning benchmarks

lmarena.ai

. It’s multi-modal and strong in code. Chosen if we favor Google’s ecosystem or if Anthropic/OpenAI usage needs to be minimized. Con: currently in preview, slightly less predictable formatting. GPT-4.5 (OpenAI Legacy): The refined version of GPT-4, widely proven. We could use GPT-4.5 if GPT-5.x is unavailable or for cost-saving (if 5.1 is premium priced). It’s reliable but may have a smaller context (32k) and slightly lower creativity than 5.1.

Implementer CodeLlama 2 34B or 70B (Meta): An open-source model fine-tuned on code. If we wanted entirely local and MoE wasn’t an option, CodeLlama2 70B 4-bit on our GPU is borderline (70B might not fit well on 32GB, but 34B could). It’s decent at coding but not as good as Qwen or DeepSeek. Pro: no API, open license. Con: lower success rate on complex tasks, slower. Anthropic Claude Codex (if available): Anthropic was reportedly working on a code-specialized Claude variant. If it exists (Claude Codex 4.5?), it could be an option for cloud implementer. Likely high quality, but would incur similar costs as GPT-5.1. We’d consider this if we observe GPT-5.1 making formatting errors or if Anthropic offers a better rate.

Validator No LLM at all (status quo): As an alternative, we can keep validation purely deterministic – run tests and have the pipeline interpret results with simple regex/heuristics (e.g., if any test failed, mark stage as fail). This is robust and doesn’t rely on AI. Pro: absolute determinism. Con: no intelligent summarization, but that might be fine. Small helper LLM (e.g. Llama-2 7B local): Instead of using the big Qwen for test logs, we could run a tiny model to summarize errors. Something like Llama2-Chat-13B on CPU might suffice. However, since the big model is already loaded, using it has near-zero marginal cost and better quality. So this is only if we wanted to offload to CPU and free GPU for other tasks during that moment.

Judge Dual-model consensus (GPT-5.1 + Claude): Rather than a single judge, we could run both GPT-5.1 and Claude on the audit, and only approve if both agree. This is a form of redundancy to catch any single model’s blind spots. It’s been disallowed by policy as authoritative (to avoid multi-agent confusion)

GitHub

, but it’s an alternative approach if we ever find the single-model audits missing issues. Pro: extra safety, Con: double cost and complexity of merging their outputs (we’d need to write logic to reconcile conflicts). Human in the loop: The default alternative to an AI Judge is a human code reviewer. We mention this for completeness – if at any point the organization requires, we can route the Audit stage to a human (perhaps through an approval UI). This obviously ensures highest fidelity, but it’s slow and breaks full automation. Likely only used for extremely sensitive changes. (Our plan keeps AI as judge, so this is truly a fallback alternative.)

Critic Sidecar Self-critique (same model): Instead of a separate Critic model, have the Architect/Implementer’s own model perform a self-review after producing output (e.g., “Now critique the above plan.”). This avoids using a second model. It’s aligned with the “self-maieutic” idea and can be done in one continuity of the conversation. Pro: no extra API call, the model knows its output well. Con: risk of the model being lenient on itself or the overhead of prompting it twice. Still, this is viable if multi-agent is disabled. OpenAI Function Calling for validation: Another approach is to embed some rules into the output schema – e.g., have the plan output include a “risk_analysis” field. GPT-5.1 can populate this with known issues. This isn’t a separate model, but an alternative technique to get the equivalent of a critic’s output. It leverages the model’s internal knowledge. Con: not as pointed as a dedicated critique prompt, but worth noting.

Security Sidecar Static analysis tools: Rather than an LLM, we could integrate static analyzers or linters (like a Rust Clippy for unsafe usage, or a security scanner) into the pipeline. These tools are deterministic and can catch certain issues. Pro: no hallucinations, very precise for known patterns. Con: limited scope (they won’t understand spec requirements or logic flaws). This could complement an LLM or replace it for certain checklists. Kimi K2 agentic mode: Kimi is not only long-context; it’s also pitched as “agentic,” meaning it can perform multi-step logical analysis. We could conceivably use Kimi for a comprehensive audit including security. But given its scale, using it as a sidecar for every change is overkill (and might overlap with Judge tasks). So this is more theoretical – if we needed the absolute best AI for security review, Kimi might do a thorough job. Currently, we stick to smaller models for speed.

Librarian Llama-2 70B with long context: If Qwen3 or local MoE wasn’t available, an alternative to handle moderately long context is Llama-2 70B fine-tuned for summarization with a 100k token context window (some community versions exist). We could run that on GPU with 4-bit, but it would be slow and probably not fit well. This is not preferable given Qwen and Kimi options. External memory service (DeepLake/Atlas): Not a model, but an alternative infra – use a cloud-hosted vector DB with built-in LLM for synthesis. For instance, if we used something like DeepLake or Atlas Cloud, they offer an API to do “search and summarize.” That might offload Librarian work. Con: introduces external dependency and cost, and possibly privacy concerns. We already have a custom solution that works, so we keep it.

Stage0 Tier2 OpenAI Retrieval Plugin / Azure Cognitive Search: Instead of NotebookLM, one could use OpenAI’s retrieval augmented generation by pushing docs into Cognitive Search or using a plugin. This might simplify using GPT-5 to do the summary with sources. Pro: one less system (Google) in the loop. Con: more engineering to set up indexing in Azure, and perhaps not as tailored as NotebookLM’s notebook paradigm. Bing Chat Enterprise / “DeepSeek Scholar”: Some services specialize in answering with citations (Bing can cite web sources). If we had an internal knowledge base web, we could query something like that. However, adapting it to our use (with custom docs) is non-trivial. NotebookLM is basically designed for custom doc Q&A, so it’s already the best fit. So alternatives here are mostly theoretical unless NotebookLM access goes away.

Local Backend SGLang: As mentioned, if we anticipate scaling to more models or want cutting-edge performance, we could invest in SGLang. It would allow combining multiple models (e.g. run Qwen and maybe a smaller assistant concurrently) and has features like RadixAttention and speculative decoding that could speed up generation by ~1.5–2× in single-stream cases

github.com

. We keep it in mind if vLLM falls behind or if we need its structured output capabilities (it can enforce JSON with a declarative syntax). TensorRT-LLM / FasterTransformer: These are low-level optimized engines. We could convert Qwen’s model to a TensorRT engine to squeeze maximum throughput. This might increase generation speed by ~20% and reduce latency slightly

northflank.com

. It’s an alternative if we find generation is a bottleneck. Downsides are loss of flexibility (harder to support very long context or incremental model updates). Given our latency requirements are met by vLLM, we don’t need this now, but it’s there if needed (especially if we try larger models that strain the hardware).

Embeddings OpenAI text-embedding-ada-003 (cloud): This is a strong baseline for embeddings and easy to use. If we didn’t want any local embedding infra, we could call OpenAI for each memory item. It yields 1536-dim vectors of high quality for text. However, doing this for every recall in automation would incur costs (~$0.0004 per vector) and reliance on external service. It also can change (OpenAI sometimes updates embeddings, affecting reproducibility). So we prefer BGE for stability. Instruction-tuned Embeddings (e.g. E5-large or Cohere): Models like E5 or Cohere’s embeddings are alternatives. E5-large (Mistral-based) is good and open-source. BGE was chosen because of its multi-function ability (dense+lexical) which matches our needs (code and prose). If BGE didn’t perform well on code, we’d consider a code-specific embedding like CodeBERT or UniXcoder. But BGE-M3 reportedly handles code semantics well, so it’s our choice.

Delta vs Current Implementation

Compared to the current system, the proposed solution introduces the following changes:

Model assignments: The pipeline will no longer use a single default model for all stages. Instead, each role/stage has a designated model or model pool as per the routing policy

GitHub

. In current code, Architect/Implementer both default to Claude 4.5 or GPT-5 (depending on config) for everything

GitHub

. After upgrade, Architect stays on a cloud model (Claude/GPT-5.1), Implementer switches to a local model first approach (new), Validator remains local (similar to current, as tests are local anyway), and Judge stays cloud (same category as current, but upgraded model). This mapping better aligns with “cloud for quality, local for speed” principle

GitHub

that the current hardcoded router hadn’t fully realized.

Local model integration: Introduction of vLLM server for Qwen-30B. Currently, no local large model is actively used (the code had placeholders for “local: claude-code” but required prefer_local flag manually). Now we will actually load and use a local LLM for Implementer (and others). This requires installing the Qwen model and vLLM, but does not require code changes to the core – just configuration (point OpenAI endpoint to localhost) and ensuring the CLI can obtain responses from it. The current system already uses OpenAI API for GPT calls; after this change, those calls hit our local vLLM. So from the speckit pipeline’s perspective, it’s the same API – thus delta is minimal in code but significant in infrastructure (running the server).

Confidence threshold increase: The policy of Architect escalating if confidence < 0.75 will be enforced. Presently, the code uses 0.65 by default

GitHub

, meaning some low-confidence plans might be auto-applied. We will raise this to 0.75 (either via config quality_gates.min_confidence_for_auto_apply or adjusting default). Effect: slightly more frequent escalations for uncertain plans. This aligns with updated MODEL-POLICY.md (v2.0) and ensures higher assurance on plan quality. It’s a one-line config or code change.

Retry logic & escalation: Currently, if Implementer’s first attempt fails tests, the system likely retries (there is a retry loop), but it probably uses the same model again. The upgrade will change this: on the third attempt (after 2 fails), the router will swap to a cloud model

GitHub

. Implementer escalation logic will be implemented explicitly. This is a moderate code delta: modifying the executor or router to check retry_count. From a pipeline perspective, it means difficult patches get handed to a more powerful AI sooner, increasing success rate at the cost of some API usage. This wasn’t automated before.

Sidecar critic activation: In current runs, sidecars are off by default (unless env var set). We plan to turn on the Critic (and Security for high-risk) by default. So the pipeline will produce critique artifacts at Plan and Implement stages. The code already supports this (no code change, just set SPEC_KIT_SIDECAR_CRITIC=true). The delta is more outputs and the gate logic incorporating them (which it does: block signals affect confidence). We need to verify that non-blocking advisories don’t prevent auto-merge (they shouldn’t). This change will make the pipeline more chatty in logs/evidence but safer.

Maieutic self-check: We are adding an explicit clarify/checklist step (either as part of Architect’s process or a minor new stage). Currently, after Plan is generated, the pipeline moves straight to Tasks. The new step would insert a prompt like “Identify any unclear requirements or risks in the plan” and get either the same model or the Critic to produce a list, then address them. This is a new behavior – not present before. Implementation could reuse existing subcommands (speckit.clarify) in an automated way. This will slightly lengthen the planning phase and may generate an updated plan or notes. It’s clearly beneficial for quality but is a functional delta. We will guard it behind a config flag initially (so we can turn it on/off easily).

Stage0 memory upgrades: Using BGE-M3 embeddings instead of whatever is used now (likely older model or plain keywords). This will improve retrieval relevance. The memory DB remains local; just the model for embedding changes. That is largely an internal detail – the Stage0 interface (lm recall etc.) remains same, but results should be better. One must re-embed existing memory entries with the new model (migration step). This is a data update, not code (except pointing to new embedding model). Also, Stage0 will now be fully enabled by default (assuming it was optional before). We consider Stage0 a standard part of /speckit.auto now, as its overhead is low and context is valuable. So if it was behind a flag, we’ll flip that to on.

Budget enforcement extension: The current code tracks NotebookLM queries only

GitHub

. We plan to unify cost tracking for all model calls. This is new functionality – we’ll hook into the API call wrappers to accumulate cost against configured limits. When 80% of monthly budget is hit, the system will emit a warning to logs (and possibly to the user in TUI). At 100%, further cloud calls will be blocked unless overridden. This might involve a new module or extending BudgetTracker to count tokens * cost rates. It’s additional code but not too complex. The delta is an operational one: previously the pipeline would cheerfully call APIs until keys fail or money spent; now it will proactively stop or warn. This prevents runaway usage (especially important since we’re adding more cloud usage in some areas like Critic and occasional escalations).

Config and documentation updates: We will update config.toml.example to reflect new defaults – e.g., model = qwen-3-coder for local use (with provider local or openai+endpoint hack), an entry for gemini/claude as Architect, etc. Also document the new environment flags (like enabling sidecars by default, budget limits). The MODEL-POLICY.md will be updated to v2.0 specifics (some already there, like new routing table

GitHub

). The OPERATIONAL-PLAYBOOK may gain notes about when escalations occur and how to respond if budget is exhausted. Essentially, ensure docs align with the implemented reality (previously some policy points were aspirational).

Testing adjustments: With new models and roles, some unit tests may need tweaking. E.g., tests expecting a single agent might still pass (since consensus off remains), but if we changed default confidence, tests for gate threshold need update (from expecting 0.65 to 0.75). We’ll update those assertions. We will also add tests for the new router behavior (ensure Implementer escalates, ensure Critic signals reduce confidence appropriately, etc.).

In summary, the main differences are: multi-model routing instead of one-model-fits-all, local model in active use, slightly stricter gating criteria, and the addition of automated self-critiques and budget checks. Most of these changes are configurations and policy toggles rather than extensive rewrites – with the exception of implementing dynamic routing logic, which is a moderate code change, and integrating the new local model which is an infra addition. The pipeline’s fundamental architecture (stages, single-owner concept, file artifacts) remains the same, so these upgrades augment rather than overhaul the system.

Upgrade Roadmap

We propose a phased rollout to incrementally apply these upgrades, allowing testing and iteration:

1. Minimal-Change Plan (Config & Docs only)

Objective: Achieve significant improvements (especially in model quality and cost control) with minimal code changes, mainly through configuration and environment toggles. This phase is low-risk and can be done immediately on the current codebase.

Deploy vLLM with local model: Set up the vLLM server hosting Qwen-3 30B. Test it independently with a few prompt completions. Then, in ~/.code/config.toml, configure the OpenAI provider to use the local endpoint

GitHub

. Also adjust the default model if needed (e.g., to “gpt-4” as a placeholder if Qwen is invoked via OpenAI compatibility). Essentially, ensure that when the CLI/TUI asks for the implementer’s response, it goes to Qwen via vLLM. No code change, but requires verifying the tokenization and response format match expectations (we might do a dry run of a simple /speckit.auto where Qwen writes a dummy function and see if parsing is okay).

Validation: Try a known simple spec and see that the Implementer response comes from Qwen (we can e.g. include a distinctive token in Qwen’s system prompt and see it in output). Ensure compile/test loop still runs. This should immediately cut API costs for implementer loops to $0. Monitor GPU memory – confirm Qwen fits in 32GB with vLLM (should use ~50% of 5090 VRAM in 4-bit).

Fallback plan: If vLLM integration has issues, we can temporarily point implementer to GPT-4 (existing) to not block the rollout. But assuming it works, we proceed with local.

Update cloud model endpoints and keys: In config, change model for Architect/Planner to “claude-sonnet-4.5” or corresponding Anthropic model, and for Judge to “gpt-5.1-high” (or GPT-4.5 if 5.1 not available yet). Also ensure the API keys for Anthropic and OpenAI are set (likely already done in .env

GitHub

). This switches the models used without code changes.

Validation: Run /speckit.plan --no-dry-run on a sample spec to force the planner to actually call the model (ensuring it hits Claude or GPT-5 as configured). Check the output JSON is well-formed. Because the prompt templates might have minor differences between models, watch for any formatting issues (e.g., extra assistant text outside JSON). Adjust prompt instructions in templates if needed (that can be done in the template files or minor code print tweaks).

Enable sidecar Critic via env: Set SPEC_KIT_SIDECAR_CRITIC=true in the environment (we can put it in config.toml under an env section or export it in our run script). This will cause the pipeline to spawn the Critic model at stages. By default, it will use the model as per DefaultRouter (Claude Haiku 4). We should verify that claude CLI command works or adjust to use the API: possibly the TUI is configured to call claude binary. In minimal change, we accept how it’s done (if claude CLI is in PATH and configured to call Anthropc API, that should be set up according to docs). If not available, we might toggle to use ChatGPT as critic (less ideal). Assuming we have Claude access, this is fine.

Validation: Trigger a plan with obvious flaw and see if a Critic artifact is produced and logged (the TUI should show it in consensus view if using TUI, or evidence file in CLI). Ensure that a Critic finding properly lowers the confidence or marks degraded=true in verdict if severe. We might craft a spec with a requirement and ensure Critic notes if plan missed it.

Confidence threshold config: In config.toml, add:

[quality_gates]

min_confidence_for_auto_apply = 0.75

This overrides the default 0.65.

Validation: Write a unit test or run a scenario where an agent returns confidence 0.7. Currently, with 0.65 threshold, it would auto-apply; with 0.75, it should escalate. Since we might not easily trigger this manually, we rely on unit tests or simulate by tweaking output confidence. But ensure the config is picked up (the PolicyToggles loader likely reads quality_gates.*). We can instrument a dry run with speckit review mode on a plan artifact to see the verdict.

Budget limit configuration: Define a budget in config, e.g.:

[cost]

monthly_limit_usd = 50.0

alert_threshold = 0.8

This doesn’t enforce by itself (since code not yet implemented), but it documents our intention and can be read by code. Possibly the PolicySnapshot includes cost toggles, but minimal phase might not implement code enforcement yet – that’s medium phase. However, we can at least include it so that if any part of code does check (maybe not yet, likely cost tracking is not wired for OpenAI calls), it’s there.

Documentation & training: Update internal docs (MODEL-POLICY.md v2) to reflect that local model is now part of the pipeline, and sidecars are on. This isn’t a code step, but crucial so team knows. For instance, state “Implementer now uses a local 30B model (Qwen) for first attempts, escalating to cloud after 2 failures” etc. Outline new env vars in OPERATIONAL-PLAYBOOK (like how to turn off critic if needed, etc.).

Monitor initial runs: Do a test run on a non-critical spec. Observe logs: verify that the local model doesn’t produce any unexpected output that breaks parsing (e.g. ensure it follows the expected JSON schema – might need to fine-tune the prompt or use few-shot from old GPT outputs). Check that the Critic outputs appear and do not stop the pipeline (unless intended). Ensure that overall time per spec is acceptable (should be roughly similar or slightly improved in speed except for maybe plan step if using Claude which is similar to GPT-4’s speed).

This minimal phase yields immediate cost savings (local model for most tokens) and some quality boost (better models for plan/judge, critic catching issues). Importantly, we have not yet changed code logic on retries or routing beyond what config can do – the implementer still always tries local in this config because we set prefer_local=true or such, but after 2 fails it wouldn’t automatically swap (that needs code). We accept that limitation in Phase 1, as the operator can manually set local_only=false and re-run implement if needed. Phase 2 will automate it.

2. Medium-Change Plan (Moderate Code Changes, High Impact)

Objective: Implement the dynamic behaviors and improvements that require touching the codebase, while still avoiding any radical architectural refactors. This phase will involve updates to the Rust code in router, executor, and Stage0, but with targeted, not sweeping, changes. Each change will be tested thoroughly.

Dynamic Router implementation: We introduce a new PolicyRouter (or enhance DefaultRouter) that follows the model routing table from MODEL-POLICY.md

GitHub

. Concretely, implement logic such as:

If role == Implementer and ctx.retry_count == 0 and ctx.is_high_risk == false -> choose local model (Qwen).

If role == Implementer and ctx.retry_count >= 2 -> choose cloud model (DeepSeek or GPT-5.1).

If role == Architect -> always cloud (Claude/GPT), fallback to DeepSeek if primary fails (perhaps we treat a very low confidence plan as “failure” and then call fallback within the same stage execution).

If role == Judge -> always cloud GPT/Claude (no local).

If role == Librarian -> local by default, but if ctx.is_high_risk or ctx.local_only == false and input size > X -> use Kimi (this one might be tricky to implement purely in router, might trigger via Librarian code).

Sidecars: if SPEC_KIT_SIDECAR_CRITIC on, then expected_agents_for_stage should include Critic role (which code already does in a test

GitHub

). Security sidecar if high risk. We ensure the router’s is_role_available respects the env toggles (we might add flags like enable_security_sidecar).

We can implement this by reading the config or environment within the router. Possibly, add a new struct RoutingPolicy loaded from MODEL-POLICY.yaml if exists. But to keep it moderate, we might hardcode the logic (with clearly marked points to change if policy updates). This code lives in Rust (spec-kit crate). We will also add unit tests for each scenario (simulate contexts and see WorkerSpec choice matches expectation).

Validation: We will test scenarios: Implementer with retry_count 0 yields local spec.id = “…:local:qwen” vs retry_count 2 yields spec.id = “…:openai:gpt-5-codex” or similar. We also simulate high_risk true for implementer to possibly choose cloud even on first try (as per policy: “cloud-first for cross-crate/unsafe/public API”)

GitHub

. How do we detect “high risk code change” in context? Possibly by number of files changed, presence of certain keywords. For now, we might approximate: if tasks > N or spec importance high, we can set StageContext.is_high_risk = true. This might require hooking into task analysis (which is not trivial in minimal code). Alternatively, we rely on tests themselves to escalate if failures persist. We might postpone complex risk detection logic. The easiest measurable: if >4 files to change (we have complex_task_files_threshold=4 in config)

GitHub

, mark it high risk. We implement that heuristic: the tasks stage outputs number of files or tasks; if >4, we set is_high_risk. This way, the router will pick cloud Implementer for a large refactor right away.

Once implemented, run pipeline on a scenario with >4 tasks and ensure it directly used cloud implementer, and another normal scenario uses local first then (if we artificially fail tests twice) uses cloud. This may require adding a controlled failure in code (e.g., make implementer produce a bug to test retry). We can simulate by instructing the model via prompt to do something wrong on purpose for test.

Automate Implementer retry escalation: With router in place, we ensure the executor calls into router for each retry. Possibly it already does (each attempt might create a StageContext anew, incrementing retry_count). If not, we adjust the loop so that after a stage fails (tests fail in Validate), when re-running Implement, it passes retry_count+1 and notifies router. The router then selects the new model. We also add logging: e.g., “Implementer escalated to GPT-5 due to 2 failed attempts”. This code likely lives in SpeckitExecutor.advance_spec_auto or in how we handle a Stage verdict.

Validation: Simulate or run a case where code doesn’t compile after two tries. Perhaps create a spec intentionally hard (like requires knowledge beyond local model’s training). Watch logs or results to see if third attempt used cloud (we might print the model id being used each attempt for clarity). This ensures our loop works. Also confirm that the pipeline doesn’t exceed 3 attempts by default – maybe config says max 3 tries.

Self-critique integration: There are two approaches:

Within the Architect’s chain: After producing plan, before finalizing it, ask the same model to “reflect” and then possibly revise. This could be done by modifying the prompt template to include a section like: “First, list potential issues… then provide final plan.” However, controlling that might be messy.

Easier: after plan is done (artifact saved), immediately call the Critic sidecar on it (we already do if enabled). Instead of just logging the critique, feed it back to Architect model as a new prompt to refine plan. But since the plan stage is already concluded in one shot, doing a second round would require either looping the Plan stage or treating Critic output as advisory only.

Given we want minimal code complexity, we might not fully implement a loop here. Instead, we settle for enabling the Critic (which we did) and document that the Architect should review Critic feedback in Audit stage. Alternatively, implement a “Clarify” stage between Plan and Tasks: this new stage (could be using the Tutor role or just reusing Architect model) would take the plan and ask “Are there any unclear requirements or risks?” The output could be a short markdown listing Q&As. Then proceed to Tasks. We can implement this as an optional stage in the executor (if enabled via config). It’s moderate complexity but doable: essentially call the Architect model with a different prompt and store the result as clarifications.md.

For medium phase, perhaps we opt for a simpler route: use the Critic as implemented, but adjust its prompt to be more Socratic (e.g., “Pose questions the plan should consider”). And ensure its output is included for the human to see (in /speckit.status or evidence). This way we at least capture maieutic output. Full integration (model revises plan automatically) could be a long-term item.

Validation: After these changes, run a spec with an intentionally vague requirement. See that Critic outputs questions. We won’t get automatic answers, but the operator/human can catch them. We could optionally log a message: “Critic sidecar identified potential issues; manual clarification may be needed or escalate to Tutor.” This alerts the user.

Extend BudgetTracker for API calls: Now the code changes: for each OpenAI/Anthropic API call, after getting usage (tokens used), add to a cumulative counter (perhaps store in memory or a static). BudgetTracker currently tracks NotebookLM queries by count, not dollars. We can either extend it or create a new CostTracker. Simpler: piggyback on BudgetTracker by interpreting “usage.total” as sum of prompt calls, and treat daily_limit as number of calls allowed. But better to incorporate actual cost. Possibly we maintain two trackers: one for queries (like NotebookLM, keep as is) and one for token usage. We know costs per model (we have them in model_presets with cost fields

GitHub

GitHub

). We can fill those in config (e.g., models.openai.cost_per_input_million = 1.25 etc.). Then on each call, do: cost = (input_tokens/1e6 * cost_in + output_tokens/1e6 * cost_out). Accumulate daily and monthly in memory (reset daily like BudgetTracker does at midnight). If cost > 0.8 * monthly_limit and not alerted yet, log a WARN. If > limit, throw an error that propagates to a controlled pipeline stop (maybe mark stage as error requiring user attention). We’ll implement storing this maybe in a simple JSON in ~/.code/cost_usage.json analogous to usage.json. This is moderate code but straightforward.

Validation: Simulate heavy usage: e.g., pretend each call is 100k tokens by multiplying for test, and see if limit triggers. Or set a low monthly_limit in config (like $0.01) and run one model call, verify it blocks subsequent. This ensures logic works. Also, test that setting SPEC_OPS_ALLOW_BUDGET_OVERFLOW=1 (an override we propose) will bypass blocking (just log) – implement that if needed (just an env check in the code that prevents error throw).

Stage0 embedding & memory update: Swap out the embedding model call in local-memory service to BGE. This might require updating that Node service’s code or its config (not in Rust code, but perhaps in a separate repo or package @theturtlecsz/local-memory-mcp). If accessible, do it; if not, as a workaround we could use the ACE “complex memory integration” skill or incorporate an embedding directly in Rust using HF libraries. Possibly easiest: use Python or Node to generate embeddings offline. However, since minimal changes are allowed here, we likely stick to updating the memory service configuration (maybe it reads a MODEL env var).

We’ll also flush and re-index existing memory. That might be manual: export all entries, embed with new model, re-import. Provide a script for that.

Validation: After deploying new embeddings, run Stage0 on an older spec where we know relevant memories exist. Check that relevant ones are still retrieved (should improve, not degrade). Also verify that the overlay dynamic score logic still works (it doesn’t depend on embedding dimension etc.). If needed, recalibrate weights (the weight of semantic similarity vs dynamic score might be tuned via config if present

GitHub

). We might run Stage0 explain mode to see the components of score and adjust if semantic similarity is under-weighted.

Testing and fine-tuning: At this stage, run the pipeline on several real specs from history (ones we know outcomes for) to gauge performance. Look at: plan quality (does it miss less?), implementer success (fewer retries?), any new failure modes (e.g., local model misunderstanding an instruction), output formatting (especially JSON compliance). Collect metrics: average tokens used in cloud vs before (should drop), time per stage (should be similar). If any stage consistently slower or worse, investigate. For instance, if local model outputs code that doesn’t compile more often than GPT-4 did, we may consider using GPT-4 as implementer for some types of tasks – but give Qwen some fine-tuning or adjust temperature first. We can adjust local model’s decoding settings easily (vLLM allows setting temperature, we can expose models.local.temperature config). E.g., use temp 0.2 for deterministic codegen. Document that in config (there was an entry for model temperature in config schema

GitHub

).

Team training: Brief the team that now the pipeline might escalate implementer automatically. This is important because previously a dev might see it retry with same model thrice; now the third try might be by GPT-5 which could produce a much larger diff or different style. Ensure they know to look at evidence of which model contributed what (the evidence JSON usually logs agent name). We might also add a note in commit message or diff description “(Escalated to GPT-5 for final attempt)”. Not crucial but helps clarity.

This medium phase completes the core functionality upgrades. At this point, the system fully implements the intended 2026 policy: local reflex first, cloud when needed, critic feedback integrated, etc. We expect quality of outputs to be noticeably higher (fewer overlooked requirements, fewer failed pipelines due to stuck implementer), and efficiency to improve (less token waste on repeated mistakes, cost savings from local compute).

3. Longer-Term Plan (Major Enhancements, Optional)

Objective: Address deeper architectural or workflow improvements that are not immediately needed but could yield substantial benefits. These may require more extensive development and testing, and are optional in scope – to be done once the above phases are stable.

Multi-turn maieutic Tutor role: Introduce an interactive “Tutor” agent in the loop for complex tasks. For example, if after two implementer attempts the code is still failing in a tricky way, instead of immediately escalating to cloud implementer, we pause and invoke a Tutor (which could be the same local model or a different one with a coaching persona) to analyze the situation. The Tutor would generate a step-by-step reasoning or ask questions that help illuminate the issue (this is effectively forcing a maieutic dialogue internally). Then feed that analysis either to the original implementer model or directly have Tutor propose the fix. This is a more advanced multi-agent pattern (two models collaborating), which our policy normally avoids, but if done in a controlled way (the Tutor only provides analysis, not code), it could improve outcomes. Implementing this would involve a new stage or a sub-loop within Implement: detect stuck situation → call Tutor model with the context of spec, plan, code, error → get some insights → append those as additional context to the next Implementer attempt.

Benefit: Might solve problems that a single-shot approach misses. Cost: additional complexity and API calls.

We would experiment with this offline first to see if it significantly improves success on tough cases. If yes, integrate as an optional path when retry_count==1 and tests still failing (before going to cloud model, try a Tutor analysis). The Tutor could be the local model itself (maybe at a higher temperature to brainstorm) or a smaller model specialized in Q&A (maybe something like Socratic models fine-tuned for debugging).

This would require new code to manage storing Tutor’s output and merging it into Implementer’s prompt. Possibly add a field in StageContext like insight that gets appended to context.

Full integration of Stage0 engine: Currently, Stage0 is somewhat bolted on (calls out to external service). We could internalize that logic in Rust for efficiency. For instance, replace the Node NotebookLM service with direct API calls to Google’s service (if available) or swap to an open-source alternative (maybe by 2026 there’s an offline “NotebooksGPT” we can host). Also, store the memory overlay in a unified database rather than a mix of local-memory service and spec-kit overlay. Essentially simplify architecture: one process does retrieval (using a Rust wrapper to Qdrant), does summarization (we have local model for that now), and calls either Google or OpenAI for final answer. This removes the dependency on the Node server and possibly the separate SQLite for overlay (could unify with main evidence DB).

This refactor would reduce maintenance and latency a bit (no JSON over HTTP overhead internally). But it requires porting logic (the formulas, etc. given in docs we have – doable).

We would schedule this once current Stage0 is stable and we have time to optimize. It should not change functionality, just performance and simplicity.

Also, if NotebookLM’s API improves (maybe direct embedding of PDFs, etc.), integrate those enhancements in Stage0 pipeline.

Unified telemetry and analytics: Build a dashboard or log aggregator that tracks each spec run metrics: total tokens consumed (local vs cloud), time taken per stage, number of retries, etc. This helps identify bottlenecks and regressions. We partly have telemetry (evidence JSON captures lots of data), but a more accessible summary would be nice. For example, add a final log line: “Spec KIT-123: Cloud tokens=8k ($0.10), Local tokens=20k (0$), Time=2min, Outcome=merged with 1 retry”. This can be sent to a monitoring service or just saved. Over time, this data can justify the cost trade-offs and pinpoint if, say, one stage is disproportionately costly.

Implementation might be a separate tool reading logs, so not core pipeline change. But we mention as a long-term improvement to aid continuous improvement.

Expand test coverage and CI gating: As we trust the pipeline more, we might want to let it run on every PR (dogfooding). That means integrating with CI systems (GitHub Actions or similar) to run speckit.auto in a dry-run mode on PRs and maybe even auto-commit small fixes. We’d need to ensure deterministic outputs or handle variability. Possibly lock certain random seeds (if any) or always run local for CI to avoid external nondeterminism. The upgrade here is not about model but about process: establishing a pipeline where our AI’s suggestions themselves undergo a quality check by running them on test branches. This solidifies the AI-human collaboration.

This is more of an ops improvement. On the code side, might add a --ci flag that runs in a stricter mode (no cloud calls unless explicitly allowed via env, for example, to keep CI costs predictable). Or use the local_only=true for CI runs to avoid external dependency (this requires our local model to be good enough to produce at least something compileable). In long term, if local model can handle 80% of cases, having CI runs with it is viable.

Tool integration (for performance analysis): For performance improvements, we might incorporate actual performance profiling tools into PerformanceReviewer sidecar (if enabled). E.g., run the code with perf or track big-O complexity if possible, feeding results to LLM for analysis. This is a complex but interesting direction, enabling the AI to make suggestions not just functionally but performance-wise. That said, it’s very speculative and likely beyond scope for now.

Periodic model updates: Plan for how to update local model (e.g., Qwen 3 to Qwen 4 if released) and others. Possibly design the system to load model names from a config file or environment so we don’t hardcode “30B” anywhere. Already our config can handle model changes easily. We just note that as new versions come (like a Qwen4 20B that has MoE efficiency, or GPT-5.2, etc.), we’ll evaluate and swap with minimal friction. This is more of a maintenance practice than code change – but ensuring the pipeline is modular enough to accept a new model ID without breaking is the key (our work in medium phase on dynamic router and config aims at that).

Risk mitigation in long-term changes: Each long-term idea should be prototyped in isolation. For example, the Tutor role: we could run a few internal experiments where an engineer uses the Tutor manually to solve a bug and measure improvement. Only if clearly beneficial we invest in coding it in. Similarly, Stage0 integration can be done after everything else is stable, so we can measure performance gains. We will avoid deploying any long-term change that isn’t well-tested, as these can add complexity.

Each phase above should be completed and stabilized before moving to the next. Phase 1 (Minimal) should deliver quick wins in week 1; Phase 2 (Medium) probably over 2-3 weeks including testing; Phase 3 (Long-term) as ongoing R&D over a longer horizon (1-3 months, as these are enhancements). By following this roadmap, we ensure the pipeline incrementally gets to the target state without disrupting current usage or overshooting the budget/policy constraints.

Risks & Unknowns

Even with careful planning, there are some uncertainties and potential risks to monitor:

Local model output quality: There is a risk that the chosen local model (Qwen-3 30B MoE) might not produce code as clean or correct as GPT-4/5 did. While Qwen-30B is state-of-the-art open, it may have quirks: e.g., it might not follow some instructions (like test-driven development cues) as strictly, or its code style might differ. If quality falls short, developers might see more fixes needed or style inconsistencies. Mitigation: we can fine-tune or few-shot the local model on our repository’s style. Also, we will initially use it for smaller tasks (where it excels) and let big tasks go to cloud model to maintain quality. Continuous evaluation on a set of sample tasks will guide if we need to adjust (like raising threshold to escalate sooner if local fails).

Parsing and format compliance: With new models, there’s a chance an output doesn’t strictly conform to expected JSON or markdown schema (especially if prompt templates aren’t tuned for them). A plan or audit JSON might come back with extra commentary or slight deviations. Mitigation: test each model’s output format on a sample prompt, adjust the few-shot examples or system prompts accordingly. For example, ensure GPT-5.1 is called with functions schema definitions if possible, to guarantee JSON structure. The pipeline’s validate_required_fields function will catch missing fields

GitHub

, but not structural errors. If we notice any, we might implement a post-processing step or switch to a more reliable model for that stage.

Performance and memory usage: Running a 30B model on a single GPU with 256k context is memory heavy. While AWQ 4-bit compresses model weights, the KV cache for 256k tokens can be huge (each token vector ~30k dims for 30B model * 256k tokens can exceed GPU memory if not managed). vLLM’s paged attention offloads old cache to CPU, but we should be careful not to OOM on very long inputs. Mitigation: We can set a lower max_model_len if needed (e.g., cap at 100k to be safe) or ensure not too much context is given to local model at once (in Librarian tasks, if need more, use Kimi). We will monitor GPU memory during heavy runs. Also, enabling sidecars means multiple models loaded (though likely not concurrently huge). Claude Instant is via API (no GPU load on us). But if we consider any local sidecar (we didn’t except Librarian which is same model), watch out for CPU high usage. Overcommit of GPU VRAM or CPU RAM could slow pipeline or crash. We should do stress tests (simulate many specs in parallel perhaps) to ensure system stability.

API reliability and latency: Relying on multiple cloud providers (OpenAI, Anthropic, Google) means higher chance one of them experiences an outage or slow response. This could stall our pipeline. Mitigation: We have fallback paths (e.g., if NotebookLM fails, use GPT; if Anthropic is down for plan, maybe use GPT as alternate). We should implement timeouts on all calls (likely already present: e.g., NotebookLM ask has 120s timeout

GitHub

, OpenAI via their client usually has timeouts). Also, budget gating: hitting budget limit is another “outage” we impose. If that triggers, we need a plan: either gracefully stop and ask for user override, or switch to local-only mode (we could implement that: if budget exceeded, set ctx.local_only=true so pipeline tries to do best locally). That could lead to lower quality outputs, but at least it completes.

Model and service drift: By 2026, models like GPT-5.1 are fairly new. Their behaviors might change slightly (OpenAI might fine-tune them over time). Our pipeline prompts might need adjustments as models evolve. Similarly, NotebookLM is a beta product – its interface could change or it might introduce new limitations. Mitigation: Keep an eye on model change logs (OpenAI usually version their models; we can pin to a version if needed). Have tests for critical prompt outputs to catch if a model update breaks format. For NotebookLM, maintain contact with that API’s updates; if it discontinues, be ready to swap to another grounded QA system (maybe we’d use an LLM + vector search ourselves as fallback).

Security and compliance: Running code suggested by an AI in an automated way carries risk – what if the AI suggests a dangerous operation? The pipeline runs in a sandbox mode (workspace-write, no network by default)

GitHub

, which is good. We should maintain those restrictions (e.g., ensure can_network=false for implementer to not allow any network calls in code execution

GitHub

). Also, the SecurityReviewer should catch obvious issues, but something subtle might slip. Ultimately, a human should glance at final output for anything catastrophic. Over time, trust can increase, but in near term we likely still have human oversight on merges. So the risk is mitigated by the multi-layer checks plus human at least doing a quick scan or having the option to intervene at Unlock stage.

Over-reliance on cloud if not managed: If our local model underperforms and the pipeline frequently escalates to cloud (or if sidecars cause frequent escalations by flagging everything high risk), we could end up spending more than expected. The budget enforcement will help prevent runaway cost, but if hitting limits often, it means the local model or policy thresholds might need adjusting. Mitigation: Fine-tune local model on our data to reduce escalations. Adjust thresholds (maybe allow medium confidence auto-apply for low-impact stages). Also monitor usage stats: if we see, e.g., 90% of Implementer tasks escalate, then our assumption was wrong – either raise local model capacity (maybe try a bigger local model if possible like a 60B on disk with streaming, though 5090 might not handle that) or adjust when we escalate. The goal is to have local handle most small tasks and cloud reserved for big ones – we need to verify this balance in practice and be ready to tweak the policy mapping (which we now can via config easily).

Team adaptation: Developers need to get used to reading Critic outputs and understanding when the AI has escalated. There’s a slight learning curve (“the AI might rewrite a lot of code if it escalates to GPT-5 – be prepared for big diffs”). If not communicated, a dev might be surprised that the third attempt was so different. Mitigation: Provide clear logging in output artifacts: perhaps annotate the implementer’s artifact or commit message with the model name used. E.g., in the diff file header, add a comment // Implemented by Qwen (local) or (by GPT-5.1 cloud) so it’s transparent. This is a minor code addition but improves clarity. Also, initial runs can be supervised by an engineer to ensure the team trusts the changes.

Edge cases in pipeline logic: The introduction of new steps or differences in output might reveal edge cases – e.g., perhaps the tasks stage is unnecessary if the plan now directly includes tasks (some models might merge plan and tasks inadvertently, though our required_fields check should prevent missing separate tasks file). We should double-check that multi-file outputs are handled (if a model tries to propose code changes to multiple files at once, does our implementer logic capture all? Likely yes, as it applies unified diff). Another example: if Critic finds something truly blocking at Plan stage, currently pipeline would escalate to a better planner – is that implemented? Possibly not fully. We might need to implement a mechanism: if verdict.consensus_ok=false at AfterPlan checkpoint due to Critic block, we should trigger either a Plan redo with a stronger model or fail the pipeline. Right now, consensus_ok just returns false but pipeline might continue with plan regardless if not coded otherwise. Mitigation: We should implement that: e.g., in gate evaluation, if consensus_ok (gate passed) is false at AfterPlan, we stop and mark plan for human review or attempt an automatic escalation (maybe call DeepSeek to generate an alternative plan). This is a code path we likely add. It's a risk in the sense of complexity – we should test a scenario where Critic raises a critical issue in plan and see how our pipeline behaves.

Regulatory / data compliance: Minor point, but using multiple cloud APIs means data is sent to them. We said privacy not a constraint, but if that changes (say we have to avoid sending certain code to cloud), we might need a toggle to force local only. Our design supports that (set local_only in config, which router respects, forcing local for all roles)

GitHub

. That’s an emergency switch if needed. Just noting that risk if the company policy changes – we can accommodate by essentially dropping back to a simpler mode (with potential quality hit).

Opportunity risk (not using best model): By recommending certain models (Claude, GPT-5.1) we assume they are top. If a new model (say “Gemini 3” full release) comes out and is clearly better, or a new open model allows local processing of larger tasks, we might be behind. We need to keep an eye out and be ready to pivot. The architecture we set – with config-driven selection – allows swapping models easily. So the risk is low as long as we remain vigilant.

In summary, most risks are manageable through configuration, testing, and iterative tuning. The safety nets (tests, guardrails, human oversight) ensure that if the AI does something unexpected, it won’t directly harm production – worst case, a spec doesn’t auto-merge and requires human intervention (which is an acceptable fallback). Unknowns mainly revolve around how well the local model will perform relative to expectations and how smoothly the multi-model interplay will work. We will address those by extensive testing in Phase 2 and adjusting thresholds and model choices as needed (for instance, if Qwen-30B disappoints, we might try an alternative local like CodeLlama-34B or consider running DeepSeek locally if it’s open). The modular design and monitoring will be key in turning these unknowns into lessons that we feed back into the system (e.g., updating prompts or switching out a model in config is much easier now than it was with everything static).

Finally, we will benchmark key metrics post-upgrade and compare to baseline: success rate of speckit.auto (how often it produces a mergeable result without human fix), average runtime, and token costs. Our goal is an 80% automated success rate on typical specs with < $1 average cost, and those are the numbers we’ll track after deployment, ready to roll back or tune if those targets aren’t met.

Appendix: Sources and Benchmarks (accessed Jan 2026)

Anthropic Claude 4.5 model tiers (Opus, Sonnet, Haiku) and pricing – Model presets from repository

GitHub

GitHub

. Shows Claude Opus 4.5 as premium quality (cost $15/$75 per 1k tokens in/out), Claude Sonnet 4.5 as balanced ($3/$15 per 1k) and Claude Haiku 4.5 as fast/cost-efficient ($1/$5 per 1k). These are used to inform role mapping and cost calculations.

OpenAI GPT-5.1 family and costs – Model presets from repository

GitHub

GitHub

. Confirms GPT-5.1 series pricing (~$1.25 per 1k input tokens, $10 per 1k output for base models, Codex variants same order) and context. Also highlights the reasoning effort levels (Minimal/Low/Medium/High) – we selected High for Judge for maximal reasoning.

LMArena Leaderboard (Dec 2025) – Snapshot of model rankings

lmarena.ai

. Shows Gemini-3 Pro at Elo ~1490 (rank #1 in Text arena), with Claude and GPT-5.1 also high. Claude Opus “thinking” mode slightly leads GPT-5.1 in WebDev (code tasks)

lmarena.ai

(Claude had 1512 vs GPT-5.2 1480). This justified using Claude for Architect and GPT-5.1 for Judge (mixing strengths) and considering Gemini as alternate.

DeepSeek and Kimi model info – Public descriptions of DeepSeek R1 as a self-reflection model

linkedin.com

and Kimi K2 as a massive MoE (32B active, 1T total) with agentic capabilities

huggingface.co

. Also Kimi pricing from a forum (Moonshot AI)

reddit.com

($0.60 per 1M input tokens). These support our use of DeepSeek for reasoning-intense coding and Kimi for rare long-context needs, with known cost structure.

Qwen-3 Coder MoE recommendation – Internal requirements doc updated Jan 2026

GitHub

GitHub

. It explicitly recommends Qwen3-Coder-30B-A3B (Mixture of Experts) as the single local model on one 5090, noting ~12–16GB VRAM usage with 4-bit quantization and huge context. Also provides expected latency: <100ms for short prompts

GitHub

. This strongly influenced our choice of local model and confidence that it can handle Librarian and reflex tasks efficiently.

vLLM vs alternatives – vLLM blog / comparisons: vLLM’s OpenAI API mode and high concurrency throughput

kanerika.com

, and SGLang’s design (RadixAttention, etc.) as open-source inference standard

github.com

github.com

. These indicate vLLM’s practicality for us and highlight SGLang as an advanced but more complex option.

Embeddings – BGE-M3 documentation

bge-model.com

describing its multi-function (dense & sparse) capabilities, suitable for code and text, and multi-linguality. Also mentions multi-vector support and 1024 dimensions

ollama.com

inference.readthedocs.io

. This gave us confidence to adopt BGE-M3 to improve memory recall.

Repository code references: The gate_policy.rs and others were used extensively to ensure alignment with implemented enums and logic (e.g., roles, sidecar toggles, default thresholds)

GitHub

GitHub

. For example, we cited the code showing sidecars available

GitHub

and tests confirming single-agent default

GitHub

to confirm current behavior. The config example and ADRs were cited to verify how multi-agent was to be configured

GitHub

GitHub

(ensuring our changes fit into that system).

All these sources were accessed and verified in January 2026. They collectively underpin our model choices, cost estimates, and performance expectations in the recommendations above.

Sources