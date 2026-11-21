use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use flowrail_core::artifact::FlowrailArtifact;
use flowrail_core::frecency::FrecencyStore;
use flowrail_core::git::GitStatusResolver;
use flowrail_core::grep::CorridorGrep;
use flowrail_core::pruning::PruningConfig;
use flowrail_core::search::FuzzySearchEngine;
use flowrail_core::slicer::{CorridorSlicer, SliceIntent};
use flowrail_mcp::server::McpStdioServer;
use flowrail_mcp::tools::CorridorMcpEngine;
use flowrail_ts_bridge::scip_loader::ScipGraphBuilder;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "flowrail",
    version,
    about = "Flowrail (Project Ariadne): Deterministic Execution & Mutation State-Machine for AI Coding Agents"
)]
struct Cli {
    #[arg(short, long, default_value = ".", global = true)]
    workspace: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs scip-typescript on the target repository to generate .flowrail/index.scip
    Index,
    /// Fast typo-tolerant fuzzy search across symbols and entrypoints with Git and frecency annotations
    Search {
        query: String,
        #[arg(long, default_value_t = false)]
        git_only: bool,
        #[arg(long, default_value_t = false)]
        definitions_only: bool,
        #[arg(long, default_value_t = 20)]
        max_results: usize,
    },
    /// Fast in-memory content grep with definition-first filtering
    Grep {
        query: String,
        #[arg(long, default_value_t = false)]
        definitions_only: bool,
        #[arg(long, default_value_t = 30)]
        max_matches: usize,
        #[arg(long, default_value_t = 1)]
        context: usize,
    },
    /// Deterministically slices a corridor from an Entry Gate and prints JSON plan
    Slice {
        #[arg(short, long)]
        entry: String,
        #[arg(short, long, default_value = "READ_FLOW")]
        intent: String,
        #[arg(short, long)]
        target: Option<String>,
        #[arg(long)]
        max_depth: Option<usize>,
        #[arg(long, default_value_t = false)]
        artifact: bool,
    },
    /// Inspect, generate, and validate Clean/Clear JSON artifacts
    Artifact {
        #[command(subcommand)]
        action: ArtifactCommands,
    },
    /// Starts the Stateful MCP Server over stdio for Claude Code / Cursor / Windsurf
    Mcp {
        #[arg(long, default_value_t = false)]
        full_typecheck: bool,
    },
    /// Inspects and verifies workspace readiness, extracted symbols, and graph statistics
    Doctor,
    /// Initializes Flowrail configuration and generates MCP server setups for Claude, Codex, OpenCode, Cursor, Windsurf, Zed, and Antigravity
    Init {
        #[arg(long, default_value = "all")]
        target: String,
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Runs a self-contained live demonstration on sample TypeScript fixtures
    Demo,
}

#[derive(Subcommand)]
enum ArtifactCommands {
    /// Shows the latest or specified artifact JSON
    Show {
        #[arg(long)]
        id: Option<String>,
        #[arg(long, default_value_t = false)]
        raw: bool,
    },
    /// Lists all generated artifacts in .flowrail/artifacts/
    List,
    /// Slices a corridor and exports a Clean/Clear JSON artifact
    Generate {
        #[arg(short, long)]
        entry: String,
        #[arg(short, long, default_value = "READ_FLOW")]
        intent: String,
        #[arg(short, long)]
        target: Option<String>,
        #[arg(long)]
        max_depth: Option<usize>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Validates an artifact and its AI enrichment against ADHD & Anti-Slop gates
    Validate {
        #[arg(short, long)]
        file: PathBuf,
    },
}

fn load_graph(workspace: &std::path::Path) -> Result<flowrail_core::graph::CodebaseGraph> {
    if workspace == std::path::Path::new("/") {
        eprintln!("[flowrail] Workspace resolved to root ('/'). Skipping recursive AST scan.");
        return Ok(flowrail_core::graph::CodebaseGraph::new());
    }

    let scip_candidates = [
        workspace.join(".flowrail/index.scip"),
        workspace.join(".corridor/index.scip"),
        workspace.join("index.scip"),
    ];

    for scip_path in &scip_candidates {
        if scip_path.exists() {
            return ScipGraphBuilder::build_from_scip_file(workspace, scip_path);
        }
    }

    // Fast local AST fallback
    ScipGraphBuilder::build_from_directory(workspace)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let raw_workspace = cli.workspace.clone();
    let mut workspace = cli.workspace.canonicalize().unwrap_or(cli.workspace);

    if raw_workspace == std::path::Path::new(".")
        && let Ok(env_ws) = std::env::var("FLOWRAIL_WORKSPACE")
    {
        let trimmed = env_ws.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            workspace = p.canonicalize().unwrap_or(p);
        }
    }

    match cli.command {
        Commands::Index => {
            let out_dir = workspace.join(".flowrail");
            std::fs::create_dir_all(&out_dir)?;
            let scip_path = out_dir.join("index.scip");

            let status = Command::new("npx")
                .args([
                    "@sourcegraph/scip-typescript",
                    "index",
                    "--output",
                    scip_path.to_str().unwrap(),
                ])
                .current_dir(&workspace)
                .status()
                .context("Failed to execute npx @sourcegraph/scip-typescript")?;

            if !status.success() {
                anyhow::bail!("scip-typescript indexing failed");
            }
            println!(
                "Indexed TypeScript semantic graph at {}",
                scip_path.display()
            );
        }
        Commands::Search {
            query,
            git_only,
            definitions_only,
            max_results,
        } => {
            let graph = load_graph(&workspace)?;
            let git_resolver = GitStatusResolver::new(workspace.clone());
            let frecency = FrecencyStore::new();
            let engine = FuzzySearchEngine::new(&graph, Some(&git_resolver), Some(&frecency));

            let hits = engine.search(&query, definitions_only, git_only, max_results);
            let pagerank = graph.compute_pagerank(0.85, 20);

            println!("Found {} symbols matching '{}':", hits.len(), query);
            for h in hits {
                let pr = pagerank.get(&h.symbol_id).copied().unwrap_or(0.0);
                let score = h.score + (pr * 50.0);
                println!(
                    "  [{:?}] {:45} {:30} {} (score: {:.1}, pagerank: {:.4})",
                    h.role,
                    h.display_name,
                    format!("{}:{}-{}", h.file_path, h.line_range.0, h.line_range.1),
                    h.git_status.badge(),
                    score,
                    pr
                );
            }
        }
        Commands::Grep {
            query,
            definitions_only,
            max_matches,
            context,
        } => {
            let mut files = Vec::new();
            ScipGraphBuilder::collect_ts_files(&workspace, &mut files)?;
            let git_resolver = GitStatusResolver::new(workspace.clone());

            let report = CorridorGrep::search(
                &workspace,
                &files,
                &query,
                definitions_only,
                None,
                context,
                max_matches,
                Some(&git_resolver),
            )?;

            println!("{}", CorridorGrep::format_report_compact(&report));
        }
        Commands::Slice {
            entry,
            intent,
            target,
            max_depth,
            artifact,
        } => {
            let graph = load_graph(&workspace)?;
            let slice_intent = if intent.eq_ignore_ascii_case("MUTATE_CONTRACT")
                || intent.eq_ignore_ascii_case("mutate")
                || intent.eq_ignore_ascii_case("mutatecontract")
            {
                SliceIntent::MutateContract
            } else {
                SliceIntent::ReadFlow
            };
            let mut pruning_config = PruningConfig::default();
            if let Some(md) = max_depth {
                pruning_config.max_corridor_depth = md.clamp(1, 100);
            }
            let plan = CorridorSlicer::build_corridor(
                &graph,
                &entry,
                slice_intent,
                target.as_deref(),
                &pruning_config,
            )?;

            let git_resolver = GitStatusResolver::new(workspace.clone());
            let art = FlowrailArtifact::from_plan(&plan, &workspace, Some(&git_resolver));
            let _ = art.save_to_workspace(&workspace);

            if artifact {
                println!("{}", art.to_clean_json()?);
            } else {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
        }
        Commands::Artifact { action } => match action {
            ArtifactCommands::Show { id, raw: _ } => {
                let artifacts_dir = workspace.join(".flowrail/artifacts");
                let file_path = if let Some(art_id) = id {
                    let direct = artifacts_dir.join(format!("{}.json", art_id));
                    if direct.exists() {
                        direct
                    } else {
                        artifacts_dir.join(art_id)
                    }
                } else {
                    artifacts_dir.join("latest.json")
                };

                if !file_path.exists() {
                    anyhow::bail!(
                        "Artifact not found at {}. Run `flowrail slice --artifact` or `flowrail doctor` first.",
                        file_path.display()
                    );
                }

                let content = std::fs::read_to_string(&file_path)?;
                println!("{}", content);
            }
            ArtifactCommands::List => {
                let artifacts_dir = workspace.join(".flowrail/artifacts");
                if !artifacts_dir.exists() {
                    println!(
                        "No artifacts directory found at {}",
                        artifacts_dir.display()
                    );
                    return Ok(());
                }

                println!("=== Flowrail Generated Artifacts ===");
                println!("Location: {}\n", artifacts_dir.display());

                let mut entries: Vec<_> = std::fs::read_dir(&artifacts_dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
                    .collect();
                entries.sort_by_key(|e| e.file_name());

                for entry in entries {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if let Ok(art) = std::fs::read_to_string(entry.path()).and_then(|c| {
                        serde_json::from_str::<FlowrailArtifact>(&c)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    }) {
                        println!(
                            "  • {:24} [{:?}] {} (Gates: {}, Contagion: {})",
                            file_name,
                            art.kind,
                            art.formatted_utc,
                            art.deterministic.total_gates,
                            art.deterministic.blast_summary.contagion_index
                        );
                        println!(
                            "    -> Headline: {}",
                            art.ai_enrichment.adhd_view.headline_action
                        );
                        println!(
                            "    -> Next Action: {}\n",
                            art.ai_enrichment.adhd_view.terminal_action
                        );
                        continue;
                    }
                    println!("  • {}", file_name);
                }
            }
            ArtifactCommands::Generate {
                entry,
                intent,
                target,
                max_depth,
                output,
            } => {
                let graph = load_graph(&workspace)?;
                let slice_intent = if intent.eq_ignore_ascii_case("MUTATE_CONTRACT")
                    || intent.eq_ignore_ascii_case("mutate")
                    || intent.eq_ignore_ascii_case("mutatecontract")
                {
                    SliceIntent::MutateContract
                } else {
                    SliceIntent::ReadFlow
                };
                let mut pruning_config = PruningConfig::default();
                if let Some(md) = max_depth {
                    pruning_config.max_corridor_depth = md.clamp(1, 100);
                }
                let plan = CorridorSlicer::build_corridor(
                    &graph,
                    &entry,
                    slice_intent,
                    target.as_deref(),
                    &pruning_config,
                )?;

                let git_resolver = GitStatusResolver::new(workspace.clone());
                let artifact = FlowrailArtifact::from_plan(&plan, &workspace, Some(&git_resolver));

                if let Some(out_path) = output {
                    if let Some(parent) = out_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&out_path, artifact.to_clean_json()?)?;
                    println!("Artifact saved to {}", out_path.display());
                } else {
                    let saved_p = artifact.save_to_workspace(&workspace)?;
                    println!("Artifact saved to {}", saved_p.display());
                }

                println!("{}", artifact.to_clean_json()?);
            }
            ArtifactCommands::Validate { file } => {
                if !file.exists() {
                    anyhow::bail!("Artifact file not found at {}", file.display());
                }

                let content = std::fs::read_to_string(&file)?;
                let mut artifact: FlowrailArtifact = serde_json::from_str(&content)
                    .context("Failed to parse file into FlowrailArtifact schema")?;

                println!("Validating artifact '{}'...", artifact.artifact_id);
                println!("  • Schema Version: {}", artifact.schema_version);
                println!("  • Kind: {:?}", artifact.kind);
                println!("  • Gates: {}", artifact.deterministic.total_gates);
                println!(
                    "  • Contagion Index: {}",
                    artifact.deterministic.blast_summary.contagion_index
                );

                let enrichment = artifact.ai_enrichment.clone();
                match artifact.apply_and_validate_enrichment(enrichment) {
                    Ok(()) => {
                        println!("\n[OK] Validation PASSED!");
                        println!(
                            "  - Zero Hallucination: All cited symbols & files verified against deterministic graph."
                        );
                        println!(
                            "  - ADHD Constraints: Headline action, terminal action, <=5 bounded steps verified."
                        );
                        println!(
                            "  - Anti-Slop Discipline: Zero conversational fluff detected. Direct payload verified."
                        );
                    }
                    Err(err) => {
                        println!("\n[FAIL] Validation REJECTED: {}", err);
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::Mcp { full_typecheck } => {
            let graph = load_graph(&workspace)?;
            let engine = CorridorMcpEngine::new(workspace, graph, full_typecheck);
            let mut server = McpStdioServer::new(engine);
            server.run_stdio_loop()?;
        }
        Commands::Doctor => {
            let graph = load_graph(&workspace)?;
            let git_resolver = GitStatusResolver::new(workspace.clone());
            let statuses = git_resolver.get_all_statuses();

            let modified_count = statuses
                .values()
                .filter(|s| **s == flowrail_core::git::GitFileStatus::Modified)
                .count();
            let staged_count = statuses
                .values()
                .filter(|s| **s == flowrail_core::git::GitFileStatus::Staged)
                .count();
            let untracked_count = statuses
                .values()
                .filter(|s| **s == flowrail_core::git::GitFileStatus::Untracked)
                .count();

            let art = FlowrailArtifact::from_doctor(
                &workspace,
                graph.node_count(),
                graph.edge_count(),
                Some(&git_resolver),
            );
            let saved_path = art.save_to_workspace(&workspace).ok();

            println!("=== Flowrail System Doctor (Ariadne + FFF Architecture) ===");
            println!("Workspace: {}", workspace.display());
            println!("Total Semantic Nodes: {}", graph.node_count());
            println!("Total Dependency Edges: {}", graph.edge_count());
            println!(
                "Git Status Cache: {} staged, {} modified, {} untracked",
                staged_count, modified_count, untracked_count
            );
            println!("Fuzzy Search & Frecency Engine: Ready");
            println!("Corridor-Scoped Grep: Ready");
            if let Some(p) = saved_path {
                println!("Audit Artifact Saved: {}", p.display());
            }
            println!("Health: All systems operational.");
        }
        Commands::Init { target, force } => {
            let flowrail_dir = workspace.join(".flowrail");
            std::fs::create_dir_all(&flowrail_dir)?;

            let workspace_arg = if raw_workspace == std::path::Path::new(".") {
                ".".to_string()
            } else {
                workspace.to_string_lossy().to_string()
            };

            let mcp_config = serde_json::json!({
                "mcpServers": {
                    "flowrail": {
                        "command": "flowrail",
                        "args": ["--workspace", workspace_arg, "mcp"]
                    }
                }
            });
            let mcp_str = serde_json::to_string_pretty(&mcp_config)?;

            let flowrail_mcp_path = flowrail_dir.join("mcp.json");
            std::fs::write(&flowrail_mcp_path, &mcp_str)?;
            println!(
                "  [OK] Created Flowrail MCP config: {}",
                flowrail_mcp_path.display()
            );

            let is_all = target == "all";

            // 1. Cursor
            if is_all || target == "cursor" {
                let cursor_dir = workspace.join(".cursor");
                std::fs::create_dir_all(&cursor_dir)?;
                let cursor_mcp_path = cursor_dir.join("mcp.json");
                if force || !cursor_mcp_path.exists() {
                    std::fs::write(&cursor_mcp_path, &mcp_str)?;
                    println!(
                        "  [OK] Created Cursor MCP config: {}",
                        cursor_mcp_path.display()
                    );
                } else {
                    println!(
                        "  [SKIP] Cursor MCP config already exists: {}",
                        cursor_mcp_path.display()
                    );
                }
                let cursor_rules_dir = cursor_dir.join("rules");
                std::fs::create_dir_all(&cursor_rules_dir)?;
                let cursor_mdc_path = cursor_rules_dir.join("flowrail.mdc");
                if force || !cursor_mdc_path.exists() {
                    let mdc_content = "---\ndescription: Flowrail deterministic execution, flow slicing, and mutation rules\nglobs: **/*\nalwaysApply: true\n---\n\n# Flowrail Execution & Mutation Rules\n\n- Discover entrypoints with flowrail_discover_entrypoints.\n- Read flows sequentially with flowrail_open({ intent: \"READ_FLOW\" }).\n- Mutate contracts with flowrail_open({ intent: \"MUTATE_CONTRACT\" }) in reverse topological order.\n";
                    std::fs::write(&cursor_mdc_path, mdc_content)?;
                    println!(
                        "  [OK] Created Cursor MDC rule: {}",
                        cursor_mdc_path.display()
                    );
                }
            }

            // 2. Claude Code & Claude Desktop
            if is_all || target == "claude" {
                let claude_mcp_path = workspace.join(".mcp.json");
                if force || !claude_mcp_path.exists() {
                    std::fs::write(&claude_mcp_path, &mcp_str)?;
                    println!(
                        "  [OK] Created Claude Code MCP config: {}",
                        claude_mcp_path.display()
                    );
                } else {
                    println!(
                        "  [SKIP] Claude MCP config already exists: {}",
                        claude_mcp_path.display()
                    );
                }
                let claude_dir = workspace.join(".claude");
                std::fs::create_dir_all(&claude_dir)?;
                let claude_rule = claude_dir.join("CLAUDE.md");
                if force || !claude_rule.exists() {
                    let rule_content = "# Flowrail Claude Directives\n\nSee `rules/AGENTS.md` for deterministic flow navigation and contract mutation rules.\n";
                    std::fs::write(&claude_rule, rule_content)?;
                    println!(
                        "  [OK] Created Claude instructions: {}",
                        claude_rule.display()
                    );
                }
            }

            // 3. OpenAI Codex
            if is_all || target == "codex" {
                let codex_dir = workspace.join(".codex");
                std::fs::create_dir_all(&codex_dir)?;
                let codex_toml = codex_dir.join("config.toml");
                if force || !codex_toml.exists() {
                    let toml_content = format!(
                        "[mcp_servers.flowrail]\ncommand = \"flowrail\"\nargs = [\"--workspace\", \"{}\", \"mcp\"]\nstartup_timeout_sec = 30.0\ntool_timeout_sec = 450.0\n",
                        workspace_arg
                    );
                    std::fs::write(&codex_toml, toml_content)?;
                    println!("  [OK] Created Codex config: {}", codex_toml.display());
                } else {
                    println!(
                        "  [SKIP] Codex config already exists: {}",
                        codex_toml.display()
                    );
                }
                let codex_rule = codex_dir.join("rules.md");
                if force || !codex_rule.exists() {
                    let rule_content = "# Flowrail Codex Directives\n\nSee `rules/AGENTS.md` for deterministic flow navigation and reverse topological mutation.\n";
                    std::fs::write(&codex_rule, rule_content)?;
                    println!("  [OK] Created Codex rules: {}", codex_rule.display());
                }
            }

            // 4. OpenCode
            if is_all || target == "opencode" {
                let opencode_dir = workspace.join(".opencode");
                std::fs::create_dir_all(&opencode_dir)?;
                let opencode_json = opencode_dir.join("opencode.json");
                if force || !opencode_json.exists() {
                    let opencode_cfg = serde_json::json!({
                        "$schema": "https://opencode.ai/config.json",
                        "default_agent": "flowrail-corridor-navigator",
                        "instructions": ["AGENTS.md", "rules/AGENTS.md"],
                        "mcpServers": {
                            "flowrail": {
                                "command": "flowrail",
                                "args": ["--workspace", workspace_arg, "mcp"],
                                "type": "stdio"
                            }
                        },
                        "permission": {
                            "flowrail_*": "allow"
                        }
                    });
                    std::fs::write(&opencode_json, serde_json::to_string_pretty(&opencode_cfg)?)?;
                    println!(
                        "  [OK] Created OpenCode config: {}",
                        opencode_json.display()
                    );
                } else {
                    println!(
                        "  [SKIP] OpenCode config already exists: {}",
                        opencode_json.display()
                    );
                }
            }

            // 5. Windsurf
            if is_all || target == "windsurf" {
                let windsurf_dir = workspace.join(".windsurf");
                std::fs::create_dir_all(&windsurf_dir)?;
                let ws_mcp = windsurf_dir.join("mcp_config.json");
                if force || !ws_mcp.exists() {
                    std::fs::write(&ws_mcp, &mcp_str)?;
                    println!("  [OK] Created Windsurf MCP config: {}", ws_mcp.display());
                } else {
                    println!(
                        "  [SKIP] Windsurf MCP config already exists: {}",
                        ws_mcp.display()
                    );
                }
            }

            // 6. Zed
            if is_all || target == "zed" {
                let zed_dir = workspace.join(".zed");
                std::fs::create_dir_all(&zed_dir)?;
                let zed_cfg = zed_dir.join("settings.json");
                if force || !zed_cfg.exists() {
                    let zed_json = serde_json::json!({
                        "context_servers": {
                            "flowrail": {
                                "command": {
                                    "path": "flowrail",
                                    "args": ["--workspace", workspace_arg, "mcp"]
                                }
                            }
                        }
                    });
                    std::fs::write(&zed_cfg, serde_json::to_string_pretty(&zed_json)?)?;
                    println!("  [OK] Created Zed settings: {}", zed_cfg.display());
                } else {
                    println!("  [SKIP] Zed settings already exist: {}", zed_cfg.display());
                }
            }

            // 7. VS Code / Cline
            if is_all || target == "vscode" {
                let vscode_dir = workspace.join(".vscode");
                std::fs::create_dir_all(&vscode_dir)?;
                let vscode_mcp = vscode_dir.join("mcp.json");
                if force || !vscode_mcp.exists() {
                    std::fs::write(&vscode_mcp, &mcp_str)?;
                    println!(
                        "  [OK] Created VS Code MCP config: {}",
                        vscode_mcp.display()
                    );
                } else {
                    println!(
                        "  [SKIP] VS Code MCP config already exists: {}",
                        vscode_mcp.display()
                    );
                }
            }

            // 8. Antigravity
            if is_all || target == "antigravity" {
                let ag_mcp = workspace.join("mcp_config.json");
                if force || !ag_mcp.exists() {
                    std::fs::write(&ag_mcp, &mcp_str)?;
                    println!(
                        "  [OK] Created Antigravity MCP config: {}",
                        ag_mcp.display()
                    );
                } else {
                    println!(
                        "  [SKIP] Antigravity MCP config already exists: {}",
                        ag_mcp.display()
                    );
                }
            }

            println!("\nFlowrail initialized successfully! MCP server configuration:");
            println!("{}", mcp_str);
        }
        Commands::Demo => {
            let fixture_candidates = [
                workspace.join("fixtures/sample-ts-ecommerce"),
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures/sample-ts-ecommerce"),
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/sample-ts-ecommerce"),
            ];

            let fixture_dir = fixture_candidates
                .iter()
                .find(|p| p.exists())
                .cloned()
                .unwrap_or_else(|| workspace.join("fixtures/sample-ts-ecommerce"));

            if !fixture_dir.exists() {
                anyhow::bail!("Fixture directory not found at {}", fixture_dir.display());
            }

            println!("=== Flowrail Deterministic State-Machine Demo ===");
            println!("Workspace Fixture: {}", fixture_dir.display());

            println!("\n1. Building Semantic Graph via OXC AST Slicer...");
            let graph = ScipGraphBuilder::build_from_directory(&fixture_dir)?;
            println!(
                "   -> Graph Built: {} nodes, {} dependency edges.",
                graph.node_count(),
                graph.edge_count()
            );

            println!("\n2. Discovering Entrypoint Gates (Fuzzy Search + Frecency)...");
            let git_resolver = GitStatusResolver::new(fixture_dir.clone());
            let frecency = FrecencyStore::new();
            let search_engine =
                FuzzySearchEngine::new(&graph, Some(&git_resolver), Some(&frecency));
            let hits = search_engine.search("checkout", true, false, 5);
            for h in &hits {
                println!(
                    "   -> [{:?}] {} ({}:{})",
                    h.role, h.display_name, h.file_path, h.line_range.0
                );
            }

            let entry_symbol = "src/modules/checkout/checkout.controller.ts#CheckoutController";
            println!(
                "\n3. Slicing Top-Down Read Corridor (Entry: {})...",
                entry_symbol
            );
            let read_plan = CorridorSlicer::build_corridor(
                &graph,
                entry_symbol,
                SliceIntent::ReadFlow,
                None,
                &PruningConfig::default(),
            )?;
            println!("   -> Read Corridor Gates (Sequential Execution Flow):");
            for (idx, g) in read_plan.gates.iter().enumerate() {
                println!(
                    "      Gate {}: [{:?}] {} ({:?})",
                    idx, g.node.role, g.node.display_name, g.status
                );
            }

            let target_symbol = "src/modules/tax/tax.types.ts#TaxCalculationResult";
            println!(
                "\n4. Slicing Bottom-Up Mutation Plan with Blast Radius (Target: {})...",
                target_symbol
            );
            let mutate_plan = CorridorSlicer::build_corridor(
                &graph,
                entry_symbol,
                SliceIntent::MutateContract,
                Some(target_symbol),
                &PruningConfig::default(),
            )?;
            println!("   -> Mutation Order (Contract Definition -> Upstream Callers):");
            for (idx, g) in mutate_plan.gates.iter().enumerate() {
                println!(
                    "      Gate {}: [{:?}] {} | Contagion Index: {}",
                    idx, g.node.role, g.node.display_name, g.blast_radius.contagion_index
                );
                if !g.blast_radius.external_dependents.is_empty() {
                    for dep in &g.blast_radius.external_dependents {
                        println!(
                            "         ⚠️  External Contagion: {} (Line {})",
                            dep.file_path, dep.call_site_line
                        );
                    }
                }
            }

            println!("\n5. Testing In-Memory Corridor Grep...");
            let mut files = Vec::new();
            ScipGraphBuilder::collect_ts_files(&fixture_dir, &mut files)?;
            let grep_report = CorridorGrep::search(
                &fixture_dir,
                &files,
                "processCheckout",
                true,
                None,
                1,
                5,
                Some(&git_resolver),
            )?;
            println!("{}", CorridorGrep::format_report_compact(&grep_report));

            println!("\n6. Generating Clean/Clear JSON Artifact (ADHD + Anti-Slop Enriched)...");
            let demo_artifact =
                FlowrailArtifact::from_plan(&mutate_plan, &fixture_dir, Some(&git_resolver));
            let demo_art_path = demo_artifact.save_to_workspace(&fixture_dir)?;
            println!(
                "   -> Clean JSON Artifact Saved: {}",
                demo_art_path.display()
            );
            println!("   -> Artifact ID: {}", demo_artifact.artifact_id);
            println!(
                "   -> Headline Action: {}",
                demo_artifact.ai_enrichment.adhd_view.headline_action
            );
            println!(
                "   -> State Anchor: {}",
                demo_artifact.ai_enrichment.adhd_view.state_anchor
            );
            println!(
                "   -> Next Action: {}",
                demo_artifact.ai_enrichment.adhd_view.terminal_action
            );

            println!("\n=== Flowrail Demo Completed Successfully ===");
        }
    }

    Ok(())
}
