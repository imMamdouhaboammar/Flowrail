#!/usr/bin/env python3
"""
Flowrail Multi-Harness Plugin Installer
Configures and links Flowrail across:
- Claude Code & Claude Desktop (~/.claude, ~/.claude.json)
- OpenAI Codex (~/.codex, ~/.codex/config.toml)
- OpenCode (~/.opencode, ~/.opencode.json)
- Antigravity / Gemini CLI (~/.gemini)
- Universal Agent Kernel (~/.agents/skills)
"""

import os
import sys
import json
from pathlib import Path

def link_safe(src: Path, dest: Path, label: str):
    try:
        dest.parent.mkdir(parents=True, exist_ok=True)
        if dest.is_symlink() or dest.exists():
            if dest.is_symlink() and dest.resolve() == src.resolve():
                print(f"  [ALREADY LINKED] {label}: {dest}")
                return
            if dest.is_symlink() or dest.is_file():
                dest.unlink()
            elif dest.is_dir():
                import shutil
                shutil.rmtree(dest)
        dest.symlink_to(src)
        print(f"  [OK] Linked {label}: {dest} -> {src}")
    except Exception as e:
        print(f"  [WARN] Failed to link {label} ({dest}): {e}", file=sys.stderr)

def install_skills(source_skills_dir: Path, target_skills_dir: Path, harness_name: str):
    if not source_skills_dir.exists():
        return
    for item in source_skills_dir.iterdir():
        if item.is_dir() and (item / "SKILL.md").exists():
            dest = target_skills_dir / item.name
            link_safe(item, dest, f"{harness_name} skill ({item.name})")

def setup_claude(root_dir: Path):
    home = Path.home()
    claude_dir = home / ".claude"
    claude_json_path = home / ".claude.json"
    
    print("\n--- [1/5] Configuring Claude Code & Claude Desktop ---")
    if claude_dir.exists():
        plugins_dir = claude_dir / "plugins"
        skills_dir = claude_dir / "skills"
        link_safe(root_dir, plugins_dir / "flowrail", "Claude Plugin")
        install_skills(root_dir / "skills", skills_dir, "Claude")
    else:
        print("  ~/.claude directory not present, skipping skills symlink.")

    # Update ~/.claude.json if present
    if claude_json_path.exists():
        try:
            with open(claude_json_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            mcp_servers = data.setdefault("mcpServers", {})
            mcp_servers["flowrail"] = {
                "command": "flowrail",
                "args": ["mcp"]
            }
            with open(claude_json_path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2)
            print(f"  [OK] Registered flowrail MCP in {claude_json_path}")
        except Exception as e:
            print(f"  [WARN] Could not update {claude_json_path}: {e}", file=sys.stderr)

def setup_codex(root_dir: Path):
    home = Path.home()
    codex_dir = home / ".codex"
    print("\n--- [2/5] Configuring OpenAI Codex ---")
    if codex_dir.exists():
        plugins_dir = codex_dir / "plugins"
        skills_dir = codex_dir / "skills"
        link_safe(root_dir, plugins_dir / "flowrail", "Codex Plugin")
        install_skills(root_dir / "skills", skills_dir, "Codex")

        config_toml = codex_dir / "config.toml"
        if config_toml.exists():
            content = config_toml.read_text(encoding="utf-8")
            if "[mcp_servers.flowrail]" not in content:
                snippet = (
                    "\n[mcp_servers.flowrail]\n"
                    'command = "flowrail"\n'
                    'args = ["mcp"]\n'
                    "startup_timeout_sec = 30.0\n"
                    "tool_timeout_sec = 450.0\n"
                )
                config_toml.write_text(content + snippet, encoding="utf-8")
                print(f"  [OK] Appended flowrail MCP server to {config_toml}")
            else:
                print(f"  [ALREADY REGISTERED] flowrail in {config_toml}")

def setup_opencode(root_dir: Path):
    home = Path.home()
    opencode_json_path = home / ".opencode.json"
    print("\n--- [3/5] Configuring OpenCode ---")
    data = {}
    if opencode_json_path.exists():
        try:
            with open(opencode_json_path, "r", encoding="utf-8") as f:
                data = json.load(f)
        except Exception:
            data = {}
    mcp_servers = data.setdefault("mcpServers", {})
    mcp_servers["flowrail"] = {
        "command": "flowrail",
        "args": ["mcp"],
        "type": "stdio"
    }
    try:
        with open(opencode_json_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
        print(f"  [OK] Registered flowrail in {opencode_json_path}")
    except Exception as e:
        print(f"  [WARN] Could not update {opencode_json_path}: {e}", file=sys.stderr)

def setup_antigravity(root_dir: Path):
    home = Path.home()
    gemini_dir = home / ".gemini"
    print("\n--- [4/5] Configuring Antigravity & Gemini CLI ---")
    if gemini_dir.exists():
        plugins_dir = gemini_dir / "config" / "plugins"
        skills_dir = gemini_dir / "config" / "skills"
        link_safe(root_dir, plugins_dir / "flowrail", "Antigravity Plugin")
        install_skills(root_dir / "skills", skills_dir, "Antigravity")

def setup_universal_agents(root_dir: Path):
    home = Path.home()
    agents_dir = home / ".agents"
    print("\n--- [5/5] Configuring Universal Agent Kernel ---")
    skills_dir = agents_dir / "skills"
    install_skills(root_dir / "skills", skills_dir, "Agent Kernel")

def main():
    root_dir = Path(__file__).resolve().parent.parent
    print(f"Flowrail Multi-Harness Installer Root: {root_dir}")
    setup_claude(root_dir)
    setup_codex(root_dir)
    setup_opencode(root_dir)
    setup_antigravity(root_dir)
    setup_universal_agents(root_dir)
    print("\nAll agent harnesses configured successfully!")

if __name__ == "__main__":
    main()
