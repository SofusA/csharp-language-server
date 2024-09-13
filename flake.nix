{
  description = "A flake for roslyn-language-server";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        roslyn-language-server = pkgs.rustPlatform.buildRustPackage {
          checkFlags = [
            # Test is unable to persist files while testing in nix
            "--skip=first_line_is_jsonrpc"
          ];

          pname = "roslyn-language-server";
          version = "0.2.2";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkgs.dotnetCorePackages.dotnet_8.sdk ];

        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = [ roslyn-language-server ];
        };

        packages.roslyn-language-server = roslyn-language-server;
      }
    );
}
