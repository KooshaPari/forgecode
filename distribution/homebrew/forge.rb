class Forge < Formula
  desc "Fastest AI coding agent — 40+ models, local-first, zero data exfiltration"
  homepage "https://github.com/KooshaPari/forgecode"
  version "2.13.21-h.0.1.6"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/KooshaPari/forgecode/releases/download/v#{version}/forge-x86_64-apple-darwin"
      sha256 "c23d506cbda6872736af266232d22caedfba3a249ed3a6e6b2c45a8c66a00135"
    else
      url "https://github.com/KooshaPari/forgecode/releases/download/v#{version}/forge-aarch64-apple-darwin"
      sha256 "6270a110cee53354a1722462612dc0efa9fd77be90118a0d5d88559696ca9937"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/KooshaPari/forgecode/releases/download/v#{version}/forge-x86_64-unknown-linux-gnu"
      sha256 "f4fd414750ba55ced92911755a65deea25cdf3ec0c428319a4b3e8d39fc978ff"
    else
      url "https://github.com/KooshaPari/forgecode/releases/download/v#{version}/forge-aarch64-unknown-linux-gnu"
      sha256 "e1bbe2fbabc21218197d4ae4b6d93a996a8b864430464a059b1bdc02328ab9cd"
    end
  end

  def install
    bin.install Dir["forge*"].first => "forge"
    bin.install Dir["forge_dbd*"].first => "forge_dbd" if Dir["forge_dbd*"].any?
  end

  test do
    assert_match "forge", shell_output("#{bin}/forge --version")
  end
end
