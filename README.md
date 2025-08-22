# ICP Domain Registry - Context Protocol Integration

## 🌐 Overview

The ICP Domain Registry is a decentralized domain name service built on the Internet Computer, designed as part of the **Context Protocol** ecosystem. This registry enables AI agents to discover and interact with dApps through Model Context Protocol (MCP) endpoints, creating an AI-powered community where any canister can be made accessible to AI assistants.

**Deployed Registry Canister:** `vldd5-kyaaa-aaaao-a4o3a-cai`

### Key Features

- **Decentralized Domain Registration**: Register unique domains on the Internet Computer
- **MCP Integration**: Each domain can expose an MCP endpoint for AI agent interaction
- **Automatic Canister Creation**: Every registered domain gets its own dedicated canister
- **Variable Pricing Model**: Domain costs based on character length (1-char = 100 ICP, 2-char = 50 ICP, etc.)
- **Role-Based Access Control**: Owner, Administrator, and Operator roles for each domain
- **Domain Discovery**: Search and discover dApps with MCP endpoints in the community

## 🏗️ Architecture

### Core Components

1. **Registry Canister** (`src/lib.rs`)
   - Main registry implementation with domain management
   - Thread-local storage using `RefCell<HashMap>`
   - Role-based access control system
   - MCP endpoint management with HTTPS validation

2. **Domain Structure**
   ```rust
   DomainRecord {
       owner: Principal,           // Domain owner
       administrator: Principal,   // Admin rights
       operator: Principal,        // Operational access
       canister_id: Principal,     // Associated canister
       registration_time: u64,
       expiration_time: u64,       // 1-year default
       custom_mcp_endpoint: Option<String>,
       was_gifted: bool
   }
   ```

3. **MCP Integration**
   - Default endpoint: `https://mcp.ctx.xyz/{domain}`
   - Custom HTTPS endpoints with admin approval
   - AI agents can discover and interact with any registered dApp

## 🚀 Quick Start

### Prerequisites

- [DFX SDK](https://internetcomputer.org/docs/current/developer-docs/setup/install) (latest version)
- Rust toolchain with `wasm32-unknown-unknown` target
- Node.js and npm (for frontend development)

### Local Development

```bash
# Start local IC replica
dfx start --background

# Deploy the registry canister
dfx deploy registry

# Check deployment status
dfx canister status registry

# Stop replica when done
dfx stop
```

## 📝 Domain Registration

### Pricing Model

| Domain Length | Cost (ICP) | Examples |
|--------------|------------|----------|
| 1 character  | 100 ICP    | `a`, `x` |
| 2 characters | 50 ICP     | `ai`, `io` |
| 3 characters | 20 ICP     | `dex`, `nft` |
| 4 characters | 10 ICP     | `defi`, `swap` |
| 5-8 characters | 5 ICP    | `trading`, `wallet` |
| 9-12 characters | 2 ICP   | `marketplace` |
| 13+ characters | 1 ICP    | `decentralized-exchange` |

### Registration Flow

1. **Validate Domain**: Alphanumeric characters and hyphens, 1-63 chars
2. **Check Availability**: Not reserved or already registered
3. **Calculate Fees**: Based on domain length
4. **Create Canister**: Automatic canister provisioning
5. **Store Record**: 1-year expiration with renewal option

## 🔌 API Reference

### Update Methods

#### `register_domain(request: RegistrationRequest) -> Result<String, String>`
Register a new domain with associated roles and payment verification.

```rust
RegistrationRequest {
    domain_name: String,
    administrator: Principal,
    operator: Principal,
    payment_block: u64
}
```

#### `admin_gift_domain(request: AdminGiftRequest) -> Result<String, String>`
Admin-only function to gift domains without payment.

#### `renew_domain(domain_name: String, payment_block: u64) -> Result<String, String>`
Extend domain registration by one year.

#### `set_custom_mcp_endpoint(domain_name: String, endpoint: Option<String>) -> Result<(), String>`
Configure custom MCP endpoint (must use HTTPS).

### Query Methods

#### `get_domain_info(domain_name: String) -> Option<DomainInfo>`
Retrieve complete domain information including MCP endpoint.

#### `discover_domains(query: String) -> Vec<SearchResult>`
Search for domains with MCP endpoints in the community.

#### `get_registration_fee(domain_name: String) -> u64`
Calculate registration cost for a domain name.

#### `can_register_domain(domain_name: String, user: Principal) -> bool`
Check if a user can register a specific domain.

#### `list_domains(owner: Option<Principal>) -> Vec<DomainInfo>`
List all domains or filter by owner.

### Admin Functions

- `add_admin(new_admin: Principal)` - Add new administrator
- `set_short_name_mode(mode: RegistrationMode)` - Control short domain access
- `approve_user_for_short_names(user: Principal)` - Whitelist user for short domains
- `set_base_fee(new_fee: u64)` - Adjust base registration fee

## 🤖 Context Protocol Integration

The Context Protocol enables AI agents to interact with Internet Computer dApps through MCP endpoints. This registry serves as the discovery layer for the ecosystem.

### How It Works

1. **Domain Registration**: Users register domains and create associated dApps
2. **MCP Endpoint Setup**: Each domain exposes an MCP endpoint for AI interaction
3. **AI Discovery**: AI agents can search and discover available dApps
4. **Seamless Interaction**: Users with Context MCP wallets can interact with any registered dApp

### Example Use Cases

- **"Show me DEX dApps in the ICP Community"** - AI discovers decentralized exchanges
- **"How many users has this dApp?"** - Query dApp metrics through MCP
- **"Create a swap on TraderJoe"** - Execute transactions via AI agents
- **"Deploy my NFT collection"** - AI assists with canister deployment

### MCP Endpoint Format

Default: `https://mcp.ctx.xyz/{domain_name}`

Custom endpoints must:
- Use HTTPS protocol
- Be approved by domain admin
- Not exceed 200 characters

## 🛠️ Development Commands

### Building & Testing

```bash
# Build Rust backend
cargo build --target wasm32-unknown-unknown

# Run checks
cargo check
cargo clippy

# Generate Candid interface
dfx generate

# Deploy with cycles
dfx deploy --with-cycles 1000000000000
```

### Frontend Development

```bash
# Install dependencies
npm install

# Start development server
npm start  # Vite dev server on port 3000

# Build for production
npm run build
```

## 🔐 Security & Permissions

### Role Hierarchy

1. **Owner**: Full control over domain and associated canister
2. **Administrator**: Can modify settings and MCP endpoints
3. **Operator**: Operational access for day-to-day management

### Reserved Domains

Protected system domains include:
- `icp`, `ic`, `dfinity`
- `api`, `www`, `admin`
- `root`, `system`, `registry`

### Short Domain Protection

Domains under 5 characters require:
- Admin approval
- Whitelist membership
- Or open registration mode (admin-controlled)

## 📊 Domain Discovery

The registry provides powerful discovery features for the AI community:

```rust
// Search for trading-related dApps
discover_domains("trade") -> Vec<SearchResult>

// Each result includes:
SearchResult {
    domain: String,
    mcp_endpoint: String,
    description: String,
    was_gifted: bool
}
```

## 🔄 Domain Lifecycle

1. **Registration**: 1-year initial period
2. **Renewal**: Extend by 1 year (1 ICP base fee)
3. **Expiration**: Domain becomes available for re-registration
4. **Grace Period**: None - immediate availability after expiration

## 🌟 Community Features

### For dApp Developers
- Register memorable domains for your canisters
- Expose MCP endpoints for AI accessibility
- Build AI-interactive applications

### For AI Users
- Discover dApps through natural language
- Interact with canisters via AI agents
- Manage wallets through Context MCP

### For the Ecosystem
- Unified discovery layer for IC dApps
- Standardized AI interaction protocol
- Growing community of AI-enabled services

## 📚 Additional Resources

- [Internet Computer Documentation](https://internetcomputer.org/docs)
- [Context Protocol Specification](https://mcp.ctx.xyz)
- [Model Context Protocol (MCP)](https://modelcontextprotocol.io)
- [DFX Command Reference](https://internetcomputer.org/docs/current/references/cli-reference/dfx-parent)

## 🤝 Contributing

This registry is part of the Context Protocol's vision to create an AI-powered blockchain ecosystem. Contributions are welcome!

### Areas for Contribution
- Frontend improvements
- MCP endpoint validators
- Domain marketplace features
- Analytics and metrics
- Documentation and examples

## 📄 License

This project is part of the Internet Computer ecosystem and follows standard IC licensing terms.

---

**Registry Canister ID**: `vldd5-kyaaa-aaaao-a4o3a-cai`

Built with ❤️ for the Context Protocol AI Community
