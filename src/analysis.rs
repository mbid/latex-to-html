use crate::ast::*;
use crate::math_svg::*;
use std::collections::HashMap;
use std::ptr::addr_of;

pub struct Analysis<'a> {
    // The number strings assigned to theorem-like document parts:
    // - TheoremLike
    // - Section
    // - Subsection
    pub doc_part_numbering: HashMap<*const DocumentPart<'a>, String>,

    // Numbering strings assigned to equations.
    pub math_numbering: HashMap<*const Math<'a>, String>,

    // The "src" attributes of math images.
    pub math_image_source: HashMap<*const Math<'a>, String>,

    // The text by which references to a given id should refer to what they are referencing.
    pub ref_display_text: HashMap<&'a str, String>,

    // The text by which citations to a given id should refer to what they are citing.
    pub cite_display_text: HashMap<&'a str, String>,
}

impl<'a> Analysis<'a> {
    pub fn new(doc: &'a Document<'a>, node_lists: &NodeLists<'a>) -> Self {
        let doc_part_numbering = doc_part_numbering(doc);
        let math_numbering = math_numbering(node_lists);
        let math_image_source = math_image_source(doc, node_lists);
        let ref_display_text =
            ref_display_text(doc, node_lists, &doc_part_numbering, &math_numbering);
        let cite_display_text = HashMap::new(); // TODO
        Analysis {
            doc_part_numbering,
            math_numbering,
            math_image_source,
            ref_display_text,
            cite_display_text,
        }
    }
}

fn doc_part_numbering<'a>(doc: &Document<'a>) -> HashMap<*const DocumentPart<'a>, String> {
    let mut map: HashMap<*const DocumentPart<'a>, String> = HashMap::new();
    let mut current_theorem_like = 0;
    let mut current_section = 0;
    let mut current_subsection = 0;
    for part in doc.parts.iter() {
        match part {
            DocumentPart::TheoremLike { .. } => {
                current_theorem_like += 1;
                map.insert(part, current_theorem_like.to_string());
            }
            DocumentPart::Section { .. } => {
                current_section += 1;
                current_subsection = 0;
                map.insert(part, current_section.to_string());
            }
            DocumentPart::Subsection { .. } => {
                current_subsection += 1;
                map.insert(part, format!("{current_section}.{current_subsection}"));
            }
            _ => (),
        }
    }
    map
}

fn math_numbering<'a>(node_lists: &NodeLists<'a>) -> HashMap<*const Math<'a>, String> {
    let mut result: HashMap<*const Math<'a>, String> = HashMap::new();
    let mut current_number = 0;
    for math in node_lists.math.iter().copied() {
        if let Some(label) = math.label() {
            if node_lists.ref_ids.iter().find(|l| **l == label).is_some() {
                current_number += 1;
                result.insert(math, format!("({current_number})"));
            }
        }
    }
    result
}

fn math_image_source<'a>(
    doc: &Document,
    node_lists: &NodeLists<'a>,
) -> HashMap<*const Math<'a>, String> {
    node_lists
        .math
        .iter()
        .copied()
        .map(|math| {
            let digest = hash_math(&doc.preamble, math);
            (addr_of!(*math), format!("{SVG_OUT_DIR}/{digest}.svg"))
        })
        .collect()
}

fn ref_display_text<'a>(
    doc: &Document<'a>,
    node_lists: &NodeLists<'a>,
    doc_part_numbering: &HashMap<*const DocumentPart, String>,
    math_numbering: &HashMap<*const Math, String>,
) -> HashMap<&'a str, String> {
    let mut text = HashMap::new();
    for part in doc.parts.iter() {
        use DocumentPart::*;
        match part {
            TheoremLike { label, .. } | Section { label, .. } | Subsection { label, .. } => {
                if let Some(label) = label {
                    let number = doc_part_numbering.get(&std::ptr::addr_of!(*part)).unwrap();
                    text.insert(*label, number.clone());
                }
            }
            _ => (),
        }
    }

    for item_list in node_lists.item_lists.iter() {
        for (i, item) in item_list.iter().enumerate() {
            if let Some(label) = item.label {
                text.insert(label, (i + 1).to_string());
            }
        }
    }

    for math in node_lists.math.iter().copied() {
        if let Some(label) = math.label() {
            if let Some(number) = math_numbering.get(&std::ptr::addr_of!(*math)) {
                text.insert(label, number.clone());
            }
        }
    }
    text
}