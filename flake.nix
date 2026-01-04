{
  description = "A flake for csharp-language-server";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    flake-utils,
    naersk,
    nixpkgs,
    rust-overlay,
    ...
  }:
    let
      overlays = {
        default = final: prev: {
          csharp-language-server = (final.callPackage naersk { }).buildPackage {
            pname = "csharp-language-server";
            src = ./.;

            nativeBuildInputs = [ final.dotnetCorePackages.dotnet_8.sdk ];

            cargoTestOptions = x: x ++ [ 
              "--" "--skip=first_line_is_jsonrpc" 
            ];
          };
        };
      };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) overlays.default ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [ pkgs.csharp-language-server ];
        };

        packages = {
          default = pkgs.csharp-language-server;
          
          csharp-language-server = throw ''
            packages.csharp-language-server has been renamed to packages.default.
          '';
        };
      }
    ) // {
      inherit overlays;
    };
}