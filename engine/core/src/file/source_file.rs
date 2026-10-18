use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::crypt::SecurityHandler;
use crate::error::{Error, Result};
use crate::file::filters::{Decoded, FilterChain};
use crate::file::object_stream::ObjectStream;
use crate::file::repair::Repair;
use crate::file::xref::{Xref, XrefEntry};
use crate::file::xref_reader::XrefReader;
use crate::file::{Header, read_header};
use crate::syntax::{Dict, ObjId, Object, Parser, Stream};

pub struct SourceFile {
    bytes: Rc<[u8]>,
    header: Header,
    xref: Xref,
    repaired: bool,
    security: Option<SecurityHandler>,
    fallback: OnceCell<Option<Xref>>,
    objects: RefCell<HashMap<ObjId, Rc<Object>>>,
    object_streams: RefCell<HashMap<u32, Rc<ObjectStream>>>,
}

impl SourceFile {
    pub fn open(bytes: impl Into<Rc<[u8]>>) -> Result<SourceFile> {
        SourceFile::open_with_password(bytes, "")
    }

    pub fn open_with_password(bytes: impl Into<Rc<[u8]>>, password: &str) -> Result<SourceFile> {
        let bytes = bytes.into();
        let header = read_header(&bytes)?;
        if let Ok(xref) = XrefReader::new(&bytes).read() {
            let mut file = SourceFile::with_xref(bytes.clone(), header, xref, false);
            file.unlock(password)?;
            if file.catalog().is_ok() {
                return Ok(file);
            }
        }
        let mut file = SourceFile::with_xref(bytes.clone(), header, Repair::new(&bytes).scan()?, true);
        file.unlock(password)?;
        file.catalog()?;
        Ok(file)
    }

    fn unlock(&mut self, password: &str) -> Result<()> {
        if self.trailer().get("Encrypt").is_none() {
            return Ok(());
        }
        self.security = Some(self.handler_for(password)?);
        self.objects.borrow_mut().clear();
        self.object_streams.borrow_mut().clear();
        Ok(())
    }

    pub fn is_owner_password(&self, password: &str) -> bool {
        self.handler_for(password).is_ok_and(|handler| handler.has_owner_rights())
    }

    fn handler_for(&self, password: &str) -> Result<SecurityHandler> {
        let entry = self.trailer().get("Encrypt").cloned().ok_or_else(|| Error::new("the file is not encrypted"))?;
        let dictionary = match &entry {
            Object::Ref(id) => self.object(*id)?.as_dict().cloned(),
            other => other.as_dict().cloned(),
        }
        .ok_or_else(|| Error::new("/Encrypt is not a dictionary"))?;
        let first_id = self
            .trailer()
            .get("ID")
            .and_then(Object::as_array)
            .and_then(|parts| parts.first())
            .and_then(Object::as_string)
            .unwrap_or_default()
            .to_vec();
        SecurityHandler::unlock(&dictionary, entry.as_reference(), &first_id, password)
    }

    pub fn security(&self) -> Option<&SecurityHandler> {
        self.security.as_ref()
    }

    fn with_xref(bytes: Rc<[u8]>, header: Header, xref: Xref, repaired: bool) -> SourceFile {
        SourceFile {
            bytes,
            header,
            xref,
            repaired,
            security: None,
            fallback: OnceCell::new(),
            objects: RefCell::new(HashMap::new()),
            object_streams: RefCell::new(HashMap::new()),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn header(&self) -> Header {
        self.header
    }

    pub fn xref(&self) -> &Xref {
        &self.xref
    }

    pub fn trailer(&self) -> &Dict {
        self.xref.trailer()
    }

    pub fn is_repaired(&self) -> bool {
        self.repaired
    }

    pub fn ids(&self) -> impl Iterator<Item = ObjId> + '_ {
        self.xref.ids()
    }

    pub fn catalog(&self) -> Result<Rc<Object>> {
        let root = self
            .trailer()
            .get("Root")
            .and_then(Object::as_reference)
            .ok_or_else(|| Error::new("trailer has no /Root"))?;
        let catalog = self.object(root)?;
        match catalog.as_dict() {
            Some(_) => Ok(catalog),
            None => Err(Error::new("/Root is not a dictionary")),
        }
    }

    pub fn object(&self, id: ObjId) -> Result<Rc<Object>> {
        if let Some(object) = self.objects.borrow().get(&id) {
            return Ok(object.clone());
        }
        let object = Rc::new(self.load(id)?);
        self.objects.borrow_mut().insert(id, object.clone());
        Ok(object)
    }

    pub fn resolve(&self, object: &Object) -> Result<Object> {
        match object {
            Object::Ref(id) => Ok((*self.object(*id)?).clone()),
            other => Ok(other.clone()),
        }
    }

    pub fn decode_stream(&self, stream: &Stream) -> Result<Decoded> {
        let chain = FilterChain::of(&stream.dict, |object| self.resolve(object).unwrap_or(Object::Null))?;
        chain.decode(&stream.raw)
    }

    fn load(&self, id: ObjId) -> Result<Object> {
        let Some(entry) = self.xref.entry(id.num) else {
            return Ok(Object::Null);
        };
        if entry.generation() != id.generation && !entry.is_free() {
            return Ok(Object::Null);
        }
        let loaded = match entry {
            XrefEntry::Free { .. } => return Ok(Object::Null),
            XrefEntry::InUse { offset, .. } => self.parse_at(offset, id),
            XrefEntry::Compressed { stream, index } => self.compressed(id.num, stream, index as usize),
        };
        loaded.or_else(|error| self.load_from_fallback(id).unwrap_or(Err(error)))
    }

    fn parse_at(&self, offset: usize, id: ObjId) -> Result<Object> {
        if offset >= self.bytes.len() {
            return Err(Error::at(offset, format!("object {id:?} points outside the file")));
        }
        let length = |length_id: ObjId| {
            (length_id != id)
                .then(|| self.object(length_id).ok())
                .flatten()
                .and_then(|length| length.as_int())
                .and_then(|length| usize::try_from(length).ok())
        };
        let (found, object) = Parser::new(&self.bytes, offset).parse_indirect(length)?;
        if found.num != id.num {
            return Err(Error::at(offset, format!("expected object {id:?}, found {found:?}")));
        }
        Ok(match &self.security {
            Some(security) if !security.is_dictionary(id) => security.decrypt(id, object),
            _ => object,
        })
    }

    fn compressed(&self, num: u32, stream_num: u32, index: usize) -> Result<Object> {
        let objects = self.object_stream(stream_num)?;
        let index = match objects.object(index) {
            Ok((member, _)) if member == num => index,
            _ => objects
                .index_of(num)
                .ok_or_else(|| Error::new(format!("object {num} is not in object stream {stream_num}")))?,
        };
        Ok(objects.object(index)?.1)
    }

    fn object_stream(&self, num: u32) -> Result<Rc<ObjectStream>> {
        if let Some(objects) = self.object_streams.borrow().get(&num) {
            return Ok(objects.clone());
        }
        let container = self.object(ObjId::new(num, 0))?;
        let stream = container.as_stream().ok_or_else(|| Error::new(format!("object stream {num} is not a stream")))?;
        let objects = Rc::new(ObjectStream::parse(&stream.dict, self.decode_stream(stream)?.data)?);
        self.object_streams.borrow_mut().insert(num, objects.clone());
        Ok(objects)
    }

    fn load_from_fallback(&self, id: ObjId) -> Option<Result<Object>> {
        if self.repaired {
            return None;
        }
        let fallback = self.fallback.get_or_init(|| Repair::new(&self.bytes).scan().ok()).as_ref()?;
        match fallback.entry(id.num)? {
            XrefEntry::InUse { offset, generation } if generation == id.generation => Some(self.parse_at(offset, id)),
            XrefEntry::Compressed { stream, index } => Some(self.compressed(id.num, stream, index as usize)),
            _ => None,
        }
    }
}
