{
  description = "CPM Bot";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs";
    poetry2nix = {
      url = "github:nix-community/poetry2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, poetry2nix }:
    (flake-utils.lib.eachDefaultSystem (system:
      let
        inherit (poetry2nix.legacyPackages.${system}) mkPoetryApplication mkPoetryEnv;
        pkgs = nixpkgs.legacyPackages.${system};
        python = pkgs.python310;

        poetryCommon = {
          projectDir = self;
          #preferWheels = true;
          python = python;
        };
        env = mkPoetryEnv poetryCommon;

        app = mkPoetryApplication poetryCommon;
      in
      {
        packages.default = app;
        devShells.default = env.env.overrideAttrs (oldAttrs: {
          buildInputs = with pkgs; [
            pandoc
            poetry
          ];
        });
      }));
}
