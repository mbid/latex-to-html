use crate::ast::*;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1};
use nom::character::complete::{char, digit1, none_of, one_of};
use nom::combinator::{cut, opt};
use nom::multi::{many0, many1};
use nom::sequence::{pair, tuple};
use nom::{IResult, Parser};
use std::str::FromStr;

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

fn non_breaking_ws(i: &str) -> Result<()> {
    let (i, _) = many0(non_breaking_ws_char)(i)?;
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

fn comment(i: &str) -> Result<()> {
    let (i, _) = char('%')(i)?;
    let (i, _) = take_while(|c| c != '\n')(i)?;
    Ok((i, ()))
}

fn ignore(i: &str) -> Result<()> {
    let (i, _) = char('%')(i)?;
    let (i, _) = non_breaking_ws(i)?;
    let (i, _) = tag("LATEX_TO_HTML_IGNORE")(i)?;
    let (i, _) = non_breaking_ws(i)?;
    let (i, _) = cut(|i| {
        let (i, _) = char('\n')(i)?;
        let (i, _) = take_while(|c| c != '\n')(i)?;
        Ok((i, ()))
    })(i)?;

    Ok((i, ()))
}

pub fn inline_ws(i: &str) -> Result<InlineWhitespace> {
    let before = i;

    let (i, _) = non_breaking_ws(i)?;
    let (i, _) = opt(comment)(i)?;
    let (i, lb) = opt(line_break)(i)?;
    if let None = lb {
        return Ok((i, InlineWhitespace(consumed_slice(before, i))));
    }

    let (i, _) = many0(tuple((
        non_breaking_ws,
        alt((ignore, comment)),
        opt(line_break),
    )))(i)?;
    let (i, _) = non_breaking_ws(i)?;

    Ok((i, InlineWhitespace(consumed_slice(before, i))))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ParagraphBreak<'a>(&'a str);

pub fn any_ws(i: &str) -> Result<()> {
    let (i, _) = many0(alt((ignore, comment, ws_char)))(i)?;
    Ok((i, ()))
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

pub fn command_with_opts<'a, Name, Opts, Args>(
    mut name_parser: impl FnMut(&'a str) -> Result<'a, Name>,
    mut opt_parser: impl FnMut(&'a str) -> Result<'a, Opts>,
    mut arg_parser: impl FnMut(&'a str) -> Result<'a, Args>,
) -> impl FnMut(&'a str) -> Result<'a, (Option<Opts>, Args)> {
    move |i: &'a str| {
        let (i, _) = char('\\')(i)?;
        let (i, _) = name_parser(i)?;
        let (i, _) = any_ws(i)?;

        let (i, opts) = opt(tuple((
            char('['),
            any_ws,
            &mut opt_parser,
            any_ws,
            char(']'),
            any_ws,
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

pub fn raw_command_arg(i: &str) -> Result<&str> {
    let before = i;
    let mut nesting = 0;
    let mut it = i.chars();
    loop {
        match it.clone().next() {
            None => break,
            Some('{') => {
                nesting += 1;
            }
            Some('}') => {
                if nesting == 0 {
                    break;
                } else {
                    nesting -= 1;
                }
            }
            Some(_) => (),
        }
        it.next();
    }

    let i = it.as_str();
    Ok((i, consumed_slice(before, i)))
}

pub fn raw_command<'a>(name: &'static str) -> impl FnMut(&'a str) -> Result<'a, &'a str> {
    move |i: &'a str| command(name, raw_command_arg)(i)
}

pub fn dyn_env<'a, T, O>(
    mut tag_parser: impl FnMut(&'a str) -> Result<'a, T>,
    mut content_parser: impl FnMut(&'a str) -> Result<'a, O>,
) -> impl FnMut(&'a str) -> Result<'a, O> {
    move |i: &'a str| {
        let (i, _) = command("begin", &mut tag_parser)(i)?;
        cut(|i: &'a str| {
            let (i, _) = inline_ws(i)?;

            let (i, content) = content_parser(i)?;

            let (i, _) = inline_ws(i)?;
            let (i, _) = command("end", &mut tag_parser)(i)?;

            Ok((i, content))
        })(i)
    }
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

pub fn raw_env<'a>(name: &'static str) -> impl Fn(&'a str) -> Result<'a, &'a str> {
    move |i: &'a str| {
        let (i, _) = command("begin", tag(name))(i)?;
        let (i, _) = inline_ws(i)?;
        let (i, (content, _)) = take_until(pair(inline_ws, command("end", tag(name))))(i)?;
        Ok((i, content))
    }
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

// TODO: [] is special only in certain contexts, e.g. when parsing the options of a command.
const SPECIAL_CHARS: &'static str = " \n\t#$%&{}[]_~^\\";

pub fn text_token(i: &str) -> Result<TextToken> {
    let before = i;
    let (i, _) = none_of(SPECIAL_CHARS)(i)?;
    let (i, _) = take_while(|c| !SPECIAL_CHARS.contains(c))(i)?;
    Ok((i, TextToken(consumed_slice(before, i))))
}

pub fn inline_math(i: &str) -> Result<Math> {
    let (i, _) = char('$')(i)?;
    let (i, math) = take_while(|c| c != '$')(i)?;
    let (i, _) = char('$')(i)?;
    Ok((i, Math::Inline(math)))
}

pub fn display_math(i: &str) -> Result<Math> {
    let (i, mut source) = raw_env("equation")(i)?;
    let label = match opt(command("label", label_value))(source)? {
        (_, None) => None,
        (j, Some(label)) => {
            let (j, _) = inline_ws(j)?;
            source = j;
            Some(label)
        }
    };

    Ok((i, Math::Display { source, label }))
}
pub fn display_math_double_dollar(i: &str) -> Result<Math> {
    let (i, _) = tag("$$")(i)?;
    let (i, _) = inline_ws(i)?;

    let (i, label) = match opt(command("label", label_value))(i)? {
        (j, None) => (j, None),
        (j, Some(label)) => {
            let (j, _) = inline_ws(j)?;
            (j, Some(label))
        }
    };

    let (i, source) = take_while(|c| c != '$')(i)?;
    let source = source.trim_end();

    let (i, _) = tag("$$")(i)?;

    Ok((i, Math::Display { source, label }))
}

pub fn mathpar(i: &str) -> Result<Math> {
    let (i, mut source) = raw_env("mathpar")(i)?;
    let label = match opt(command("label", label_value))(source)? {
        (_, None) => None,
        (j, Some(label)) => {
            let (j, _) = inline_ws(j)?;
            source = j;
            Some(label)
        }
    };

    Ok((i, Math::Mathpar { source, label }))
}

pub fn label_value(i: &str) -> Result<&str> {
    take_while1(|c: char| "-_:".find(c).is_some() || c.is_ascii_alphanumeric())(i)
}

pub fn cite_value(i: &str) -> Result<&str> {
    take_while1(|c: char| "-_:".find(c).is_some() || c.is_ascii_alphanumeric())(i)
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

pub fn textbf(i: &str) -> Result<ParagraphPart> {
    let (i, par) = command("textbf", paragraph)(i)?;
    Ok((i, ParagraphPart::Textbf(par)))
}

pub fn textit(i: &str) -> Result<ParagraphPart> {
    let (i, par) = command("textit", paragraph)(i)?;
    Ok((i, ParagraphPart::Textit(par)))
}

pub fn paragraph_qed(i: &str) -> Result<ParagraphPart> {
    let (i, _) = command_no_args("qed")(i)?;
    Ok((i, ParagraphPart::Qed))
}

pub fn eqref(i: &str) -> Result<ParagraphPart> {
    let (i, val) = command("eqref", label_value)(i)?;
    Ok((i, ParagraphPart::Ref(val)))
}

pub fn cite(i: &str) -> Result<ParagraphPart> {
    let arg_sep = tuple((any_ws, tag(","), any_ws));
    let arg_parser = intersperse0(cite_value, arg_sep);
    let opt_parser = paragraph;
    let command_name_parser = alt((tag("citep"), tag("citet"), tag("cite")));
    let (i, (text, ids)) = command_with_opts(command_name_parser, opt_parser, arg_parser)(i)?;
    Ok((i, ParagraphPart::Cite { text, ids }))
}

pub fn item(i: &str) -> Result<Item> {
    let (i, _) = command_no_args("item")(i)?;
    let (i, label) = opt(|i| {
        let (i, _) = any_ws(i)?;
        let (i, val) = command("label", label_value)(i)?;
        Ok((i, val))
    })(i)?;
    let (i, _) = inline_ws(i)?;
    let (i, content) = many1(paragraph)(i)?;
    let item = Item { content, label };
    Ok((i, item))
}

pub fn itemize(i: &str) -> Result<ParagraphPart> {
    let (i, items) = env("itemize", intersperse0(item, any_ws))(i)?;
    for item in items.iter() {
        assert!(
            item.label.is_none(),
            "Label for item in an itemize environment not allowed"
        );
    }
    Ok((i, ParagraphPart::Itemize(items)))
}

pub fn enumerate(i: &str) -> Result<ParagraphPart> {
    let (i, items) = env("enumerate", intersperse0(item, any_ws))(i)?;
    Ok((i, ParagraphPart::Enumerate(items)))
}

pub fn todo(i: &str) -> Result<ParagraphPart> {
    let (i, _) = raw_command("todo")(i)?;
    Ok((i, ParagraphPart::Todo))
}

pub fn footnote(i: &str) -> Result<ParagraphPart> {
    let (i, content) = command("footnote", intersperse0(paragraph, any_ws))(i)?;
    Ok((i, ParagraphPart::Footnote(content)))
}

pub fn paragraph<'a>(i: &'a str) -> Result<Paragraph<'a>> {
    let ws_part = |i: &'a str| {
        let (i, ws) = inline_ws(i)?;
        Ok((i, ParagraphPart::InlineWhitespace(ws.0)))
    };
    let text = |i: &'a str| {
        let (i, tok) = text_token(i)?;
        Ok((i, ParagraphPart::TextToken(tok.0)))
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
            display_math.map(ParagraphPart::Math),
            display_math_double_dollar.map(ParagraphPart::Math),
            inline_math.map(ParagraphPart::Math),
            mathpar.map(ParagraphPart::Math),
            ref_command,
            eqref,
            cite,
            emph,
            textbf,
            textit,
            paragraph_qed,
            itemize,
            enumerate,
            todo,
            footnote,
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
    let (i, name) = command("section", paragraph)(i)?;
    let (i, label) = opt(|i| {
        let (i, _) = any_ws(i)?;
        let (i, val) = command("label", label_value)(i)?;
        Ok((i, val))
    })(i)?;
    Ok((i, DocumentPart::Section { name, label }))
}

pub fn subsection<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    let (i, name) = command("subsection", paragraph)(i)?;
    let (i, label) = opt(|i| {
        let (i, _) = any_ws(i)?;
        let (i, val) = command("label", label_value)(i)?;
        Ok((i, val))
    })(i)?;
    Ok((i, DocumentPart::Subsection { name, label }))
}

pub fn abstract_env<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    env("abstract", paragraphs0)
        .map(DocumentPart::Abstract)
        .parse(i)
}

pub fn theorem_like<'a, 'b>(
    configs: &'b [TheoremLikeConfig<'a>],
    i: &'a str,
) -> Result<'a, DocumentPart<'a>> {
    let (first, tail) = match configs {
        [] => {
            return Err(nom::Err::Error(Error::new(i, nom::error::ErrorKind::IsNot)));
        }
        [first, tail @ ..] => (first, tail),
    };

    let head_content_parser = |i: &'a str| {
        let (i, note_tuple) = opt(tuple((char('['), any_ws, paragraph, any_ws, char(']'))))(i)?;
        let note = note_tuple.map(|t| t.2);
        let (i, _) = inline_ws(i)?;

        let (i, label) = opt(command("label", label_value))(i)?;
        let (i, _) = inline_ws(i)?;

        let (i, content) = paragraphs0(i)?;
        Ok((
            i,
            DocumentPart::TheoremLike {
                tag: first.tag,
                note,
                label,
                content,
            },
        ))
    };
    let head_parser = |i: &'a str| {
        let (i, doc_part) = dyn_env(tag(first.tag), head_content_parser)(i)?;
        Ok((i, doc_part))
    };

    let tail_parser: Box<dyn Fn(&'a str) -> Result<'a, DocumentPart<'a>>> =
        Box::new(move |i| theorem_like(tail, i));

    let (i, doc_part) = alt((head_parser, tail_parser))(i)?;
    Ok((i, doc_part))
}

pub fn proof<'a>(i: &'a str) -> Result<DocumentPart<'a>> {
    env("proof", paragraphs0).map(DocumentPart::Proof).parse(i)
}

pub fn bibliography<'a>(i: &'a str) -> Result<'a, DocumentPart<'a>> {
    let (i, _) = command("bibliography", take_while(|c| c != '{' && c != '}'))(i)?;
    Ok((i, DocumentPart::Bibliography))
}

pub fn document_part<'a, 'b>(
    config: &'b DocumentConfig<'a>,
    i: &'a str,
) -> Result<'a, DocumentPart<'a>> {
    let free_paragraph = paragraph.map(DocumentPart::FreeParagraph);
    let theorem_like = |i| theorem_like(&config.theorem_like_configs, i);
    let (i, part) = alt((
        free_paragraph,
        title,
        author,
        date,
        maketitle,
        section,
        subsection,
        abstract_env,
        theorem_like,
        proof,
        bibliography,
    ))(i)?;
    Ok((i, part))
}

pub fn documentclass<'a>(i: &'a str) -> Result<()> {
    let (i, _) = command_with_opts(
        tag("documentclass"),
        many0(none_of("[]{}")),
        many0(none_of("[]{}")),
    )(i)?;
    Ok((i, ()))
}

pub fn preamble_lines<'a>(mut i: &'a str) -> Result<'a, Vec<&'a str>> {
    let before = i;
    let mut result = Vec::new();

    loop {
        if let (j, Some(_)) = opt(pair(non_breaking_ws, ignore))(i)? {
            i = j;
        } else {
            let (j, _) = take_while(|c| c != '\n')(i)?;
            result.push(consumed_slice(i, j));
            i = j;
        }

        if let (j, Some(_)) = opt(char('\n'))(i)? {
            i = j;
            continue;
        }

        return Ok((consumed_slice(before, i), result));
    }
}

pub fn document<'a>(i: &'a str) -> Result<Document<'a>> {
    let (i, _) = any_ws(i)?;
    let (i, _) = documentclass(i)?;
    let (i, (preamble, _)) = take_until(command("begin", tag("document")))(i)?;
    let preamble = preamble_lines(preamble).unwrap().1.join("\n");
    let config = DocumentConfig::default();
    let (i, _) = any_ws(i)?;
    let document_part = |i: &'a str| document_part(&config, i);
    let (i, parts) = intersperse0(document_part, any_ws)(i)?;
    let (i, _) = any_ws(i)?;
    let (i, _) = command("end", tag("document"))(i)?;
    let (i, _) = any_ws(i)?;
    let doc = Document {
        config,
        preamble,
        parts,
    };
    Ok((i, doc))
}

pub fn bib_ws<'a>(i: &'a str) -> Result<'a, ()> {
    let (i, _) = take_while(|c| " \t\n".find(c).is_some())(i)?;
    Ok((i, ()))
}

pub fn bib_entry_type<'a>(i: &'a str) -> Result<'a, BibEntryType> {
    use BibEntryType::*;
    alt((
        tag("misc").map(|_| Misc),
        tag("article").map(|_| Article),
        tag("book").map(|_| Book),
        tag("inproceedings").map(|_| Inproceedings),
        tag("thesis").map(|_| Thesis),
        tag("incollection").map(|_| Incollection),
    ))(i)
}

fn bib_entry_tag<'a>(i: &'a str) -> Result<'a, &'a str> {
    let (i, val) = take_while(|c| " \t\n,".find(c).is_none())(i)?;
    Ok((i, val))
}

fn bib_entry_item<'a, O, N>(
    mut name_parser: impl FnMut(&'a str) -> Result<'a, N>,
    mut value_parser: impl FnMut(&'a str) -> Result<'a, O>,
) -> impl FnMut(&'a str) -> Result<'a, O> {
    move |i| {
        let (i, _) = name_parser(i)?;
        let (i, _) = bib_ws(i)?;
        let (i, _) = char('=')(i)?;
        let (i, _) = bib_ws(i)?;
        let (i, _) = char('{')(i)?;
        let (i, _) = bib_ws(i)?;
        let (i, o) = value_parser(i)?;
        let (i, _) = bib_ws(i)?;
        let (i, _) = char('}')(i)?;
        Ok((i, o))
    }
}

fn bib_item_raw_value<'a>(i: &'a str) -> Result<'a, &'a str> {
    //let (i, value) = take_while(|c| c != '{' && c != '}')(i)?;
    let (i, value) = raw_command_arg(i)?;
    Ok((i, value.trim_end()))
}

fn bib_title_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("title"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Title(val)))
}

fn bib_year_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("year"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Year(val)))
}

fn bib_abbreviated_first_name<'a>(i: &'a str) -> Result<'a, FirstName<'a>> {
    let before = i;
    let (i, _) = none_of(",;={} \t\n")(i)?;
    let first_name = FirstName::Abbreviation(consumed_slice(before, i));
    let (i, _) = char('.')(i)?;
    Ok((i, first_name))
}

fn bib_full_first_name<'a>(i: &'a str) -> Result<'a, FirstName<'a>> {
    let before = i;
    let (i, _) = take_while1(|c| !",;={}. \t\n".contains(c))(i)?;
    let value = consumed_slice(before, i);
    if value == "and" {
        return Err(nom::Err::Error(Error::new(i, nom::error::ErrorKind::IsA)));
    }
    let first_name = FirstName::Full(value);
    Ok((i, first_name))
}

fn bib_first_name<'a>(i: &'a str) -> Result<'a, FirstName<'a>> {
    alt((bib_abbreviated_first_name, bib_full_first_name))(i)
}

fn bib_last_name<'a>(i: &'a str) -> Result<'a, &'a str> {
    let before = i;
    let (i, _) = take_while1(|c| !",;={}. \t\n".contains(c))(i)?;
    let value = consumed_slice(before, i);
    if value == "and" {
        return Err(nom::Err::Error(Error::new(i, nom::error::ErrorKind::IsA)));
    }
    Ok((i, value))
}

fn bib_person<'a>(i: &'a str) -> Result<'a, BibPerson> {
    let (i, last_name) = bib_last_name(i)?;
    let (i, _) = bib_ws(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = bib_ws(i)?;
    let (i, first_names) = intersperse0(bib_first_name, bib_ws)(i)?;
    Ok((
        i,
        BibPerson {
            first_names,
            last_name,
        },
    ))
}

fn bib_authors_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let sep = tuple((bib_ws, tag("and"), bib_ws));
    let (i, authors) = bib_entry_item(
        alt((tag("author"), tag("author"))),
        intersperse0(bib_person, sep),
    )(i)?;
    Ok((i, BibEntryItem::Authors(authors)))
}

fn bib_url_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("url"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Url(val)))
}

fn bib_journal_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("journal"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Journal(val)))
}

fn bib_booktitle_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("booktitle"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Booktitle(val)))
}

fn bib_series_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("series"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Series(val)))
}

fn bib_publisher_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("publisher"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Publisher(val)))
}

fn bib_volume_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("volume"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Volume(val)))
}

fn bib_number_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let (i, val) = bib_entry_item(tag("number"), bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Number(val)))
}

fn bib_pages_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    bib_entry_item(tag("pages"), |i| {
        let (i, first) = digit1(i)?;
        let first = u64::from_str(first).unwrap();
        let (i, last) = opt(|i| {
            let (i, _) = alt((tag("--"), tag("–"), tag("-")))(i)?;
            let (i, last) = digit1(i)?;
            let last = u64::from_str(last).unwrap();
            Ok((i, last))
        })(i)?;
        Ok((i, BibEntryItem::Pages(BibPages { first, last })))
    })(i)
}

fn unused_bib_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    let name = take_while(|c| !" ={}".contains(c));
    let (i, _) = bib_entry_item(name, bib_item_raw_value)(i)?;
    Ok((i, BibEntryItem::Unused))
}

fn bib_item<'a>(i: &'a str) -> Result<'a, BibEntryItem> {
    alt((
        bib_title_item,
        bib_year_item,
        bib_authors_item,
        bib_url_item,
        bib_journal_item,
        bib_booktitle_item,
        bib_series_item,
        bib_publisher_item,
        bib_volume_item,
        bib_number_item,
        bib_pages_item,
        unused_bib_item,
    ))(i)
}

fn make_bib_entry<'a, 'b>(
    entry_type: BibEntryType,
    tag: &'a str,
    items: Vec<BibEntryItem<'a>>,
) -> BibEntry<'a> {
    let mut result = BibEntry {
        tag,
        entry_type,
        title: None,
        year: None,
        authors: None,
        url: None,
        journal: None,
        booktitle: None,
        series: None,
        publisher: None,
        volume: None,
        number: None,
        pages: None,
    };

    for item in items {
        use BibEntryItem::*;
        match item {
            Title(title) => {
                assert!(result.title.is_none(), "Duplicate title value");
                result.title = Some(title);
            }
            Year(year) => {
                assert!(result.year.is_none(), "Duplicate year value");
                result.year = Some(year);
            }
            Authors(authors) => {
                assert!(result.authors.is_none(), "Duplicate authors value");
                result.authors = Some(authors);
            }
            Url(url) => {
                assert!(result.url.is_none(), "Duplicate url value");
                result.url = Some(url);
            }
            Journal(journal) => {
                assert!(result.journal.is_none(), "Duplicate journal value");
                result.journal = Some(journal);
            }
            Booktitle(booktitle) => {
                assert!(result.booktitle.is_none(), "Duplicate booktitle value");
                result.booktitle = Some(booktitle);
            }
            Series(series) => {
                assert!(result.series.is_none(), "Duplicate series value");
                result.series = Some(series);
            }
            Publisher(publisher) => {
                assert!(result.publisher.is_none(), "Duplicate publisher value");
                result.publisher = Some(publisher);
            }
            Volume(volume) => {
                assert!(result.volume.is_none(), "Duplicate volume value");
                result.volume = Some(volume);
            }
            Number(number) => {
                assert!(result.number.is_none(), "Duplicate number value");
                result.number = Some(number);
            }
            Pages(pages) => {
                assert!(result.pages.is_none(), "Duplicate pages value");
                result.pages = Some(pages);
            }
            Unused => (),
        }
    }

    result
}

pub fn bib_entry<'a>(i: &'a str) -> Result<'a, BibEntry<'a>> {
    let (i, _) = char('@')(i)?;
    let (i, entry_type) = bib_entry_type(i)?;
    let (i, _) = bib_ws(i)?;
    let (i, _) = char('{')(i)?;
    let (i, _) = bib_ws(i)?;

    let (i, tag) = bib_entry_tag(i)?;

    let (i, _) = bib_ws(i)?;
    let (i, _) = char(',')(i)?;
    let (i, _) = bib_ws(i)?;

    let item_sep = tuple((bib_ws, char(','), bib_ws));
    let (i, items) = intersperse0(bib_item, item_sep)(i)?;

    let (i, _) = bib_ws(i)?;

    let (i, _) = opt(pair(char(','), bib_ws))(i)?;

    let (i, _) = char('}')(i)?;

    Ok((i, make_bib_entry(entry_type, tag, items)))
}

pub fn bib<'a>(i: &'a str) -> Result<'a, Vec<BibEntry<'a>>> {
    let (i, _) = bib_ws(i)?;
    let (i, entries) = intersperse0(bib_entry, bib_ws)(i)?;
    let (i, _) = bib_ws(i)?;
    Ok((i, entries))
}
