use std::borrow::Cow;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Math<'a> {
    // Inline holds onto the content, i.e. what's in-between $ and $, but not to $ itself. Display
    // and Mathpar have the whole environment, i.e. including \begin{equation} and \end{equation}.
    // TODO: Make this more uniform.
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

impl<'a> Math<'a> {
    pub fn label(&self) -> Option<&'a str> {
        use Math::*;
        match self {
            Inline(_) => None,
            Display { label, .. } | Mathpar { label, .. } => *label,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item<'a> {
    pub content: Vec<Paragraph<'a>>,
    pub label: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphPart<'a> {
    InlineWhitespace(&'a str),
    TextToken(Cow<'a, str>),
    Math(Math<'a>),
    Ref(&'a str),
    Cite {
        ids: Vec<&'a str>,
        text: Option<Paragraph<'a>>,
    },
    Emph(Paragraph<'a>),
    Textbf(Paragraph<'a>),
    Textit(Paragraph<'a>),
    Texttt(Paragraph<'a>),
    Qed,
    Enumerate(Vec<Item<'a>>),
    Itemize(Vec<Item<'a>>),
    Todo,
    Footnote(Vec<Paragraph<'a>>),
    Href {
        text: Paragraph<'a>,
        link: &'a str,
    },
    Code(&'a str),
}

pub type Paragraph<'a> = Vec<ParagraphPart<'a>>;

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum TheoremStyle {
    Theorem,
    Definition,
    Remark,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheoremLikeConfig<'a> {
    pub tag: &'a str,
    pub name: Paragraph<'a>,
    pub style: TheoremStyle,
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
        note: Option<Paragraph<'a>>,
        content: Vec<Paragraph<'a>>,
        label: Option<&'a str>,
    },
    Proof(Vec<Paragraph<'a>>),
    Bibliography,
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
                    name: vec![ParagraphPart::TextToken(Cow::from("Theorem"))],
                    style: TheoremStyle::Theorem,
                },
                TheoremLikeConfig {
                    tag: "proposition",
                    name: vec![ParagraphPart::TextToken(Cow::from("Proposition"))],
                    style: TheoremStyle::Theorem,
                },
                TheoremLikeConfig {
                    tag: "definition",
                    name: vec![ParagraphPart::TextToken(Cow::from("Definition"))],
                    style: TheoremStyle::Definition,
                },
                TheoremLikeConfig {
                    tag: "lemma",
                    name: vec![ParagraphPart::TextToken(Cow::from("Lemma"))],
                    style: TheoremStyle::Theorem,
                },
                TheoremLikeConfig {
                    tag: "remark",
                    name: vec![ParagraphPart::TextToken(Cow::from("Remark"))],
                    style: TheoremStyle::Remark,
                },
                TheoremLikeConfig {
                    tag: "corollary",
                    name: vec![ParagraphPart::TextToken(Cow::from("Corollary"))],
                    style: TheoremStyle::Theorem,
                },
                TheoremLikeConfig {
                    tag: "example",
                    style: TheoremStyle::Definition,
                    name: vec![ParagraphPart::TextToken(Cow::from("Example"))],
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document<'a> {
    pub preamble: Vec<&'a str>,
    pub parts: Vec<DocumentPart<'a>>,
    pub config: DocumentConfig<'a>,
}

pub struct NodeLists<'a> {
    // The list of all math nodes.
    pub math: Vec<&'a Math<'a>>,

    // The list containing the list of items for each \itemize or \enumerate.
    pub item_lists: Vec<&'a Vec<Item<'a>>>,

    // The set of \ref or \eqref values.
    pub ref_ids: HashSet<&'a str>,

    // The set of \cite values.
    pub cite_ids: HashSet<&'a str>,
}

impl<'a> NodeLists<'a> {
    pub fn new(doc: &'a Document<'a>) -> Self {
        let mut result = NodeLists {
            math: Vec::new(),
            item_lists: Vec::new(),
            ref_ids: HashSet::new(),
            cite_ids: HashSet::new(),
        };

        doc.parts.iter().for_each(|part| result.add_doc_part(part));
        result
    }

    fn add_doc_part(&mut self, part: &'a DocumentPart<'a>) {
        use DocumentPart::*;
        match part {
            Date() | Maketitle() | Bibliography => (),
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
            TheoremLike {
                content,
                note,
                tag: _,
                label: _,
            } => {
                content
                    .iter()
                    .flatten()
                    .for_each(|part| self.add_par_part(part));
                note.iter()
                    .flatten()
                    .for_each(|part| self.add_par_part(part));
            }
            Abstract(pars) | Proof(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| self.add_par_part(part));
            }
        }
    }

    fn add_par_part(&mut self, part: &'a ParagraphPart<'a>) {
        use ParagraphPart::*;
        match part {
            InlineWhitespace(_) | TextToken(_) | Qed | Todo => (),
            Cite { ids, text } => {
                for id in ids.iter().copied() {
                    self.cite_ids.insert(id);
                }
                text.iter()
                    .flatten()
                    .for_each(|part| self.add_par_part(part));
            }
            Ref(id) => {
                self.ref_ids.insert(id);
            }
            Math(math) => {
                self.math.push(math);
            }
            Emph(par) | Textbf(par) | Textit(par) | Texttt(par) => {
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
            Footnote(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| self.add_par_part(part));
            }
            Href { text, link: _ } => {
                text.iter().for_each(|part| self.add_par_part(part));
            }
            Code(_) => (),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BibEntryType {
    Misc,
    Article,
    Book,
    Inproceedings,
    Thesis,
    Incollection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstName<'a> {
    Full(&'a str),
    Abbreviation(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibPerson<'a> {
    pub first_names: Vec<FirstName<'a>>,
    pub last_name: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibPages {
    pub first: u64,
    pub last: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BibEntryItem<'a> {
    Title(&'a str),
    Year(&'a str),
    Authors(Vec<BibPerson<'a>>),
    Url(&'a str),
    Journal(&'a str),
    Booktitle(&'a str),
    Series(&'a str),
    Publisher(&'a str),
    Volume(&'a str),
    Number(&'a str),
    Pages(BibPages),
    Unused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibEntry<'a> {
    pub entry_type: BibEntryType,
    pub tag: &'a str,

    pub title: Option<&'a str>,
    pub year: Option<&'a str>,
    pub authors: Option<Vec<BibPerson<'a>>>,
    pub url: Option<&'a str>,
    pub journal: Option<&'a str>,
    pub booktitle: Option<&'a str>,
    pub series: Option<&'a str>,
    pub publisher: Option<&'a str>,
    pub volume: Option<&'a str>,
    pub number: Option<&'a str>,
    pub pages: Option<BibPages>,
}
