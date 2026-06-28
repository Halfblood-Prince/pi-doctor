class PiDoctor < Formula
  desc "Human-first Raspberry Pi diagnostics"
  homepage "https://github.com/Halfblood-Prince/pi-doctor"
  version "0.1.0"
  license "Apache-2.0"

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/Halfblood-Prince/pi-doctor/releases/download/v0.1.0/pi-doctor-v0.1.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SIGNED_RELEASE_SHA256"
    elsif Hardware::CPU.intel?
      url "https://github.com/Halfblood-Prince/pi-doctor/releases/download/v0.1.0/pi-doctor-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SIGNED_RELEASE_SHA256"
    end
  end

  def install
    bin.install "pi-doctor"
    bash_completion.install "completions/pi-doctor.bash" => "pi-doctor"
    zsh_completion.install "completions/_pi-doctor"
    fish_completion.install "completions/pi-doctor.fish"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/pi-doctor --version")
    system "#{bin}/pi-doctor", "support-bundle", "--dry-run"
  end
end
