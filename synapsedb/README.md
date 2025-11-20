# SynapseDB Documentation

**Last Updated**: 2025-11-02
**Version**: 0.1.1

## Overview

This folder contains **user-facing documentation** for SynapseDB - guides, API references, and installation instructions for end users and developers integrating SynapseDB into their applications.

## 📚 Documentation Files

### Installation & Setup
- **[BINARY_INSTALLATION.md](./BINARY_INSTALLATION.md)** - **Binary locations and system-wide installation** (START HERE for using from other projects)
- **[INSTALL.md](./INSTALL.md)** - Installation guide for different platforms
- **[BUILD.md](./BUILD.md)** - Building from source instructions
- **[UNINSTALL.md](./UNINSTALL.md)** - Uninstallation procedures
- **[UPGRADE.md](./UPGRADE.md)** - Version upgrade guide

### Usage Guides
- **[AI_ASSISTANT_GUIDE.md](./AI_ASSISTANT_GUIDE.md)** - **Comprehensive onboarding guide for AI coding assistants** (START HERE for AI tools)
- **[QUERY_API_REALITY_CHECK.md](./QUERY_API_REALITY_CHECK.md)** - ⚠️ **IMPORTANT: What actually works vs documentation** (READ THIS FIRST!)
- **[RUSTDOC_MCP_GUIDE.md](./RUSTDOC_MCP_GUIDE.md)** - Using MCP Rustdoc Parser for automated documentation generation
- **[EMBEDDED_SDK.md](./EMBEDDED_SDK.md)** - Embedded mode usage guide
- **[QUERY_API_GUIDE.md](./QUERY_API_GUIDE.md)** - Query API documentation (type-specific vs unified) ⚠️ *Contains aspirational features*
- **[SynapseDBAPIReferenceUsageExamples.md](./SynapseDBAPIReferenceUsageExamples.md)** - Complete API reference with examples
- **[ADMIN_CLI.md](./ADMIN_CLI.md)** - Admin CLI tool documentation

### Data & Migration
- **[ltmc-migration/](./ltmc-migration/)** - **Complete LTMC code graph migration documentation**
  - 📊 Migration summary with 8,858 nodes from LTMC codebase
  - 📝 Working Cypher query examples
  - 📘 Complete usage guide for querying LTMC data
  - 🔧 Troubleshooting and performance tips
  - ⚡ Quick reference card for common queries

### Operations
- **[SLOS.md](./SLOS.md)** - Service Level Objectives
- **[TODO_TRACKER.md](./TODO_TRACKER.md)** - Known issues and planned improvements

### Other
- **[packaging/](./packaging/)** - Packaging configurations for different distributions
- **json_execution_test.md** - Test execution documentation

## 🔗 Related Documentation

- **Research & Analysis**: See `/research/` folder for technical research, architectural designs, and implementation analysis
- **Development Tasks**: See `/tasks/` folder for PRDs, implementation plans, and development roadmap

## 📖 Quick Start

### For AI Coding Assistants
**START HERE**: [AI_ASSISTANT_GUIDE.md](./AI_ASSISTANT_GUIDE.md) - Complete onboarding guide with architecture, patterns, and examples

### For Human Developers
1. **Installation**: Start with [INSTALL.md](./INSTALL.md)
2. **Building**: For source builds, see [BUILD.md](./BUILD.md)
3. **Usage**: Check [EMBEDDED_SDK.md](./EMBEDDED_SDK.md) for embedded mode
4. **Queries**: Read [QUERY_API_GUIDE.md](./QUERY_API_GUIDE.md) for query patterns
5. **API Reference**: Complete API documentation in [SynapseDBAPIReferenceUsageExamples.md](./SynapseDBAPIReferenceUsageExamples.md)

## 🎯 Documentation Philosophy

This folder contains **ONLY** documentation needed for:
- Installing and configuring SynapseDB
- Using SynapseDB APIs and features
- Operating and maintaining SynapseDB instances
- Understanding SynapseDB's capabilities

**Not included here**:
- Research papers and analysis (→ `/research/`)
- Implementation progress reports (→ `/research/implementation/`)
- Development tasks and PRDs (→ `/tasks/`)
- Architecture design documents (→ `/research/architecture/`)

## 📮 Contributing Documentation

When adding new documentation:
1. **User-facing docs** go in this folder
2. **Research/analysis** goes in `/research/`
3. **Development plans** go in `/tasks/`

See [../CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.

---

**SynapseDB** - Next-generation multimodal database unifying SQL, graph, vector, text, cache, and RAG
