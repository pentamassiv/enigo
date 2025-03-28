use std::fmt::Display;
use std::fs;
use std::path::Path;

use nom::branch::permutation;
use nom::bytes::complete::{is_not, tag, take_until};
use nom::character::complete::{alphanumeric0, multispace0};
use nom::character::complete::{char, digit1};
use nom::character::streaming::u32;
use nom::error::ParseError;
use nom::multi::many0;
use nom::sequence::{delimited, pair, separated_pair, terminated};
use nom::{IResult, Parser};

type Keycode = u32;

trait Parse {
    fn parse(input: &str) -> IResult<&str, Self>
    where
        Self: Sized;
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
struct Keymap {
    keycodes: Keycodes,
    // Don't parse this, just keep it as is
    types: String,
    // Don't parse this, just keep it as is
    compatibility: String,
    symbols: Symbols,
    // Don't parse this, just keep it as is
    geometry: String,
}

impl Display for Keymap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "xkb_keymap {{")?;
        write!(
            f,
            "{}{}{}{}{}",
            self.keycodes, self.types, self.compatibility, self.symbols, self.geometry
        )?;
        writeln!(f, "}};")
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
struct Keycodes {
    name: Name,
    minimum: Keycode,
    maximum: Keycode,
    keycodes: Vec<KeycodeEntry>,
    indicators: Vec<IndicatorEntry>,
    aliases: Vec<AliasEntry>,
}

impl Display for Keycodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "xkb_keycodes {} {{", self.name)?;
        writeln!(f, "    minimum = {};", self.minimum)?;
        writeln!(f, "    maximum = {};", self.maximum)?;
        for keycode in &self.keycodes {
            writeln!(f, "    {keycode}")?;
        }
        for indicators in &self.indicators {
            writeln!(f, "    {indicators}")?;
        }
        for alias in &self.aliases {
            writeln!(f, "    {alias}")?;
        }
        writeln!(f, "}};")
    }
}

impl Parse for Keycodes {
    fn parse(input: &str) -> IResult<&str, Self> {
        let (remaining, name) = parse_section(input, "xkb_keycodes").unwrap();
        let minimum_parser = delimited(pair(ws(tag("minimum")), ws(tag("="))), u32, tag(";"));
        let maximum_parser = delimited(pair(ws(tag("maximum")), ws(tag("="))), u32, tag(";"));
        let content_parser = permutation((
            minimum_parser,
            maximum_parser,
            many0(KeycodeEntry::parse),
            many0(IndicatorEntry::parse),
            many0(AliasEntry::parse),
        ));
        let (remaining, (minimum, maximum, keycodes, indicators, aliases)) =
            terminated(content_parser, ws(tag("};"))).parse(remaining)?;
        Ok((
            remaining,
            Keycodes {
                name,
                minimum,
                maximum,
                keycodes,
                indicators,
                aliases,
            },
        ))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
struct KeycodeEntry {
    identifier: Identifier,
    code: Keycode,
}

impl Display for KeycodeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Replace with
        // write!(f, "{:>11} = {};", self.identifier, self.code)
        // The above statement did not leftpad at all
        for _ in self.identifier.identifier.len()..4 {
            write!(f, " ")?;
        }
        write!(f, "{} = {};", self.identifier, self.code)
    }
}

impl Parse for KeycodeEntry {
    fn parse(input: &str) -> IResult<&str, Self> {
        let mapping_parser = separated_pair(Identifier::parse, ws(char('=')), u32);
        terminated(ws(mapping_parser), tag(";"))
            .parse(input)
            .map(|(r, (identifier, code))| (r, Self { identifier, code }))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
struct IndicatorEntry {
    idx: u32,
    name: Name,
}

impl Display for IndicatorEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "indicator {} = {};", self.idx, self.name)
    }
}

impl Parse for IndicatorEntry {
    fn parse(input: &str) -> IResult<&str, Self> {
        let mapping_parser = separated_pair(u32, ws(char('=')), Name::parse);
        delimited(ws(tag("indicator")), mapping_parser, tag(";"))
            .parse(input)
            .map(|(r, (idx, name))| (r, Self { idx, name }))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
struct AliasEntry {
    alias: Identifier,
    name: Identifier,
}

impl Display for AliasEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "alias {} = {};", self.alias, self.name)
    }
}

impl Parse for AliasEntry {
    fn parse(input: &str) -> IResult<&str, Self> {
        let mapping_parser = separated_pair(Identifier::parse, ws(char('=')), Identifier::parse);
        delimited(ws(tag("alias")), mapping_parser, tag(";"))
            .parse(input)
            .map(|(r, (a, n))| (r, Self { alias: a, name: n }))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
struct Name {
    name: String,
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.name)
    }
}

impl Parse for Name {
    fn parse(input: &str) -> IResult<&str, Self> {
        let (remaining, name) = delimited(char('"'), take_until("\""), char('"')).parse(input)?;
        let name = Self {
            name: name.to_string(),
        };
        Ok((remaining, name))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
struct Identifier {
    identifier: String,
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}>", self.identifier)
    }
}

impl Parse for Identifier {
    fn parse(input: &str) -> IResult<&str, Self> {
        let (remaining, id) = delimited(char('<'), take_until(">"), char('>')).parse(input)?;
        let id = Self {
            identifier: id.to_string(),
        };
        Ok((remaining, id))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
struct Symbols {
    name: Name,
    groups: Vec<String>,
    keys: Vec<String>,
    modifier_map: Vec<String>,
}

impl Display for Symbols {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "xkb_symbols {} {{", self.name)?;
        writeln!(f, "")?;
        for (idx, group) in self.groups.iter().enumerate() {
            writeln!(f, "    name[group{idx}]=\"{group}\";")?;
        }
        writeln!(f, "")?;
        for key in &self.keys {
            writeln!(f, "    key {key}")?;
        }
        for mod_map in &self.modifier_map {
            writeln!(f, "    modifier_map {mod_map}")?;
        }
        writeln!(f, "}};")
    }
}

fn parse_section<'a, 'b>(input: &'a str, struct_tag: &'b str) -> IResult<&'a str, Name> {
    delimited(ws(tag(struct_tag)), Name::parse, ws(tag("{"))).parse(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also
/// consumes both leading and trailing whitespace, returning the output of
/// `inner`.
pub fn ws<'a, O, E: ParseError<&'a str>, F>(inner: F) -> impl Parser<&'a str, Output = O, Error = E>
where
    F: Parser<&'a str, Output = O, Error = E>,
{
    delimited(multispace0, inner, multispace0)
}

fn read_xkb_file<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    fs::read_to_string(path)
}

fn main() {
    let test = "a";
    println!("{:>11}", test);
    println!("{}", test);

    let formatted_id = format!("{:>21}", test);
    print!("{} = 3;", formatted_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_section() {
        let parse_str = "xkb_keycodes \"(unnamed)\" {
    minimum = 8;";

        let correct_res = Ok((
            "minimum = 8;",
            Name {
                name: "(unnamed)".to_string(),
            },
        ));

        assert_eq!(parse_section(parse_str, "xkb_keycodes"), correct_res);
    }

    #[test]
    fn test_parse_keycodes() {
        let keycodes_str = "xkb_keycodes \"(unnamed)\" {
    minimum = 8;
    maximum = 255;
     <ESC> = 9;
        <> = 10;
      <UP> = 111;
    <VOL-> = 122;
    <VOL+> = 123;
     <CUT> = 145;
    <FK24> = 202;
    <LVL5> = 203;
     <ALT> = 204;
    <META> = 205;
    <I254> = 254;
    <I255> = 255;
    indicator 1 = \"Caps Lock\";
    indicator 2 = \"Num Lock\";
    indicator 13 = \"Group 2\";
    indicator 14 = \"Mouse Keys\";
    alias <AC12> = <BKSL>;
    alias <ALGR> = <RALT>;
    alias <MENU> = <COMP>;
    alias <HZTG> = <TLDE>;
};
";

        let correct_keycodes = vec![
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "ESC".to_string(),
                },
                code: 9,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "".to_string(),
                },
                code: 10,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "UP".to_string(),
                },
                code: 111,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "VOL-".to_string(),
                },
                code: 122,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "VOL+".to_string(),
                },
                code: 123,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "CUT".to_string(),
                },
                code: 145,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "FK24".to_string(),
                },
                code: 202,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "LVL5".to_string(),
                },
                code: 203,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "ALT".to_string(),
                },
                code: 204,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "META".to_string(),
                },
                code: 205,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "I254".to_string(),
                },
                code: 254,
            },
            KeycodeEntry {
                identifier: Identifier {
                    identifier: "I255".to_string(),
                },
                code: 255,
            },
        ];

        let correct_indicators = vec![
            IndicatorEntry {
                idx: 1,
                name: Name {
                    name: "Caps Lock".to_string(),
                },
            },
            IndicatorEntry {
                idx: 2,
                name: Name {
                    name: "Num Lock".to_string(),
                },
            },
            IndicatorEntry {
                idx: 13,
                name: Name {
                    name: "Group 2".to_string(),
                },
            },
            IndicatorEntry {
                idx: 14,
                name: Name {
                    name: "Mouse Keys".to_string(),
                },
            },
        ];

        let correct_aliases = vec![
            AliasEntry {
                alias: Identifier {
                    identifier: "AC12".to_string(),
                },
                name: Identifier {
                    identifier: "BKSL".to_string(),
                },
            },
            AliasEntry {
                alias: Identifier {
                    identifier: "ALGR".to_string(),
                },
                name: Identifier {
                    identifier: "RALT".to_string(),
                },
            },
            AliasEntry {
                alias: Identifier {
                    identifier: "MENU".to_string(),
                },
                name: Identifier {
                    identifier: "COMP".to_string(),
                },
            },
            AliasEntry {
                alias: Identifier {
                    identifier: "HZTG".to_string(),
                },
                name: Identifier {
                    identifier: "TLDE".to_string(),
                },
            },
        ];

        let correct_keycodes_struct = Keycodes {
            name: Name {
                name: "(unnamed)".to_string(),
            },
            minimum: 8,
            maximum: 255,
            keycodes: correct_keycodes,
            indicators: correct_indicators,
            aliases: correct_aliases,
        };

        println!("{correct_keycodes_struct}");

        assert_eq!(
            Keycodes::parse(keycodes_str),
            Ok(("", correct_keycodes_struct))
        );
    }

    #[test]
    fn test_parse_keycode_entry() {
        let test_cases = vec![
            (
                "<ESC> = 9;",
                Ok((
                    "",
                    KeycodeEntry {
                        identifier: Identifier {
                            identifier: "ESC".to_string(),
                        },
                        code: 9,
                    },
                )),
            ),
            (
                "<AE01> = 10;",
                Ok((
                    "",
                    KeycodeEntry {
                        identifier: Identifier {
                            identifier: "AE01".to_string(),
                        },
                        code: 10,
                    },
                )),
            ),
            (
                "<TAB> = 23;",
                Ok((
                    "",
                    KeycodeEntry {
                        identifier: Identifier {
                            identifier: "TAB".to_string(),
                        },
                        code: 23,
                    },
                )),
            ),
            (
                "<UP> = 111;",
                Ok((
                    "",
                    KeycodeEntry {
                        identifier: Identifier {
                            identifier: "UP".to_string(),
                        },
                        code: 111,
                    },
                )),
            ),
            (
                "<VOL-> = 122;",
                Ok((
                    "",
                    KeycodeEntry {
                        identifier: Identifier {
                            identifier: "VOL-".to_string(),
                        },
                        code: 122,
                    },
                )),
            ),
            (
                "<VOL+> = 123;",
                Ok((
                    "",
                    KeycodeEntry {
                        identifier: Identifier {
                            identifier: "VOL+".to_string(),
                        },
                        code: 123,
                    },
                )),
            ),
            (
                "<> = 0;",
                Ok((
                    "",
                    KeycodeEntry {
                        identifier: Identifier {
                            identifier: "".to_string(),
                        },
                        code: 0,
                    },
                )),
            ),
            (
                "<I255> = 255;",
                Ok((
                    "",
                    KeycodeEntry {
                        identifier: Identifier {
                            identifier: "I255".to_string(),
                        },
                        code: 255,
                    },
                )),
            ),
        ];
        for (parse_str, correct_res) in &test_cases {
            assert_eq!(KeycodeEntry::parse(parse_str), *correct_res);
        }
    }

    #[test]
    fn test_parse_indicator() {
        let test_cases = vec![
            (
                "indicator 1 = \"Caps Lock\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 1,
                        name: Name {
                            name: "Caps Lock".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 1 = \"Caps Lock\";
    indicator 2 = \"Num Lock\";
    indicator 3 = \"Scroll Lock\";",
                Ok((
                    "
    indicator 2 = \"Num Lock\";
    indicator 3 = \"Scroll Lock\";",
                    IndicatorEntry {
                        idx: 1,
                        name: Name {
                            name: "Caps Lock".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 2 = \"Num Lock\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 2,
                        name: Name {
                            name: "Num Lock".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 3 = \"Scroll Lock\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 3,
                        name: Name {
                            name: "Scroll Lock".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 4 = \"Compose\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 4,
                        name: Name {
                            name: "Compose".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 5 = \"Kana\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 5,
                        name: Name {
                            name: "Kana".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 6 = \"Sleep\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 6,
                        name: Name {
                            name: "Sleep".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 7 = \"Suspend\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 7,
                        name: Name {
                            name: "Suspend".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 8 = \"Mute\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 8,
                        name: Name {
                            name: "Mute".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 9 = \"Misc\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 9,
                        name: Name {
                            name: "Misc".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 10 = \"Mail\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 10,
                        name: Name {
                            name: "Mail".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 11 = \"Charging\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 11,
                        name: Name {
                            name: "Charging".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 12 = \"Shift Lock\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 12,
                        name: Name {
                            name: "Shift Lock".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 13 = \"Group 2\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 13,
                        name: Name {
                            name: "Group 2".to_string(),
                        },
                    },
                )),
            ),
            (
                "indicator 14 = \"Mouse Keys\";",
                Ok((
                    "",
                    IndicatorEntry {
                        idx: 14,
                        name: Name {
                            name: "Mouse Keys".to_string(),
                        },
                    },
                )),
            ),
        ];
        for (parse_str, correct_res) in &test_cases {
            assert_eq!(IndicatorEntry::parse(parse_str), *correct_res);
        }
    }

    #[test]
    fn test_parse_alias() {
        let test_cases = vec![
            (
                "alias <I141> = <COPY>;",
                Ok((
                    "",
                    AliasEntry {
                        alias: Identifier {
                            identifier: "I141".to_string(),
                        },
                        name: Identifier {
                            identifier: "COPY".to_string(),
                        },
                    },
                )),
            ),
            (
                "\n    alias <I141> = <COPY>;\n    ",
                Ok((
                    "\n    ",
                    AliasEntry {
                        alias: Identifier {
                            identifier: "I141".to_string(),
                        },
                        name: Identifier {
                            identifier: "COPY".to_string(),
                        },
                    },
                )),
            ),
            (
                "alias <I123> = <VOL+>;",
                Ok((
                    "",
                    AliasEntry {
                        alias: Identifier {
                            identifier: "I123".to_string(),
                        },
                        name: Identifier {
                            identifier: "VOL+".to_string(),
                        },
                    },
                )),
            ),
        ];
        for (parse_str, correct_res) in &test_cases {
            assert_eq!(AliasEntry::parse(parse_str), *correct_res);
        }
    }

    #[test]
    fn test_parse_identifier() {
        let test_cases = vec![
            (
                "<ESC>",
                Ok((
                    "",
                    Identifier {
                        identifier: "ESC".to_string(),
                    },
                )),
            ),
            (
                "<I255>",
                Ok((
                    "",
                    Identifier {
                        identifier: "I255".to_string(),
                    },
                )),
            ),
            (
                "<TAB>",
                Ok((
                    "",
                    Identifier {
                        identifier: "TAB".to_string(),
                    },
                )),
            ),
            (
                "<I255>",
                Ok((
                    "",
                    Identifier {
                        identifier: "I255".to_string(),
                    },
                )),
            ),
            (
                "<UP>",
                Ok((
                    "",
                    Identifier {
                        identifier: "UP".to_string(),
                    },
                )),
            ),
            (
                "<VOL->",
                Ok((
                    "",
                    Identifier {
                        identifier: "VOL-".to_string(),
                    },
                )),
            ),
            (
                "<VOL+>",
                Ok((
                    "",
                    Identifier {
                        identifier: "VOL+".to_string(),
                    },
                )),
            ),
            (
                "<I167>",
                Ok((
                    "",
                    Identifier {
                        identifier: "I167".to_string(),
                    },
                )),
            ),
            (
                "<>",
                Ok((
                    "",
                    Identifier {
                        identifier: "".to_string(),
                    },
                )),
            ),
            (
                "<LatM>",
                Ok((
                    "",
                    Identifier {
                        identifier: "LatM".to_string(),
                    },
                )),
            ),
        ];
        for (id_str, correct_id) in &test_cases {
            assert_eq!(Identifier::parse(id_str), *correct_id);
        }
    }

    #[test]
    fn test_parse_name() {
        let test_cases = vec![
            (
                "\"(unnamed)\"",
                Ok((
                    "",
                    Name {
                        name: "(unnamed)".to_string(),
                    },
                )),
            ),
            (
                "\"enigo\"",
                Ok((
                    "",
                    Name {
                        name: "enigo".to_string(),
                    },
                )),
            ),
        ];
        for (name_str, correct_name) in &test_cases {
            assert_eq!(Name::parse(name_str), *correct_name);
        }
    }
}
