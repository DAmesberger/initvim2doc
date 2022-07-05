use analisar::aware::ast::{Args, Field, FunctionCall, LiteralString, Name, Table};
use analisar::aware::{
    ast::{Expression, Statement, Suffixed},
    Parser,
};
use color_eyre::Result;
use regex::Regex;
use std::cell::Cell;
use std::fs::File;
use std::io::{prelude::*, BufReader};

#[derive(clap::Parser)]
struct App {
    #[clap(short, long, default_value = "~/.config/nvim/init.vim")]
    initvim: String,
}

#[derive(PartialEq)]
enum ParseMode {
    Unknown,
    Comment,
    Map,
    Lua,
}

fn main() -> Result<()> {
    let app: App = clap::Parser::parse();

    let filename = shellexpand::tilde(&app.initvim);

    println!("parsing {}", &filename);

    let f = File::open(&*filename)?;

    let reader = BufReader::new(f);

    let map_regex = Regex::new(
        r"^(?P<prefix>.*?)map\s(?P<silent><silent>\s)?(?P<shortcut>.*?)\s+(?P<command>.*)$",
    )?;

    let mut comment_lines = vec![];
    let mut lua_code = String::new();
    let mut parse_mode = ParseMode::Unknown;

    for (linenumber, line) in reader.lines().enumerate().map(|(i, l)| (i + 1, l.unwrap())) {
        let trim_line = line.trim();

        if trim_line == "EOF" {
            //TODO: fail gracefully
            assert!(
                parse_mode == ParseMode::Lua,
                "Error in line {}, EOF without matching 'lua <<EOF' before",
                linenumber
            );
            parse_lua(&lua_code);
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
            continue; //ignore empty line
        } else if let Some(trim_line) = trim_line.strip_prefix('\"') {
            comment_lines.push(trim_line.to_owned());
            parse_mode = ParseMode::Comment;
            continue;
        } else if let Some(capture) = map_regex.captures(trim_line) {
            let shortcut = capture.name("shortcut");
            let command = capture.name("command");

            if let (Some(shortcut), Some(command)) = (shortcut, command) {
                if parse_mode == ParseMode::Comment {
                    //println!("{}:\n\t{}", shortcut.as_str(), comment_lines.join("\n"));
                }
            }
            parse_mode = ParseMode::Map;
            comment_lines.clear(); //remove comments
        } else if line.starts_with("lua") {
            if line.split_whitespace().collect::<String>() == "lua<<EOF" {
                parse_mode = ParseMode::Lua;
                lua_code.clear();
            } else {
                assert!(
                    false,
                    "Error in line {}, line starting with lua but is not 'lua <<EOF'",
                    linenumber
                );
            }
        } else {
            parse_mode = ParseMode::Unknown;
            comment_lines.clear(); //remove comments
        }
    }

    Ok(())
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

fn args_to_table<'a> (args: &'a Args) -> Option<&'a Table<'a>> {
    match &args {
        Args::Table(t) => Some(t),
        Args::ExpList {
            open_paren: _,
            exprs,
            close_paren: _,
        } if exprs.len() == 1 => {
            if let analisar::aware::ast::ExpListItem::Expr(Expression::TableCtor(t)) = &exprs[0]
            {
                return Some(t);
            } 
            None
        }
        _ => None,
    }
}

fn parse_lua(lua_code: &str) -> Option<Vec<String>> {
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

fn create_map(name: &str, value: &str, f: &FunctionCall) -> Vec<String> {
    let mut mappings = vec![];

    let name = name.to_string();
    let value = value[1..value.len() - 1].to_string();
    if let Some(table) = args_to_table(&f.args) {
        if let ("require", root) = (name.as_str(), value.as_str()) {
            let results = walk_fields(&table.field_list);
            for result in results {
                mappings.push(format!("{}.{}", root, result.into_inner()));
            }

            println!("{:#?}", &mappings);
        };
    };

    mappings
}

fn walk_fields(field_list: &Vec<Field>) -> Vec<Cell<String>> {
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
                    Expression::Name(name) => Some(format!("{}.", name.name)),
                    _ => None,
                };

                if let Some((lh, rh)) = match (name, value) {
                    (Expression::LiteralString(lh), Expression::LiteralString(rh)) => {
                        Some((lh.value.to_string(), rh.value.to_string()))
                    }
                    (_, Expression::TableCtor(t)) => {
                        let child_paths = walk_fields(&t.field_list);
                        if let Some(pathname) = &pathname {
                            for mut child in child_paths {
                                child.get_mut().insert_str(0, pathname.as_str());
                                paths.push(child);
                            }
                        }
                        None
                    }
                    (_lh, _rh) => {
                        None
                    }
                } {
                    let leaf = Cell::new(format!("{}:{}", rh, lh));
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
