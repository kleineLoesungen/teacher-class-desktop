#!/usr/bin/env node
// Die kleinen wiederkehrenden Handgriffe — in Node statt in der Shell.
//
// Warum nicht in Bash: `just` ruft seine Rezepte über eine Shell auf, und die
// ist auf Windows nicht `sh`. Ein Rezept mit `rm -rf`, `VAR=x befehl` oder
// `read` liefe dort nicht. Node ist in diesem Kit ohnehin Pflicht (npm, Vite,
// Biome) — damit ist dieselbe Datei auf allen drei Systemen dieselbe.
//
// Alles, was ein schlichter Befehl ist, steht weiterhin direkt im Justfile.
// Hierher kommt nur, was ohne Shell-Eigenheiten nicht auskommt.

import { spawnSync } from "node:child_process";
import { readdirSync, rmSync } from "node:fs";
import path from "node:path";
import { createInterface } from "node:readline/promises";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const LOCAL = path.join(ROOT, ".local");

// npm und npx sind auf Windows Batch-Dateien, die Node nur über die Shell
// starten darf. cargo ist überall ein echtes Programm — und wird deshalb ohne
// Shell gestartet, damit Argumente mit Leerzeichen heil ankommen.
const BRAUCHT_SHELL = new Set(["npm", "npx"]);

function run(cmd, args, env = {}) {
  const ergebnis = spawnSync(cmd, args, {
    cwd: ROOT,
    stdio: "inherit",
    shell: BRAUCHT_SHELL.has(cmd),
    env: { ...process.env, ...env },
  });
  if (ergebnis.error) {
    console.error(`${cmd} ließ sich nicht starten: ${ergebnis.error.message}`);
    process.exit(1);
  }
  if (ergebnis.status !== 0) {
    process.exit(ergebnis.status ?? 1);
  }
}

// Die Kistennamen stehen hier als Konstanten, nicht in jeder Zeile.
// Das ist nicht Kosmetik: `scripts/new-project.mjs` ersetzt „starter"
// überall durch den neuen Namen. Steckt der Name mitten in einer langen
// Zeile, wächst die beim Umbenennen über die Formatgrenze von Biome — und
// ein frisches Projekt meldet beim allerersten `just check` einen
// Formatfehler. Kurze Zeilen hier können das nicht.
const CLI = "starter-cli";
const DESKTOP = "starter-desktop";

// Das Wegwerfverzeichnis. Absolute Pfade, weil `tauri dev` die Anwendung aus
// crates/desktop heraus startet — ein relativer Pfad landete dort.
const sandbox = {
  STARTER_DATA_DIR: path.join(LOCAL, "data"),
  STARTER_CONFIG_DIR: path.join(LOCAL, "config"),
};

const aufgaben = {
  "dev-sandbox": () => run("npm", ["run", "tauri", "dev"], sandbox),

  sandbox: (args) => run("cargo", ["run", "-q", "-p", CLI, "--", ...args], sandbox),

  bindings: () => {
    run("cargo", ["test", "-q", "-p", DESKTOP, "--test", "bindings"], {
      STARTER_UPDATE_BINDINGS: "1",
    });
    console.log("ui/src/bindings.ts geschrieben. Weiter mit:  npm run build:ui");
  },

  icons: () => {
    run("npx", ["tauri", "icon", "assets/icon.png", "-o", "crates/desktop/icons"]);
    // `tauri icon` erzeugt auch Symbole für Android, iOS und den Microsoft
    // Store. Dieses Kit liefert nichts davon aus; die Dateien lägen nur
    // unbenutzt im Verwaltungssystem.
    const unnoetig = /^(android|ios|Square.*\.png|StoreLogo\.png|64x64\.png)$/;
    const verzeichnis = path.join(ROOT, "crates", "desktop", "icons");
    for (const name of readdirSync(verzeichnis)) {
      if (unnoetig.test(name)) {
        rmSync(path.join(verzeichnis, name), { recursive: true, force: true });
      }
    }
    console.log("Symbole erzeugt. icon.ico braucht Windows, icon.icns macOS.");
  },

  clean: () => {
    run("cargo", ["clean"]);
    for (const dir of ["dist", ".local", path.join("ui", "dist")]) {
      rmSync(path.join(ROOT, dir), { recursive: true, force: true });
    }
    console.log("dist, .local und ui/dist entfernt.");
  },

  // ACHTUNG: löscht die echten Daten der Anwendung.
  "data-reset": async () => {
    const pfade = spawnSync("cargo", ["run", "-q", "-p", CLI, "--", "paths", "--json"], {
      cwd: ROOT,
      encoding: "utf8",
    });
    if (pfade.status !== 0) {
      process.stderr.write(pfade.stderr ?? "");
      process.exit(pfade.status ?? 1);
    }
    const { data_dir } = JSON.parse(pfade.stdout);

    console.log(`Löscht das Datenverzeichnis:\n  ${data_dir}`);
    const frage = createInterface({ input: process.stdin, output: process.stdout });
    const antwort = await frage.question("Wirklich? [j/N] ");
    frage.close();

    if (antwort.trim().toLowerCase() !== "j") {
      console.log("Abgebrochen.");
      return;
    }
    rmSync(data_dir, { recursive: true, force: true });
    console.log("Gelöscht.");
  },
};

const [name, ...args] = process.argv.slice(2);
const aufgabe = aufgaben[name];

if (!aufgabe) {
  const was = name ? `Unbekannte Aufgabe ${JSON.stringify(name)}.` : "Keine Aufgabe genannt.";
  console.error(`${was} Bekannt sind:`);
  for (const bekannt of Object.keys(aufgaben)) {
    console.error(`  ${bekannt}`);
  }
  process.exit(2);
}

await aufgabe(args);
