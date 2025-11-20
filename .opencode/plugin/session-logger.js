/**
 * Session Logger Plugin
 * Purpose: Log all AI sessions for audit and learning
 * - Tracks session start/end times
 * - Records tool usage patterns
 * - Creates audit trail
 */

import { writeFile } from "fs/promises"
import { join } from "path"

export const SessionLoggerPlugin = async ({ directory }) => {
  let sessionStart = null
  let toolCalls = []

  return {
    "session.start": async () => {
      sessionStart = new Date()
      toolCalls = []
      console.log(`📝 Session started: ${sessionStart.toISOString()}`)
    },

    "tool.before": async (tool, input) => {
      toolCalls.push({
        tool,
        input,
        timestamp: new Date().toISOString()
      })
    },

    "session.idle": async () => {
      if (sessionStart && toolCalls.length > 0) {
        const sessionEnd = new Date()
        const duration = sessionEnd - sessionStart

        const logEntry = {
          start: sessionStart.toISOString(),
          end: sessionEnd.toISOString(),
          duration_ms: duration,
          tool_calls_count: toolCalls.length,
          tools_used: [...new Set(toolCalls.map(t => t.tool))],
          directory
        }

        const logPath = join(directory, ".opencode", "sessions.jsonl")
        await writeFile(logPath, JSON.stringify(logEntry) + "\n", { flag: "a" })

        console.log(`📊 Session logged: ${toolCalls.length} tool calls, ${duration}ms`)
      }
    }
  }
}
