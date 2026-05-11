use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, serialize, ParseOpts};
use markup5ever_rcdom::{Handle, NodeData, RcDom, SerializableHandle};
use serde_json::{json, Value};
use std::io::Cursor;

pub fn js_opt_html() -> &'static str {
    include_str!("../assets/simphtml_opt.js")
}

pub fn js_find_main_list() -> &'static str {
    include_str!("../assets/simphtml_find_list.js")
}

pub fn optimize_html_for_tokens(html: &str) -> String {
    let dom = parse_document(RcDom::default(), ParseOpts::default()).one(html.to_string());
    clean_node(&dom.document);
    let mut bytes = Vec::new();
    let serializable = SerializableHandle::from(dom.document.clone());
    if serialize(&mut bytes, &serializable, Default::default()).is_ok() {
        String::from_utf8_lossy(&bytes).into_owned()
    } else {
        html.to_string()
    }
}

fn clean_node(handle: &Handle) {
    if let NodeData::Element { name, attrs, .. } = &handle.data {
        let tag = name.local.as_ref();
        let mut attrs = attrs.borrow_mut();
        if tag.eq_ignore_ascii_case("svg") {
            attrs.clear();
            handle.children.borrow_mut().clear();
            return;
        }
        attrs.retain(|attr| {
            let key = attr.name.local.as_ref();
            matches!(
                key,
                "id" | "class"
                    | "name"
                    | "src"
                    | "href"
                    | "alt"
                    | "value"
                    | "type"
                    | "placeholder"
                    | "disabled"
                    | "checked"
                    | "selected"
                    | "readonly"
                    | "required"
                    | "multiple"
                    | "role"
                    | "aria-label"
                    | "aria-expanded"
                    | "aria-hidden"
                    | "contenteditable"
                    | "title"
                    | "for"
                    | "action"
                    | "method"
                    | "target"
                    | "colspan"
                    | "rowspan"
            ) || key.starts_with("data-")
        });
        for attr in attrs.iter_mut() {
            let key = attr.name.local.as_ref();
            let value = attr.value.to_string();
            if key == "src" {
                if value.starts_with("data:") {
                    attr.value.clear();
                    attr.value.push_slice("__img__");
                } else if value.len() > 30 {
                    attr.value.clear();
                    attr.value.push_slice("__url__");
                }
            } else if (key == "href" || key == "action") && value.len() > 30 {
                attr.value.clear();
                attr.value
                    .push_slice(if key == "href" { "__link__" } else { "__url__" });
            } else if matches!(key, "value" | "title" | "alt") && value.len() > 100 {
                attr.value.clear();
                attr.value.push_slice(&format!(
                    "{} ...",
                    value.chars().take(50).collect::<String>()
                ));
            } else if key.starts_with("data-") && !key.starts_with("data-v") && value.len() > 20 {
                attr.value.clear();
                attr.value.push_slice("__data__");
            }
        }
    }
    let children = handle.children.borrow().clone();
    for child in children {
        clean_node(&child);
    }
}

pub fn smart_truncate(html: String, budget: usize) -> String {
    if html.len() <= budget {
        return html;
    }
    let keep = budget.saturating_sub(32);
    let mut result: String = html.chars().take(keep).collect();
    result.push_str(" [TRUNCATED]");
    result
}

pub fn changed_elements(before_html: &str, after_html: &str) -> Value {
    if before_html == after_html {
        return json!({ "changed": 0 });
    }
    let preview: String = after_html.chars().take(2000).collect();
    json!({ "changed": 1, "top_change": preview })
}

#[allow(dead_code)]
fn _parse_html_for_validation(html: &str) -> RcDom {
    parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut Cursor::new(html.as_bytes()))
        .unwrap_or_default()
}
