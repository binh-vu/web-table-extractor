use scraper::Html;

pub mod context_v1;
pub mod table;

use pyo3::prelude::*;

#[pyclass(unsendable)]
pub struct Document {
    url: String,
    doc: String,
    html: Html,
}

#[pymethods]
impl Document {
    #[new]
    pub fn new(url: String, doc: String) -> Self {
        let html = Html::parse_document(&doc);
        Document { url, doc, html }
    }
}