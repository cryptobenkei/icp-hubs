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

thread_local! {
    static DOMAINS: RefCell<HashMap<String, DomainRecord>> = RefCell::new(HashMap::new());
    static RESERVED_NAMES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static ADMIN_PRINCIPALS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::new());
    static SHORT_NAME_MODE: RefCell<RegistrationMode> = RefCell::new(RegistrationMode::WhitelistOnly);
    static APPROVED_SHORT_USERS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::new());
    static BASE_FEE: RefCell<u64> = RefCell::new(100_000_000);
    static DOMAIN_CANISTER_WASM: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

fn calculate_registration_fee(domain_name: &str) -> u64 {
    BASE_FEE.with(|base| {
        let base_fee = *base.borrow();
        let length = domain_name.len();
        
        match length {
            1 => base_fee * 100,      // 100 ICP for 1 character
            2 => base_fee * 50,       // 50 ICP for 2 characters  
            3 => base_fee * 20,       // 20 ICP for 3 characters
            4 => base_fee * 10,       // 10 ICP for 4 characters
            5..=8 => base_fee * 5,    // 5 ICP for 5-8 characters
            9..=12 => base_fee * 2,   // 2 ICP for 9-12 characters
            _ => base_fee,            // 1 ICP for 13+ characters
        }
    })
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
    
    let required_fee = calculate_registration_fee(&request.domain_name);
    let is_admin_caller = is_admin(caller);
    
    // Create new canister for this domain
    let canister_id = create_domain_canister(
        &request.domain_name, 
        caller, 
        request.administrator, 
        request.operator
    ).await?;
    
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
    };
    
    DOMAINS.with(|domains| {
        domains.borrow_mut().insert(request.domain_name.clone(), domain_record);
    });
    
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
    };
    
    DOMAINS.with(|domains| {
        domains.borrow_mut().insert(request.domain_name.clone(), domain_record);
    });
    
    Ok(format!(
        "Domain {} gifted to {} with canister {} (FREE admin gift)",
        request.domain_name, request.recipient, canister_id
    ))
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
    calculate_registration_fee(&domain_name)
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

// Helper function to create domain canister (simplified for now)
async fn create_domain_canister(
    _domain_name: &str,
    owner: Principal,
    administrator: Principal,
    _operator: Principal,
) -> Result<Principal, String> {
    let settings = CanisterSettings {
        controllers: Some(vec![id(), owner, administrator]),
        compute_allocation: None,
        memory_allocation: None,
        freezing_threshold: None,
        reserved_cycles_limit: None, // Added required field
    };
    
    let create_args = CreateCanisterArgument {
        settings: Some(settings),
    };
    
    // Fixed: create_canister now takes cycles as second parameter
    match create_canister(create_args, 1_000_000_000_000u128).await {
        Ok((canister_id_record,)) => {
            Ok(canister_id_record.canister_id)
        }
        Err(e) => Err(format!("Failed to create canister: {:?}", e)),
    }
}

ic_cdk::export_candid!();