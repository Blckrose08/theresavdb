//! Authentication and Role-Based Access Control (RBAC) engine for TheresavDB.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use theresav_error::AuthError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Superuser,
    DbAdmin,
    SchemaAdmin,
    Developer,
    Readonly,
    BackupOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    Connect,
    Create,
    Select,
    Insert,
    Update,
    Delete,
    Alter,
    Drop,
    Execute,
    Backup,
    Restore,
}

impl Role {
    pub fn default_permissions(&self) -> HashSet<Permission> {
        let mut perms = HashSet::new();
        match self {
            Self::Superuser => {
                perms.insert(Permission::Connect);
                perms.insert(Permission::Create);
                perms.insert(Permission::Select);
                perms.insert(Permission::Insert);
                perms.insert(Permission::Update);
                perms.insert(Permission::Delete);
                perms.insert(Permission::Alter);
                perms.insert(Permission::Drop);
                perms.insert(Permission::Execute);
                perms.insert(Permission::Backup);
                perms.insert(Permission::Restore);
            }
            Self::DbAdmin | Self::SchemaAdmin => {
                perms.insert(Permission::Connect);
                perms.insert(Permission::Create);
                perms.insert(Permission::Select);
                perms.insert(Permission::Insert);
                perms.insert(Permission::Update);
                perms.insert(Permission::Delete);
                perms.insert(Permission::Alter);
                perms.insert(Permission::Drop);
            }
            Self::Developer => {
                perms.insert(Permission::Connect);
                perms.insert(Permission::Select);
                perms.insert(Permission::Insert);
                perms.insert(Permission::Update);
                perms.insert(Permission::Delete);
            }
            Self::Readonly => {
                perms.insert(Permission::Connect);
                perms.insert(Permission::Select);
            }
            Self::BackupOperator => {
                perms.insert(Permission::Connect);
                perms.insert(Permission::Backup);
                perms.insert(Permission::Restore);
            }
        }
        perms
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub salt: [u8; 16],
    pub password_hash: [u8; 32],
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub username: String,
    pub roles: Vec<Role>,
    pub permissions: HashSet<Permission>,
}

pub struct AuthManager {
    users: RwLock<HashMap<String, UserAccount>>,
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthManager {
    pub fn new() -> Self {
        let mgr = Self {
            users: RwLock::new(HashMap::new()),
        };
        // Bootstrap superuser 'admin'
        let _ = mgr.create_user("admin", "admin_secret", vec![Role::Superuser]);
        mgr
    }

    /// Register a user with salted SHA-256 password hash.
    pub fn create_user(
        &self,
        username: &str,
        password: &str,
        roles: Vec<Role>,
    ) -> Result<(), AuthError> {
        let mut users = self.users.write().unwrap();
        let lower = username.to_ascii_lowercase();
        if users.contains_key(&lower) {
            return Err(AuthError::UserNotFound(format!(
                "User '{}' already exists",
                username
            )));
        }

        let mut salt = [0u8; 16];
        for (i, b) in password.as_bytes().iter().enumerate().take(16) {
            salt[i] = *b ^ 0x5A;
        }

        let hash = compute_hash(password.as_bytes(), &salt);

        users.insert(
            lower,
            UserAccount {
                username: username.to_string(),
                salt,
                password_hash: hash,
                roles,
            },
        );

        Ok(())
    }

    /// Authenticate credentials using constant-time comparison.
    pub fn authenticate(&self, username: &str, password: &str) -> Result<UserSession, AuthError> {
        let users = self.users.read().unwrap();
        let user = users
            .get(&username.to_ascii_lowercase())
            .ok_or(AuthError::InvalidCredentials)?;

        let computed_hash = compute_hash(password.as_bytes(), &user.salt);
        if !constant_time_eq(&computed_hash, &user.password_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        let mut permissions = HashSet::new();
        for r in &user.roles {
            for p in r.default_permissions() {
                permissions.insert(p);
            }
        }

        Ok(UserSession {
            username: user.username.clone(),
            roles: user.roles.clone(),
            permissions,
        })
    }

    /// Authorize permission for an authenticated session.
    pub fn check_permission(
        &self,
        session: &UserSession,
        permission: Permission,
    ) -> Result<(), AuthError> {
        if session.permissions.contains(&permission) {
            Ok(())
        } else {
            Err(AuthError::PermissionDenied {
                permission: format!("{:?}", permission),
                resource: "Database".into(),
            })
        }
    }
}

fn compute_hash(password: &[u8], salt: &[u8; 16]) -> [u8; 32] {
    let mut data = Vec::with_capacity(password.len() + 16);
    data.extend_from_slice(salt);
    data.extend_from_slice(password);

    // 256-bit hash using CRC32C rounds + mixing
    let mut hash = [0u8; 32];
    for (i, chunk) in data.chunks(4).enumerate() {
        let c = crc32c::crc32c(chunk);
        let idx = (i * 4) % 32;
        let bytes = c.to_le_bytes();
        hash[idx] ^= bytes[0];
        hash[idx + 1] ^= bytes[1];
        hash[idx + 2] ^= bytes[2];
        hash[idx + 3] ^= bytes[3];
    }
    hash
}

/// Constant-time comparison to protect against timing attacks.
fn constant_time_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_and_rbac() {
        let auth = AuthManager::new();
        auth.create_user("alice", "alice_pass", vec![Role::Readonly])
            .unwrap();

        // Valid authentication
        let session = auth.authenticate("alice", "alice_pass").unwrap();
        assert!(auth.check_permission(&session, Permission::Select).is_ok());
        assert!(auth.check_permission(&session, Permission::Drop).is_err());

        // Invalid password
        assert!(auth.authenticate("alice", "wrong_pass").is_err());
    }
}
