/**
 * Flowrail OpenCode Plugin Integration
 */
export const flowrailPlugin = {
  name: "flowrail",
  version: "0.1.2",
  description: "Deterministic Execution & Mutation State-Machine for AI Coding Agents",
  mcpServer: {
    command: "flowrail",
    args: ["mcp"],
    type: "stdio",
  },
  skillsPath: "./skills",
};

export default flowrailPlugin;
