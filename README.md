# roslyn-language-server
A wrapper around the language server behind the C# Visual Studio Code extension, `Microsoft.CodeAnalysis.LanguageServer`, which makes it compatible with other editors, e.g., Helix.
This is more stable and faster than OmniSharp.

This tool works around the quirks of `Microsoft.CodeAnalysis.LanguageServer` in the following way: 
- Installs `Microsoft.CodeAnalysis.LanguageServer` in `~/.roslyn`
- Launches `Microsoft.CodeAnalysis.LanguageServer` as a process
- Passes the provided `unix socket` or named pipe and forwards all communication to `stdio` 
- Waits for `capabilities` notification from the server, and forces `pull diagnostics` to be available. This forces the server respect clients who do not support dynamic registration of diagnostic capabilities.
- Waits for an `initialize` notification from the client, and finds relevant `.sln` or `.csproj` files and sends them to the server as a custom `open` notification.

## Installation
### Binaries
Download the binaries that matches your platform under Releases

### Nix
If you use `nix`, you can use this repository's `nix flake`. 

### Others
Alternatively, install with `cargo`: `cargo install --git https://github.com/SofusA/roslyn-language-server` 

## Usage

### Use with Helix
Since `Microsoft.CodeAnalysis.LanguageServer` only supports `pull diagnostics` and Helix does not (yet), you would need to use my branch at `github:sofusa/helix-pull-diagnostics`.

```toml
[language-server.roslyn]
command = "roslyn-language-server"

[[language]]
name = "c-sharp"
language-servers = ["roslyn"]
```
