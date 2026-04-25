//! Open-document tracking. One `OpenDoc` per URL the client has open;
//! each holds the latest text + version + a cached `LineIndex` for
//! position translation.

use std::collections::HashMap;

use tower_lsp::lsp_types::Url;

use crate::position::LineIndex;

#[derive(Debug, Clone)]
pub struct OpenDoc {
    pub version: i32,
    pub line_index: LineIndex,
}

impl OpenDoc {
    pub fn new(text: String, version: i32) -> Self {
        Self { version, line_index: LineIndex::new(text) }
    }

    pub fn text(&self) -> &str {
        self.line_index.text()
    }
}

#[derive(Debug, Default)]
pub struct OpenDocs {
    docs: HashMap<Url, OpenDoc>,
}

impl OpenDocs {
    pub fn open(&mut self, uri: Url, text: String, version: i32) {
        self.docs.insert(uri, OpenDoc::new(text, version));
    }

    /// Full-text replace. We advertise `TextDocumentSyncKind::FULL` so each
    /// `didChange` carries the entire new buffer.
    pub fn replace(&mut self, uri: &Url, text: String, version: i32) {
        if let Some(doc) = self.docs.get_mut(uri) {
            doc.version = version;
            doc.line_index = LineIndex::new(text);
        } else {
            self.docs.insert(uri.clone(), OpenDoc::new(text, version));
        }
    }

    pub fn close(&mut self, uri: &Url) {
        self.docs.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<&OpenDoc> {
        self.docs.get(uri)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Url, &OpenDoc)> {
        self.docs.iter()
    }
}
