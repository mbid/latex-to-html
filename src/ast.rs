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
    Eqref(&'a str),
    Emph(Paragraph<'a>),
    Comment(&'a str),
    Label(&'a str),
    Qed,
    Enumerate(Vec<Vec<Paragraph<'a>>>),
    Itemize(Vec<Vec<Paragraph<'a>>>),
    Todo,
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
    Remark(Vec<Paragraph<'a>>),
    Corollary(Vec<Paragraph<'a>>),
    Theorem(Vec<Paragraph<'a>>),
    Proof(Vec<Paragraph<'a>>),
    Label(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document<'a> {
    pub preamble: &'a str,
    pub parts: Vec<DocumentPart<'a>>,
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
        Eqref(_) => (),
        Emph(par) => {
            for part in par {
                for_each_math_paragraph_part_impl(part, f);
            }
        }
        Comment(_) => (),
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
            Proposition(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| part.for_each_math(&mut f));
            }
            Definition(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| part.for_each_math(&mut f));
            }
            Lemma(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| part.for_each_math(&mut f));
            }
            Remark(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| part.for_each_math(&mut f));
            }
            Corollary(pars) => {
                pars.iter()
                    .flatten()
                    .for_each(|part| part.for_each_math(&mut f));
            }
            Theorem(pars) => {
                pars.iter()
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
