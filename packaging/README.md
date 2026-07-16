# ForgeCode distribution

ForgeCode distributes `forge-dev` only through GitHub Releases in
[`KooshaPari/forgecode`](https://github.com/KooshaPari/forgecode). Each release
contains platform archives, `SHA256SUMS`, and the identical `install.sh` used by
the repository. Registry and package-manager manifests are intentionally not a
supported distribution surface.

Run `scripts/release-scorecard.sh --release-dir <assets> --version v2.10.0`
before publishing a release.
