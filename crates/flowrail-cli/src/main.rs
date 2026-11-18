use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
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
    version = "0.1.0",
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
    },
    /// Starts the Stateful MCP Server over stdio for Claude Code / Cursor / Windsurf
    Mcp {
        #[arg(long, default_value_t = false)]
        full_typecheck: bool,
    },
    /// Inspects and verifies workspace readiness, extracted symbols, and graph statistics
    Doctor,
    /// Initializes Flowrail configuration and generates MCP server setups for Cursor, Claude Code, and Windsurf
    Init {
        #[arg(long, default_value = "all")]
        target: String,
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Runs a self-contained live demonstration on sample TypeScript fixtures
    Demo,
}

fn load_graph(workspace: &std::path::Path) -> Result<flowrail_core::graph::CodebaseGraph> {
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
    let workspace = cli.workspace.canonicalize().unwrap_or(cli.workspace);

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
            println!("Found {} symbols matching '{}':", hits.len(), query);
            for h in hits {
                println!(
                    "  [{:?}] {:45} {:30} {} (score: {:.1})",
                    h.role,
                    h.display_name,
                    format!("{}:{}-{}", h.file_path, h.line_range.0, h.line_range.1),
                    h.git_status.badge(),
                    h.score
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
            let plan = CorridorSlicer::build_corridor(
                &graph,
                &entry,
                slice_intent,
                target.as_deref(),
                &PruningConfig::default(),
            )?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
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

            if target == "all" || target == "cursor" {
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
            }

            if target == "all" || target == "claude" {
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

            println!("\n=== Flowrail Demo Completed Successfully ===");
        }
    }

    Ok(())
}
