# initvimparse
> This parses your `init.vim` file and outputs your keybindings.

I grew tired of having my `init.vim` file open to look up my keybindings especially for plugins that I don't use that regularly so I wrote a little tool to help me with that.

Currently it uses two different approaces:
1. It parses `{modifiers}map {lhs} {rhs}` and uses the comment in the lines above as documentation outputs
2. It parses embedded Lua code blocks fenced by `lua <<EOF` and `EOF` and maps the (simplified) AST into a JSON search query that is run against JSON doc files in the 'definitions' folder

Functionality is still limited (and might remain limited), 


## Meta

Daniel Amesberger – [@DAmesberger](https://twitter.com/DAmesberger) – daniel.amesberger@amescon.com

Distributed under the MIT license. See ``LICENSE`` for more information.

[https://github.com/DAmesberger/github-link](https://github.com/dbader/)

