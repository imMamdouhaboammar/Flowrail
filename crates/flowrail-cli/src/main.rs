use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use flowrail_core::pruning::PruningConfig;
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
        Commands::Slice {
            entry,
            intent,
            target,
        } => {
            let graph = load_graph(&workspace)?;
            let slice_intent = if intent == "MUTATE_CONTRACT" {
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
            println!("=== Flowrail System Doctor ===");
            println!("Workspace: {}", workspace.display());
            println!("Total Semantic Nodes: {}", graph.node_count());
            println!("Total Dependency Edges: {}", graph.edge_count());
            println!("Health: All systems operational.");
        }
    }

    Ok(())
}
