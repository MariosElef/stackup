# Homebrew formula, for a tap (a repository named homebrew-stackup):
#   brew tap MariosElef/stackup && brew install stackup
# Before publishing a version, fill in the sha256 of that tag's source tarball:
#   curl -fsSL https://github.com/MariosElef/stackup/archive/refs/tags/vX.Y.Z.tar.gz | shasum -a 256
class Stackup < Formula
  desc "Scaffold a full-stack project from the official tools, then run it"
  homepage "https://github.com/MariosElef/stackup"
  url "https://github.com/MariosElef/stackup/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_THE_SHA256_OF_THE_TARBALL"
  license "MIT"
  head "https://github.com/MariosElef/stackup.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/stackup --version")
  end
end
