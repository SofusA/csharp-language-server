{
  description = "A flake for csharp-language-server";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    nixpkgs,
    flake-utils,
    ...
  }:
    let
      overlay = _: super: {
        csharp-language-server = super.rustPlatform.buildRustPackage {
          checkFlags = [
            # Test is unable to persist files while testing in nix
            "--skip=first_line_is_jsonrpc"
          ];

          pname = "csharp-language-server";
          version = "0.6.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [ super.dotnetCorePackages.dotnet_8.sdk ];
        };
      };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ overlay ];
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [ csharp-language-server ];
        };

        packages = with pkgs; {
          default = csharp-language-server;
          inherit csharp-language-server;
        };
      }
    ) // {
      overlays.default = overlay;
    };
}
