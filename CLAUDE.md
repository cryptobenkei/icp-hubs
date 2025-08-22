# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Internet Computer (IC) Domain Registry System - A decentralized domain name service allowing users to register and manage domains with associated MCP (Model Context Protocol) endpoints.

## Essential Commands

### Development Workflow
```bash
# Start local IC replica
dfx start --background

# Deploy all canisters
dfx deploy

# Deploy specific canister
dfx deploy registry_backend
dfx deploy registry_frontend

# Generate Candid interfaces
dfx generate

# Frontend development
npm start         # Starts Vite dev server on port 3000
npm run build     # Build frontend for production

# Check canister status
dfx canister status --all

# Stop replica
dfx stop
```

### Testing & Validation
```bash
# Build Rust backend
cargo build --target wasm32-unknown-unknown

# Check Rust code
cargo check
cargo clippy

# Frontend checks
npm run lint      # If configured
npm run typecheck # If configured
```

## Architecture Overview

### Core Components

1. **Main Registry Implementation** (`/src/lib.rs`):
   - Complete domain registration system with variable pricing (1-char = 100 ICP, 2-char = 50 ICP, etc.)
   - Role-based access control (Owner, Administrator, Operator)
   - MCP endpoint management with HTTPS validation
   - Automatic canister creation per domain
   - Uses thread-local storage pattern with `RefCell<HashMap>`

2. **Frontend** (`/src/registry_frontend/`):
   - Vite.js + Lit-HTML architecture
   - TypeScript support with SCSS styling
   - Direct IC canister communication via agent

### Critical Configuration Issues

**IMPORTANT**: The project has configuration misalignments that need attention:

1. **dfx.json** references wrong Candid path: `src/registry/registry.did` should be `src/registry_backend/registry_backend.did`
2. **Dependency versions** differ: Main lib.rs uses `ic-cdk = "0.13"`, backend uses `ic-cdk = "0.18"`
3. **Candid interface mismatch**: Current `.did` file only defines `greet` function, not the full registry API

### Key Data Structures

- **DomainRecord**: Core domain data with ownership, expiration, and payment info
- **RegistrationMode**: Controls short domain access (Open/WhitelistOnly/Closed)
- **Principal-based authentication** for all operations

### Domain Registration Flow

1. Validate domain (alphanumeric, 1-64 chars)
2. Check availability and reserved status
3. Calculate fees based on length
4. Create domain canister with proper controllers
5. Store record with 1-year expiration

### MCP Integration

- Default endpoint: `https://mcp.ctx.xyz/{domain}`
- Custom endpoints require HTTPS and admin approval
- Stored in domain records for discovery

## Development Patterns

- **Update vs Query**: Distinguish state-changing from read-only operations
- **Error Handling**: Use `Result<T, String>` for all public methods
- **Authentication**: Check caller principal for role-based access
- **State Management**: Thread-local storage with proper initialization checks