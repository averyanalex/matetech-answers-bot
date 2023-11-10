{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    import-cargo.url = "github:edolstra/import-cargo";
    flake-utils.url = "github:numtide/flake-utils";
    matetech-engine = {
      url = "git+ssh://git@github.com/cpmbot/engine.git?rev=cb4a7e61170e9c78fab31060a27acdea5097b109";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    import-cargo,
    matetech-engine,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};
        rustVersion = pkgs.rust-bin.nightly.latest.default;
        inherit (import-cargo.builders) importCargo;

        cpmbot = pkgs.stdenv.mkDerivation {
          name = "cpmbot";
          src = self;

          buildInputs = with pkgs; [
            openssl
          ];

          nativeBuildInputs = [
            (importCargo { lockFile = ./Cargo.lock; inherit pkgs; }).cargoHome
            rustVersion
            pkgs.pkg-config
          ];

          buildPhase = ''
            ln -sf ${matetech-engine} matetech-engine
            cargo build --release --offline
          '';

          installPhase = ''
            install -Dm775 ./target/release/cpm_bot $out/bin/cpmbot
          '';
        };

        pgstart = pkgs.writeShellScriptBin "pgstart" ''
          if [ ! -d $PGHOST ]; then
            mkdir -p $PGHOST
          fi
          if [ ! -d $PGDATA ]; then
            echo 'Initializing postgresql database...'
            LC_ALL=C.utf8 initdb $PGDATA --auth=trust >/dev/null
          fi
          OLD_PGDATABASE=$PGDATABASE
          export PGDATABASE=postgres
          pg_ctl start -l $LOG_PATH -o "-c listen_addresses= -c unix_socket_directories=$PGHOST"
          psql -tAc "SELECT 1 FROM pg_database WHERE datname = 'cpmbot'" | grep -q 1 || psql -tAc 'CREATE DATABASE "cpmbot"'
          export PGDATABASE=$OLD_PGDATABASE
        '';

        pgstop = pkgs.writeShellScriptBin "pgstop" ''
          pg_ctl -D $PGDATA stop | true
        '';
      in {
        packages.default = cpmbot;
        devShells = {
          default = pkgs.mkShell {
            buildInputs = with pkgs;
              [
                sqlx-cli
                postgresql
                openssl
                pkg-config
              ]
              ++ [
                pgstart
                pgstop
                rustVersion
              ];

            shellHook = ''
              ln -sf ${matetech-engine} matetech-engine
              export PGDATA=$PWD/postgres/data
              export PGHOST=$PWD/postgres
              export LOG_PATH=$PWD/postgres/LOG
              export PGDATABASE=cpmbot
              export DATABASE_URL=postgresql:///cpmbot?host=$PWD/postgres;
            '';
          };
          norust = pkgs.mkShell {
            buildInputs = with pkgs;
              [
                sqlx-cli
                postgresql
                openssl
                pkg-config
              ]
              ++ [
                pgstart
                pgstop
                # rustVersion
              ];

            shellHook = ''
              ln -sf ${matetech-engine} matetech-engine
              export PGDATA=$PWD/postgres/data
              export PGHOST=$PWD/postgres
              export LOG_PATH=$PWD/postgres/LOG
              export PGDATABASE=cpmbot
              export DATABASE_URL=postgresql:///cpmbot?host=$PWD/postgres;
            '';
          };
        };
      }
    );
}
