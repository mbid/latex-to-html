use crate::ast::*;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1};
use nom::character::complete::{char, none_of, one_of};
use nom::combinator::{cut, opt};
use nom::multi::{many0, many1};
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

const SPECIAL_CHARS: &'static str = " \n\t#$%&{}_~^\\";

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

pub fn paragraph_qed(i: &str) -> Result<ParagraphPart> {
    let (i, _) = command_no_args("qed")(i)?;
    Ok((i, ParagraphPart::Qed))
}

pub fn eqref(i: &str) -> Result<ParagraphPart> {
    let (i, val) = command("eqref", label_value)(i)?;
    Ok((i, ParagraphPart::Ref(val)))
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
            inline_math.map(ParagraphPart::Math),
            display_math.map(ParagraphPart::Math),
            mathpar.map(ParagraphPart::Math),
            ref_command,
            eqref,
            emph,
            paragraph_qed,
            itemize,
            enumerate,
            todo,
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
        let (i, label) = opt(command("label", label_value))(i)?;
        let (i, _) = inline_ws(i)?;
        let (i, content) = paragraphs0(i)?;
        Ok((
            i,
            DocumentPart::TheoremLike {
                tag: first.tag,
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
