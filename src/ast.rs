use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Math<'a> {
    Inline(&'a str),
    Display(&'a str),
    Mathpar(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphPart<'a> {
    InlineWhitespace(&'a str),
    TextToken(&'a str),
    Math(Math<'a>),
    Ref(&'a str),
    Emph(Paragraph<'a>),
    Label(&'a str),
    Qed,
    Enumerate(Vec<Vec<Paragraph<'a>>>),
    Itemize(Vec<Vec<Paragraph<'a>>>),
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
    Section(Paragraph<'a>),
    Subsection(Paragraph<'a>),
    Abstract(Vec<Paragraph<'a>>),
    TheoremLike {
        tag: &'a str,
        content: Vec<Paragraph<'a>>,
        label: Option<&'a str>,
    },
    Proof(Vec<Paragraph<'a>>),
    Label(&'a str),
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
    pub preamble: &'a str,
    pub parts: Vec<DocumentPart<'a>>,
    pub config: DocumentConfig<'a>,
    pub label_names: Option<HashMap<&'a str, String>>,
}

pub trait Syntax {
    fn for_each_math<'a>(self: &'a Self, f: impl FnMut(Math<'a>) -> ());
}

impl<'a> Syntax for Math<'a> {
    fn for_each_math<'b>(self: &'b Self, mut f: impl FnMut(Math<'b>) -> ()) {
        f(*self)
    }
}

fn for_each_math_paragraph_part_impl<'a>(
    part: &'a ParagraphPart,
    f: &mut impl FnMut(Math<'a>) -> (),
) {
    use ParagraphPart::*;
    match part {
        InlineWhitespace(_) => (),
        TextToken(_) => (),
        Math(math) => math.for_each_math(f),
        Ref(_) => (),
        Emph(par) => {
            for part in par {
                for_each_math_paragraph_part_impl(part, f);
            }
        }
        Label(_) => (),
        Qed => (),
        Enumerate(items) | Itemize(items) => {
            for item in items {
                for par in item {
                    for part in par {
                        for_each_math_paragraph_part_impl(part, f);
                    }
                }
            }
        }
        Todo => (),
    }
}

impl<'a> Syntax for ParagraphPart<'a> {
    fn for_each_math<'b>(self: &'b Self, mut f: impl FnMut(Math<'b>) -> ()) {
        for_each_math_paragraph_part_impl(self, &mut f);
    }
}

impl<'a> Syntax for DocumentPart<'a> {
    fn for_each_math<'b>(self: &'b Self, mut f: impl FnMut(Math<'b>) -> ()) {
        use DocumentPart::*;
        match self {
            FreeParagraph(par) => {
                par.iter().for_each(|part| part.for_each_math(&mut f));
            }
            Title(par) => {
                par.iter().for_each(|part| part.for_each_math(&mut f));
            }
            Author(par) => {
                par.iter().for_each(|part| part.for_each_math(&mut f));
            }
            Date() => (),
            Maketitle() => (),
            Section(par) => {
                par.iter().for_each(|part| part.for_each_math(&mut f));
            }
            Subsection(par) => {
                par.iter().for_each(|part| part.for_each_math(&mut f));
            }
            Abstract(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| part.for_each_math(&mut f));
            }
            TheoremLike { content, .. } => {
                content
                    .iter()
                    .flatten()
                    .for_each(|part| part.for_each_math(&mut f));
            }
            Proof(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| part.for_each_math(&mut f));
            }
            Label(_) => (),
        }
    }
}

impl<'a> Syntax for TheoremLikeConfig<'a> {
    fn for_each_math<'b>(self: &'b Self, mut f: impl FnMut(Math<'b>) -> ()) {
        self.name.iter().for_each(|part| part.for_each_math(&mut f));
    }
}

impl<'a> Syntax for DocumentConfig<'a> {
    fn for_each_math<'b>(self: &'b Self, mut f: impl FnMut(Math<'b>) -> ()) {
        self.theorem_like_configs
            .iter()
            .for_each(|c| c.for_each_math(&mut f));
    }
}

impl<'a> Syntax for Document<'a> {
    fn for_each_math<'b>(self: &'b Self, mut f: impl FnMut(Math<'b>) -> ()) {
        self.parts
            .iter()
            .for_each(|part| part.for_each_math(&mut f));
        self.config.for_each_math(&mut f);
    }
}
