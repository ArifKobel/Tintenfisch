use crate::document::Document;
use crate::document::document_form_design::ensure_form;
use crate::error::{Error, Result};
use crate::forms::FieldKind;
use crate::save::SaveMode;
use crate::signing::{
    CmsCheck, CmsSignature, DigestAlgorithm, SignaturePlaceholder, SignatureReport, SignatureStatus, SigningDetails,
    SigningIdentity,
};
use crate::syntax::{Dict, Name, ObjId, Object, decode_text, encode_text};

const SIGNATURES_EXIST_APPEND_ONLY: i64 = 3;
const ROOM_TO_SPARE: usize = 64;
const DETACHED: [&str; 2] = ["ETSI.CAdES.detached", "adbe.pkcs7.detached"];

impl Document {
    pub fn sign_field(
        &mut self,
        field: ObjId,
        identity: &SigningIdentity,
        details: &SigningDetails,
    ) -> Result<Vec<u8>> {
        if self.protection().encrypted {
            return Err(Error::unsupported("encrypted documents cannot be signed yet"));
        }
        let target = self
            .fields()?
            .into_iter()
            .find(|candidate| candidate.id == field)
            .ok_or_else(|| Error::new("no such field"))?;
        if target.kind != FieldKind::Signature {
            return Err(Error::unsupported(format!("{} is not a signature field", target.name)));
        }
        if self.store.object(field)?.as_dict().is_some_and(|dict| dict.get("V").is_some()) {
            return Err(Error::unsupported(format!("{} is signed already", target.name)));
        }
        let capacity = CmsSignature::create(identity, &[0; 32])?.len() + ROOM_TO_SPARE;
        let base = self.save(false)?;
        let base_length = base.len();
        let mut clean = Document::open(base, "")?;
        clean.store.transact("Sign", "", |transaction| {
            let mut signature: Dict = [
                ("Type", Object::Name(Name::from("Sig"))),
                ("Filter", Object::Name(Name::from("Adobe.PPKLite"))),
                ("SubFilter", Object::Name(Name::from(DETACHED[0]))),
                ("ByteRange", SignaturePlaceholder::byte_range()),
                ("Contents", SignaturePlaceholder::contents(capacity)),
                ("M", Object::String(encode_text(&details.date))),
                ("Name", Object::String(encode_text(&identity.signer_name()))),
            ]
            .into_iter()
            .map(|(key, value)| (Name::from(key), value))
            .collect();
            for (key, value) in [("Reason", &details.reason), ("Location", &details.location)] {
                if !value.is_empty() {
                    signature.insert(key, Object::String(encode_text(value)));
                }
            }
            let signature = transaction.allocate(Object::Dict(signature));
            let mut dict = transaction
                .object(field)?
                .as_dict()
                .cloned()
                .ok_or_else(|| Error::new("a field is not a dictionary"))?;
            dict.insert("V", Object::Ref(signature));
            transaction.put(field, Object::Dict(dict))?;
            let form = ensure_form(transaction)?;
            let mut form_dict = transaction.object(form)?.as_dict().cloned().unwrap_or_default();
            form_dict.insert("SigFlags", Object::Int(SIGNATURES_EXIST_APPEND_ONLY));
            transaction.put(form, Object::Dict(form_dict))
        })?;
        let mut bytes = SaveMode::Incremental.write(&clean.store)?;
        let placeholder = SignaturePlaceholder::find(&bytes, base_length, capacity)?;
        let digest = {
            let [head, tail] = placeholder.write_byte_range(&mut bytes)?;
            DigestAlgorithm::Sha256.digest(&[head, tail])
        };
        placeholder.write_contents(&mut bytes, &CmsSignature::create(identity, &digest)?)?;
        *self = Document::open(bytes.clone(), "")?;
        Ok(bytes)
    }

    pub fn signatures(&self) -> Result<Vec<SignatureReport>> {
        let bytes = self.store.source().bytes();
        let mut reports = Vec::new();
        for field in self.fields()?.into_iter().filter(|field| field.kind == FieldKind::Signature) {
            let dict = self.store.object(field.id)?.as_dict().cloned().unwrap_or_default();
            let Some(value) = dict.get("V") else { continue };
            let signature = self.store.resolve(value)?.as_dict().cloned().unwrap_or_default();
            let text = |key: &str| signature.get(key).and_then(Object::as_string).map(decode_text).unwrap_or_default();
            let range: Vec<usize> = signature
                .get("ByteRange")
                .and_then(Object::as_array)
                .unwrap_or_default()
                .iter()
                .filter_map(|number| number.as_int().and_then(|number| usize::try_from(number).ok()))
                .collect();
            let contents = signature.get("Contents").and_then(Object::as_string).unwrap_or_default();
            let sub_filter = signature
                .name("SubFilter")
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                .unwrap_or_default();
            let (status, signer, end) = match range[..] {
                [0, before, after, length]
                    if before < after
                        && after.checked_add(length).is_some_and(|end| end <= bytes.len())
                        && bytes[before] == b'<'
                        && bytes[after - 1] == b'>' =>
                {
                    if DETACHED.contains(&sub_filter.as_str()) {
                        let check = CmsCheck::run(contents, &[&bytes[..before], &bytes[after..after + length]]);
                        (check.status, check.signer, after + length)
                    } else {
                        (
                            SignatureStatus::Unsupported(format!("{sub_filter} signatures cannot be checked yet")),
                            None,
                            after + length,
                        )
                    }
                }
                _ => (SignatureStatus::Broken("the signed byte range is not valid".to_string()), None, bytes.len()),
            };
            reports.push(SignatureReport {
                field: field.id,
                field_name: field.name,
                signer: signer.or_else(|| Some(text("Name")).filter(|name| !name.is_empty())),
                signed_at: text("M"),
                reason: text("Reason"),
                status,
                changed_after: bytes[end..].iter().any(|byte| !byte.is_ascii_whitespace()),
            });
        }
        Ok(reports)
    }
}
