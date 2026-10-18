use crate::crypt::{Permissions, Protection};
use crate::document::Document;
use crate::error::{Error, Result};
use crate::save::Encryption;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtectionState {
    pub encrypted: bool,
    pub permissions: Permissions,
    pub owner_rights: bool,
}

impl Document {
    pub fn protection(&self) -> ProtectionState {
        let source = self.store.source().security();
        let owner_rights = self.owner_rights || source.is_none_or(|security| security.has_owner_rights());
        let (encrypted, permissions) = match &self.encryption {
            Some(Encryption::Sealed { handler, .. }) => (true, handler.permissions()),
            Some(Encryption::Plain) => (false, Permissions::ALL),
            Some(Encryption::Source) | None => {
                (source.is_some(), source.map_or(Permissions::ALL, |security| security.permissions()))
            }
        };
        ProtectionState { encrypted, permissions, owner_rights }
    }

    pub fn grant_owner_rights(&mut self, password: &str) -> bool {
        self.owner_rights = self.owner_rights || self.store.source().is_owner_password(password);
        self.protection().owner_rights
    }

    pub fn protect(&mut self, protection: &Protection, entropy: &[u8]) -> Result<()> {
        self.require_owner_rights()?;
        if protection.user_password.is_empty() && protection.owner_password.is_empty() {
            return Err(Error::new("protection needs a password to open or one to change permissions"));
        }
        let (dictionary, handler) = protection.seal(entropy)?;
        self.encryption = Some(Encryption::Sealed { dictionary, handler });
        Ok(())
    }

    pub fn unprotect(&mut self) -> Result<()> {
        self.require_owner_rights()?;
        self.encryption = Some(Encryption::Plain);
        Ok(())
    }

    fn require_owner_rights(&self) -> Result<()> {
        if self.protection().owner_rights {
            Ok(())
        } else {
            Err(Error::needs_password("changing the protection needs the owner password"))
        }
    }
}
