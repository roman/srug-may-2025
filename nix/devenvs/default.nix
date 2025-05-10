{pkgs, inputs, ...}:
let
  rustToolchain = inputs.fenix.packages.${pkgs.system}.fromToolchainFile {
    file = "${inputs.self}/rust-toolchain.toml";
    sha256 = "sha256-X/4ZBHO3iW0fOenQ3foEvscgAPJYl2abspaBThDOukI=";
  };
in
{
  packages = builtins.attrValues {
    inherit rustToolchain;
    inherit (pkgs) figlet lolcat gnumake;
  };

  enterShell = ''
    figlet -f slant capataz-demo | lolcat
  '';
}
