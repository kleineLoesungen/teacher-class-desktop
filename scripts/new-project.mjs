#!/usr/bin/env node
// Macht aus dem Kit ein eigenes Projekt.
//
//   node scripts/new-project.mjs notizbuch "Notizbuch"
//
// Ersetzt überall "teacher-class" durch den neuen Namen: Kistennamen, Binaries,
// Umgebungsvariablen, Datenverzeichnis, Bündelkennung, Fenstertitel,
// package.json, tauri.conf.json, Workflows, Doku.
//
// DIREKT NACH DEM KLONEN AUSFÜHREN, vor jedem `just`-Befehl. Bis dahin heißt
// der Klon noch "teacher-class" und benutzt DASSELBE Datenverzeichnis wie das Kit —
// zwei Projekte mit demselben Namen teilen sich unbemerkt eine Datenbank.
//
// In Node statt in Bash, damit es auf Windows ohne Git Bash läuft. Das nimmt
// nebenbei eine Falle weg: Bash liest ein Skript stückweise, während es läuft,
// und dieses Skript schreibt sich selbst mit um — die alte Fassung brauchte
// dafür einen Trick. Node liest die Datei vollständig, bevor es loslegt.

import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const KIT = "teacher-class";

const [slug, displayArg] = process.argv.slice(2);

if (!slug) {
  console.error(`Aufruf: node scripts/new-project.mjs <kurzname> [Anzeigename]

  kurzname     Kleinbuchstaben, Ziffern, Bindestriche. Wird zum Namen des
               Binaries, der Kisten und des Datenverzeichnisses.
  Anzeigename  Steht im Fenstertitel und unter dem Symbol.
               Voreinstellung: kurzname mit großem Anfangsbuchstaben.

Beispiel: node scripts/new-project.mjs notizbuch "Notizbuch"`);
  process.exit(2);
}

if (!/^[a-z][a-z0-9-]*$/.test(slug)) {
  console.error(
    `Fehler: '${slug}' muss mit einem Kleinbuchstaben beginnen und darf nur\n` +
      "        Kleinbuchstaben, Ziffern und Bindestriche enthalten.",
  );
  process.exit(2);
}

if (slug === KIT) {
  console.error(`Fehler: '${KIT}' ist der Name des Kits selbst.`);
  process.exit(2);
}

const underscore = slug.replaceAll("-", "_");
const upper = underscore.toUpperCase();
const display = displayArg || slug[0].toUpperCase() + slug.slice(1);

console.log(`Kurzname:     ${slug}`);
console.log(`Anzeigename:  ${display}`);
console.log(`Umgebung:     ${upper}_DATA_DIR / ${upper}_CONFIG_DIR\n`);

// Reihenfolge ist wichtig: spezifische Schreibweisen zuerst, damit das
// allgemeine "teacher-class" am Schluss nichts zerlegt, was schon ersetzt ist.
//
// Die Bündelkennung steht ganz vorn und nimmt den Kurznamen MIT Bindestrichen:
// Tauri erlaubt dort nur Buchstaben, Ziffern, "-" und "." — ein Unterstrich
// bricht `tauri build` ab.
//
// Der Verweis auf das Schwester-Kit wird geparkt und am Ende zurückgestellt;
// der Name eines fremden Repositorys bleibt, wie er ist. Der Platzhalter ist
// zusammengesetzt, damit er in dieser Datei nie wörtlich steht — sie schreibt
// sich schließlich selbst mit um.
const park = `@@SCHWESTER${"KIT@@"}`;

const ersetzungen = [
  [/vibe-starter-desktop/g, park],
  [/de\.kleineloesungen\.teacher-class/g, `de.example.${slug}`],
  [/TEACHER_CLASS_/g, `${upper}_`],
  [/teacher_class_/g, `${underscore}_`],
  [/teacher-class/g, slug],
  [/teacher-class-/g, `${slug}-`],
  [/Teacher Class/g, display],
  [/teacher-class/g, slug],
  [new RegExp(park, "g"), "vibe-starter-desktop"],
];

// Symbole sind keine Textdateien.
const BINAER = /\.(png|icns|ico|jpg|jpeg|gif|webp|woff2?|ttf)$/i;
const UEBERSPRINGEN = new Set(["target", "dist", ".git", ".local", "node_modules"]);

/** Alle verwalteten Textdateien. Ohne git: alles außer dem Gebauten. */
function dateien(dir = ROOT) {
  const gefunden = [];
  for (const name of readdirSync(dir)) {
    const voll = path.join(dir, name);
    if (UEBERSPRINGEN.has(name)) continue;
    if (statSync(voll).isDirectory()) {
      gefunden.push(...dateien(voll));
    } else if (!BINAER.test(name)) {
      gefunden.push(voll);
    }
  }
  return gefunden;
}

let geaendert = 0;
for (const datei of dateien()) {
  const vorher = readFileSync(datei, "utf8");
  let nachher = vorher;
  for (const [suche, ersatz] of ersetzungen) {
    nachher = nachher.replaceAll(suche, ersatz);
  }
  // Nur schreiben, wenn sich etwas geändert hat — sonst wird bei jedem Lauf
  // die Änderungszeit jeder Datei neu gesetzt.
  if (nachher !== vorher) {
    writeFileSync(datei, nachher);
    geaendert += 1;
  }
}

// Die Umbenennung verschiebt die alphabetische Reihenfolge der `use`-Zeilen:
// "teacher_class_core" sortiert vor "time" und "uuid", ein Name wie
// "vorlagentest_core" danach. Ohne diesen Lauf scheitert `just check` beim
// allerersten Mal an einem reinen Formatierungsunterschied — der denkbar
// schlechteste erste Eindruck für ein frisches Projekt.
const still = { cwd: ROOT, stdio: "ignore", shell: false };
if (spawnSync("cargo", ["fmt", "--all"], still).status === 0) {
  console.log(`${geaendert} Dateien geändert, Importe neu sortiert.`);
} else {
  console.log(`${geaendert} Dateien geändert.`);
  console.log("Hinweis: cargo nicht gefunden — einmal 'cargo fmt --all' nachholen,");
  console.log("         sonst meldet 'just check' Formatierungsunterschiede.");
}

// package-lock.json trägt den Namen aus package.json mit, und Biome räumt
// Zeilen auf, die durch den neuen Namen über die Formatgrenze gewachsen
// sind — dasselbe Anliegen wie `cargo fmt` oben.
//
// Beim üblichen Weg (klonen, umbenennen, dann erst `just setup`) gibt es
// node_modules noch nicht. Dafür stehen die Kistennamen in
// `scripts/tasks.mjs` in kurzen Konstanten statt mitten in langen Zeilen:
// Dann ist nach dem Umbenennen nichts umzubrechen.
if (existsSync(path.join(ROOT, "node_modules"))) {
  spawnSync("npm", ["install", "--package-lock-only"], { ...still, shell: true });
  spawnSync("npx", ["biome", "check", "--write"], { ...still, shell: true });
}

console.log("\nWeiter:");

// Ob `origin` weg muss, hängt davon ab, wie dieses Projekt entstanden ist.
// Über „Use this template" auf GitHub gehört das Repo bereits dem neuen
// Projekt — dann wäre das Entfernen falsch. Nach einem Klon des Kits zeigt
// origin noch auf das Kit.
//
// Erkannt wird das an der Anzahl der Commits: ein Template-Repo startet mit
// genau einem, ein Klon bringt die Geschichte des Kits mit. Das überlebt auch
// die Umbenennung oben, ein Vergleich der origin-URL täte es nicht.
const commits = spawnSync("git", ["rev-list", "--count", "HEAD"], {
  cwd: ROOT,
  encoding: "utf8",
});
if (Number(commits.stdout?.trim() ?? 0) > 1) {
  console.log("  git remote remove origin        # der Klon zeigt noch auf das Kit");
}

console.log("  just setup                      # npm-Pakete, einmal durchbauen");
console.log("  just check                      # muss grün sein");
console.log("  just dev");
console.log("\nassets/icon.png ist noch das Symbol des Kits — ersetzen (1024×1024)");
console.log("und danach 'just icons' laufen lassen.");
console.log("\nDamit Claude auf Issues antwortet, fehlen noch zwei Dinge:");
console.log("  1. die GitHub-App  https://github.com/apps/claude  auf diesem Repo");
console.log("  2. das Secret      CLAUDE_CODE_OAUTH_TOKEN");
console.log("Beides in docs/AUTOMATION.md beschrieben.");
