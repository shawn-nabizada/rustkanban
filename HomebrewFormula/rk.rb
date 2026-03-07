class Rk < Formula
  desc "A terminal (TUI) kanban board with vim-inspired navigation"
  homepage "https://github.com/shawn-nabizada/rustkanban"
  version "0.1.0"
  license "BSL-1.1"

  on_macos do
    on_arm do
      url "https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-macos-aarch64"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-macos-x86_64"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-linux-aarch64"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-linux-x86_64"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install stable.url.split("/").last => "rk"
  end

  test do
    assert_match "kanban", shell_output("#{bin}/rk --help")
  end
end
