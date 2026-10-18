use serde::{Deserialize, Serialize};
use tintenfisch::crypt::{Permissions, Protection};
use wasm_bindgen::prelude::*;

use crate::engine::Engine;
use crate::replies::{fail, reply};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PermissionsMessage {
    print: bool,
    print_faithful: bool,
    modify: bool,
    copy: bool,
    annotate: bool,
    fill_forms: bool,
    assemble: bool,
}

impl From<Permissions> for PermissionsMessage {
    fn from(permissions: Permissions) -> PermissionsMessage {
        let Permissions { print, print_faithful, modify, copy, annotate, fill_forms, assemble } = permissions;
        PermissionsMessage { print, print_faithful, modify, copy, annotate, fill_forms, assemble }
    }
}

impl From<PermissionsMessage> for Permissions {
    fn from(message: PermissionsMessage) -> Permissions {
        let PermissionsMessage { print, print_faithful, modify, copy, annotate, fill_forms, assemble } = message;
        Permissions { print, print_faithful, modify, copy, annotate, fill_forms, assemble }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProtectionReply {
    encrypted: bool,
    owner_rights: bool,
    permissions: PermissionsMessage,
}

#[wasm_bindgen]
impl Engine {
    pub fn protection(&self) -> Result<JsValue, JsError> {
        let state = self.document()?.protection();
        reply(&ProtectionReply {
            encrypted: state.encrypted,
            owner_rights: state.owner_rights,
            permissions: state.permissions.into(),
        })
    }

    #[wasm_bindgen(js_name = grantOwnerRights)]
    pub fn grant_owner_rights(&mut self, password: &str) -> Result<bool, JsError> {
        Ok(self.document_mut()?.grant_owner_rights(password))
    }

    pub fn protect(
        &mut self,
        user_password: &str,
        owner_password: &str,
        permissions: JsValue,
        entropy: &[u8],
    ) -> Result<(), JsError> {
        let permissions: PermissionsMessage = serde_wasm_bindgen::from_value(permissions)?;
        let protection = Protection {
            user_password: user_password.to_string(),
            owner_password: owner_password.to_string(),
            permissions: permissions.into(),
        };
        self.document_mut()?.protect(&protection, entropy).map_err(fail)
    }

    pub fn unprotect(&mut self) -> Result<(), JsError> {
        self.document_mut()?.unprotect().map_err(fail)
    }
}
