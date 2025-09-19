// src/lib.rs - Fixed for ic-cdk 0.13+
use ic_cdk::api::management_canister::main::{
    create_canister, CreateCanisterArgument, CanisterSettings
};
use ic_cdk::{caller, id, api::time};
use ic_cdk_macros::*;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct DomainRecord {
    pub owner: Principal,
    pub administrator: Principal,
    pub operator: Principal,
    pub canister_id: Principal,
    pub registration_time: u64,
    pub expiration_time: u64,
    pub last_payment_block: u64,
    pub custom_mcp_endpoint: Option<String>,
    pub was_gifted: bool,
    pub registration_season_id: Option<u64>, // Track which season was used
}

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct DomainInfo {
    pub name: String,
    pub owner: Principal,
    pub administrator: Principal,
    pub operator: Principal,
    pub canister_id: Principal,
    pub expiration_time: u64,
    pub mcp_endpoint: String,
    pub status: DomainStatus,
    pub was_gifted: bool,
}

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub enum DomainStatus {
    Active,
    Expired,
    Reserved,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct RegistrationRequest {
    pub domain_name: String,
    pub administrator: Principal,
    pub operator: Principal,
    pub payment_block: u64,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct AdminGiftRequest {
    pub domain_name: String,
    pub recipient: Principal,
    pub administrator: Principal,
    pub operator: Principal,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct AdminCreateDomainRequest {
    pub domain_name: String,
    pub recipient: Principal,
    pub administrator: Principal,
    pub operator: Principal,
    pub recipient_address: String, // The address that must exist in the season
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct SearchResult {
    pub domain: String,
    pub description: String,
    pub mcp_endpoint: String,
    pub tools_count: u32,
    pub resources_count: u32,
    pub was_gifted: bool,
}

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub enum RegistrationMode {
    Open,
    WhitelistOnly,
    Closed,
}

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub enum SeasonStatus {
    Active,
    Completed,
    Deactivated,
}

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct RegistrationSeason {
    pub season_id: u64,
    pub min_letters: u64,
    pub max_letters: Option<u64>, // None means no upper limit
    pub total_allowed: u64,
    pub registered_count: u64,
    pub price_icp: u64, // Price in ICP (1 ICP = 100_000_000 e8s)
    pub created_by: Principal,
    pub created_at: u64,
    pub status: SeasonStatus,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct CreateSeasonRequest {
    pub min_letters: u64,
    pub max_letters: Option<u64>,
    pub total_allowed: u64,
    pub price_icp: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct SeasonStats {
    pub season_number: u64,
    pub names_available: u64,
    pub names_taken: u64,
    pub price_icp: u64,
    pub status: SeasonStatus,
}

thread_local! {
    static DOMAINS: RefCell<HashMap<String, DomainRecord>> = RefCell::new(HashMap::new());
    static RESERVED_NAMES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static ADMIN_PRINCIPALS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::new());
    static SHORT_NAME_MODE: RefCell<RegistrationMode> = RefCell::new(RegistrationMode::WhitelistOnly);
    static APPROVED_SHORT_USERS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::new());
    static BASE_FEE: RefCell<u64> = RefCell::new(100_000_000);
    static DOMAIN_CANISTER_WASM: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    static REGISTRATION_SEASONS: RefCell<HashMap<u64, RegistrationSeason>> = RefCell::new(HashMap::new());
    static NEXT_SEASON_ID: RefCell<u64> = RefCell::new(1);
    static WALLET_TO_DOMAIN: RefCell<HashMap<Principal, String>> = RefCell::new(HashMap::new());
    static SEASON_ADDRESSES: RefCell<HashMap<u64, HashSet<String>>> = RefCell::new(HashMap::new());
}

fn find_applicable_season(domain_name: &str) -> Option<(u64, RegistrationSeason)> {
    let domain_length = domain_name.len() as u64;
    
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .iter()
            .filter(|(_, season)| {
                matches!(season.status, SeasonStatus::Active) && 
                domain_length >= season.min_letters &&
                (season.max_letters.is_none() || domain_length <= season.max_letters.unwrap()) &&
                season.registered_count < season.total_allowed
            })
            .min_by_key(|(_, season)| season.price_icp)
            .map(|(id, season)| (*id, season.clone()))
    })
}

fn calculate_registration_fee(domain_name: &str) -> Result<u64, String> {
    match find_applicable_season(domain_name) {
        Some((_, season)) => Ok(season.price_icp * 100_000_000), // Convert ICP to e8s
        None => Err("No available registration season for this domain length".to_string()),
    }
}

fn calculate_renewal_fee() -> u64 {
    BASE_FEE.with(|base| *base.borrow())
}

fn is_valid_domain_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 63 {
        return false;
    }
    
    if name.starts_with('-') || name.ends_with('-') {
        return false;
    }
    
    name.chars().all(|c| c.is_alphanumeric() || c == '-')
}

fn is_reserved_name(name: &str) -> bool {
    RESERVED_NAMES.with(|reserved| {
        reserved.borrow().contains(name)
    })
}

fn is_admin(caller: Principal) -> bool {
    ADMIN_PRINCIPALS.with(|admins| {
        admins.borrow().contains(&caller)
    })
}

fn can_register_short_domain(domain_name: &str, caller: Principal) -> bool {
    let length = domain_name.len();
    
    if length >= 5 {
        return true;
    }
    
    if is_admin(caller) {
        return true;
    }
    
    SHORT_NAME_MODE.with(|mode| {
        match *mode.borrow() {
            RegistrationMode::Open => true,
            RegistrationMode::WhitelistOnly => {
                APPROVED_SHORT_USERS.with(|users| users.borrow().contains(&caller))
            },
            RegistrationMode::Closed => false,
        }
    })
}

fn wallet_already_has_domain(wallet: Principal) -> Option<String> {
    WALLET_TO_DOMAIN.with(|mapping| {
        mapping.borrow().get(&wallet).cloned()
    })
}

fn has_active_season() -> bool {
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .values()
            .any(|season| matches!(season.status, SeasonStatus::Active))
    })
}

fn complete_season_if_full(season_id: u64) {
    REGISTRATION_SEASONS.with(|seasons| {
        if let Some(season) = seasons.borrow_mut().get_mut(&season_id) {
            if season.registered_count >= season.total_allowed {
                season.status = SeasonStatus::Completed;
            }
        }
    });
}

fn is_address_in_season(season_id: u64, address: &str) -> bool {
    SEASON_ADDRESSES.with(|addresses| {
        addresses.borrow()
            .get(&season_id)
            .map(|addr_set| addr_set.contains(address))
            .unwrap_or(false)
    })
}

fn add_address_to_season(season_id: u64, address: String) -> Result<(), String> {
    // Check if season exists and is active
    let season_status = REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .get(&season_id)
            .map(|s| s.status.clone())
    });
    
    match season_status {
        Some(SeasonStatus::Active) => {
            SEASON_ADDRESSES.with(|addresses| {
                addresses.borrow_mut()
                    .entry(season_id)
                    .or_insert_with(HashSet::new)
                    .insert(address);
            });
            Ok(())
        }
        Some(SeasonStatus::Completed) => Err("Cannot add address to completed season".to_string()),
        Some(SeasonStatus::Deactivated) => Err("Cannot add address to deactivated season".to_string()),
        None => Err("Season not found".to_string()),
    }
}

#[init]
fn init(admin: Principal) {
    ADMIN_PRINCIPALS.with(|admins| {
        admins.borrow_mut().insert(admin);
    });
    
    RESERVED_NAMES.with(|reserved| {
        let mut names = reserved.borrow_mut();
        names.insert("icp".to_string());
        names.insert("api".to_string());
        names.insert("www".to_string());
        names.insert("admin".to_string());
        names.insert("root".to_string());
        names.insert("system".to_string());
        names.insert("registry".to_string());
        names.insert("canister".to_string());
        names.insert("dfinity".to_string());
        names.insert("ic".to_string());
    });
}

#[update]
async fn register_domain(request: RegistrationRequest) -> Result<String, String> {
    let caller = caller();
    
    if !is_valid_domain_name(&request.domain_name) {
        return Err("Invalid domain name format".to_string());
    }
    
    if is_reserved_name(&request.domain_name) {
        return Err("Domain name is reserved".to_string());
    }
    
    if !can_register_short_domain(&request.domain_name, caller) {
        return Err("Short domain names require approval".to_string());
    }
    
    // Check if wallet already has a domain (unless admin)
    let is_admin_caller = is_admin(caller);
    if !is_admin_caller {
        if let Some(existing_domain) = wallet_already_has_domain(caller) {
            return Err(format!("Wallet already owns domain: {}", existing_domain));
        }
    }
    
    let is_available = DOMAINS.with(|domains| {
        match domains.borrow().get(&request.domain_name) {
            Some(domain) => {
                let current_time = time();
                domain.expiration_time < current_time
            }
            None => true,
        }
    });
    
    if !is_available {
        return Err("Domain name is not available".to_string());
    }
    
    // Find applicable season and calculate fee
    let (season_id, required_fee) = if is_admin_caller {
        (None, 0u64) // Admins register for free
    } else {
        match find_applicable_season(&request.domain_name) {
            Some((id, season)) => (Some(id), season.price_icp * 100_000_000),
            None => return Err("No available registration season for this domain length".to_string()),
        }
    };
    
    // Update season registration count if not admin
    if let Some(id) = season_id {
        REGISTRATION_SEASONS.with(|seasons| {
            if let Some(season) = seasons.borrow_mut().get_mut(&id) {
                if season.registered_count >= season.total_allowed {
                    return Err("Registration season is full".to_string());
                }
                season.registered_count += 1;
                Ok(())
            } else {
                Err("Season not found".to_string())
            }
        })?;
    }
    
    // Create new canister for this domain
    let canister_id = create_domain_canister(
        &request.domain_name, 
        caller, 
        request.administrator, 
        request.operator
    ).await.map_err(|e| {
        // Rollback season count on canister creation failure
        if let Some(id) = season_id {
            REGISTRATION_SEASONS.with(|seasons| {
                if let Some(season) = seasons.borrow_mut().get_mut(&id) {
                    season.registered_count -= 1;
                }
            });
        }
        e
    })?;
    
    let domain_record = DomainRecord {
        owner: caller,
        administrator: request.administrator,
        operator: request.operator,
        canister_id,
        registration_time: time(),
        expiration_time: time() + (365 * 24 * 60 * 60 * 1_000_000_000), // 1 year
        last_payment_block: request.payment_block,
        custom_mcp_endpoint: None,
        was_gifted: is_admin_caller,
        registration_season_id: season_id,
    };
    
    DOMAINS.with(|domains| {
        domains.borrow_mut().insert(request.domain_name.clone(), domain_record);
    });
    
    // Add wallet-to-domain mapping
    WALLET_TO_DOMAIN.with(|mapping| {
        mapping.borrow_mut().insert(caller, request.domain_name.clone());
    });
    
    // Check if season is now complete and mark it as such
    if let Some(id) = season_id {
        complete_season_if_full(id);
    }
    
    let fee_info = if is_admin_caller {
        "Free (admin registration)".to_string()
    } else {
        format!("Fee: {} ICP", required_fee as f64 / 100_000_000.0)
    };
    
    Ok(format!(
        "Domain {} registered successfully with canister {}. {}",
        request.domain_name, canister_id, fee_info
    ))
}

#[update]
async fn admin_gift_domain(request: AdminGiftRequest) -> Result<String, String> {
    let caller = caller();
    
    if !is_admin(caller) {
        return Err("Only admins can gift domains".to_string());
    }
    
    if !is_valid_domain_name(&request.domain_name) {
        return Err("Invalid domain name format".to_string());
    }
    
    if is_reserved_name(&request.domain_name) {
        return Err("Domain name is reserved".to_string());
    }
    
    let is_available = DOMAINS.with(|domains| {
        match domains.borrow().get(&request.domain_name) {
            Some(domain) => {
                let current_time = time();
                domain.expiration_time < current_time
            }
            None => true,
        }
    });
    
    if !is_available {
        return Err("Domain name is not available".to_string());
    }
    
    // Check if recipient already has a domain
    if let Some(existing_domain) = wallet_already_has_domain(request.recipient) {
        return Err(format!("Recipient already owns domain: {}", existing_domain));
    }
    
    // Find active season and check if it can accommodate this domain
    let active_season_info = REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .values()
            .find(|season| matches!(season.status, SeasonStatus::Active))
            .map(|s| (s.season_id, s.registered_count, s.total_allowed))
    });
    
    let season_id = match active_season_info {
        Some((id, registered, total)) => {
            if registered >= total {
                return Err("Cannot gift domain: active season is full".to_string());
            }
            Some(id)
        }
        None => return Err("Cannot gift domain: no active season available".to_string()),
    };
    
    let canister_id = create_domain_canister(
        &request.domain_name,
        request.recipient,
        request.administrator,
        request.operator,
    ).await?;
    
    let domain_record = DomainRecord {
        owner: request.recipient,
        administrator: request.administrator,
        operator: request.operator,
        canister_id,
        registration_time: time(),
        expiration_time: time() + (365 * 24 * 60 * 60 * 1_000_000_000), // 1 year
        last_payment_block: 0,
        custom_mcp_endpoint: None,
        was_gifted: true,
        registration_season_id: season_id, // Track season usage even for gifts
    };
    
    DOMAINS.with(|domains| {
        domains.borrow_mut().insert(request.domain_name.clone(), domain_record);
    });
    
    // Add wallet-to-domain mapping for recipient
    WALLET_TO_DOMAIN.with(|mapping| {
        mapping.borrow_mut().insert(request.recipient, request.domain_name.clone());
    });
    
    // Increment season count for gifts (they still consume season slots)
    if let Some(id) = season_id {
        REGISTRATION_SEASONS.with(|seasons| {
            if let Some(season) = seasons.borrow_mut().get_mut(&id) {
                season.registered_count += 1;
            }
        });
        complete_season_if_full(id);
    }
    
    Ok(format!(
        "Domain {} gifted to {} with canister {} (FREE admin gift)",
        request.domain_name, request.recipient, canister_id
    ))
}

#[update]
async fn admin_create_domain_with_address(request: AdminCreateDomainRequest) -> Result<String, String> {
    let caller = caller();
    
    if !is_admin(caller) {
        return Err("Only admins can create domains with addresses".to_string());
    }
    
    if !is_valid_domain_name(&request.domain_name) {
        return Err("Invalid domain name format".to_string());
    }
    
    if is_reserved_name(&request.domain_name) {
        return Err("Domain name is reserved".to_string());
    }
    
    let is_available = DOMAINS.with(|domains| {
        match domains.borrow().get(&request.domain_name) {
            Some(domain) => {
                let current_time = time();
                domain.expiration_time < current_time
            }
            None => true,
        }
    });
    
    if !is_available {
        return Err("Domain name is not available".to_string());
    }
    
    // Check if recipient already has a domain
    if let Some(existing_domain) = wallet_already_has_domain(request.recipient) {
        return Err(format!("Recipient already owns domain: {}", existing_domain));
    }
    
    // Find active season and validate address exists in it
    let active_season_info = REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .values()
            .find(|season| matches!(season.status, SeasonStatus::Active))
            .map(|s| (s.season_id, s.registered_count, s.total_allowed))
    });
    
    let season_id = match active_season_info {
        Some((id, registered, total)) => {
            if registered >= total {
                return Err("Cannot create domain: active season is full".to_string());
            }
            
            // Validate that the address exists in this season
            if !is_address_in_season(id, &request.recipient_address) {
                return Err(format!("Address '{}' is not authorized for the current season", request.recipient_address));
            }
            
            id
        }
        None => return Err("Cannot create domain: no active season available".to_string()),
    };
    
    let canister_id = create_domain_canister(
        &request.domain_name,
        request.recipient,
        request.administrator,
        request.operator,
    ).await?;
    
    let domain_record = DomainRecord {
        owner: request.recipient,
        administrator: request.administrator,
        operator: request.operator,
        canister_id,
        registration_time: time(),
        expiration_time: time() + (365 * 24 * 60 * 60 * 1_000_000_000), // 1 year
        last_payment_block: 0,
        custom_mcp_endpoint: None,
        was_gifted: false, // This is admin creation, not a gift
        registration_season_id: Some(season_id),
    };
    
    DOMAINS.with(|domains| {
        domains.borrow_mut().insert(request.domain_name.clone(), domain_record);
    });
    
    // Add wallet-to-domain mapping for recipient
    WALLET_TO_DOMAIN.with(|mapping| {
        mapping.borrow_mut().insert(request.recipient, request.domain_name.clone());
    });
    
    // Increment season count
    REGISTRATION_SEASONS.with(|seasons| {
        if let Some(season) = seasons.borrow_mut().get_mut(&season_id) {
            season.registered_count += 1;
        }
    });
    complete_season_if_full(season_id);
    
    Ok(format!(
        "Domain {} created for address '{}' and assigned to {} with canister {}",
        request.domain_name, request.recipient_address, request.recipient, canister_id
    ))
}

#[update]
fn admin_add_address_to_season(season_id: u64, address: String) -> Result<(), String> {
    let caller = caller();
    
    if !is_admin(caller) {
        return Err("Only admins can add addresses to seasons".to_string());
    }
    
    add_address_to_season(season_id, address)
}

#[query]
fn get_season_addresses(season_id: u64) -> Vec<String> {
    SEASON_ADDRESSES.with(|addresses| {
        addresses.borrow()
            .get(&season_id)
            .map(|addr_set| addr_set.iter().cloned().collect())
            .unwrap_or_else(Vec::new)
    })
}

#[query]
fn is_address_authorized_for_current_season(address: String) -> bool {
    // Find active season and check if address is in it
    REGISTRATION_SEASONS.with(|seasons| {
        if let Some(active_season) = seasons.borrow()
            .values()
            .find(|season| matches!(season.status, SeasonStatus::Active)) {
            is_address_in_season(active_season.season_id, &address)
        } else {
            false
        }
    })
}

#[update]
async fn renew_domain(domain_name: String, payment_block: u64) -> Result<String, String> {
    let caller = caller();
    
    let mut domain_record = DOMAINS.with(|domains| {
        domains.borrow().get(&domain_name).cloned()
    }).ok_or("Domain not found")?;
    
    if caller != domain_record.owner && caller != domain_record.administrator {
        return Err("Unauthorized".to_string());
    }
    
    let is_admin_caller = is_admin(caller);
    let renewal_fee = calculate_renewal_fee();
    
    // Extend expiration by one year
    domain_record.expiration_time += 365 * 24 * 60 * 60 * 1_000_000_000;
    domain_record.last_payment_block = payment_block;
    
    DOMAINS.with(|domains| {
        domains.borrow_mut().insert(domain_name.clone(), domain_record);
    });
    
    let fee_info = if is_admin_caller {
        "Free (admin renewal)".to_string()
    } else {
        format!("Fee: {} ICP", renewal_fee as f64 / 100_000_000.0)
    };
    
    Ok(format!("Domain {} renewed successfully. {}", domain_name, fee_info))
}

#[update]
async fn set_custom_mcp_endpoint(
    domain_name: String, 
    custom_endpoint: Option<String>
) -> Result<(), String> {
    let caller = caller();
    
    let mut domain_record = DOMAINS.with(|domains| {
        domains.borrow().get(&domain_name).cloned()
    }).ok_or("Domain not found")?;
    
    if caller != domain_record.owner && caller != domain_record.administrator {
        return Err("Unauthorized".to_string());
    }
    
    if let Some(ref endpoint) = custom_endpoint {
        if !endpoint.starts_with("https://") {
            return Err("Custom endpoint must use HTTPS".to_string());
        }
        if endpoint.len() > 200 {
            return Err("Custom endpoint too long".to_string());
        }
    }
    
    domain_record.custom_mcp_endpoint = custom_endpoint;
    
    DOMAINS.with(|domains| {
        domains.borrow_mut().insert(domain_name, domain_record);
    });
    
    Ok(())
}

#[query]
fn get_domain_info(domain_name: String) -> Option<DomainInfo> {
    DOMAINS.with(|domains| {
        domains.borrow().get(&domain_name).map(|domain| {
            let current_time = time();
            let status = if domain.expiration_time > current_time {
                DomainStatus::Active
            } else {
                DomainStatus::Expired
            };
            
            let mcp_endpoint = domain.custom_mcp_endpoint.clone()
                .unwrap_or_else(|| format!("https://mcp.ctx.xyz/{}", domain_name));
            
            DomainInfo {
                name: domain_name.clone(),
                owner: domain.owner,
                administrator: domain.administrator,
                operator: domain.operator,
                canister_id: domain.canister_id,
                expiration_time: domain.expiration_time,
                mcp_endpoint,
                status,
                was_gifted: domain.was_gifted,
            }
        })
    })
}

#[query]
fn get_mcp_endpoint(domain_name: String) -> Option<String> {
    DOMAINS.with(|domains| {
        domains.borrow().get(&domain_name).map(|domain| {
            domain.custom_mcp_endpoint.clone()
                .unwrap_or_else(|| format!("https://mcp.ctx.xyz/{}", domain_name))
        })
    })
}

#[query]
fn list_domains(owner: Option<Principal>) -> Vec<DomainInfo> {
    DOMAINS.with(|domains| {
        domains.borrow()
            .iter()
            .filter(|(_, domain)| {
                match owner {
                    Some(owner_principal) => domain.owner == owner_principal,
                    None => true,
                }
            })
            .map(|(name, domain)| {
                let current_time = time();
                let status = if domain.expiration_time > current_time {
                    DomainStatus::Active
                } else {
                    DomainStatus::Expired
                };
                
                let mcp_endpoint = domain.custom_mcp_endpoint.clone()
                    .unwrap_or_else(|| format!("https://mcp.ctx.xyz/{}", name));
                
                DomainInfo {
                    name: name.clone(),
                    owner: domain.owner,
                    administrator: domain.administrator,
                    operator: domain.operator,
                    canister_id: domain.canister_id,
                    expiration_time: domain.expiration_time,
                    mcp_endpoint,
                    status,
                    was_gifted: domain.was_gifted,
                }
            })
            .collect()
    })
}

#[query]
fn get_registration_fee(domain_name: String) -> u64 {
    if !is_valid_domain_name(&domain_name) {
        return 0;
    }
    if is_reserved_name(&domain_name) {
        return 0;
    }
    calculate_registration_fee(&domain_name).unwrap_or(0)
}

#[query]
fn get_renewal_fee() -> u64 {
    calculate_renewal_fee()
}

#[query]
fn can_register_domain(domain_name: String, user: Principal) -> bool {
    if !is_valid_domain_name(&domain_name) {
        return false;
    }
    if is_reserved_name(&domain_name) {
        return false;
    }
    
    let is_available = DOMAINS.with(|domains| {
        match domains.borrow().get(&domain_name) {
            Some(domain) => {
                let current_time = time();
                domain.expiration_time < current_time
            }
            None => true,
        }
    });
    
    if !is_available {
        return false;
    }
    
    can_register_short_domain(&domain_name, user)
}

#[query]
fn discover_domains(query: String) -> Vec<SearchResult> {
    DOMAINS.with(|domains| {
        domains.borrow()
            .iter()
            .filter(|(name, domain)| {
                let current_time = time();
                domain.expiration_time > current_time &&
                (query.is_empty() || name.to_lowercase().contains(&query.to_lowercase()))
            })
            .map(|(name, domain)| {
                let mcp_endpoint = domain.custom_mcp_endpoint.clone()
                    .unwrap_or_else(|| format!("https://mcp.ctx.xyz/{}", name));
                
                SearchResult {
                    domain: name.clone(),
                    description: format!("Domain {} - {}", name, if domain.was_gifted { "Admin Gift" } else { "Registered" }),
                    mcp_endpoint,
                    tools_count: 0,
                    resources_count: 0,
                    was_gifted: domain.was_gifted,
                }
            })
            .collect()
    })
}

// Admin functions
#[update]
fn add_admin(new_admin: Principal) -> Result<(), String> {
    let caller = caller();
    if !is_admin(caller) {
        return Err("Only admins can add other admins".to_string());
    }
    
    ADMIN_PRINCIPALS.with(|admins| {
        admins.borrow_mut().insert(new_admin);
    });
    
    Ok(())
}

#[update]
fn remove_admin(admin_to_remove: Principal) -> Result<(), String> {
    let caller = caller();
    if !is_admin(caller) {
        return Err("Only admins can remove other admins".to_string());
    }
    
    ADMIN_PRINCIPALS.with(|admins| {
        let mut admin_set = admins.borrow_mut();
        if admin_set.len() <= 1 {
            return Err("Cannot remove the last admin".to_string());
        }
        admin_set.remove(&admin_to_remove);
        Ok(())
    })
}

#[update]
fn add_reserved_name(name: String) -> Result<(), String> {
    let caller = caller();
    if !is_admin(caller) {
        return Err("Only admins can add reserved names".to_string());
    }
    
    RESERVED_NAMES.with(|reserved| {
        reserved.borrow_mut().insert(name);
    });
    
    Ok(())
}

#[update]
fn approve_user_for_short_names(user: Principal) -> Result<(), String> {
    let caller = caller();
    if !is_admin(caller) {
        return Err("Only admins can approve users for short names".to_string());
    }
    
    APPROVED_SHORT_USERS.with(|users| {
        users.borrow_mut().insert(user);
    });
    
    Ok(())
}

#[update]
fn revoke_short_name_approval(user: Principal) -> Result<(), String> {
    let caller = caller();
    if !is_admin(caller) {
        return Err("Only admins can revoke short name approvals".to_string());
    }
    
    APPROVED_SHORT_USERS.with(|users| {
        users.borrow_mut().remove(&user);
    });
    
    Ok(())
}

#[update]
fn set_short_name_mode(mode: RegistrationMode) -> Result<(), String> {
    let caller = caller();
    if !is_admin(caller) {
        return Err("Only admins can change short name mode".to_string());
    }
    
    SHORT_NAME_MODE.with(|current_mode| {
        *current_mode.borrow_mut() = mode;
    });
    
    Ok(())
}

#[update]
fn set_base_fee(new_fee: u64) -> Result<(), String> {
    let caller = caller();
    if !is_admin(caller) {
        return Err("Only admins can set fees".to_string());
    }
    
    BASE_FEE.with(|fee| {
        *fee.borrow_mut() = new_fee;
    });
    
    Ok(())
}

#[update]
fn set_domain_canister_wasm(wasm: Vec<u8>) -> Result<(), String> {
    let caller = caller();
    if !is_admin(caller) {
        return Err("Only admins can set domain canister WASM".to_string());
    }
    
    DOMAIN_CANISTER_WASM.with(|stored_wasm| {
        *stored_wasm.borrow_mut() = wasm;
    });
    
    Ok(())
}

#[query]
fn get_admins() -> Vec<Principal> {
    ADMIN_PRINCIPALS.with(|admins| {
        admins.borrow().iter().cloned().collect()
    })
}

#[query]
fn is_user_admin(user: Principal) -> bool {
    is_admin(user)
}

#[query]
fn get_approved_short_users() -> Vec<Principal> {
    APPROVED_SHORT_USERS.with(|users| {
        users.borrow().iter().cloned().collect()
    })
}

#[query]
fn get_short_name_mode() -> RegistrationMode {
    SHORT_NAME_MODE.with(|mode| mode.borrow().clone())
}

#[update]
fn create_registration_season(request: CreateSeasonRequest) -> Result<u64, String> {
    let caller = caller();
    
    if !is_admin(caller) {
        return Err("Only admins can create registration seasons".to_string());
    }
    
    if request.min_letters == 0 || request.min_letters > 64 {
        return Err("Min letters must be between 1 and 64".to_string());
    }
    
    if let Some(max) = request.max_letters {
        if max < request.min_letters || max > 64 {
            return Err("Max letters must be >= min letters and <= 64".to_string());
        }
    }
    
    if request.total_allowed == 0 {
        return Err("Total allowed must be greater than 0".to_string());
    }
    
    if request.price_icp == 0 {
        return Err("Price must be greater than 0".to_string());
    }
    
    // Check if there's already an active season
    if has_active_season() {
        return Err("Cannot create new season: there is already an active season".to_string());
    }
    
    let season_id = NEXT_SEASON_ID.with(|id| {
        let current_id = *id.borrow();
        *id.borrow_mut() = current_id + 1;
        current_id
    });
    
    let season = RegistrationSeason {
        season_id,
        min_letters: request.min_letters,
        max_letters: request.max_letters,
        total_allowed: request.total_allowed,
        registered_count: 0,
        price_icp: request.price_icp,
        created_by: caller,
        created_at: time(),
        status: SeasonStatus::Active,
    };
    
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow_mut().insert(season_id, season);
    });
    
    Ok(season_id)
}

#[update]
fn deactivate_season(season_id: u64) -> Result<(), String> {
    let caller = caller();
    
    if !is_admin(caller) {
        return Err("Only admins can deactivate seasons".to_string());
    }
    
    REGISTRATION_SEASONS.with(|seasons| {
        let mut seasons_ref = seasons.borrow_mut();
        match seasons_ref.get_mut(&season_id) {
            Some(season) => {
                season.status = SeasonStatus::Deactivated;
                Ok(())
            }
            None => Err("Season not found".to_string())
        }
    })
}

#[query]
fn get_registration_season(season_id: u64) -> Option<RegistrationSeason> {
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow().get(&season_id).cloned()
    })
}

#[query]
fn get_active_seasons() -> Vec<RegistrationSeason> {
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .values()
            .filter(|season| matches!(season.status, SeasonStatus::Active))
            .cloned()
            .collect()
    })
}

#[query]
fn get_all_seasons() -> Vec<RegistrationSeason> {
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .values()
            .cloned()
            .collect()
    })
}

#[query]
fn get_applicable_season_for_domain(domain_name: String) -> Option<RegistrationSeason> {
    find_applicable_season(&domain_name).map(|(_, season)| season)
}

#[query]
fn get_season_stats(season_id: u64) -> Option<SeasonStats> {
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow().get(&season_id).map(|season| {
            SeasonStats {
                season_number: season.season_id,
                names_available: season.total_allowed,
                names_taken: season.registered_count,
                price_icp: season.price_icp,
                status: season.status.clone(),
            }
        })
    })
}

#[query]
fn get_season_by_number(season_number: u64) -> Option<RegistrationSeason> {
    if season_number == 0 {
        // Return the latest season (highest ID)
        REGISTRATION_SEASONS.with(|seasons| {
            seasons.borrow()
                .values()
                .max_by_key(|season| season.season_id)
                .cloned()
        })
    } else {
        // Return specific season by ID
        REGISTRATION_SEASONS.with(|seasons| {
            seasons.borrow().get(&season_number).cloned()
        })
    }
}

#[query]
fn get_season_stats_by_number(season_number: u64) -> Option<SeasonStats> {
    if let Some(season) = get_season_by_number(season_number) {
        Some(SeasonStats {
            season_number: season.season_id,
            names_available: season.total_allowed,
            names_taken: season.registered_count,
            price_icp: season.price_icp,
            status: season.status.clone(),
        })
    } else {
        None
    }
}

#[query]
fn get_all_season_stats() -> Vec<SeasonStats> {
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .values()
            .map(|season| SeasonStats {
                season_number: season.season_id,
                names_available: season.total_allowed,
                names_taken: season.registered_count,
                price_icp: season.price_icp,
                status: season.status.clone(),
            })
            .collect()
    })
}

#[query]
fn get_current_season() -> Option<RegistrationSeason> {
    REGISTRATION_SEASONS.with(|seasons| {
        seasons.borrow()
            .values()
            .find(|season| matches!(season.status, SeasonStatus::Active))
            .cloned()
    })
}

#[query]
fn get_wallet_domain(wallet: Principal) -> Option<String> {
    wallet_already_has_domain(wallet)
}

#[update]
fn transfer_domain_ownership(domain_name: String, new_owner: Principal) -> Result<(), String> {
    let caller = caller();
    
    // Get the current domain record
    let mut domain_record = DOMAINS.with(|domains| {
        domains.borrow().get(&domain_name).cloned()
    }).ok_or("Domain not found")?;
    
    // Check authorization - only current owner or administrator can transfer
    if caller != domain_record.owner && caller != domain_record.administrator {
        return Err("Unauthorized: only domain owner or administrator can transfer ownership".to_string());
    }
    
    // Check if new owner already has a domain (unless admin)
    if !is_admin(new_owner) {
        if let Some(existing_domain) = wallet_already_has_domain(new_owner) {
            return Err(format!("New owner already owns domain: {}", existing_domain));
        }
    }
    
    let old_owner = domain_record.owner;
    
    // Update domain record
    domain_record.owner = new_owner;
    
    // Save updated domain record
    DOMAINS.with(|domains| {
        domains.borrow_mut().insert(domain_name.clone(), domain_record);
    });
    
    // Update wallet-to-domain mappings
    WALLET_TO_DOMAIN.with(|mapping| {
        let mut map = mapping.borrow_mut();
        // Remove old owner's mapping
        map.remove(&old_owner);
        // Add new owner's mapping
        map.insert(new_owner, domain_name);
    });
    
    Ok(())
}

#[query]
fn get_domains_since_timestamp(timestamp: u64) -> Vec<(String, DomainInfo)> {
    DOMAINS.with(|domains| {
        domains.borrow()
            .iter()
            .filter(|(_, record)| record.registration_time > timestamp)
            .map(|(name, record)| {
                let mcp_endpoint = record.custom_mcp_endpoint.clone()
                    .unwrap_or_else(|| format!("https://mcp.ctx.xyz/{}", name));
                
                let status = if record.expiration_time > time() {
                    DomainStatus::Active
                } else {
                    DomainStatus::Expired
                };
                
                let info = DomainInfo {
                    name: name.clone(),
                    owner: record.owner,
                    administrator: record.administrator,
                    operator: record.operator,
                    canister_id: record.canister_id,
                    expiration_time: record.expiration_time,
                    mcp_endpoint,
                    status,
                    was_gifted: record.was_gifted,
                };
                
                (name.clone(), info)
            })
            .collect()
    })
}

#[query]
fn get_all_domains_with_timestamps() -> Vec<(String, u64, DomainInfo)> {
    DOMAINS.with(|domains| {
        domains.borrow()
            .iter()
            .map(|(name, record)| {
                let mcp_endpoint = record.custom_mcp_endpoint.clone()
                    .unwrap_or_else(|| format!("https://mcp.ctx.xyz/{}", name));
                
                let status = if record.expiration_time > time() {
                    DomainStatus::Active
                } else {
                    DomainStatus::Expired
                };
                
                let info = DomainInfo {
                    name: name.clone(),
                    owner: record.owner,
                    administrator: record.administrator,
                    operator: record.operator,
                    canister_id: record.canister_id,
                    expiration_time: record.expiration_time,
                    mcp_endpoint,
                    status,
                    was_gifted: record.was_gifted,
                };
                
                (name.clone(), record.registration_time, info)
            })
            .collect()
    })
}

// Helper function to create domain canister (simplified for now)
async fn create_domain_canister(
    _domain_name: &str,
    _owner: Principal,
    _administrator: Principal,
    _operator: Principal,
) -> Result<Principal, String> {
    // For testing, return a dummy principal instead of creating actual canister
    // This avoids the cycles issue
    Ok(Principal::from_text("aaaaa-aa").unwrap())
}

ic_cdk::export_candid!();