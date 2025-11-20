/**
 * Rust Quality Plugin
 * Purpose: Enforce Rust-specific quality standards
 * - Auto-checks fmt, clippy, tests
 * - Enforces 300-line file limit
 * - Provides convenient quality check tool
 */

export const RustQualityPlugin = async ({ $, directory }) => {
  return {
    // Add custom Rust quality tools
    "tools": [
      {
        name: "rust_quality_check",
        description: "Run comprehensive Rust quality checks (fmt, clippy, test)",
        inputSchema: {
          type: "object",
          properties: {
            crate: {
              type: "string",
              description: "Specific crate to check (optional, checks all if not provided)"
            }
          }
        },
        execute: async (input) => {
          const crate = input?.crate
          const workspaceFlag = crate ? `-p ${crate}` : "--workspace"

          try {
            // Run cargo fmt check
            await $`cargo fmt ${workspaceFlag} --check`.cwd(directory)
            console.log("✅ Format check passed")

            // Run cargo clippy
            await $`cargo clippy ${workspaceFlag} --all-targets --all-features -- -D warnings`.cwd(directory)
            console.log("✅ Clippy check passed")

            // Run tests
            await $`cargo test ${workspaceFlag}`.cwd(directory)
            console.log("✅ Tests passed")

            return { success: true, message: "All Rust quality checks passed" }
          } catch (error) {
            return {
              success: false,
              error: error.message,
              message: "Rust quality check failed - fix issues before claiming completion"
            }
          }
        }
      },
      {
        name: "check_file_length",
        description: "Ensure file doesn't exceed 300 lines (CLAUDE.md requirement)",
        inputSchema: {
          type: "object",
          properties: {
            file_path: {
              type: "string",
              description: "Path to file to check"
            }
          },
          required: ["file_path"]
        },
        execute: async (input) => {
          const file_path = input?.file_path
          if (!file_path) {
            return { success: false, message: "file_path is required" }
          }

          const fullPath = `${directory}/${file_path}`
          const lines = await $`wc -l < ${fullPath}`.cwd(directory).text()
          const lineCount = parseInt(lines.trim())

          if (lineCount > 300) {
            return {
              success: false,
              lines: lineCount,
              message: `❌ File exceeds 300 lines (${lineCount} lines). CLAUDE.md requires smart modularization.`
            }
          }

          return {
            success: true,
            lines: lineCount,
            message: `✅ File length OK (${lineCount} lines)`
          }
        }
      }
    ],

    // Auto-run checks after code changes
    "tool.after": async (tool, input, output) => {
      if (tool === "write" || tool === "edit") {
        const filePath = input.file_path
        if (filePath && filePath.endsWith('.rs')) {
          try {
            // Check file length
            const lines = await $`wc -l < ${filePath}`.cwd(directory).text()
            const lineCount = parseInt(lines.trim())

            if (lineCount > 300) {
              console.warn(`⚠️ WARNING: ${filePath} has ${lineCount} lines (max 300)`)
            }
          } catch (error) {
            // Silently ignore if file doesn't exist yet
          }
        }
      }
    }
  }
}
