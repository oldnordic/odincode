# OdinCode

**OdinCode is an open-source AI code engineering system written in Rust. This project is currently a work in progress and does not yet function as intended.**

## Project Status

⚠️ **WARNING: This is an incomplete work-in-progress project. The codebase is under active development and many components are not yet implemented or functional.**

- **Current State**: Architecture foundation and basic structure in place
- **Functionality**: Limited - most AI features are not yet working
- **Stability**: Unstable - APIs may change significantly
- **Documentation**: Minimal and may be outdated

## Project Vision

The goal of OdinCode is to create a next-generation AI coding assistant that combines:

- **Rust Implementation**: Built entirely in Rust for performance and safety
- **Persistent Memory**: LTMC (Long-Term Memory and Context) system using multiple databases
- **AI Agents**: Specialized agents for different coding tasks
- **Code Analysis**: Multi-language support with semantic understanding
- **Terminal Integration**: Native terminal user interface

## What's Currently Implemented

### Architecture Foundation
- Rust workspace structure with 7 crates:
  - `core`: Basic code analysis engine structure
  - `agents`: Framework for AI agents (mostly empty)
  - `tools`: Tool integration system (basic structure)
  - `ltmc`: Memory system foundation (not functional)
  - `databases`: Database connection utilities (basic)
  - `api`: HTTP API framework (minimal)
  - `tui`: Terminal UI framework (basic)

### Basic Components
- Language analyzer framework for 25+ programming languages
- ML integration structure (no actual ML functionality)
- LLM integration framework (no actual LLM connectivity)
- Database connection utilities (SQLite, Neo4j, Redis, FAISS)
- Basic MCP (Model Context Protocol) integration structure

### What's NOT Working
- AI code generation and analysis
- LTMC memory system
- LLM integration
- Most agent functionality
- Terminal user interface
- API endpoints
- Actual ML models and training

## Technical Requirements

- Rust 1.75+ (latest stable)
- Cargo
- Git

## Building

```bash
# Clone the repository
git clone https://github.com/oldnordic/odincode.git
cd odincode

# Build the project
cargo build --workspace

# Run tests (most will fail due to incomplete implementation)
cargo test --workspace
```

## Project Structure

```
odincode/
├── core/           # Core code analysis engine
├── agents/         # AI agent implementations
├── tools/          # Tool integrations
├── ltmc/           # Long-term memory system
├── databases/      # Database connectors
├── api/            # HTTP API layer
├── tui/            # Terminal user interface
└── src/            # Main application entry point
```

## Development Philosophy

This project follows these principles:

- **Quality over Speed**: No shortcuts or placeholder implementations
- **No Technical Debt**: Clean, maintainable code only
- **Modular Design**: Maximum 300 lines per file
- **Real Functionality**: No mocks or stubs in production code
- **Comprehensive Testing**: Test-driven development approach

## Contributing

**Contributions are welcome, but please understand this is an early-stage project.**

Before contributing:
1. Understand that this is a work-in-progress
2. Check existing issues for what needs to be implemented
3. Follow the coding standards outlined in the project
4. Ensure all tests pass for your changes
5. Focus on functional implementations, not placeholders

## Roadmap

The development is planned in phases:

1. **Phase 1** (Current): Foundation and architecture
2. **Phase 2**: Basic AI functionality and LLM integration
3. **Phase 3**: Advanced features and production readiness
4. **Phase 4**: Optimization and deployment tools

## License

This project is licensed under the GNU General Public License v3.0 (GPL-3.0). See the [LICENSE](LICENSE) file for details.

## Disclaimer

This is an experimental open-source project. The code is provided as-is without any warranties. The project may change significantly or be abandoned at any time. Use at your own risk.

## Contact

For questions, issues, or contributions, please use the GitHub issue tracker.