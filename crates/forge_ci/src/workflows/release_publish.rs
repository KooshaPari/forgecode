use crate::release_matrix::ReleaseMatrix;
use crate::workflow_model::{Event, Job, Level, Permissions, Step, Workflow};

/// Generate the fork-owned release workflow with checksums and attestations.
pub fn release_publish() {
    let matrix = serde_json::Value::from(ReleaseMatrix::default());
    let build_release = Job::new("build-release")
        .runs_on("${{ matrix.os }}")
        .permissions(Permissions::default().contents(Level::Write))
        .strategy(serde_json::json!({ "matrix": matrix }))
        .add_step(Step::new("Checkout Code").uses(
            "actions",
            "checkout",
            "d23441a48e516b6c34aea4fa41551a30e30af803",
        ))
        .add_step(
            Step::new("Setup Protobuf Compiler")
                .if_condition("${{ matrix.cross == 'false' }}")
                .uses("arduino", "setup-protoc", "c65c819552d16ad3c9b72d9dfd5ba5237b9c906b")
                .input("repo-token", "${{ secrets.GITHUB_TOKEN }}"),
        )
        .add_step(
            Step::new("Setup Cross Toolchain")
                .if_condition("${{ matrix.cross == 'false' }}")
                .uses(
                    "taiki-e",
                    "setup-cross-toolchain-action",
                    "12b7ad4acfa95a1476779d6c06699b96ec1691f8",
                )
                .input("target", "${{ matrix.target }}"),
        )
        .add_step(
            Step::new("Add Rust target")
                .if_condition("${{ matrix.cross == 'false' }}")
                .run("rustup target add ${{ matrix.target }}"),
        )
        .add_step(
            Step::new("Set Rust Flags")
                .if_condition(
                    "!(contains(matrix.target, '-unknown-linux-') || contains(matrix.target, '-android'))",
                )
                .run("echo \"RUSTFLAGS=-C target-feature=+crt-static\" >> \"$GITHUB_ENV\""),
        )
        .add_step(
            Step::new("Build Binary")
                .uses(
                    "ClementTsang",
                    "cargo-action",
                    "2438cc5f3ba4e971289fffca2a00dedea6911f14",
                )
                .input("command", "build --release")
                .input("args", "--target ${{ matrix.target }}")
                .input("use-cross", "${{ matrix.cross }}")
                .input("cross-version", "0.2.5")
                .env("RUSTFLAGS", "${{ env.RUSTFLAGS }}")
                .env("POSTHOG_API_SECRET", "${{secrets.POSTHOG_API_SECRET}}")
                .env("APP_VERSION", "${{ github.event.release.tag_name }}"),
        )
        .add_step(Step::new("Copy Binary").run("cp ${{ matrix.binary_path }} ${{ matrix.binary_name }}"))
        .add_step(
            Step::new("Generate SHA-256 checksum")
                .run("if command -v sha256sum >/dev/null 2>&1; then sha256sum \"${{ matrix.binary_name }}\" > \"${{ matrix.binary_name }}.sha256\"; else shasum -a 256 \"${{ matrix.binary_name }}\" > \"${{ matrix.binary_name }}.sha256\"; fi")
                .shell("bash"),
        )
        .add_step(
            Step::new("Upload to Release")
                .uses(
                    "xresloader",
                    "upload-to-github-release",
                    "7c5757a90c0bcf0c0e1741da8f2abd7b85e675d0",
                )
                .input("release_id", "${{ github.event.release.id }}")
                .input("file", "${{ matrix.binary_name }}")
                .input("overwrite", "true"),
        )
        .add_step(
            Step::new("Upload checksum to Release")
                .uses(
                    "xresloader",
                    "upload-to-github-release",
                    "7c5757a90c0bcf0c0e1741da8f2abd7b85e675d0",
                )
                .input("release_id", "${{ github.event.release.id }}")
                .input("file", "${{ matrix.binary_name }}.sha256")
                .input("overwrite", "true"),
        );
    let attest_release_assets = Job::new("Attest release assets")
        .needs("build_release")
        .permissions(
            Permissions::default()
                .contents(Level::Read)
                .id_token(Level::Write)
                .attestations(Level::Write),
        )
        .add_step(
            Step::new("Download release assets")
                .env("GH_TOKEN", "${{ github.token }}")
                .run(
                    r#"set -euo pipefail
mkdir -p release-assets
gh release download "${{ github.event.release.tag_name }}" \
  --repo "${{ github.repository }}" \
  --dir release-assets \
  --pattern "forge-*"
test "$(find release-assets -maxdepth 1 -type f | wc -l | tr -d ' ')" -eq 18"#,
                ),
        )
        .add_step(
            Step::new("Attest release assets")
                .uses(
                    "actions",
                    "attest-build-provenance",
                    "0f67c3f4856b2e3261c31976d6725780e5e4c373",
                )
                .input("subject-path", "release-assets/*"),
        );
    let workflow = Workflow::new("Multi Channel Release")
        .on(Event::default().release(["published"]))
        .permissions(Permissions::default().contents(Level::Read))
        .add_job("build_release", build_release)
        .add_job("attest_release_assets", attest_release_assets);

    super::generate_private_workflow(workflow, "release.yml");
}
