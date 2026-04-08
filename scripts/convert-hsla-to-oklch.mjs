#!/usr/bin/env node
/**
 * convert-hsla-to-oklch.mjs
 *
 * Converts Rust source files from Zed's HSLA color system to Raijin's OKLCH system.
 * Uses culori (from .reference/culori-main) for mathematically exact color conversion.
 *
 * Usage:
 *   node scripts/convert-hsla-to-oklch.mjs <input-file>           # prints to stdout
 *   node scripts/convert-hsla-to-oklch.mjs <input-file> --inplace # overwrites file
 *
 * What it does:
 *   1. Renames type `Hsla` → `Oklch`
 *   2. Renames `use gpui::` → `use inazuma::`
 *   3. Converts `Hsla { h: ..., s: ..., l: ..., a: ... }` struct literals to `Oklch { l, c, h, a }`
 *   4. Converts `hsla(h, s, l, a)` function calls to `oklch(l, c, h)` or `oklcha(l, c, h, a)`
 *
 * Leaves hsla() calls with non-literal/variable args untouched (they still work
 * since inazuma's hsla() returns Oklch).
 */

import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

// Import culori from the reference source
const __dirname = dirname(fileURLToPath(import.meta.url));
const culoriPath = resolve(__dirname, '..', '.reference', 'culori-main', 'src', 'index.js');
const { converter } = await import(culoriPath);

const toOklch = converter('oklch');

// ── Color conversion ─────────────────────────────────────────────────

function hslToOklch(h_zed, s, l, a) {
  // Zed convention: h is 0–1 (fraction of 360°). culori expects degrees.
  const h_deg = h_zed <= 1.0 ? h_zed * 360 : h_zed;
  const ok = toOklch({ mode: 'hsl', h: h_deg, s, l });
  return { l: ok.l ?? 0, c: ok.c ?? 0, h: ok.h ?? 0, a };
}

function fmt(v) {
  if (Math.abs(v) < 0.00005) return '0.0';
  let s = v.toFixed(4).replace(/0+$/, '');
  if (s.endsWith('.')) s += '0';
  return s;
}

function toOklchStruct(ok) {
  return `Oklch { l: ${fmt(ok.l)}, c: ${fmt(ok.c)}, h: ${fmt(ok.h)}, a: ${fmt(ok.a)} }`;
}

function toOklchCall(ok) {
  if (Math.abs(ok.a - 1.0) < 0.0001) {
    return `oklch(${fmt(ok.l)}, ${fmt(ok.c)}, ${fmt(ok.h)})`;
  }
  return `oklcha(${fmt(ok.l)}, ${fmt(ok.c)}, ${fmt(ok.h)}, ${fmt(ok.a)})`;
}

/** Try to evaluate a simple numeric expression like "225. / 360." */
function tryEval(expr) {
  const trimmed = expr.trim();
  // Only allow digits, dots, spaces, and basic arithmetic
  if (!/^[\d.\s+\-*/()_]+$/.test(trimmed)) return NaN;
  try {
    return eval(trimmed.replace(/_/g, ''));
  } catch {
    return NaN;
  }
}

// ── Main transform ───────────────────────────────────────────────────

function transform(input) {
  let output = input;

  // 1. Type rename: Hsla → Oklch
  output = output.replace(/\bHsla\b/g, 'Oklch');

  // 2. gpui → inazuma
  output = output.replace(/\bgpui::/g, 'inazuma::');
  output = output.replace(/use gpui\b/g, 'use inazuma');

  // 3. Multiline struct literals: Oklch { h: ..., s: ..., l: ..., a: ... }
  //    (these were Hsla structs, type was already renamed in step 1)
  output = output.replace(
    /Oklch\s*\{[^}]*?h:\s*([^,]+?)\s*,[\s\S]*?s:\s*([^,]+?)\s*,[\s\S]*?l:\s*([^,]+?)\s*,[\s\S]*?a:\s*([^,}]+?)\s*,?\s*\}/gs,
    (match, hExpr, sExpr, lExpr, aExpr) => {
      const h = tryEval(hExpr);
      const s = tryEval(sExpr);
      const l = tryEval(lExpr);
      const a = tryEval(aExpr);
      if ([h, s, l, a].some(isNaN)) return match;
      return toOklchStruct(hslToOklch(h, s, l, a));
    }
  );

  // 4. hsla() function calls with literal-evaluable args
  //    Matches: hsla(225. / 360., 12. / 100., 17. / 100., 1.)
  output = output.replace(
    /\bhsla\(\s*([\s\S]+?)\s*,\s*([\s\S]+?)\s*,\s*([\s\S]+?)\s*,\s*([\s\S]+?)\s*,?\s*\)/g,
    (match, hExpr, sExpr, lExpr, aExpr) => {
      const h = tryEval(hExpr);
      const s = tryEval(sExpr);
      const l = tryEval(lExpr);
      const a = tryEval(aExpr);
      if ([h, s, l, a].some(isNaN)) return match; // variable args → leave as hsla()
      return toOklchCall(hslToOklch(h, s, l, a));
    }
  );

  return output;
}

// ── CLI ──────────────────────────────────────────────────────────────

const args = process.argv.slice(2);
const inplace = args.includes('--inplace');
const filePath = args.find(a => !a.startsWith('-'));

if (!filePath) {
  console.error('Usage: node scripts/convert-hsla-to-oklch.mjs <file> [--inplace]');
  process.exit(1);
}

const input = readFileSync(filePath, 'utf8');
const output = transform(input);

if (inplace) {
  writeFileSync(filePath, output, 'utf8');
  const remaining = (output.match(/\bHsla\b/g) || []).length;
  const hslaLeft = (output.match(/\bhsla\(/g) || []).length;
  console.log(`✓ ${filePath} (Hsla: ${remaining}, hsla(): ${hslaLeft})`);
} else {
  process.stdout.write(output);
}
