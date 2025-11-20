/**
 * Enforcement Plugin
 * Purpose: Enforce CLAUDE.md/QWEN.md quality rules at OpenCode level
 * Prevents GLM from creating stubs, placeholders, or mock implementations
 */

export const EnforcementPlugin = async ({ client, $ }) => {
  // Keywords that indicate stub/placeholder code
  const FORBIDDEN_PATTERNS = [
    /TODO:?\s/i,
    /FIXME:?\s/i,
    /pass\s*$/m,
    /placeholder/i,
    /stub/i,
    /\.unwrap\(\)/,  // Rust unwrap (discouraged in production)
    /unimplemented!\(\)/,  // Rust unimplemented macro
    /panic!\(\)/,  // Rust panic (discouraged in production)
  ]

  return {
    // Intercept write operations
    "tool.before": async (tool, input) => {
      if (tool === "write" || tool === "edit") {
        const content = input.content || input.new_string || ""

        // Check for forbidden patterns
        for (const pattern of FORBIDDEN_PATTERNS) {
          if (pattern.test(content)) {
            throw new Error(
              `❌ ENFORCEMENT VIOLATION: Code contains forbidden pattern: ${pattern.source}\n` +
              `CLAUDE.md Rule: NO stubs, mocks, placeholders, or pass statements EVER\n` +
              `Please provide a real implementation.`
            )
          }
        }
      }
    },

    // Log enforcement checks
    "tool.after": async (tool, input, output) => {
      if (tool === "write" || tool === "edit") {
        console.log("✅ Enforcement check passed")
      }
    }
  }
}
