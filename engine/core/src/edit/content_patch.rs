use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::content::{ContentStream, Operation, Segment, Splice};
use crate::error::{Error, Result};
use crate::pages::{Page, PageTree, drawn_forms};
use crate::store::{ObjectStore, Transaction};
use crate::syntax::{Dict, Name, ObjId, Object, Stream};

#[derive(Clone, Debug, PartialEq)]
pub enum EditTarget {
    PageContent,
    Form { id: ObjId, copy_as: Option<Name> },
}

impl EditTarget {
    pub fn for_scope(store: &ObjectStore, page: &Page, scope: Option<ObjId>) -> Result<EditTarget> {
        let Some(id) = scope else { return Ok(EditTarget::PageContent) };
        let shared = PageTree::new(store)
            .pages()?
            .iter()
            .filter(|other| other.id != page.id)
            .any(|other| drawn_forms(store, &other.resources).contains(&id));
        if !shared {
            return Ok(EditTarget::Form { id, copy_as: None });
        }
        let xobjects = match page.resources.get("XObject") {
            Some(xobjects) => store.resolve(xobjects)?.as_dict().cloned().unwrap_or_default(),
            None => Dict::new(),
        };
        let name = xobjects
            .iter()
            .find(|(_, entry)| entry.as_reference() == Some(id))
            .map(|(name, _)| name.clone())
            .ok_or_else(|| {
                Error::unsupported("a form XObject shared with other pages through another form cannot be edited yet")
            })?;
        Ok(EditTarget::Form { id, copy_as: Some(name) })
    }
}

#[derive(Clone)]
pub struct ContentPatch {
    pub page: Page,
    pub target: EditTarget,
    pub replacements: Vec<(usize, Vec<u8>)>,
    pub appended: Option<Vec<u8>>,
}

impl ContentPatch {
    pub fn is_empty(&self) -> bool {
        self.replacements.is_empty() && self.appended.is_none()
    }

    pub fn apply(
        &self,
        transaction: &mut Transaction,
        page: &mut Dict,
        resources: &mut Dict,
        fonts: &[(Name, ObjId)],
    ) -> Result<()> {
        match &self.target {
            EditTarget::PageContent => {
                for (name, font) in fonts {
                    add_font(transaction, resources, name, *font)?;
                }
                self.splice_page_content(transaction, page)
            }
            EditTarget::Form { id, copy_as } => {
                let form = self.spliced_form(transaction, *id, fonts, resources)?;
                match copy_as {
                    Some(name) => {
                        let copy = transaction.allocate(form);
                        let mut xobjects = match resources.get("XObject") {
                            Some(xobjects) => transaction.resolve(xobjects)?.as_dict().cloned().unwrap_or_default(),
                            None => Dict::new(),
                        };
                        xobjects.insert(name.clone(), Object::Ref(copy));
                        resources.insert("XObject", Object::Dict(xobjects));
                        Ok(())
                    }
                    None => transaction.put(*id, form),
                }
            }
        }
    }

    fn splice_page_content(&self, transaction: &mut Transaction, page: &mut Dict) -> Result<()> {
        let content = ContentStream::load(transaction.store(), &self.page)?;
        let mut splices: BTreeMap<usize, Splice> = BTreeMap::new();
        for (ordinal, bytes) in &self.replacements {
            let operation = find_operation(&content, *ordinal)?;
            splices.entry(operation.segment).or_default().replace(operation.span.clone(), bytes.clone());
        }
        let shared = streams_used_elsewhere(transaction.store(), self.page.id.0)?;
        let mut contents = page_contents(transaction, page)?;
        for (segment, splice) in splices {
            let source = &content.segments[segment];
            let original = transaction.object(source.stream)?;
            let stream = rewritten_stream(&original, splice.apply(&source.data)?);
            if shared.contains(&source.stream) {
                let copy = transaction.allocate(stream);
                for entry in contents.iter_mut().filter(|entry| **entry == source.stream) {
                    *entry = copy;
                }
            } else {
                transaction.put(source.stream, stream)?;
            }
        }
        if let Some(appended) = &self.appended {
            if leaves_state_changed(&content) {
                contents.insert(0, transaction.allocate(plain_stream(b"q\n".to_vec())));
                contents.push(transaction.allocate(plain_stream(b"Q\n".to_vec())));
            }
            contents.push(transaction.allocate(plain_stream(appended.clone())));
        }
        page.insert(
            "Contents",
            match contents.as_slice() {
                [single] => Object::Ref(*single),
                several => Object::Array(several.iter().map(|id| Object::Ref(*id)).collect()),
            },
        );
        Ok(())
    }

    fn spliced_form(
        &self,
        transaction: &mut Transaction,
        id: ObjId,
        fonts: &[(Name, ObjId)],
        page_resources: &mut Dict,
    ) -> Result<Object> {
        let original = transaction.object(id)?;
        let stream = original.as_stream().ok_or_else(|| Error::new("a form XObject is not a stream"))?;
        let data = transaction.store().decode_stream(stream)?.data;
        let content = ContentStream::parse(vec![Segment { stream: id, data }]);
        let mut splice = Splice::default();
        for (ordinal, bytes) in &self.replacements {
            splice.replace(find_operation(&content, *ordinal)?.span.clone(), bytes.clone());
        }
        let mut form = rewritten_stream(&original, splice.apply(&content.segments[0].data)?);
        if !fonts.is_empty() {
            match stream.dict.get("Resources") {
                Some(own) => {
                    let mut own = transaction.resolve(own)?.as_dict().cloned().unwrap_or_default();
                    for (name, font) in fonts {
                        add_font(transaction, &mut own, name, *font)?;
                    }
                    if let Object::Stream(form) = &mut form {
                        form.dict.insert("Resources", Object::Dict(own));
                    }
                }
                None => {
                    for (name, font) in fonts {
                        add_font(transaction, page_resources, name, *font)?;
                    }
                }
            }
        }
        Ok(form)
    }
}

fn leaves_state_changed(content: &ContentStream) -> bool {
    let mut depth = 0i64;
    for operation in &content.operations {
        match operation.operator.as_str() {
            "q" => depth += 1,
            "Q" => depth -= 1,
            "cm" | "W" | "W*" if depth <= 0 => return true,
            _ => {}
        }
    }
    depth != 0
}

fn plain_stream(data: Vec<u8>) -> Object {
    Object::Stream(Stream { dict: Dict::new(), raw: Rc::from(data) })
}

fn find_operation(content: &ContentStream, ordinal: usize) -> Result<&Operation> {
    content
        .operations
        .iter()
        .find(|operation| operation.ordinal == ordinal)
        .ok_or_else(|| Error::new("the edited operation is gone"))
}

fn rewritten_stream(original: &Object, data: Vec<u8>) -> Object {
    let mut dict = original.as_dict().cloned().unwrap_or_default();
    for key in ["Filter", "DecodeParms", "Length", "DL"] {
        dict.remove(key);
    }
    Object::Stream(Stream { dict, raw: Rc::from(data) })
}

fn add_font(transaction: &Transaction, resources: &mut Dict, name: &Name, font: ObjId) -> Result<()> {
    let mut fonts = match resources.get("Font") {
        Some(fonts) => transaction.resolve(fonts)?.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    fonts.insert(name.clone(), Object::Ref(font));
    resources.insert("Font", Object::Dict(fonts));
    Ok(())
}

fn page_contents(transaction: &Transaction, page: &Dict) -> Result<Vec<ObjId>> {
    Ok(match page.get("Contents") {
        Some(Object::Ref(id)) => match &*transaction.object(*id)? {
            Object::Array(items) => items.iter().filter_map(Object::as_reference).collect(),
            _ => vec![*id],
        },
        Some(Object::Array(items)) => items.iter().filter_map(Object::as_reference).collect(),
        _ => Vec::new(),
    })
}

fn streams_used_elsewhere(store: &ObjectStore, page: ObjId) -> Result<BTreeSet<ObjId>> {
    let mut used = BTreeSet::new();
    for other in PageTree::new(store).pages()?.iter().filter(|other| other.id.0 != page) {
        used.extend(other.contents.iter().copied());
    }
    Ok(used)
}
