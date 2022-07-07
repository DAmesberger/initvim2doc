# initvim2doc
> A little tool to parse your Neovim `init.vim` file and outputs your keybindings with a description.

I grew tired of having my `init.vim` file open to look up my keybindings especially for plugins that I don't use that regularly so I wrote a little tool to help me with that.

Currently it uses two different approaces:
1. It parses `{modifiers}map {lhs} {rhs}` and uses the comment in the lines above as documentation outputs
2. It parses embedded Lua code blocks fenced by `lua <<EOF` and `EOF` and maps the (simplified) AST into a JSON search query that is run against JSON doc files in the 'definitions' folder

Functionality is still limited (and might remain limited), 

## Installation


Clone the repository:
``` sh
git clone https://github.com/DAmesberger/initvim2doc.git
```

If you don't have the Rust toolchain installed, install that first:
``` sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install the binary
``` sh
cargo install --path .
```

Copy or symlink the definitions file to your nvim config folder
``` sh
ln -s ./definitions ~/.config/nvim/definitions
```

## Usage

Just run the executable

``` sh
initvim2doc
```


## Meta

Daniel Amesberger – [@DAmesberger](https://twitter.com/DAmesberger) – daniel.amesberger@amescon.com

Distributed under the MIT license. See ``LICENSE`` for more information.

[https://github.com/DAmesberger/](https://github.com/DAmesberger/)

