use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::{HashMap, HashSet};

use super::{
    Issue, IssueKind, Layer, Severity,
    zip_layer::{ZipHandle, read_entry},
};

#[derive(Debug, Clone)]
pub struct Creator {
    pub name: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SeriesMeta {
    pub name: String,
    pub position: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct OpfData {
    /// All manifest items: id → href
    pub manifest: HashMap<String, String>,
    /// Spine idrefs (after removing broken refs)
    pub spine_idrefs: Vec<String>,
    /// OPF path within the archive (needed by repair and other layers)
    pub opf_path: String,
    /// Raw W3C accessibility metadata from `<meta>` elements, if any
    pub accessibility_metadata: Option<serde_json::Value>,
    /// Dublin Core: title
    pub title: Option<String>,
    /// Dublin Core: creators with optional role
    pub creators: Vec<Creator>,
    /// Dublin Core: description (may contain HTML)
    pub description: Option<String>,
    /// Dublin Core: publisher
    pub publisher: Option<String>,
    /// Dublin Core: date (raw string)
    pub date: Option<String>,
    /// Dublin Core: language
    pub language: Option<String>,
    /// Dublin Core: all identifier values (ISBNs, URNs, etc.)
    pub identifiers: Vec<String>,
    /// Dublin Core: subject values
    pub subjects: Vec<String>,
    /// Series metadata (calibre or EPUB 3 collection)
    pub series_meta: Option<SeriesMeta>,
}

/// Extract the local name from a possibly-namespaced element name.
/// e.g. b"dc:title" → b"title", b"title" → b"title"
fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().position(|&b| b == b':') {
        Some(pos) => &name[pos + 1..],
        None => name,
    }
}

/// Validate the OPF file. Returns `None` if OPF cannot be read.
pub fn validate(
    handle: &ZipHandle,
    opf_path: Option<&str>,
    issues: &mut Vec<Issue>,
) -> Option<OpfData> {
    let path = opf_path?;
    let bytes = read_entry(handle, path)?;
    let xml = std::str::from_utf8(&bytes).ok()?;

    let mut manifest: HashMap<String, String> = HashMap::new();
    let mut spine_idrefs: Vec<String> = Vec::new();
    let mut accessibility_meta: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

    // Dublin Core fields
    let mut title: Option<String> = None;
    let mut creators: Vec<Creator> = Vec::new();
    let mut description: Option<String> = None;
    let mut publisher: Option<String> = None;
    let mut date: Option<String> = None;
    let mut language: Option<String> = None;
    let mut identifiers: Vec<String> = Vec::new();
    let mut subjects: Vec<String> = Vec::new();

    // Series metadata (calibre or EPUB 3).
    // Note: EPUB 3 group-position matching assumes belongs-to-collection appears
    // before group-position in the XML. If reversed, position won't be captured.
    // This is acceptable for MVP (best-effort).
    let mut calibre_series_name: Option<String> = None;
    let mut calibre_series_index: Option<f64> = None;
    let mut epub3_collection_name: Option<String> = None;
    let mut epub3_collection_id: Option<String> = None;
    let mut epub3_collection_position: Option<f64> = None;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event().ok()? {
            // EPUB 3 text-content meta: <meta property="schema:accessMode">textual</meta>
            // Also handles belongs-to-collection and group-position.
            // Must come BEFORE general Event::Start arm to avoid shadowing.
            Event::Start(e) if e.name().as_ref() == b"meta" => {
                let e = e.into_owned(); // release reader buffer borrow before read_text
                let prop = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"property")
                    .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));
                let content_attr = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"content")
                    .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));
                let id_attr = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"id")
                    .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));
                let refines_attr = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"refines")
                    .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));

                if let Some(ref prop) = prop {
                    if prop == "belongs-to-collection" {
                        let text = reader
                            .read_text(e.name())
                            .ok()
                            .map(|t| t.trim().to_string())
                            .filter(|s| !s.is_empty());
                        if let Some(name) = text {
                            epub3_collection_name = Some(name);
                            epub3_collection_id = id_attr;
                        }
                        continue;
                    }
                    if prop == "group-position" {
                        if let Some(ref refines) = refines_attr
                            && epub3_collection_id
                                .as_ref()
                                .is_some_and(|id| refines == &format!("#{id}"))
                        {
                            let text = content_attr.or_else(|| {
                                reader
                                    .read_text(e.name())
                                    .ok()
                                    .map(|t| t.trim().to_string())
                                    .filter(|s| !s.is_empty())
                            });
                            epub3_collection_position = text.and_then(|t| t.parse::<f64>().ok());
                        }
                        continue;
                    }
                    if prop.starts_with("schema:access") || prop.starts_with("dcterms:") {
                        let val = content_attr.or_else(|| {
                            reader
                                .read_text(e.name())
                                .ok()
                                .map(|t| t.trim().to_string())
                                .filter(|s| !s.is_empty())
                        });
                        if let Some(v) = val {
                            accessibility_meta.insert(prop.clone(), serde_json::Value::String(v));
                        }
                    }
                }
            }
            // EPUB 2 attribute-style meta: <meta name="..." content="..."/>
            Event::Empty(e) if e.name().as_ref() == b"meta" => {
                let prop = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"property")
                    .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));
                let name_attr = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"name")
                    .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));
                let content = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"content")
                    .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));

                // Accessibility meta via property attribute
                if let Some(ref prop) = prop
                    && (prop.starts_with("schema:access") || prop.starts_with("dcterms:"))
                    && let Some(ref v) = content
                {
                    accessibility_meta.insert(prop.clone(), serde_json::Value::String(v.clone()));
                }

                // Calibre series meta via name attribute
                if let Some(ref name) = name_attr
                    && let Some(ref c) = content
                {
                    match name.as_str() {
                        "calibre:series" => calibre_series_name = Some(c.clone()),
                        "calibre:series_index" => calibre_series_index = c.parse::<f64>().ok(),
                        _ => {}
                    }
                }
            }
            // Dublin Core elements: <dc:title>, <dc:creator>, etc.
            Event::Start(e)
                if matches!(
                    local_name(e.name().as_ref()),
                    b"title"
                        | b"creator"
                        | b"description"
                        | b"publisher"
                        | b"date"
                        | b"language"
                        | b"identifier"
                        | b"subject"
                ) && e.name().as_ref() != b"meta" =>
            {
                let local = local_name(e.name().as_ref()).to_vec();
                // Extract opf:role for creators (EPUB 2).
                // EPUB 3 uses <meta refines="#id" property="role"> — not resolved here (MVP).
                let role = e
                    .attributes()
                    .flatten()
                    .find(|a| {
                        let k = a.key.as_ref();
                        k == b"opf:role" || k == b"role"
                    })
                    .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));

                let e = e.into_owned();
                let text = reader
                    .read_text(e.name())
                    .ok()
                    .map(|t| t.trim().to_string())
                    .filter(|s| !s.is_empty());

                if let Some(text) = text {
                    match local.as_slice() {
                        b"title" if title.is_none() => title = Some(text),
                        b"creator" => creators.push(Creator { name: text, role }),
                        b"description" if description.is_none() => description = Some(text),
                        b"publisher" if publisher.is_none() => publisher = Some(text),
                        b"date" if date.is_none() => date = Some(text),
                        b"language" if language.is_none() => language = Some(text),
                        b"identifier" => identifiers.push(text),
                        b"subject" => subjects.push(text),
                        _ => {}
                    }
                }
            }
            // General arm — meta and DC already handled by guarded arms above
            Event::Empty(e) | Event::Start(e) => match e.name().as_ref() {
                b"item" => {
                    let attrs: HashMap<String, String> = e
                        .attributes()
                        .flatten()
                        .filter_map(|a| {
                            let k = std::str::from_utf8(a.key.as_ref()).ok()?.to_string();
                            let v = std::str::from_utf8(&a.value).ok()?.to_string();
                            Some((k, v))
                        })
                        .collect();

                    if let (Some(id), Some(href)) = (attrs.get("id"), attrs.get("href")) {
                        // C4: validate href path safety via shared helper.
                        if !super::is_safe_path(href) {
                            issues.push(Issue {
                                layer: Layer::Opf,
                                severity: Severity::Degraded,
                                kind: IssueKind::UnsafeManifestHref { href: href.clone() },
                            });
                        } else {
                            manifest.insert(id.clone(), href.clone());
                        }
                    }
                }
                b"itemref" => {
                    if let Some(idref) = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == b"idref")
                        && let Ok(v) = std::str::from_utf8(&idref.value)
                    {
                        spine_idrefs.push(v.to_string());
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }

    // Validate spine refs against manifest
    let manifest_ids: HashSet<&String> = manifest.keys().collect();
    let mut valid_spine: Vec<String> = Vec::new();
    for idref in &spine_idrefs {
        if manifest_ids.contains(idref) {
            valid_spine.push(idref.clone());
        } else {
            issues.push(Issue {
                layer: Layer::Opf,
                severity: Severity::Repaired,
                kind: IssueKind::BrokenSpineRef {
                    idref: idref.clone(),
                },
            });
        }
    }

    let accessibility_metadata = if accessibility_meta.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(accessibility_meta))
    };

    // Resolve series: prefer calibre (more common), fall back to EPUB 3 collection
    let series_meta = calibre_series_name
        .map(|name| SeriesMeta {
            name,
            position: calibre_series_index,
        })
        .or_else(|| {
            epub3_collection_name.map(|name| SeriesMeta {
                name,
                position: epub3_collection_position,
            })
        });

    Some(OpfData {
        manifest,
        spine_idrefs: valid_spine,
        opf_path: path.to_string(),
        accessibility_metadata,
        title,
        creators,
        description,
        publisher,
        date,
        language,
        identifiers,
        subjects,
        series_meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::epub::zip_layer::ZipHandle;

    fn make_handle(opf_content: &[u8]) -> ZipHandle {
        use std::io::Write;
        let buf = std::io::Cursor::new(Vec::new());
        let mut w = zip::ZipWriter::new(buf);
        let opts: zip::write::FileOptions<zip::write::ExtendedFileOptions> =
            zip::write::FileOptions::default();
        w.start_file("OEBPS/content.opf", opts).unwrap();
        w.write_all(opf_content).unwrap();
        let bytes = w.finish().unwrap().into_inner();
        ZipHandle {
            bytes,
            entries: vec!["OEBPS/content.opf".to_string()],
        }
    }

    #[test]
    fn broken_spine_ref_emits_repaired_issue() {
        let opf = br#"<package>
            <manifest>
                <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
            </manifest>
            <spine>
                <itemref idref="ch1"/>
                <itemref idref="ch2"/>
            </spine>
        </package>"#;
        let handle = make_handle(opf);
        let mut issues = Vec::new();
        let result = validate(&handle, Some("OEBPS/content.opf"), &mut issues);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.spine_idrefs, vec!["ch1"]);
        assert!(issues.iter().any(|i| {
            i.severity == Severity::Repaired
                && matches!(&i.kind, IssueKind::BrokenSpineRef { idref } if idref == "ch2")
        }));
    }

    #[test]
    fn epub3_accessibility_meta_parsed() {
        let opf = br#"<package>
            <metadata>
                <meta property="schema:accessMode">textual</meta>
            </metadata>
            <manifest/>
            <spine/>
        </package>"#;
        let handle = make_handle(opf);
        let mut issues = Vec::new();
        let result = validate(&handle, Some("OEBPS/content.opf"), &mut issues);
        assert!(result.is_some());
        let data = result.unwrap();
        let meta = data.accessibility_metadata.unwrap();
        assert_eq!(meta["schema:accessMode"], "textual");
    }

    #[test]
    fn dc_metadata_extracted() {
        let opf = br#"<package xmlns:dc="http://purl.org/dc/elements/1.1/">
            <metadata>
                <dc:title>The Hobbit</dc:title>
                <dc:creator opf:role="aut">J. R. R. Tolkien</dc:creator>
                <dc:description>A fantasy novel</dc:description>
                <dc:publisher>Allen &amp; Unwin</dc:publisher>
                <dc:date>1937-09-21</dc:date>
                <dc:language>en</dc:language>
                <dc:identifier>urn:isbn:9780547928227</dc:identifier>
                <dc:identifier>urn:uuid:12345</dc:identifier>
                <dc:subject>Fantasy</dc:subject>
                <dc:subject>Adventure</dc:subject>
            </metadata>
            <manifest/>
            <spine/>
        </package>"#;
        let handle = make_handle(opf);
        let mut issues = Vec::new();
        let result = validate(&handle, Some("OEBPS/content.opf"), &mut issues);
        let data = result.unwrap();
        assert_eq!(data.title.as_deref(), Some("The Hobbit"));
        assert_eq!(data.creators.len(), 1);
        assert_eq!(data.creators[0].name, "J. R. R. Tolkien");
        assert_eq!(data.creators[0].role.as_deref(), Some("aut"));
        assert_eq!(data.description.as_deref(), Some("A fantasy novel"));
        assert_eq!(data.publisher.as_deref(), Some("Allen &amp; Unwin"));
        assert_eq!(data.date.as_deref(), Some("1937-09-21"));
        assert_eq!(data.language.as_deref(), Some("en"));
        assert_eq!(data.identifiers.len(), 2);
        assert_eq!(data.subjects, vec!["Fantasy", "Adventure"]);
    }

    #[test]
    fn calibre_series_meta_extracted() {
        let opf = br#"<package>
            <metadata>
                <dc:title>The Two Towers</dc:title>
                <meta name="calibre:series" content="The Lord of the Rings"/>
                <meta name="calibre:series_index" content="2"/>
            </metadata>
            <manifest/>
            <spine/>
        </package>"#;
        let handle = make_handle(opf);
        let mut issues = Vec::new();
        let result = validate(&handle, Some("OEBPS/content.opf"), &mut issues);
        let data = result.unwrap();
        let series = data.series_meta.unwrap();
        assert_eq!(series.name, "The Lord of the Rings");
        assert_eq!(series.position, Some(2.0));
    }

    #[test]
    fn epub3_collection_series_extracted() {
        let opf = br##"<package>
            <metadata>
                <dc:title>A Game of Thrones</dc:title>
                <meta property="belongs-to-collection" id="c01">A Song of Ice and Fire</meta>
                <meta refines="#c01" property="group-position">1</meta>
            </metadata>
            <manifest/>
            <spine/>
        </package>"##;
        let handle = make_handle(opf);
        let mut issues = Vec::new();
        let result = validate(&handle, Some("OEBPS/content.opf"), &mut issues);
        let data = result.unwrap();
        let series = data.series_meta.unwrap();
        assert_eq!(series.name, "A Song of Ice and Fire");
        assert_eq!(series.position, Some(1.0));
    }

    #[test]
    fn empty_metadata_returns_none_fields() {
        let opf = br#"<package>
            <metadata/>
            <manifest/>
            <spine/>
        </package>"#;
        let handle = make_handle(opf);
        let mut issues = Vec::new();
        let result = validate(&handle, Some("OEBPS/content.opf"), &mut issues);
        let data = result.unwrap();
        assert!(data.title.is_none());
        assert!(data.creators.is_empty());
        assert!(data.description.is_none());
        assert!(data.identifiers.is_empty());
        assert!(data.series_meta.is_none());
    }

    #[test]
    fn multiple_creators_with_roles() {
        let opf = br#"<package xmlns:dc="http://purl.org/dc/elements/1.1/">
            <metadata>
                <dc:creator opf:role="aut">Author One</dc:creator>
                <dc:creator opf:role="edt">Editor Two</dc:creator>
                <dc:creator>No Role Three</dc:creator>
            </metadata>
            <manifest/>
            <spine/>
        </package>"#;
        let handle = make_handle(opf);
        let mut issues = Vec::new();
        let result = validate(&handle, Some("OEBPS/content.opf"), &mut issues);
        let data = result.unwrap();
        assert_eq!(data.creators.len(), 3);
        assert_eq!(data.creators[0].role.as_deref(), Some("aut"));
        assert_eq!(data.creators[1].role.as_deref(), Some("edt"));
        assert!(data.creators[2].role.is_none());
    }
}
