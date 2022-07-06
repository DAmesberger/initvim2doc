use analisar::aware::ast::{Args, Field, FunctionCall, LiteralString, Name, Table};
use analisar::aware::{
    ast::{Expression, Statement, Suffixed},
    Parser,
};
use thiserror::Error;
use regex::Regex;
use std::io::{prelude::*, BufReader};

use crate::{ Keybinding, KeybindingDoc };

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Error in line {0}, line starting with lua but is not 'lua <<EOF'")]
    InvalidLuaBlock(usize),
    
    #[error("Error in line {0}, EOF without matching 'lua <<EOF' before")]
    UnmatchedEOF(usize)
}

#[derive(PartialEq)]
enum ParseMode {
    Unknown,
    Comment,
    Map,
    Lua,
}

pub fn parse<R>(reader: BufReader<R>) -> Result<Vec<Keybinding>, ParseError>
where
    R: Read,
{
    let map_regex = Regex::new(
        r"^(?P<mode>[nvsxomilct])?(?P<nonrecursive>nore)?map\s(?P<silent><silent>\s)?(?P<shortcut>.*?)\s+(?P<command>.*)$",
    ).unwrap(); //this should never fail

    let mut keymaps: Vec<Keybinding> = vec![];

    let mut comment_lines = vec![];
    let mut lua_code = String::new();
    let mut parse_mode = ParseMode::Unknown;

    for (linenumber, line) in reader.lines().enumerate().map(|(i, l)| (i + 1, l.unwrap())) {
        let trim_line = line.trim();

        if trim_line == "EOF" {
            if parse_mode != ParseMode::Lua {
                return Err(ParseError::UnmatchedEOF(linenumber));
            }

            if let Some(ref mut values) = parse_lua(&lua_code) {
                keymaps.append(values);
            };
            parse_mode = ParseMode::Unknown;
        }

        // add to Lua Code block
        if parse_mode == ParseMode::Lua {
            lua_code.push_str(&line);
            lua_code.push('\n');
            continue;
        }

        // not lua code, analyze vim config
        if trim_line.is_empty() {
            comment_lines.clear(); //remove comments
            continue; //ignore empty line
        } else if let Some(trim_line) = trim_line.strip_prefix('\"') {
            comment_lines.push(trim_line.to_owned());
            parse_mode = ParseMode::Comment;
            continue;
        } else if let Some(capture) = map_regex.captures(trim_line) {
            let shortcut = capture.name("shortcut");
            let command = capture.name("command");

            if let (Some(shortcut), Some(command)) = (shortcut, command) {
                let doc = if parse_mode == ParseMode::Comment {
                    Some(KeybindingDoc { description: comment_lines.join(" "), examples: None })
                } else { None };

                keymaps.push(Keybinding {
                    root: String::new(),
                    keymap: shortcut.as_str().to_owned(),
                    command: command.as_str().to_owned(),
                    doc 
                });
            }
            parse_mode = ParseMode::Map;
            comment_lines.clear(); //remove comments
        } else if line.starts_with("lua") {
            if line.split_whitespace().collect::<String>() == "lua<<EOF" {
                parse_mode = ParseMode::Lua;
                lua_code.clear();
            } else {
                return Err(ParseError::InvalidLuaBlock(linenumber));
            }
        } else {
            parse_mode = ParseMode::Unknown;
            comment_lines.clear(); //remove comments
        }
    }
    Ok(keymaps)
}

/// turns the first Arg into an owned String if possible.
/// This only handles two cases:
/// 1. the Arg is an Args::String(LiteralString))
/// 2. the Arg is an ExpListItem
///    - in that case the first argument if the expression list is returned as String
fn args_to_lit_string(args: &Args) -> Option<String> {
    match &args {
        Args::String(LiteralString { span: _, value }) => Some(value.to_string()),
        Args::ExpList {
            open_paren: _,
            exprs,
            close_paren: _,
        } if exprs.len() == 1 => {
            if let analisar::aware::ast::ExpListItem::Expr(Expression::LiteralString(
                LiteralString { span: _, value },
            )) = &exprs[0]
            {
                Some(value.to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn args_to_table<'a>(args: &'a Args) -> Option<&'a Table<'a>> {
    match &args {
        Args::Table(t) => Some(t),
        Args::ExpList {
            open_paren: _,
            exprs,
            close_paren: _,
        } if exprs.len() == 1 => {
            if let analisar::aware::ast::ExpListItem::Expr(Expression::TableCtor(t)) = &exprs[0] {
                return Some(t);
            }
            None
        }
        _ => None,
    }
}

fn parse_lua(lua_code: &str) -> Option<Vec<Keybinding>> {
    if let Some(Ok(swc)) = Parser::new(lua_code.as_bytes()).next() {
        if let Statement::Expression(Expression::FuncCall(f)) = swc.statement {
            if let Expression::Suffixed(s) = &*f.prefix {
                let Suffixed {
                    subject,
                    property: _,
                } = &**s;
                if let Expression::FuncCall(FunctionCall { prefix, args }) = subject {
                    match &**prefix {
                        Expression::Name(Name {
                            name_span: _,
                            name,
                            attr: _,
                        }) => {
                            if let Some(value) = args_to_lit_string(args) {
                                return Some(create_map(name, &value, &f));
                            }
                        }
                        _ => {
                            eprintln!("ignored root: {:#?}", args);
                        }
                    };
                };
            }
        }
    }
    None
}

fn create_map(name: &str, value: &str, f: &FunctionCall) -> Vec<Keybinding> {
    let mut mappings = vec![];

    let name = name.to_owned();
    let value = value[1..value.len() - 1].to_owned();
    if let Some(table) = args_to_table(&f.args) {
        if let ("require", root) = (name.as_str(), value.as_str()) {
            let (root, prefix) = root.split_once('.')
                .map_or((root, String::from(".")), |(root, prefix)| (root, format!(".{}.", prefix)));
            mappings.append(&mut walk_fields(root, &prefix, &table.field_list));
        };
    };

    mappings
}

fn walk_fields(root: &str, prefix: &str, field_list: &Vec<Field>) -> Vec<Keybinding> {
    let mut paths = vec![];
    for field in field_list {
        match field {
            Field::Record {
                name,
                eq: _,
                value,
                sep: _,
            } => {
                //generate pathname
                let pathname = match &name {
                    Expression::Name(name) => Some(format!("{}{}.", prefix, name.name)),
                    _ => None,
                };

                if let Some((lh, rh)) = match (name, value) {
                    (Expression::LiteralString(lh), Expression::LiteralString(rh)) => {
                        Some((lh.value.to_string(), rh.value.to_string()))
                    }
                    (_, Expression::TableCtor(t)) => {
                        let child_paths = walk_fields(root, "", &t.field_list);
                        if let Some(pathname) = &pathname {
                            for mut child in child_paths {
                                child.command.insert_str(0, pathname.as_str());
                                paths.push(child);
                            }
                        }
                        None
                    }
                    (_lh, _rh) => None,
                } {
                    //remove quotes
                    //TODO: do this nicer and in a safe way
                    let rh = &rh.as_str()[1..rh.len() - 1];
                    let lh = &lh.as_str()[1..lh.len() - 1];
                    let leaf = Keybinding {
                        root: root.to_owned(),
                        keymap: lh.to_string(),
                        command: rh.replace('.', "_"),
                        doc: None
                    };
                    paths.push(leaf);
                }
            }
            Field::List { value: _, sep: _ } => {
                //println!("list: {:#?}", value);
            }
        }
    }

    paths
}
