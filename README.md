# initvim2doc
> A little tool to parse your Neovim `init.vim` file and outputs your keybindings with a description.

I grew tired of having my `init.vim` file open to look up my keybindings especially for plugins that I don't use that regularly so I wrote a little tool to help me with that.

Currently it uses two different approaces:
1. It parses `{modifiers}map {lhs} {rhs}` and uses the comment in the lines above as documentation outputs
2. It parses embedded Lua code blocks fenced by `lua <<EOF` and `EOF` and maps the (simplified) AST into a JSON search query that is run against JSON doc files in the 'definitions' folder

Functionality is still limited (and might remain limited), it is currently only targeted at Neovim users

## Installation

If you don't have the Rust toolchain installed, install that first:
``` sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then clone the repository, install the binary and copy or symlink the definitions to your Neovim config folder
``` sh
git clone https://github.com/DAmesberger/initvim2doc.git
cd initvim2doc
cargo install --path .
ln -s $PWD/definitions ~/.config/nvim/definitions
```

## Usage

Just run the executable

``` sh
initvim2doc
```

It should output your configured keybindings in text format like this:
``` sh
af             (nvim-treesitter) Selection of outer function
if             (nvim-treesitter) Selection of inner function
ac             (nvim-treesitter) Selection of inner class
ab             (nvim-treesitter) Selection of inner block
]m             (nvim-treesitter) Move to start of outer function
]]             (nvim-treesitter) Move to start of outer class
]M             (nvim-treesitter) Move to end of outer function
][             (nvim-treesitter) Move to end of outer class
[m             (nvim-treesitter) Move to start of previous outer function
[[             (nvim-treesitter) Move to start of previous outer class
[M             (nvim-treesitter) Move to end of previous outer function
[]             (nvim-treesitter) Move to end of previous outer class
<leader>df     (nvim-treesitter) Peek the definition of outer function
<leader>dF     (nvim-treesitter) Peek the definition of outer class
gD             LSP goto declaration
gd             LSP goto definition
K              LSP hover type info
gi             LSP hover type info
<leader>lu     Toggle Diagnostics
<C-p>          Open hotkeys
<leader>w      Quick-save
U              undotree
...
```
## Meta

Daniel Amesberger – [@DAmesberger](https://twitter.com/DAmesberger) – daniel.amesberger@amescon.com

Distributed under the MIT license.

[https://github.com/DAmesberger/](https://github.com/DAmesberger/)

