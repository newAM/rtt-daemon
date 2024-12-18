{
  description = "A daemon to retrieve RTT logs using probe-rs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
  }:
    nixpkgs.lib.recursiveUpdate
    (flake-utils.lib.eachSystem [
        "aarch64-linux"
        "x86_64-linux"
      ] (
        system: let
          pkgs = nixpkgs.legacyPackages.${system};
          cargoToml = nixpkgs.lib.importTOML ./Cargo.toml;
          craneLib = crane.mkLib pkgs;

          src = craneLib.cleanCargoSource ./.;
          nativeBuildInputs = with pkgs; [pkg-config];
          buildInputs = with pkgs; [libusb1 udev];

          cargoArtifacts = craneLib.buildDepsOnly {
            inherit src nativeBuildInputs buildInputs;
            strictDeps = true;
          };
        in {
          packages.default = craneLib.buildPackage {
            inherit src nativeBuildInputs buildInputs cargoArtifacts;
            strictDeps = true;
          };

          checks = let
            nixSrc = nixpkgs.lib.sources.sourceFilesBySuffices ./. [".nix"];
          in {
            pkg = self.packages.${system}.default;

            clippy = craneLib.cargoClippy {
              inherit src nativeBuildInputs buildInputs cargoArtifacts;
              strictDeps = true;
              cargoClippyExtraArgs = "-- --deny warnings";
            };

            rustfmt = craneLib.cargoFmt {inherit src;};

            alejandra = pkgs.runCommand "alejandra" {} ''
              ${pkgs.alejandra}/bin/alejandra --check ${nixSrc}
              touch $out
            '';
          };
        }
      ))
    {
      overlays.default = final: prev: {
        rtt-daemon = self.packages.${prev.system}.default;
      };
      nixosModules.default = import ./module.nix;
    };
}
