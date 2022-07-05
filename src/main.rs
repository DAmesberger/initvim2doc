use analisar::aware::ast::{Args, FunctionCall, LiteralString, Name, Table, Field};
use analisar::aware::{
    ast::{Expression, Statement, Suffixed},
    Parser,
};
use color_eyre::Result;
use regex::Regex;
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

fn parse_lua(lua_code: &str) {
    if let Some(Ok(swc)) = Parser::new(lua_code.as_bytes()).next() {
        if let Statement::Expression(Expression::FuncCall(f)) = swc.statement {
            if let Expression::Suffixed(s) = *f.prefix {
                let Suffixed {
                    subject,
                    property: _,
                } = *s;
                //println!("subject {:#?}, property {:#?}", subject, property);
                if let Expression::FuncCall(FunctionCall { prefix, args }) = subject {
                    if let (
                        Expression::Name(Name {
                            name_span,
                            name,
                            attr,
                        }),
                        Args::String(LiteralString { span, value }),
                    ) = (*prefix, args)
                    {
                        let name = name.to_string();
                        let value = value[1..value.len() - 1].to_string();
                        if let Args::Table(table) = f.args {
                            match (name.as_str(), value.as_str()) {
                                ("require", "nvim-treesitter.configs") => {
                                    let mut path_str = String::new();
                                    walk_fields(table.field_list, &mut path_str);
                                }
                                _ => {}
                            };
                        };
                    }
                }
            }
        }
    }
}

fn walk_fields(field_list: Vec<Field>, path_str: &mut String) {
    for field in field_list {
        match field {
            Field::Record { name, eq, value, sep } => {
                //println!("Record: {:#?}", name);
                expr(name, value, path_str);
            },
            Field::List { value, sep } => {
                println!("list: {:#?}", value);
            },
        }
    }
}

fn expr(lh: Expression, rh: Expression, path_str: &mut String) {
    match (lh, rh) {
        (Expression::LiteralString(lh), Expression::LiteralString(s)) => { 
            println!("{}: {}", lh.value, s.value);
        },
        (lh, Expression::TableCtor(t)) => { 
            //path_str.push_str(lh.value);
            println!("iterate {:#?}", lh);
            walk_fields(t.field_list, path_str); },
        (_, _) => ()
    }

}
