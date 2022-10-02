use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1};
use nom::character::complete::{char, none_of, one_of};
use nom::combinator::{cut, opt};
use nom::multi::{count, many0};
use nom::sequence::{pair, tuple};
use nom::{IResult, Parser};

type Error<'a> = nom::error::Error<&'a str>;

type Result<'a, O> = IResult<&'a str, O, Error<'a>>;

pub fn consumed_slice<'a>(before: &'a str, after: &'a str) -> &'a str {
    assert!(after.len() <= before.len());
    let len = before.len() - after.len();
    debug_assert_eq!(&before[len..], after);
    &before[..len]
}

fn non_breaking_ws_char(i: &str) -> Result<()> {
    let (i, _) = one_of(" \t")(i)?;
    Ok((i, ()))
}

fn line_break(i: &str) -> Result<()> {
    let (i, _) = char('\n')(i)?;
    Ok((i, ()))
}

fn ws_char(i: &str) -> Result<()> {
    let (i, _) = one_of(" \t\n")(i)?;
    Ok((i, ()))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct InlineWhitespace<'a>(&'a str);

pub fn inline_ws(i: &str) -> Result<InlineWhitespace> {
    let before = i;

    let (i, _) = many0(non_breaking_ws_char)(i)?;
    let (i, _) = opt(pair(line_break, many0(non_breaking_ws_char)))(i)?;

    Ok((i, InlineWhitespace(consumed_slice(before, i))))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ParagraphBreak<'a>(&'a str);

pub fn paragraph_break(i: &str) -> Result<ParagraphBreak> {
    let before = i;

    let (i, _) = count(pair(many0(non_breaking_ws_char), line_break), 2)(i)?;
    let (i, _) = many0(non_breaking_ws_char)(i)?;

    Ok((i, ParagraphBreak(consumed_slice(before, i))))
}

pub fn any_ws(i: &str) -> Result<()> {
    let (i, _) = many0(ws_char)(i)?;
    Ok((i, ()))
}

#[test]
fn test_whitespace() {
    assert_eq!(inline_ws(""), Ok(("", InlineWhitespace(""))));
    assert_eq!(inline_ws(" \t"), Ok(("", InlineWhitespace(" \t"))));
    assert_eq!(inline_ws("\t "), Ok(("", InlineWhitespace("\t "))));
    assert_eq!(inline_ws("\t \n"), Ok(("", InlineWhitespace("\t \n"))));
    assert_eq!(inline_ws("\t \n  "), Ok(("", InlineWhitespace("\t \n  "))));
    assert_eq!(
        inline_ws("\t \n \n"),
        Ok(("\n", InlineWhitespace("\t \n ")))
    );
    assert_eq!(
        inline_ws("\t \n asdf"),
        Ok(("asdf", InlineWhitespace("\t \n ")))
    );

    assert!(paragraph_break("").is_err());
    assert!(paragraph_break("\t \n").is_err());
    assert_eq!(
        paragraph_break("\t \n \n"),
        Ok(("", ParagraphBreak("\t \n \n")))
    );
    assert_eq!(
        paragraph_break("\t \n \n \t"),
        Ok(("", ParagraphBreak("\t \n \n \t")))
    );
    assert_eq!(
        paragraph_break("\t \n \n asdf"),
        Ok(("asdf", ParagraphBreak("\t \n \n ")))
    );

    assert_eq!(any_ws(""), Ok(("", ())));
    assert_eq!(any_ws("\t\n   \t\n\n\t"), Ok(("", ())));
}

pub fn command_no_args<'a>(name: &'static str) -> impl Fn(&'a str) -> Result<'a, ()> {
    move |i: &'a str| {
        let (i, _) = char('\\')(i)?;
        let (i, _) = tag(name)(i)?;
        Ok((i, ()))
    }
}

pub fn command<'a, O>(
    name: &'static str,
    mut arg_parser: impl FnMut(&'a str) -> Result<'a, O>,
) -> impl FnMut(&'a str) -> Result<'a, O> {
    move |i: &'a str| {
        let (i, _) = char('\\')(i)?;
        let (i, _) = tag(name)(i)?;
        let (i, _) = any_ws(i)?;

        let (i, _) = char('{')(i)?;
        let (i, _) = any_ws(i)?;

        let (i, arg) = arg_parser(i)?;

        let (i, _) = any_ws(i)?;
        let (i, _) = char('}')(i)?;
        Ok((i, arg))
    }
}

pub fn command_with_opts<'a, Opts, Args>(
    name: &'static str,
    mut opt_parser: impl FnMut(&'a str) -> Result<'a, Opts>,
    mut arg_parser: impl FnMut(&'a str) -> Result<'a, Args>,
) -> impl FnMut(&'a str) -> Result<'a, (Option<Opts>, Args)> {
    move |i: &'a str| {
        let (i, _) = char('\\')(i)?;
        let (i, _) = tag(name)(i)?;
        let (i, _) = any_ws(i)?;

        let (i, opts) = opt(tuple((
            char('['),
            inline_ws,
            &mut opt_parser,
            inline_ws,
            char(']'),
            inline_ws,
        )))
        .parse(i)?;
        let opts = opts.map(|opts| opts.2);

        let (i, _) = char('{')(i)?;
        let (i, _) = any_ws(i)?;

        let (i, arg) = arg_parser(i)?;

        let (i, _) = any_ws(i)?;
        let (i, _) = char('}')(i)?;

        Ok((i, (opts, arg)))
    }
}

#[test]
fn test_command() {
    assert_eq!(command("asdf", tag(""))("\\asdf{}"), Ok(("", "")));
    assert_eq!(command("asdf", tag(""))("\\asdf {}"), Ok(("", "")));
    assert_eq!(command("asdf", tag("xyz"))("\\asdf {xyz}"), Ok(("", "xyz")));
    assert_eq!(
        command_with_opts("asdf", tag("123"), tag("xyz"))("\\asdf {xyz}"),
        Ok(("", (None, "xyz")))
    );
    assert_eq!(
        command_with_opts("asdf", tag("123"), tag("xyz"))("\\asdf [123] {xyz}"),
        Ok(("", (Some("123"), "xyz")))
    );
}

pub fn env<'a, O>(
    name: &'static str,
    mut content_parser: impl FnMut(&'a str) -> Result<'a, O>,
) -> impl FnMut(&'a str) -> Result<'a, O> {
    move |i: &'a str| {
        let (i, _) = command("begin", tag(name))(i)?;
        cut(|i: &'a str| {
            let (i, _) = inline_ws(i)?;

            let (i, content) = content_parser(i)?;

            let (i, _) = inline_ws(i)?;
            let (i, _) = command("end", tag(name))(i)?;

            Ok((i, content))
        })(i)
    }
}

#[test]
fn test_env() {
    assert_eq!(
        env("asdf", tag("123"))("\\begin{asdf}\n123\n\\end{asdf}"),
        Ok(("", "123"))
    );
    assert!(env("asdf", tag("123"))("\\begin{asdf}\n1\n\\end{asdf}").is_err());
    assert!(env("asdf", tag("123"))("\\begin{asdf}\n123\n\\end{xyz}").is_err());
}

pub fn take_until<'a, O>(
    mut until_parser: impl FnMut(&'a str) -> Result<'a, O>,
) -> impl FnMut(&'a str) -> Result<'a, (&'a str, O)> {
    move |mut i: &'a str| {
        let before = i;
        loop {
            match until_parser(i) {
                Ok((j, x)) => {
                    return Ok((j, (consumed_slice(before, i), x)));
                }
                Err(_) => {
                    let mut chars = i.chars();
                    if let None = chars.next() {
                        return Err(nom::Err::Error(Error::new(i, nom::error::ErrorKind::IsNot)));
                    }
                    i = chars.as_str();
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RawContent<'a>(&'a str);

pub fn raw_env<'a>(name: &'static str) -> impl Fn(&'a str) -> Result<'a, RawContent<'a>> {
    move |i: &'a str| {
        let (i, _) = command("begin", tag(name))(i)?;
        let (i, _) = inline_ws(i)?;
        let (i, (content, _)) = take_until(pair(inline_ws, command("end", tag(name))))(i)?;
        Ok((i, RawContent(content)))
    }
}

#[test]
fn test_raw_env() {
    assert_eq!(
        raw_env("asdf")("\\begin{asdf}123\\end{asdf}"),
        Ok(("", RawContent("123")))
    );
    assert_eq!(
        raw_env("asdf")("\\begin{asdf}\n123\n\\end{asdf}"),
        Ok(("", RawContent("123")))
    );

    // TODO: This is somewhat questionable -- should we forget about the empty line?
    assert_eq!(
        raw_env("asdf")("\\begin{asdf}\n\n\\end{asdf}"),
        Ok(("", RawContent("")))
    );
}

pub fn no_arg_command<'a>(name: &'static str) -> impl FnMut(&'a str) -> Result<()> {
    move |i: &'a str| {
        let (i, _) = char('\\')(i)?;
        let (i, _) = tag(name).parse(i)?;
        Ok((i, ()))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TextToken<'a>(&'a str);

const SPECIAL_CHARS: &'static str = " \n\t#$%&{}_~^\\";

pub fn text_token(i: &str) -> Result<TextToken> {
    let before = i;
    let (i, _) = none_of(SPECIAL_CHARS)(i)?;
    let (i, _) = take_while(|c| !SPECIAL_CHARS.contains(c))(i)?;
    Ok((i, TextToken(consumed_slice(before, i))))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct InlineMath<'a>(&'a str);

pub fn inline_math(i: &str) -> Result<InlineMath> {
    let (i, _) = char('$')(i)?;
    let (i, math) = take_while(|c| c != '$')(i)?;
    let (i, _) = char('$')(i)?;
    Ok((i, InlineMath(math)))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DisplayMath<'a>(&'a str);

pub fn display_math(i: &str) -> Result<DisplayMath> {
    let (i, content) = raw_env("equation")(i)?;
    Ok((i, DisplayMath(content.0)))
}

pub fn label_value(i: &str) -> Result<&str> {
    take_while1(|c| c != '{' && c != '}')(i)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Ref<'a>(&'a str);

pub fn ref_command(i: &str) -> Result<Ref> {
    let (i, val) = command("ref", label_value)(i)?;
    Ok((i, Ref(val)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emph<'a>(Paragraph<'a>);

pub fn emph(i: &str) -> Result<Emph> {
    let (i, par) = command("emph", paragraph)(i)?;
    Ok((i, Emph(par)))
}

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

pub fn comment(i: &str) -> Result<ParagraphPart> {
    let (i, _) = char('%')(i)?;
    let (i, comment) = take_while(|c| c != '\n')(i)?;
    Ok((i, ParagraphPart::Comment(comment)))
}

pub fn paragraph_label(i: &str) -> Result<ParagraphPart> {
    command("label", label_value)
        .map(ParagraphPart::Label)
        .parse(i)
}

pub fn paragraph_qed(i: &str) -> Result<ParagraphPart> {
    let (i, _) = command_no_args("qed")(i)?;
    Ok((i, ParagraphPart::Qed))
}

pub fn eqref(i: &str) -> Result<ParagraphPart> {
    let (i, val) = command("eqref", label_value)(i)?;
    Ok((i, ParagraphPart::Eqref(val)))
}

pub type Paragraph<'a> = Vec<ParagraphPart<'a>>;

pub fn paragraph<'a>(i: &'a str) -> Result<Paragraph<'a>> {
    let ws_part = |i: &'a str| {
        let (i, ws) = inline_ws(i)?;
        Ok((i, ParagraphPart::InlineWhitespace(ws.0)))
    };

    let text = |i: &'a str| {
        let (i, tok) = text_token(i)?;
        Ok((i, ParagraphPart::TextToken(tok.0)))
    };
    let inline_math = |i: &'a str| {
        let (i, inl_math) = inline_math(i)?;
        Ok((i, ParagraphPart::InlineMath(inl_math.0)))
    };
    let display_math = |i: &'a str| {
        let (i, disp_math) = display_math(i)?;
        Ok((i, ParagraphPart::DisplayMath(disp_math.0)))
    };
    let ref_command = |i: &'a str| {
        let (i, r) = ref_command(i)?;
        Ok((i, ParagraphPart::Ref(r.0)))
    };
    let emph = |i: &'a str| {
        let (i, emph) = emph(i)?;
        Ok((i, ParagraphPart::Emph(emph.0)))
    };

    let non_ws_part = |i: &'a str| {
        alt((
            text,
            inline_math,
            display_math,
            ref_command,
            eqref,
            emph,
            comment,
            paragraph_label,
            paragraph_qed,
        ))(i)
    };

    let (mut i, head) = non_ws_part(i)?;
    let mut result = vec![head];

    loop {
        let (j, parts) = opt(pair(ws_part, non_ws_part))(i)?;
        i = j;
        match parts {
            None => {
                break;
            }
            Some((ws, non_ws)) => {
                result.push(ws);
                result.push(non_ws);
            }
        };
    }

    Ok((i, result))
}

#[test]
fn paragraph_test() {
    let p1 = indoc::indoc! {"
        asdf fjfj  \t $ 5 = \\mathbb{N}$
        \\ref{123}
        \\begin{equation}
            k = 5
        \\end{equation}

        123
    "};

    let (i, paragraph) = paragraph(p1).unwrap();
    assert_eq!(i, "\n\n123\n");

    use ParagraphPart::*;
    assert_eq!(
        paragraph,
        vec![
            TextToken("asdf"),
            InlineWhitespace(" "),
            TextToken("fjfj"),
            InlineWhitespace("  \t "),
            InlineMath(" 5 = \\mathbb{N}"),
            InlineWhitespace("\n"),
            Ref("123"),
            InlineWhitespace("\n"),
            DisplayMath("k = 5"),
        ]
    );
}

fn intersperse0<'a, Item, Sep>(
    mut item_parser: impl FnMut(&'a str) -> Result<'a, Item>,
    mut sep_parser: impl FnMut(&'a str) -> Result<'a, Sep>,
) -> impl FnMut(&'a str) -> Result<'a, Vec<Item>> {
    move |i: &'a str| {
        let (mut i, head) = opt(&mut item_parser)(i)?;
        let head = match head {
            Some(head) => head,
            None => {
                return Ok((i, Vec::new()));
            }
        };

        let mut result = vec![head];
        loop {
            let (j, sep_item) = opt(pair(&mut sep_parser, &mut item_parser))(i)?;
            i = j;
            match sep_item {
                None => {
                    break;
                }
                Some((_, item)) => {
                    result.push(item);
                }
            };
        }
        Ok((i, result))
    }
}

fn paragraphs0<'a>(i: &'a str) -> Result<'a, Vec<Paragraph<'a>>> {
    intersperse0(paragraph, any_ws)(i)
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
    Proposition(Vec<Paragraph<'a>>),
    Definition(Vec<Paragraph<'a>>),
    Lemma(Vec<Paragraph<'a>>),
    Proof(Vec<Paragraph<'a>>),
    Label(&'a str),
}

pub fn title<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    command("title", paragraph)
        .map(DocumentPart::Title)
        .parse(i)
}

pub fn author<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    command("author", paragraph)
        .map(DocumentPart::Author)
        .parse(i)
}

pub fn date<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    let (i, _) = command("date", many0(none_of("{}"))).parse(i)?;
    Ok((i, DocumentPart::Date()))
}

pub fn maketitle<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    let (i, _) = command_no_args("maketitle")(i)?;
    Ok((i, DocumentPart::Maketitle()))
}

pub fn section<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    command("section", paragraph)
        .map(DocumentPart::Section)
        .parse(i)
}

pub fn subsection<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    command("subsection", paragraph)
        .map(DocumentPart::Subsection)
        .parse(i)
}

pub fn abstract_env<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    env("abstract", paragraphs0)
        .map(DocumentPart::Abstract)
        .parse(i)
}

pub fn label<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    command("label", label_value)
        .map(DocumentPart::Label)
        .parse(i)
}

pub fn proposition<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    env("proposition", paragraphs0)
        .map(DocumentPart::Proposition)
        .parse(i)
}

pub fn definition<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    env("definition", paragraphs0)
        .map(DocumentPart::Definition)
        .parse(i)
}

pub fn lemma<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    env("lemma", paragraphs0).map(DocumentPart::Lemma).parse(i)
}

pub fn proof<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    env("proof", paragraphs0).map(DocumentPart::Proof).parse(i)
}

pub fn document_part<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    let free_paragraph = paragraph.map(DocumentPart::FreeParagraph);
    let (i, part) = alt((
        free_paragraph,
        title,
        author,
        date,
        maketitle,
        section,
        subsection,
        abstract_env,
        label,
        proposition,
        definition,
        lemma,
        proof,
    ))(i)?;
    Ok((i, part))
}

pub fn documentclass<'a>(i: &'a str) -> Result<()> {
    let (i, _) = command_with_opts(
        "documentclass",
        many0(none_of("[]{}")),
        many0(none_of("[]{}")),
    )(i)?;
    Ok((i, ()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document<'a> {
    pub preamble: &'a str,
    pub parts: Vec<DocumentPart<'a>>,
}

pub fn document<'a>(i: &'a str) -> Result<Document<'a>> {
    let (i, _) = any_ws(i)?;
    let (i, _) = documentclass(i)?;
    let (i, (preamble, _)) = take_until(command("begin", tag("document")))(i)?;
    let (i, _) = any_ws(i)?;
    let (i, parts) = intersperse0(document_part, any_ws)(i)?;
    let (i, _) = any_ws(i)?;
    let (i, _) = command("end", tag("document"))(i)?;
    let (i, _) = any_ws(i)?;
    Ok((i, Document { preamble, parts }))
}
