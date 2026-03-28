#!/usr/bin/env node
/**
 * Convert withfig/autocomplete specs to Raijin JSON format.
 *
 * Reads compiled Fig specs from node_modules/@withfig/autocomplete/build/
 * and outputs Raijin-compatible JSON to ../../crates/raijin-completions/specs/
 *
 * Usage:
 *   npm install
 *   node convert.mjs
 *
 * Limitations:
 *   - Dynamic generators (JS functions that run shell commands) are dropped.
 *     Only recognizable patterns (git branch, git tag, git remote) are mapped
 *     to ArgTemplate values. All other generators produce no template.
 *   TODO: Implement generator pattern recognition for more tools (docker ps, etc.)
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { join, basename } from "node:path";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const BUILD_DIR = join(
  __dirname,
  "node_modules/@withfig/autocomplete/build"
);
const SPECS_DIR = join(__dirname, "../../crates/raijin-completions/specs");
const EXTERNAL_DIR = join(SPECS_DIR, "external");

// Top 50 commands to embed as Tier 1 (compiled into the binary)
const TIER1 = new Set([
  "git", "cargo", "docker", "npm", "yarn", "pnpm", "node", "python", "pip",
  "brew", "apt", "pacman", "kubectl", "aws", "gcloud", "ssh", "scp", "rsync",
  "curl", "wget", "tar", "zip", "unzip", "make", "cmake", "go", "rustup",
  "gh", "hub", "terraform", "ansible", "vagrant", "heroku", "fly", "vercel",
  "netlify", "prisma", "jest", "vitest", "eslint", "prettier", "tsc", "deno",
  "bun", "rg", "fd", "bat", "eza", "tmux", "screen", "ruby", "gem", "pip3",
  "python3", "dotnet", "mvn", "gradle", "helm", "systemctl", "journalctl",
  "chmod", "chown", "ln", "find", "grep", "sed", "awk", "xargs", "less",
  "cat", "head", "tail", "wc", "sort", "uniq", "diff", "patch", "man",
]);

/**
 * Convert a Fig spec object to Raijin CliSpec JSON.
 */
function convertSpec(figSpec) {
  if (!figSpec || typeof figSpec !== "object") return null;

  const name = Array.isArray(figSpec.name) ? figSpec.name[0] : figSpec.name;
  if (!name || typeof name !== "string") return null;

  const aliases = Array.isArray(figSpec.name)
    ? figSpec.name.slice(1).filter((n) => typeof n === "string")
    : [];

  const raijin = {
    name,
    aliases,
    description: figSpec.description || null,
    subcommands: [],
    options: [],
    args: [],
  };

  // Convert subcommands (recursive)
  if (Array.isArray(figSpec.subcommands)) {
    for (const sub of figSpec.subcommands) {
      const converted = convertSpec(sub);
      if (converted) {
        raijin.subcommands.push(converted);
      }
    }
  }

  // Convert options
  if (Array.isArray(figSpec.options)) {
    for (const opt of figSpec.options) {
      const converted = convertOption(opt);
      if (converted) {
        raijin.options.push(converted);
      }
    }
  }

  // Convert positional args
  if (Array.isArray(figSpec.args)) {
    for (const arg of figSpec.args) {
      const converted = convertArg(arg);
      if (converted) {
        raijin.args.push(converted);
      }
    }
  } else if (figSpec.args && typeof figSpec.args === "object") {
    const converted = convertArg(figSpec.args);
    if (converted) {
      raijin.args.push(converted);
    }
  }

  return raijin;
}

/**
 * Convert a Fig option to Raijin CliOption.
 */
function convertOption(figOpt) {
  if (!figOpt) return null;

  const names = Array.isArray(figOpt.name)
    ? figOpt.name.filter((n) => typeof n === "string")
    : typeof figOpt.name === "string"
      ? [figOpt.name]
      : [];

  if (names.length === 0) return null;

  const raijin = {
    names,
    description: figOpt.description || null,
    takes_arg: false,
    arg_name: null,
    arg_template: null,
    is_repeatable: !!figOpt.isRepeatable,
  };

  // Check if option takes an argument
  if (figOpt.args) {
    const args = Array.isArray(figOpt.args) ? figOpt.args : [figOpt.args];
    if (args.length > 0 && args[0]) {
      raijin.takes_arg = true;
      raijin.arg_name = args[0].name || null;
      raijin.arg_template = resolveTemplate(args[0]);
    }
  }

  return raijin;
}

/**
 * Convert a Fig arg to Raijin CliArg.
 */
function convertArg(figArg) {
  if (!figArg) return null;

  return {
    name: figArg.name || "arg",
    description: figArg.description || null,
    template: resolveTemplate(figArg),
    is_optional: !!figArg.isOptional,
    is_variadic: !!figArg.isVariadic,
  };
}

/**
 * Resolve a Fig template/generator/suggestions to a Raijin ArgTemplate.
 */
function resolveTemplate(figArg) {
  if (!figArg) return null;

  // Direct template
  if (figArg.template) {
    const tpl = Array.isArray(figArg.template)
      ? figArg.template[0]
      : figArg.template;
    if (tpl === "filepaths") return "filepaths";
    if (tpl === "folders") return "folders";
    if (tpl === "history") return "history";
  }

  // Static suggestions
  if (Array.isArray(figArg.suggestions)) {
    const values = figArg.suggestions
      .flatMap((s) => {
        if (typeof s === "string") return [s];
        if (Array.isArray(s)) return s.filter((v) => typeof v === "string");
        if (s?.name) {
          if (Array.isArray(s.name)) return s.name.filter((v) => typeof v === "string");
          if (typeof s.name === "string") return [s.name];
        }
        return [];
      })
      .filter(Boolean);
    if (values.length > 0) {
      return { custom: values };
    }
  }

  // Generator heuristics (try to recognize common patterns)
  if (figArg.generators) {
    const gens = Array.isArray(figArg.generators)
      ? figArg.generators
      : [figArg.generators];
    for (const gen of gens) {
      const template = recognizeGenerator(gen);
      if (template) return template;
    }
  }

  return null;
}

/**
 * Try to recognize a Fig generator pattern and map to ArgTemplate.
 * TODO: Add more patterns (docker ps, kubectl get, etc.)
 */
function recognizeGenerator(gen) {
  if (!gen) return null;

  // Check script/command string for known patterns
  const script =
    typeof gen.script === "string"
      ? gen.script
      : Array.isArray(gen.script)
        ? gen.script.join(" ")
        : typeof gen.custom === "function"
          ? gen.custom.toString()
          : "";

  if (!script) return null;

  if (/git\s+branch/.test(script) || /branch\s+--list/.test(script)) {
    return "git_branches";
  }
  if (/git\s+tag/.test(script)) {
    return "git_tags";
  }
  if (/git\s+remote/.test(script)) {
    return "git_remotes";
  }
  if (/git\s+(diff|ls-files|ls-tree)/.test(script)) {
    return "git_files";
  }
  if (/\benv\b|\bprintenv\b/.test(script)) {
    return "env_vars";
  }
  if (/\bps\b|\bpgrep\b/.test(script)) {
    return "process_ids";
  }

  return null;
}

/**
 * Load a compiled Fig spec from the build directory.
 * Fig specs are compiled JS that export { default: <spec> }.
 */
async function loadFigSpec(filePath) {
  try {
    const module = await import(filePath);
    return module.default || module;
  } catch {
    return null;
  }
}

async function main() {
  console.log("Fig-to-Raijin Spec Converter");
  console.log("============================\n");

  if (!existsSync(BUILD_DIR)) {
    console.error(
      `Build directory not found: ${BUILD_DIR}\n` +
        "Run: npm install"
    );
    process.exit(1);
  }

  // Ensure output directories exist
  await mkdir(SPECS_DIR, { recursive: true });
  await mkdir(EXTERNAL_DIR, { recursive: true });

  // Read all spec files
  const files = (await readdir(BUILD_DIR)).filter(
    (f) => f.endsWith(".js") && !f.startsWith("_") && !f.startsWith(".")
  );

  console.log(`Found ${files.length} Fig specs\n`);

  let tier1Count = 0;
  let tier2Count = 0;
  let failCount = 0;

  for (const file of files) {
    const specName = basename(file, ".js");
    const filePath = join(BUILD_DIR, file);

    try {
      const figSpec = await loadFigSpec(filePath);
      if (!figSpec) {
        failCount++;
        continue;
      }

      const raijinSpec = convertSpec(figSpec);
      if (!raijinSpec) {
        failCount++;
        continue;
      }

      const json = JSON.stringify(raijinSpec, null, 2);
      const isTier1 = TIER1.has(specName);
      const outDir = isTier1 ? SPECS_DIR : EXTERNAL_DIR;
      const outPath = join(outDir, `${specName}.json`);

      await writeFile(outPath, json + "\n");

      if (isTier1) {
        tier1Count++;
      } else {
        tier2Count++;
      }
    } catch (err) {
      failCount++;
    }
  }

  console.log(`\nResults:`);
  console.log(`  Tier 1 (embedded):  ${tier1Count} specs`);
  console.log(`  Tier 2 (external):  ${tier2Count} specs`);
  console.log(`  Failed:             ${failCount} specs`);
  console.log(`  Total converted:    ${tier1Count + tier2Count} specs`);
}

main().catch(console.error);
