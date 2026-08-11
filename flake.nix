{
  description = "A todo TUI, built with ratatui — one Markdown file, no cloud, no account";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      # Linux only, and deliberately: the terminal handling is tested on Linux
      # and nowhere else. A platform in this list is a claim, not a hope.
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      # Read from the manifest rather than written twice. A version that only
      # goes stale in one place is a version that goes stale.
      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = self.packages.${system}.ratodo;

          ratodo = pkgs.rustPlatform.buildRustPackage {
            pname = "ratodo";
            inherit version;
            src = ./.;

            # The lock file rather than a `cargoHash`: the hash is a second copy
            # of what the lock already says, and it is the copy that rots.
            cargoLock.lockFile = ./Cargo.lock;

            # Hand-written, so they ship rather than being generated at install
            # time — see docs/cli.md.
            postInstall = ''
              install -Dm644 completions/ratodo.bash \
                $out/share/bash-completion/completions/ratodo
              install -Dm644 completions/ratodo.zsh \
                $out/share/zsh/site-functions/_ratodo
              install -Dm644 completions/ratodo.fish \
                $out/share/fish/vendor_completions.d/ratodo.fish
            '';

            meta = {
              description = "A todo TUI, built with ratatui — one Markdown file, no cloud, no account";
              homepage = "https://github.com/murat-akpinar/ratodo";
              changelog = "https://github.com/murat-akpinar/ratodo/blob/v${version}/CHANGELOG.md";
              license = nixpkgs.lib.licenses.gpl3Only;
              mainProgram = "ratodo";
              platforms = systems;
            };
          };
        }
      );
    };
}
