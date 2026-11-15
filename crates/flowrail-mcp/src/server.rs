use crate::tools::CorridorMcpEngine;
use anyhow::Result;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

pub struct McpStdioServer {
    engine: CorridorMcpEngine,
}

impl McpStdioServer {
    pub fn new(engine: CorridorMcpEngine) -> Self {
        Self { engine }
    }

    pub fn run_stdio_loop(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let req: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": format!("Parse error: {}", e) }
                    });
                    writeln!(stdout, "{}", err_resp)?;
                    stdout.flush()?;
                    continue;
                }
            };

            let id = req.get("id").cloned().unwrap_or(Value::Null);
            let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

            if method.starts_with("notifications/") {
                continue;
            }

            let response = match method {
                "initialize" => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": "flowrail-mcp",
                            "version": "0.1.0"
                        }
                    }
                }),
                "tools/list" => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": CorridorMcpEngine::list_tool_definitions()
                }),
                "tools/call" => {
                    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let arguments = params
                        .get("arguments")
                        .cloned()
                        .unwrap_or_else(|| json!({}));

                    match self.engine.dispatch_tool(tool_name, &arguments) {
                        Ok(text_payload) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "isError": false,
                                "content": [{ "type": "text", "text": text_payload }]
                            }
                        }),
                        Err(err) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "isError": true,
                                "content": [{ "type": "text", "text": err.to_string() }]
                            }
                        }),
                    }
                }
                _ => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": format!("Method not found: {}", method) }
                }),
            };

            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }

        Ok(())
    }
}
