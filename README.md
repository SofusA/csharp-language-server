# csharp-language-server
A wrapper around the language server behind the C# Visual Studio Code extension, `Microsoft.CodeAnalysis.LanguageServer`, which makes it compatible with other editors, e.g., Helix or Neovim.
This language server is more stable and faster than OmniSharp.

This tool assists the use of Microsoft.CodeAnalysis.LanguageServer:
- Downloads `Microsoft.CodeAnalysis.LanguageServer`
- Launches `Microsoft.CodeAnalysis.LanguageServer` as a process
- Waits for an `initialize` notification from the client, and finds relevant `.sln`, `.slnx` or `.csproj` files and sends them to the server as a custom `open` notification.

## Installation
### Binaries
Download the binaries that match your platform under Releases

### Others
Alternatively, install with `cargo`: `cargo install --git https://github.com/SofusA/csharp-language-server` 

## First launch
The tool will download `Microsoft.CodeAnalysis.LanguageServer` at the first launch. It may take some seconds. To avoid this, you can run `csharp-language-server --download` before your first launch. This is useful for install scripts.

## Usage

### Helix
```toml
[language-server.csharp]
command = "csharp-language-server"

[[language]]
name = "c-sharp"
language-servers = ["csharp"]
```

### Neovim
```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'cs',
  callback = function(args)
    local root_dir = vim.fs.dirname(
      vim.fs.find({ '.sln', '.slnx', '.csproj', '.git' }, { upward = true })[1]
    )
    vim.lsp.start({
      name = 'csharp-language-server',
      cmd = {'csharp-language-server'},
      root_dir = root_dir,
    })
  end,
})
``` 

### Zed
**IMPORTANT** There is currently a known bug in Zed's implementation of pull diagnostics.
Latest working version is v0.199.6.

Override your `omnisharp`-config by setting this in `settings`:
```json
"lsp": {
  "omnisharp": {
    "binary": {
      "path": "csharp-language-server"
    }
  }
}
```
