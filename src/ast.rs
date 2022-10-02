#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphPart<'a> {
    InlineWhitespace(&'a str),
    TextToken(&'a str),
    InlineMath(&'a str),
    DisplayMath(&'a str),
    Ref(&'a str),
    Eqref(&'a str),
    Emph(Paragraph<'a>),
    Comment(&'a str),
    Label(&'a str),
    Qed,
}

pub type Paragraph<'a> = Vec<ParagraphPart<'a>>;

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
    Proposition(Vec<Paragraph<'a>>),
    Definition(Vec<Paragraph<'a>>),
    Lemma(Vec<Paragraph<'a>>),
    Proof(Vec<Paragraph<'a>>),
    Label(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document<'a> {
    pub preamble: &'a str,
    pub parts: Vec<DocumentPart<'a>>,
}
