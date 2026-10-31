#!/usr/bin/env node
// Baut dist/teacher-class-cli-<version>-<os>-<arch>.tar.gz — die Kommandozeile zum
// Weitergeben, auf allen drei Systemen gleich.
//
// Warum getrennt vom Desktop-Bündel: Tauri nimmt keine zweite ausführbare
// Datei ins Bündel, ohne sie als „Sidecar" mit Zielplattform im Namen zu
// verlangen. Das wäre mehr Umweg als ein eigener Tarball.
//
// `tar` gibt es auf macOS, Linux und seit Windows 10 auch dort — ein Format
// für alle, statt zip auf der einen und tar auf der anderen Seite.

import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const NAME = "teacher-class";

function run(cmd, args, options = {}) {
  const ergebnis = spawnSync(cmd, args, { cwd: ROOT, stdio: "inherit", ...options });
  if (ergebnis.error) {
    console.error(`${cmd} ließ sich nicht starten: ${ergebnis.error.message}`);
    process.exit(1);
  }
  if (ergebnis.status !== 0) {
    process.exit(ergebnis.status ?? 1);
  }
  return ergebnis;
}

// Die Fassung steht an genau einer Stelle: [workspace.package] in Cargo.toml.
const version = readFileSync(path.join(ROOT, "Cargo.toml"), "utf8").match(
  /^version = "(.*)"$/m,
)?.[1];
if (!version) {
  console.error("Keine version in Cargo.toml gefunden.");
  process.exit(1);
}

// Nur der NAME der Ausgabedatei hängt vom System ab, nicht ihr Aufbau.
const system =
  { darwin: "macos", win32: "windows", linux: "linux" }[process.platform] ?? process.platform;
const arch = { x64: "x86_64", arm64: "aarch64" }[process.arch] ?? process.arch;
const exe = process.platform === "win32" ? ".exe" : "";

const ordner = `${NAME}-cli-${version}-${system}-${arch}`;
const stage = path.join(ROOT, "dist", ordner);

rmSync(stage, { recursive: true, force: true });
for (const unter of ["bin", "share/man/man1", "share/completions"]) {
  mkdirSync(path.join(stage, unter), { recursive: true });
}

run("cargo", ["build", "--release", "-p", `${NAME}-cli`]);

const binaer = path.join(ROOT, "target", "release", `${NAME}${exe}`);
writeFileSync(path.join(stage, "bin", `${NAME}${exe}`), readFileSync(binaer));
chmodSync(path.join(stage, "bin", `${NAME}${exe}`), 0o755);

// Handbuchseiten und Vervollständigungen erzeugt die CLI selbst — so können
// sie nicht veralten.
run(binaer, ["man", "--out", path.join(stage, "share", "man", "man1")], { stdio: "ignore" });
for (const shell of ["bash", "zsh", "fish"]) {
  const aus = run(binaer, ["completions", shell], { stdio: ["ignore", "pipe", "inherit"] });
  writeFileSync(path.join(stage, "share", "completions", `${NAME}.${shell}`), aus.stdout);
}

writeFileSync(
  path.join(stage, "install.sh"),
  `#!/usr/bin/env sh
# Installiert nach ~/.local — ohne Sonderrechte. Für macOS und Linux.
set -eu
PREFIX="\${PREFIX:-$HOME/.local}"
DIR=$(cd "$(dirname "$0")" && pwd)

mkdir -p "$PREFIX/bin" "$PREFIX/share"
cp "$DIR"/bin/* "$PREFIX/bin/"
cp -r "$DIR"/share/man "$PREFIX/share/"

echo "Installiert nach $PREFIX."
echo "Vervollständigungen liegen in $DIR/share/completions."
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "Hinweis: $PREFIX/bin liegt nicht im PATH." ;;
esac
`,
);
chmodSync(path.join(stage, "install.sh"), 0o755);

writeFileSync(
  path.join(stage, "LIESMICH.txt"),
  `${NAME} ${version} — Kommandozeile

macOS, Linux
  ./install.sh          legt alles unter ~/.local ab

Windows
  bin\\${NAME}.exe in einen Ordner kopieren, der im PATH liegt.
  Die Handbuchseiten unter share/man brauchen dort kein Werkzeug —
  "${NAME} --help" sagt dasselbe.

Vervollständigungen für bash, zsh und fish liegen in share/completions.
`,
);

run("tar", ["-czf", path.join("dist", `${ordner}.tar.gz`), "-C", "dist", ordner]);
rmSync(stage, { recursive: true, force: true });
console.log(`Fertig: dist/${ordner}.tar.gz`);
