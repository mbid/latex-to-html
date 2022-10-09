#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Math<'a> {
    Inline(&'a str),
    Display {
        source: &'a str,
        label: Option<&'a str>,
    },
    Mathpar {
        source: &'a str,
        label: Option<&'a str>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item<'a> {
    pub content: Vec<Paragraph<'a>>,
    pub label: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphPart<'a> {
    InlineWhitespace(&'a str),
    TextToken(&'a str),
    Math(Math<'a>),
    Ref(&'a str),
    Emph(Paragraph<'a>),
    Qed,
    Enumerate(Vec<Item<'a>>),
    Itemize(Vec<Item<'a>>),
    Todo,
}

pub type Paragraph<'a> = Vec<ParagraphPart<'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheoremLikeConfig<'a> {
    pub tag: &'a str,
    pub name: Paragraph<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentPart<'a> {
    FreeParagraph(Paragraph<'a>),
    Title(Paragraph<'a>),
    Author(Paragraph<'a>),
    Date(),
    Maketitle(),
    Section {
        label: Option<&'a str>,
        name: Paragraph<'a>,
    },
    Subsection {
        label: Option<&'a str>,
        name: Paragraph<'a>,
    },
    Abstract(Vec<Paragraph<'a>>),
    TheoremLike {
        tag: &'a str,
        content: Vec<Paragraph<'a>>,
        label: Option<&'a str>,
    },
    Proof(Vec<Paragraph<'a>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentConfig<'a> {
    pub theorem_like_configs: Vec<TheoremLikeConfig<'a>>,
}

impl Default for DocumentConfig<'static> {
    fn default() -> Self {
        DocumentConfig {
            theorem_like_configs: vec![
                TheoremLikeConfig {
                    tag: "theorem",
                    name: vec![ParagraphPart::TextToken("Theorem")],
                },
                TheoremLikeConfig {
                    tag: "proposition",
                    name: vec![ParagraphPart::TextToken("Proposition")],
                },
                TheoremLikeConfig {
                    tag: "definition",
                    name: vec![ParagraphPart::TextToken("Definition")],
                },
                TheoremLikeConfig {
                    tag: "lemma",
                    name: vec![ParagraphPart::TextToken("Lemma")],
                },
                TheoremLikeConfig {
                    tag: "remark",
                    name: vec![ParagraphPart::TextToken("Remark")],
                },
                TheoremLikeConfig {
                    tag: "corollary",
                    name: vec![ParagraphPart::TextToken("Corollary")],
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document<'a> {
    pub preamble: String,
    pub parts: Vec<DocumentPart<'a>>,
    pub config: DocumentConfig<'a>,
}

pub struct NodeLists<'a> {
    pub math: Vec<Math<'a>>,
    pub item_lists: Vec<&'a Vec<Item<'a>>>,
}

impl<'a> NodeLists<'a> {
    pub fn new(doc: &'a Document<'a>) -> Self {
        let mut result = NodeLists {
            math: Vec::new(),
            item_lists: Vec::new(),
        };

        doc.parts.iter().for_each(|part| result.add_doc_part(part));
        result
    }

    fn add_doc_part(&mut self, part: &'a DocumentPart<'a>) {
        use DocumentPart::*;
        match part {
            Date() | Maketitle() => (),
            FreeParagraph(par)
            | Title(par)
            | Author(par)
            | Section {
                name: par,
                label: _,
            }
            | Subsection {
                name: par,
                label: _,
            } => {
                par.iter().for_each(|part| self.add_par_part(part));
            }
            Abstract(pars)
            | TheoremLike {
                content: pars,
                tag: _,
                label: _,
            }
            | Proof(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| self.add_par_part(part));
            }
        }
    }

    fn add_par_part(&mut self, part: &'a ParagraphPart<'a>) {
        use ParagraphPart::*;
        match part {
            InlineWhitespace(_) | TextToken(_) | Ref(_) | Qed | Todo => (),
            Math(math) => {
                self.math.push(*math);
            }
            Emph(par) => {
                par.iter().for_each(|part| self.add_par_part(part));
            }
            Enumerate(items) | Itemize(items) => {
                self.item_lists.push(items);
                items
                    .iter()
                    .map(|it| &it.content)
                    .flatten()
                    .flatten()
                    .for_each(|part| {
                        self.add_par_part(part);
                    });
            }
        }
    }
}
