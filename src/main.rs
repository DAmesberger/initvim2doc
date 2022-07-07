use color_eyre::Result;
use color_eyre::eyre::Context;
use serde_derive::Deserialize;
use std::io::BufReader;
use std::{collections::HashMap, fs::File};

mod parser;

#[derive(clap::Parser)]
struct App {
    #[clap(short, long, default_value = "~/.config/nvim/init.vim")]
    initvim: String,
    #[clap(short, long, default_value = "~/.config/nvim/definitions")]
    definitions: String,
    #[clap(short)]
    show_missing_docs: bool,
}

use access_json::JSONQuery;
use serde_json::{self, Value};

#[derive(Debug, Deserialize)]
pub struct Keybinding {
    root: String,
    keymap: String,
    command: String,
    doc: Option<KeybindingDoc>,
}

#[derive(Debug, Deserialize)]
pub struct KeybindingDoc {
    description: String,
    examples: Option<Vec<String>>,
}

fn main() -> Result<()> {
    let app: App = clap::Parser::parse();

    let filename = shellexpand::tilde(&app.initvim);
    let f = File::open(&*filename)?;
    let reader = BufReader::new(f);

    //parsing init.vim
    let mut keymaps = parser::parse(reader)?;

    // read definition file paths
    let mut definitions: HashMap<String, HashEntry> = HashMap::new();

    let definitions_dir = shellexpand::tilde(&app.definitions);
    for de in std::fs::read_dir(&*definitions_dir)
        .wrap_err_with(|| format!("cannot get files from defitions folder {}", definitions_dir))? {
        match de {
            Ok(dir) => {
                if let Some(dir) = dir.path().to_str() {
                    if let Some(file) = std::path::Path::new(dir).file_stem() {
                        if let Some(file) = file.to_str() {
                            definitions.insert(file.to_owned(), HashEntry::Path(dir.to_owned()));
                        }
                    }
                }
            }
            Err(_) => todo!(),
        }
    }

    map_keymaps_to_doc(&mut keymaps, definitions, app.show_missing_docs)?;

    for keymap in keymaps {
        if let Some(doc) = keymap.doc {
            if keymap.root.is_empty() {
                println!("{: <15}{}", keymap.keymap, doc.description);
            } else {
                println!("{: <15}({}) {}", keymap.keymap, keymap.root, doc.description);
            }
        }
    }

    Ok(())
}

enum HashEntry {
    Path(String),
    Value(Value),
    Unresolvable,
}

fn map_keymaps_to_doc(
    keymaps: &mut Vec<Keybinding>,
    mut definitions: HashMap<String, HashEntry>,
    show_missing_docs: bool,
) -> Result<()> {

    fn lookup_keymap(keybind: &mut Keybinding, doc: &Value, show_missing_docs: bool) -> Result<()> {
        if keybind.doc.is_none() && keybind.command.starts_with('.') {
            if let Ok(Some(output)) = JSONQuery::parse(&keybind.command)?.execute(doc) {
                keybind.doc = Some(serde_json::from_value::<KeybindingDoc>(output)?);
            } else if show_missing_docs {
                println!("cannot find doc for {} in ({})", keybind.command, keybind.root);
            }
        }
        Ok(())
    }

    for keymap in keymaps {
        let entry = definitions.get(&keymap.root);
        if let Some(entry) = entry {
            match entry {
                HashEntry::Path(path) => {
                    //promote to Value
                    match std::fs::read_to_string(path) {
                        Ok(content) => {
                            match serde_json::from_str(&content) {
                                Ok(value) => {
                                    lookup_keymap(keymap, &value, show_missing_docs)?;
                                    *(definitions.get_mut(&keymap.root).unwrap()) =
                                        HashEntry::Value(value);
                                }
                                Err(e) => {
                                    *(definitions.get_mut(&keymap.root).unwrap()) =
                                        HashEntry::Unresolvable;

                                    //could not deserialize
                                    //TODO use an Error type and bubble up
                                    eprintln!("Deserialize {:#?}", e);
                                }
                            };
                        }
                        Err(e) => {
                            *(definitions.get_mut(&keymap.root).unwrap()) = HashEntry::Unresolvable;
                            //cannot read into string
                            //TODO use an Error type and bubble up
                            eprintln!("Read {:#?}", e);
                        }
                    };
                }
                HashEntry::Value(value) => {
                    lookup_keymap(keymap, value, show_missing_docs)?;
                }
                HashEntry::Unresolvable => {
                    //we cannot do anything here, matching definition was not found
                }
            }
        };

        //if let Ok(Some(output)) = JSONQuery::parse(&bindings.command)?.execute(&definitions) {
        //    let k = serde_json::from_value::<String>(output)?;
        //    println!("{}: {}", bindings.keymap, k);
        //}
    }
    Ok(())
}
