# roslyn-language-server
A wrapper around the language server behind the C# Visual Studio Code extension, `Microsoft.CodeAnalysis.LanguageServer`, which makes it compatible with other editors, e.g., Helix.
This is more stable and faster than OmniSharp.

This tool works around the quirks of `Microsoft.CodeAnalysis.LanguageServer` in the following way: 
- Downloads `Microsoft.CodeAnalysis.LanguageServer`
- Launches `Microsoft.CodeAnalysis.LanguageServer` as a process
- Passes the provided `unix socket` or named pipe and forwards all communication to `stdio` 
- Waits for an `initialize` notification from the client, and finds relevant `.sln` or `.csproj` files and sends them to the server as a custom `open` notification.

## Installation
### Binaries
Download the binaries that match your platform under Releases

### Nix
If you use `nix`, you can use this repository's `nix flake`. 

### Others
Alternatively, install with `cargo`: `cargo install --git https://github.com/SofusA/roslyn-language-server` 

## Usage

### Helix
Since `Microsoft.CodeAnalysis.LanguageServer` only supports `pull diagnostics` and Helix does not [yet](https://github.com/helix-editor/helix/pull/11315), you will need to use my branch: `github:sofusa/helix-pull-diagnostics`.

```toml
[language-server.roslyn]
command = "roslyn-language-server"

[[language]]
name = "c-sharp"
language-servers = ["roslyn"]
```

### Neovim
```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'cs',
  callback = function(args)
    local root_dir = vim.fs.dirname(
      vim.fs.find({ '.sln', '.csproj', '.git' }, { upward = true })[1]
    )
    vim.lsp.start({
      name = 'roslyn-language-server',
      cmd = {'roslyn-language-server'},
      root_dir = root_dir,
    })
  end,
})
``` 
