use color_eyre::Result;
use std::fs::File;
use std::io::BufReader;

mod parser;

#[derive(clap::Parser)]
struct App {
    #[clap(short, long, default_value = "~/.config/nvim/init.vim")]
    initvim: String,
}

use access_json::JSONQuery;
use serde_json::{self, Value};


fn main() -> Result<()> {
    let app: App = clap::Parser::parse();

    let filename = shellexpand::tilde(&app.initvim);

    //println!("parsing {}", &filename);

    let f = File::open(&*filename)?;

    let reader = BufReader::new(f);

    let keymaps = parser::parse(reader)?;

    let definitions: Value = serde_json::from_str(r#"{
       "nvim-treesitter": {
          "configs" : {
              "textobjects": {
                "select": {
                    "keymaps": {
                        "@function_outer" : {
                            "name": "Outer Function",
                            "description": "Selection of outer function"
                        }
                    }
                }

              }
          }
       }
    }"#).unwrap();


    for (k, v) in keymaps.iter() {
        if let Ok(Some(output)) = JSONQuery::parse(k)?.execute(&definitions) {
            let k = serde_json::from_value::<String>(output)?;
            println!("{}: {}", v, k);
        }
    }

    Ok(())
}


