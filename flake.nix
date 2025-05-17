{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    # rust support
    fenix.url = "github:nix-community/fenix";

    # devenv support
    devenv.url = "github:cachix/devenv";
    nix2container.url = "github:nlewo/nix2container";
    nix2container.inputs = { nixpkgs.follows = "nixpkgs"; };

    # flake-parts + nixDir
    nixDir.url = "github:roman/nixDir/v3";
    flake-parts.url = "github:hercules-ci/flake-parts";

    #
    systems.url = "github:nix-systems/default";
    systems.flake = false;
  };

  outputs = { flake-parts, ... } @ inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      imports = [
        inputs.nixDir.flakeModule
        inputs.devenv.flakeModule
      ];
      nixDir = {
        enable = true;
        root = ./.;
      };
      perSystem = {pkgs, ...}: {
      };
    };
}
