# csharp-language-server
A wrapper around the language server behind the C# Visual Studio Code extension, `Microsoft.CodeAnalysis.LanguageServer`, which makes it compatible with other editors, e.g., Helix or Neovim.
This language server is more stable and faster than OmniSharp.

This tool assists the use of Microsoft.CodeAnalysis.LanguageServer:
- Downloads `Microsoft.CodeAnalysis.LanguageServer`
- Launches `Microsoft.CodeAnalysis.LanguageServer` as a process
- Waits for an `initialize` notification from the client, and finds relevant `.sln`, `.slnx` or `.csproj` files and sends them to the server as a custom `open` notification.

## Quirks
`Microsoft.CodeAnalysis.LanguageServer` is not intended to function as a standalone language server; it is designed to work together with an editor extension. This project is not an extension, it is only a tool to download and run `Microsoft.CodeAnalysis.LanguageServer`.
As a result, there are a few quirks you should be aware of. These can all be resolved through editor extension code, but not here, as doing so would break communication between the server and client.

- Projects are not automatically restored
  - Microsoft.CodeAnalysis.LanguageServer sends a custom LSP notification indicating that the project needs to be restored. If hover or diagnostics for external libraries do not work, this is likely the cause.

- Diagnostics are pulled before the project is fully loaded
  - The first document opened will only show diagnostics that do not require a loaded project (e.g., missing ;). All subsequent diagnostic pulls will be correct. You may need to save the document or open another one to refresh diagnostics.

## Installation
### Binaries
Download the binaries that match your platform under Releases

### Cargo
```cargo install csharp-language-server```

## First launch
The tool will download `Microsoft.CodeAnalysis.LanguageServer` at the first launch. It may take some seconds. To avoid this, you can run `csharp-language-server --download` before your first launch. This is useful for install scripts.

## Usage

### Helix
Helix requires the latest version from git, to support diagnostics.
Use `sofusa/helix` fork to support multiple projects in same git repository. Until helix-editor/helix#15081 is merged.

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
This is now the default language server for `zed 0.218` and later.
No need to configure anything. 
