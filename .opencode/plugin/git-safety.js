/**
 * Git Safety Plugin
 * Purpose: Prevent accidental force pushes and protect main branch
 * - Blocks force push
 * - Warns on push to main/master
 * - Provides safe push tool with pre-push checks
 */

export const GitSafetyPlugin = async ({ $, directory }) => {
  return {
    "tool.before": async (tool, input) => {
      if (tool === "bash") {
        const command = input.command || ""

        // Block force push
        if (command.includes("git push") && command.includes("--force")) {
          throw new Error(
            `❌ GIT SAFETY: Force push blocked by plugin\n` +
            `Use 'git push --force-with-lease' instead, or disable this plugin.`
          )
        }

        // Warn on push to main/master
        if (command.includes("git push") && !command.includes("--dry-run")) {
          try {
            const branch = await $`git rev-parse --abbrev-ref HEAD`.cwd(directory).text()
            const currentBranch = branch.trim()

            if (currentBranch === "main" || currentBranch === "master") {
              console.warn(`⚠️ WARNING: Pushing to ${currentBranch} branch`)
            }
          } catch (error) {
            // Silently ignore if not a git repo
          }
        }
      }
    },

    "tools": [
      {
        name: "safe_git_push",
        description: "Safely push changes with pre-push checks (fmt, clippy, test)",
        inputSchema: {
          type: "object",
          properties: {
            remote: {
              type: "string",
              description: "Remote name (default: origin)",
              default: "origin"
            },
            branch: {
              type: "string",
              description: "Branch name (current if not specified)"
            },
            force: {
              type: "boolean",
              description: "Use --force-with-lease (safer than --force)",
              default: false
            }
          }
        },
        execute: async (input) => {
          const remote = input?.remote || "origin"
          const force = input?.force || false
          let branch = input?.branch

          try {
            // Run pre-push checks
            console.log("🔍 Running pre-push quality checks...")

            await $`cargo fmt --all --check`.cwd(directory)
            console.log("✅ Format check passed")

            await $`cargo clippy --workspace --all-targets -- -D warnings`.cwd(directory)
            console.log("✅ Clippy check passed")

            await $`cargo test --workspace`.cwd(directory)
            console.log("✅ Tests passed")

            // Get current branch if not specified
            if (!branch) {
              const branchName = await $`git rev-parse --abbrev-ref HEAD`.cwd(directory).text()
              branch = branchName.trim()
            }

            // Build push command
            let pushCmd = `git push ${remote} ${branch}`
            if (force) {
              pushCmd += " --force-with-lease"
            }

            // Execute push
            await $`${pushCmd}`.cwd(directory)

            return {
              success: true,
              message: `✅ Successfully pushed to ${remote}/${branch}`
            }
          } catch (error) {
            return {
              success: false,
              error: error.message,
              message: `❌ Push failed: ${error.message}`
            }
          }
        }
      }
    ]
  }
}
